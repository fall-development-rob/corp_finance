//! Collateral management analytics for repo and securities financing.
//!
//! Covers:
//! 1. **Haircut calculation** -- risk-based haircuts by collateral type, credit
//!    rating, maturity, volatility, liquidity, and cross-currency adjustment.
//! 2. **Margin call analysis** -- current margin ratio, excess/deficit, LTV,
//!    margin call triggering and amount to restore to initial margin.
//! 3. **Rehypothecation analysis** -- max rehypothecable amount, funding benefit,
//!    collateral velocity, counterparty exposure, regulatory limit checks.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate, Years};
use crate::CorpFinanceResult;

use super::repo_rates::CollateralType;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Credit rating for haircut lookup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CreditRating {
    AAA,
    AA,
    A,
    BBB,
    BB,
    B,
    CCC,
}

impl std::fmt::Display for CreditRating {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreditRating::AAA => write!(f, "AAA"),
            CreditRating::AA => write!(f, "AA"),
            CreditRating::A => write!(f, "A"),
            CreditRating::BBB => write!(f, "BBB"),
            CreditRating::BB => write!(f, "BB"),
            CreditRating::B => write!(f, "B"),
            CreditRating::CCC => write!(f, "CCC"),
        }
    }
}

// ---------------------------------------------------------------------------
// A) Haircut Calculation
// ---------------------------------------------------------------------------

/// Input for risk-based haircut calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaircutInput {
    /// Type of collateral.
    pub collateral_type: CollateralType,
    /// Credit rating of the issuer.
    pub credit_rating: CreditRating,
    /// Remaining maturity in years.
    pub remaining_maturity: Years,
    /// Historical price volatility (annualized, decimal).
    pub price_volatility: Rate,
    /// Market liquidity score (0 = illiquid, 1 = perfectly liquid).
    pub market_liquidity_score: Decimal,
    /// Whether the collateral currency differs from the loan currency.
    pub is_cross_currency: bool,
    /// Collateral market value (for eligible value calculation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collateral_value: Option<Money>,
}

/// Output of the haircut calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaircutOutput {
    /// Base haircut from type + rating lookup table.
    pub base_haircut: Decimal,
    /// Maturity adjustment (longer maturity -> higher haircut).
    pub maturity_adjustment: Decimal,
    /// Volatility adjustment (higher vol -> higher haircut).
    pub volatility_adjustment: Decimal,
    /// Liquidity adjustment (less liquid -> higher haircut).
    pub liquidity_adjustment: Decimal,
    /// Cross-currency haircut add-on.
    pub fx_adjustment: Decimal,
    /// Total haircut = sum of all adjustments, capped at 0.99.
    pub total_haircut: Decimal,
    /// Eligible value = collateral * (1 - total_haircut), if value provided.
    pub eligible_value: Option<Money>,
}

/// Calculate risk-based haircut.
pub fn calculate_haircut(input: &HaircutInput) -> CorpFinanceResult<HaircutOutput> {
    validate_haircut_input(input)?;

    // Base haircut from lookup table
    let base_haircut = base_haircut_lookup(&input.collateral_type, &input.credit_rating);

    // Maturity adjustment: +0.5% per year beyond 1 year, capped at 10%
    let maturity_adjustment = if input.remaining_maturity > Decimal::ONE {
        let extra_years = input.remaining_maturity - Decimal::ONE;
        (extra_years * dec!(0.005)).min(dec!(0.10))
    } else {
        Decimal::ZERO
    };

    // Volatility adjustment: additional haircut proportional to vol above 5%
    let vol_threshold = dec!(0.05);
    let volatility_adjustment = if input.price_volatility > vol_threshold {
        // Scale: each 1% vol above 5% adds 0.5% haircut
        let excess_vol = input.price_volatility - vol_threshold;
        (excess_vol * dec!(0.5)).min(dec!(0.15))
    } else {
        Decimal::ZERO
    };

    // Liquidity adjustment: lower liquidity -> higher haircut
    // score=1 (liquid) -> 0, score=0 (illiquid) -> 10%
    let liquidity_adjustment = (Decimal::ONE - input.market_liquidity_score) * dec!(0.10);

    // FX adjustment: +8% for cross-currency
    let fx_adjustment = if input.is_cross_currency {
        dec!(0.08)
    } else {
        Decimal::ZERO
    };

    // Total haircut, capped at 99%
    let raw_total = base_haircut
        + maturity_adjustment
        + volatility_adjustment
        + liquidity_adjustment
        + fx_adjustment;
    let total_haircut = raw_total.min(dec!(0.99));

    // Eligible value
    let eligible_value = input
        .collateral_value
        .map(|cv| cv * (Decimal::ONE - total_haircut));

    Ok(HaircutOutput {
        base_haircut,
        maturity_adjustment,
        volatility_adjustment,
        liquidity_adjustment,
        fx_adjustment,
        total_haircut,
        eligible_value,
    })
}

/// Base haircut lookup table by collateral type and credit rating.
///
/// | Type       | AAA   | AA    | A     | BBB   | BB    | B     | CCC   |
/// |------------|-------|-------|-------|-------|-------|-------|-------|
/// | Treasury   | 0.005 | 0.01  | 0.015 | 0.02  | 0.05  | 0.08  | 0.15  |
/// | Agency     | 0.01  | 0.015 | 0.02  | 0.03  | 0.06  | 0.10  | 0.18  |
/// | Corporate  | 0.02  | 0.03  | 0.04  | 0.06  | 0.10  | 0.15  | 0.25  |
/// | Equity     | 0.10  | 0.12  | 0.15  | 0.18  | 0.25  | 0.30  | 0.40  |
fn base_haircut_lookup(ctype: &CollateralType, rating: &CreditRating) -> Decimal {
    match (ctype, rating) {
        // Treasury
        (CollateralType::Treasury, CreditRating::AAA) => dec!(0.005),
        (CollateralType::Treasury, CreditRating::AA) => dec!(0.01),
        (CollateralType::Treasury, CreditRating::A) => dec!(0.015),
        (CollateralType::Treasury, CreditRating::BBB) => dec!(0.02),
        (CollateralType::Treasury, CreditRating::BB) => dec!(0.05),
        (CollateralType::Treasury, CreditRating::B) => dec!(0.08),
        (CollateralType::Treasury, CreditRating::CCC) => dec!(0.15),
        // Agency
        (CollateralType::Agency, CreditRating::AAA) => dec!(0.01),
        (CollateralType::Agency, CreditRating::AA) => dec!(0.015),
        (CollateralType::Agency, CreditRating::A) => dec!(0.02),
        (CollateralType::Agency, CreditRating::BBB) => dec!(0.03),
        (CollateralType::Agency, CreditRating::BB) => dec!(0.06),
        (CollateralType::Agency, CreditRating::B) => dec!(0.10),
        (CollateralType::Agency, CreditRating::CCC) => dec!(0.18),
        // Corporate
        (CollateralType::Corporate, CreditRating::AAA) => dec!(0.02),
        (CollateralType::Corporate, CreditRating::AA) => dec!(0.03),
        (CollateralType::Corporate, CreditRating::A) => dec!(0.04),
        (CollateralType::Corporate, CreditRating::BBB) => dec!(0.06),
        (CollateralType::Corporate, CreditRating::BB) => dec!(0.10),
        (CollateralType::Corporate, CreditRating::B) => dec!(0.15),
        (CollateralType::Corporate, CreditRating::CCC) => dec!(0.25),
        // Equity
        (CollateralType::Equity, CreditRating::AAA) => dec!(0.10),
        (CollateralType::Equity, CreditRating::AA) => dec!(0.12),
        (CollateralType::Equity, CreditRating::A) => dec!(0.15),
        (CollateralType::Equity, CreditRating::BBB) => dec!(0.18),
        (CollateralType::Equity, CreditRating::BB) => dec!(0.25),
        (CollateralType::Equity, CreditRating::B) => dec!(0.30),
        (CollateralType::Equity, CreditRating::CCC) => dec!(0.40),
    }
}

fn validate_haircut_input(input: &HaircutInput) -> CorpFinanceResult<()> {
    if input.remaining_maturity < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_maturity".into(),
            reason: "Remaining maturity cannot be negative.".into(),
        });
    }
    if input.price_volatility < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "price_volatility".into(),
            reason: "Price volatility cannot be negative.".into(),
        });
    }
    if input.market_liquidity_score < Decimal::ZERO || input.market_liquidity_score > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_liquidity_score".into(),
            reason: "Liquidity score must be in [0, 1].".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// B) Margin Call Analysis
// ---------------------------------------------------------------------------

/// Input for margin call analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginCallInput {
    /// Initial collateral market value at trade inception.
    pub initial_collateral_value: Money,
    /// Current collateral market value.
    pub current_collateral_value: Money,
    /// Loan amount (cash lent against collateral).
    pub loan_amount: Money,
    /// Initial margin percentage (e.g. 1.05 = 105%).
    pub initial_margin_pct: Decimal,
    /// Maintenance margin percentage (e.g. 1.02 = 102%).
    pub maintenance_margin_pct: Decimal,
    /// Variation margin percentage (threshold for incremental calls).
    pub variation_margin_pct: Decimal,
    /// Haircut applied to collateral.
    pub haircut_pct: Decimal,
}

/// Output of the margin call analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginCallOutput {
    /// Current margin ratio = (collateral * (1 - haircut)) / loan.
    pub current_margin_ratio: Decimal,
    /// Margin excess or deficit (positive = excess, negative = deficit) in dollars.
    pub margin_excess_deficit: Money,
    /// Whether a margin call is triggered.
    pub margin_call_triggered: bool,
    /// Margin call amount to restore to initial margin (zero if no call).
    pub margin_call_amount: Money,
    /// Loan-to-value ratio = loan / collateral.
    pub ltv_ratio: Decimal,
    /// Collateral coverage ratio = collateral / loan.
    pub collateral_coverage_ratio: Decimal,
}

/// Analyse margin call status.
pub fn analyze_margin_call(input: &MarginCallInput) -> CorpFinanceResult<MarginCallOutput> {
    validate_margin_call_input(input)?;

    let adjusted_collateral = input.current_collateral_value * (Decimal::ONE - input.haircut_pct);

    // Current margin ratio
    let current_margin_ratio = if input.loan_amount.is_zero() {
        Decimal::ZERO
    } else {
        adjusted_collateral / input.loan_amount
    };

    // Maintenance margin level in dollars
    let maintenance_level = input.loan_amount * input.maintenance_margin_pct;
    let margin_excess_deficit = adjusted_collateral - maintenance_level;

    // Margin call triggered?
    let margin_call_triggered = current_margin_ratio < input.maintenance_margin_pct;

    // Amount to restore to initial margin
    let margin_call_amount = if margin_call_triggered {
        let initial_level = input.loan_amount * input.initial_margin_pct;
        let deficit = initial_level - adjusted_collateral;
        deficit.max(Decimal::ZERO)
    } else {
        Decimal::ZERO
    };

    // LTV = loan / collateral
    let ltv_ratio = if input.current_collateral_value.is_zero() {
        Decimal::ZERO
    } else {
        input.loan_amount / input.current_collateral_value
    };

    // Coverage = collateral / loan
    let collateral_coverage_ratio = if input.loan_amount.is_zero() {
        Decimal::ZERO
    } else {
        input.current_collateral_value / input.loan_amount
    };

    Ok(MarginCallOutput {
        current_margin_ratio,
        margin_excess_deficit,
        margin_call_triggered,
        margin_call_amount,
        ltv_ratio,
        collateral_coverage_ratio,
    })
}

fn validate_margin_call_input(input: &MarginCallInput) -> CorpFinanceResult<()> {
    if input.loan_amount < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "loan_amount".into(),
            reason: "Loan amount cannot be negative.".into(),
        });
    }
    if input.current_collateral_value < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_collateral_value".into(),
            reason: "Collateral value cannot be negative.".into(),
        });
    }
    if input.haircut_pct < Decimal::ZERO || input.haircut_pct >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "haircut_pct".into(),
            reason: "Haircut must be in [0, 1).".into(),
        });
    }
    if input.maintenance_margin_pct <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maintenance_margin_pct".into(),
            reason: "Maintenance margin must be positive.".into(),
        });
    }
    if input.initial_margin_pct < input.maintenance_margin_pct {
        return Err(CorpFinanceError::InvalidInput {
            field: "initial_margin_pct".into(),
            reason: "Initial margin must be >= maintenance margin.".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// C) Rehypothecation Analysis
// ---------------------------------------------------------------------------

/// Input for rehypothecation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RehypothecationInput {
    /// Total collateral received from counterparties.
    pub total_collateral_received: Money,
    /// Maximum percentage of collateral that can be rehypothecated (0-1).
    pub rehypothecation_limit_pct: Decimal,
    /// Rate at which collateral is reused (0-1).
    pub collateral_reuse_rate: Rate,
    /// Funding rate saved by reusing collateral.
    pub funding_rate: Rate,
    /// Term in calendar days.
    pub term_days: u32,
    /// Day-count basis denominator.
    pub day_count_basis: u32,
    /// Number of reuse chains (times collateral is re-pledged).
    pub num_reuse_chains: u32,
    /// Counterparty risk cost per unit of rehypothecated collateral (bps).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterparty_risk_cost_bps: Option<Decimal>,
    /// Regulatory framework for limit checks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regulatory_framework: Option<String>,
}

/// Output of the rehypothecation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RehypothecationOutput {
    /// Maximum amount that can be rehypothecated.
    pub max_rehypothecable: Money,
    /// Funding benefit from reusing collateral.
    pub funding_benefit: Money,
    /// Collateral velocity (effective reuse multiplier).
    pub collateral_velocity: Decimal,
    /// Counterparty exposure from rehypothecation chains.
    pub counterparty_exposure: Money,
    /// Net benefit = funding_benefit - counterparty_risk_cost.
    pub net_benefit: Money,
    /// Regulatory limit compliance check.
    pub regulatory_limit_check: RegulatoryLimitCheck,
}

/// Regulatory limit compliance result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryLimitCheck {
    /// Whether the rehypothecation is within regulatory limits.
    pub compliant: bool,
    /// Applicable regulatory limit (e.g. 140% for Dodd-Frank).
    pub applicable_limit_pct: Decimal,
    /// Actual rehypothecation percentage.
    pub actual_pct: Decimal,
    /// Description of the applicable rule.
    pub rule_description: String,
}

/// Analyse rehypothecation economics and compliance.
pub fn analyze_rehypothecation(
    input: &RehypothecationInput,
) -> CorpFinanceResult<RehypothecationOutput> {
    validate_rehypothecation_input(input)?;

    let days = Decimal::from(input.term_days);
    let basis = Decimal::from(input.day_count_basis);
    let time_frac = days / basis;

    // Max rehypothecable amount
    let max_rehypothecable = input.total_collateral_received * input.rehypothecation_limit_pct;

    // Actually rehypothecated amount
    let actually_reused = max_rehypothecable * input.collateral_reuse_rate;

    // Funding benefit: interest saved by reusing collateral
    let funding_benefit = actually_reused * input.funding_rate * time_frac;

    // Collateral velocity: geometric series sum for reuse chains
    // velocity = 1 + reuse_rate + reuse_rate^2 + ... = 1/(1-reuse_rate) for infinite
    // For finite chains: sum = (1 - r^n) / (1 - r) where r = reuse_rate, n = chains
    let velocity = if input.collateral_reuse_rate >= Decimal::ONE {
        Decimal::from(input.num_reuse_chains + 1)
    } else if input.collateral_reuse_rate.is_zero() {
        Decimal::ONE
    } else {
        let r = input.collateral_reuse_rate;
        let n = input.num_reuse_chains;
        // (1 - r^n) / (1 - r)
        let mut r_n = Decimal::ONE;
        for _ in 0..n {
            r_n *= r;
        }
        (Decimal::ONE - r_n) / (Decimal::ONE - r)
    };

    let collateral_velocity = velocity;

    // Counterparty exposure: each link in the chain creates exposure
    let counterparty_exposure = actually_reused * Decimal::from(input.num_reuse_chains);

    // Counterparty risk cost
    let bps_divisor = dec!(10000);
    let risk_cost = input
        .counterparty_risk_cost_bps
        .map(|bps| counterparty_exposure * (bps / bps_divisor) * time_frac)
        .unwrap_or(Decimal::ZERO);

    // Net benefit
    let net_benefit = funding_benefit - risk_cost;

    // Regulatory limit check
    let regulatory_limit_check = check_regulatory_limits(input, max_rehypothecable);

    Ok(RehypothecationOutput {
        max_rehypothecable,
        funding_benefit,
        collateral_velocity,
        counterparty_exposure,
        net_benefit,
        regulatory_limit_check,
    })
}

/// Check regulatory limits (Dodd-Frank 140% limit, EMIR segregation).
fn check_regulatory_limits(
    input: &RehypothecationInput,
    max_rehypothecable: Money,
) -> RegulatoryLimitCheck {
    let framework = input
        .regulatory_framework
        .as_deref()
        .unwrap_or("Dodd-Frank");

    let (limit_pct, rule_desc) = match framework {
        "EMIR" => (
            dec!(0.0),
            "EMIR requires full segregation of initial margin -- no rehypothecation permitted."
                .to_string(),
        ),
        _ => (
            dec!(1.40),
            "Dodd-Frank / SEC Rule 15c3-3: rehypothecation limited to 140% of customer debit balances."
                .to_string(),
        ),
    };

    let actual_pct = if input.total_collateral_received.is_zero() {
        Decimal::ZERO
    } else {
        max_rehypothecable / input.total_collateral_received
    };

    let compliant = actual_pct <= limit_pct || limit_pct.is_zero() && actual_pct.is_zero();

    RegulatoryLimitCheck {
        compliant,
        applicable_limit_pct: limit_pct,
        actual_pct,
        rule_description: rule_desc,
    }
}

fn validate_rehypothecation_input(input: &RehypothecationInput) -> CorpFinanceResult<()> {
    if input.total_collateral_received < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_collateral_received".into(),
            reason: "Total collateral cannot be negative.".into(),
        });
    }
    if input.rehypothecation_limit_pct < Decimal::ZERO
        || input.rehypothecation_limit_pct > Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "rehypothecation_limit_pct".into(),
            reason: "Rehypothecation limit must be in [0, 1].".into(),
        });
    }
    if input.collateral_reuse_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "collateral_reuse_rate".into(),
            reason: "Reuse rate cannot be negative.".into(),
        });
    }
    if input.term_days == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "term_days".into(),
            reason: "Term must be at least 1 day.".into(),
        });
    }
    if input.day_count_basis == 0 {
        return Err(CorpFinanceError::DivisionByZero {
            context: "day_count_basis cannot be zero".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Wrapper: CollateralInput / CollateralOutput
// ---------------------------------------------------------------------------

/// Enum selecting the collateral model to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollateralModel {
    Haircut(HaircutInput),
    MarginCall(MarginCallInput),
    Rehypothecation(RehypothecationInput),
}

/// Top-level collateral analytics input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralInput {
    pub model: CollateralModel,
}

/// Top-level collateral analytics output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CollateralOutput {
    Haircut(HaircutOutput),
    MarginCall(MarginCallOutput),
    Rehypothecation(RehypothecationOutput),
}

/// Unified entry point for all collateral analytics.
pub fn analyze_collateral(
    input: &CollateralInput,
) -> CorpFinanceResult<ComputationOutput<CollateralOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    let (result, methodology) = match &input.model {
        CollateralModel::Haircut(ref hi) => {
            let out = calculate_haircut(hi)?;
            if out.total_haircut > dec!(0.50) {
                warnings.push("Total haircut exceeds 50% -- collateral quality is low.".into());
            }
            (
                CollateralOutput::Haircut(out),
                "Risk-Based Haircut Calculation",
            )
        }
        CollateralModel::MarginCall(ref mi) => {
            let out = analyze_margin_call(mi)?;
            if out.margin_call_triggered {
                warnings.push(format!(
                    "Margin call triggered. Call amount: {}.",
                    out.margin_call_amount
                ));
            }
            if out.ltv_ratio > dec!(0.90) {
                warnings.push("LTV exceeds 90% -- high default risk.".into());
            }
            (CollateralOutput::MarginCall(out), "Margin Call Analysis")
        }
        CollateralModel::Rehypothecation(ref ri) => {
            let out = analyze_rehypothecation(ri)?;
            if !out.regulatory_limit_check.compliant {
                warnings.push(format!(
                    "Rehypothecation exceeds regulatory limit: {}",
                    out.regulatory_limit_check.rule_description
                ));
            }
            if out.collateral_velocity > dec!(3) {
                warnings.push("Collateral velocity > 3x -- high systemic risk.".into());
            }
            (
                CollateralOutput::Rehypothecation(out),
                "Rehypothecation Analysis",
            )
        }
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        methodology,
        &serde_json::json!({
            "model": methodology,
            "framework": "Basel III / Dodd-Frank / EMIR"
        }),
        warnings,
        elapsed,
        result,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -- Haircut tests -----------------------------------------------------

    #[test]
    fn test_treasury_haircut_lt_corporate() {
        let treasury = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AAA,
            remaining_maturity: dec!(5),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.95),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        let corporate = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Corporate,
            credit_rating: CreditRating::AAA,
            remaining_maturity: dec!(5),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.95),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        assert!(
            treasury.total_haircut < corporate.total_haircut,
            "Treasury haircut {} should be less than Corporate haircut {}",
            treasury.total_haircut,
            corporate.total_haircut
        );
    }

    #[test]
    fn test_corporate_haircut_lt_equity() {
        let corp = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Corporate,
            credit_rating: CreditRating::A,
            remaining_maturity: dec!(3),
            price_volatility: dec!(0.04),
            market_liquidity_score: dec!(0.80),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        let equity = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Equity,
            credit_rating: CreditRating::A,
            remaining_maturity: dec!(3),
            price_volatility: dec!(0.04),
            market_liquidity_score: dec!(0.80),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        assert!(
            corp.total_haircut < equity.total_haircut,
            "Corporate haircut {} should be less than Equity haircut {}",
            corp.total_haircut,
            equity.total_haircut
        );
    }

    #[test]
    fn test_longer_maturity_higher_haircut() {
        let short = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AA,
            remaining_maturity: dec!(0.5),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.90),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        let long = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AA,
            remaining_maturity: dec!(10),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.90),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        assert!(long.total_haircut > short.total_haircut);
        assert!(long.maturity_adjustment > short.maturity_adjustment);
    }

    #[test]
    fn test_lower_rating_higher_haircut() {
        let aaa = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Corporate,
            credit_rating: CreditRating::AAA,
            remaining_maturity: dec!(5),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.80),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        let bbb = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Corporate,
            credit_rating: CreditRating::BBB,
            remaining_maturity: dec!(5),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.80),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        assert!(bbb.base_haircut > aaa.base_haircut);
        assert!(bbb.total_haircut > aaa.total_haircut);
    }

    #[test]
    fn test_cross_currency_adds_8pct() {
        let domestic = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AAA,
            remaining_maturity: dec!(2),
            price_volatility: dec!(0.02),
            market_liquidity_score: dec!(1.0),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        let cross_ccy = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AAA,
            remaining_maturity: dec!(2),
            price_volatility: dec!(0.02),
            market_liquidity_score: dec!(1.0),
            is_cross_currency: true,
            collateral_value: None,
        })
        .unwrap();

        assert_eq!(cross_ccy.fx_adjustment, dec!(0.08));
        assert_eq!(cross_ccy.total_haircut - domestic.total_haircut, dec!(0.08));
    }

    #[test]
    fn test_higher_vol_higher_haircut() {
        let low_vol = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Corporate,
            credit_rating: CreditRating::A,
            remaining_maturity: dec!(3),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.80),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        let high_vol = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Corporate,
            credit_rating: CreditRating::A,
            remaining_maturity: dec!(3),
            price_volatility: dec!(0.15),
            market_liquidity_score: dec!(0.80),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        assert!(high_vol.volatility_adjustment > low_vol.volatility_adjustment);
        assert!(high_vol.total_haircut > low_vol.total_haircut);
    }

    #[test]
    fn test_less_liquid_higher_haircut() {
        let liquid = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Corporate,
            credit_rating: CreditRating::A,
            remaining_maturity: dec!(3),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(1.0),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        let illiquid = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Corporate,
            credit_rating: CreditRating::A,
            remaining_maturity: dec!(3),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.2),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        assert!(illiquid.liquidity_adjustment > liquid.liquidity_adjustment);
        assert!(illiquid.total_haircut > liquid.total_haircut);
    }

    #[test]
    fn test_eligible_value_calculation() {
        let out = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AAA,
            remaining_maturity: dec!(0.5),
            price_volatility: dec!(0.02),
            market_liquidity_score: dec!(1.0),
            is_cross_currency: false,
            collateral_value: Some(dec!(1_000_000)),
        })
        .unwrap();

        let expected = dec!(1_000_000) * (Decimal::ONE - out.total_haircut);
        assert_eq!(out.eligible_value, Some(expected));
    }

    #[test]
    fn test_haircut_capped_at_99pct() {
        // CCC equity with max adjustments
        let out = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Equity,
            credit_rating: CreditRating::CCC,
            remaining_maturity: dec!(30),
            price_volatility: dec!(0.50),
            market_liquidity_score: dec!(0.0),
            is_cross_currency: true,
            collateral_value: None,
        })
        .unwrap();

        assert!(out.total_haircut <= dec!(0.99));
    }

    #[test]
    fn test_zero_maturity_no_maturity_adjustment() {
        let out = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AA,
            remaining_maturity: dec!(0.5),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.90),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        assert_eq!(out.maturity_adjustment, Decimal::ZERO);
    }

    #[test]
    fn test_low_vol_no_vol_adjustment() {
        let out = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AA,
            remaining_maturity: dec!(2),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.90),
            is_cross_currency: false,
            collateral_value: None,
        })
        .unwrap();

        assert_eq!(out.volatility_adjustment, Decimal::ZERO);
    }

    #[test]
    fn test_haircut_validation_negative_maturity() {
        let result = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AA,
            remaining_maturity: dec!(-1),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(0.90),
            is_cross_currency: false,
            collateral_value: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_haircut_validation_liquidity_out_of_range() {
        let result = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AA,
            remaining_maturity: dec!(2),
            price_volatility: dec!(0.03),
            market_liquidity_score: dec!(1.5),
            is_cross_currency: false,
            collateral_value: None,
        });
        assert!(result.is_err());
    }

    // -- Margin Call tests -------------------------------------------------

    #[test]
    fn test_margin_call_triggered_when_below_maintenance() {
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_050_000),
            current_collateral_value: dec!(900_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.02),
        })
        .unwrap();

        // adjusted = 900k * 0.98 = 882k, ratio = 882k/1M = 0.882 < 1.02
        assert!(out.margin_call_triggered);
    }

    #[test]
    fn test_no_margin_call_when_above_maintenance() {
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_100_000),
            current_collateral_value: dec!(1_100_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.02),
        })
        .unwrap();

        // adjusted = 1.1M * 0.98 = 1.078M, ratio = 1.078 > 1.02
        assert!(!out.margin_call_triggered);
        assert_eq!(out.margin_call_amount, Decimal::ZERO);
    }

    #[test]
    fn test_margin_call_restores_to_initial_margin() {
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_050_000),
            current_collateral_value: dec!(800_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        })
        .unwrap();

        // adjusted = 800k, ratio = 0.8 < 1.02 => triggered
        // call amount = 1.05M - 800k = 250k
        assert!(out.margin_call_triggered);
        assert_eq!(out.margin_call_amount, dec!(250_000));
    }

    #[test]
    fn test_ltv_calculation() {
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_200_000),
            current_collateral_value: dec!(1_200_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        })
        .unwrap();

        // LTV = 1M / 1.2M = 0.8333...
        let expected_ltv = dec!(1_000_000) / dec!(1_200_000);
        assert_eq!(out.ltv_ratio, expected_ltv);
    }

    #[test]
    fn test_ltv_inversely_related_to_margin() {
        let high_collateral = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(2_000_000),
            current_collateral_value: dec!(2_000_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        })
        .unwrap();

        let low_collateral = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_100_000),
            current_collateral_value: dec!(1_100_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        })
        .unwrap();

        // Higher collateral => higher margin ratio, lower LTV
        assert!(high_collateral.current_margin_ratio > low_collateral.current_margin_ratio);
        assert!(high_collateral.ltv_ratio < low_collateral.ltv_ratio);
    }

    #[test]
    fn test_collateral_coverage_ratio() {
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_500_000),
            current_collateral_value: dec!(1_500_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        })
        .unwrap();

        assert_eq!(out.collateral_coverage_ratio, dec!(1.5));
    }

    #[test]
    fn test_fully_collateralized_no_margin_call() {
        // 100% margin: collateral = loan
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_050_000),
            current_collateral_value: dec!(1_050_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.0),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        })
        .unwrap();

        assert!(!out.margin_call_triggered);
    }

    #[test]
    fn test_margin_excess_positive_when_healthy() {
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_200_000),
            current_collateral_value: dec!(1_200_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        })
        .unwrap();

        // adjusted = 1.2M, maintenance_level = 1M * 1.02 = 1.02M
        // excess = 1.2M - 1.02M = 180k
        assert!(out.margin_excess_deficit > Decimal::ZERO);
        assert_eq!(out.margin_excess_deficit, dec!(180_000));
    }

    #[test]
    fn test_margin_validation_negative_loan() {
        let result = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_000_000),
            current_collateral_value: dec!(1_000_000),
            loan_amount: dec!(-100_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_margin_validation_initial_lt_maintenance() {
        let result = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_000_000),
            current_collateral_value: dec!(1_000_000),
            loan_amount: dec!(900_000),
            initial_margin_pct: dec!(1.00),
            maintenance_margin_pct: dec!(1.05),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_zero_haircut_margin() {
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_050_000),
            current_collateral_value: dec!(1_050_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.0),
        })
        .unwrap();

        // adjusted = 1.05M, ratio = 1.05
        assert_eq!(out.current_margin_ratio, dec!(1.05));
    }

    // -- Rehypothecation tests ---------------------------------------------

    #[test]
    fn test_max_rehypothecable_equals_total_times_limit() {
        let out = analyze_rehypothecation(&RehypothecationInput {
            total_collateral_received: dec!(10_000_000),
            rehypothecation_limit_pct: dec!(0.80),
            collateral_reuse_rate: dec!(0.50),
            funding_rate: dec!(0.04),
            term_days: 90,
            day_count_basis: 360,
            num_reuse_chains: 3,
            counterparty_risk_cost_bps: None,
            regulatory_framework: None,
        })
        .unwrap();

        assert_eq!(out.max_rehypothecable, dec!(8_000_000));
    }

    #[test]
    fn test_funding_benefit_positive() {
        let out = analyze_rehypothecation(&RehypothecationInput {
            total_collateral_received: dec!(10_000_000),
            rehypothecation_limit_pct: dec!(0.80),
            collateral_reuse_rate: dec!(0.50),
            funding_rate: dec!(0.04),
            term_days: 90,
            day_count_basis: 360,
            num_reuse_chains: 3,
            counterparty_risk_cost_bps: None,
            regulatory_framework: None,
        })
        .unwrap();

        // actually_reused = 8M * 0.5 = 4M, benefit = 4M * 0.04 * 90/360 = 40k
        assert_eq!(out.funding_benefit, dec!(40_000));
    }

    #[test]
    fn test_velocity_with_zero_reuse_rate() {
        let out = analyze_rehypothecation(&RehypothecationInput {
            total_collateral_received: dec!(1_000_000),
            rehypothecation_limit_pct: dec!(0.50),
            collateral_reuse_rate: Decimal::ZERO,
            funding_rate: dec!(0.04),
            term_days: 90,
            day_count_basis: 360,
            num_reuse_chains: 3,
            counterparty_risk_cost_bps: None,
            regulatory_framework: None,
        })
        .unwrap();

        assert_eq!(out.collateral_velocity, Decimal::ONE);
    }

    #[test]
    fn test_velocity_le_theoretical_max() {
        // Theoretical max for infinite chain: 1/(1-r)
        let r = dec!(0.70);
        let theoretical_max = Decimal::ONE / (Decimal::ONE - r);

        let out = analyze_rehypothecation(&RehypothecationInput {
            total_collateral_received: dec!(1_000_000),
            rehypothecation_limit_pct: dec!(1.0),
            collateral_reuse_rate: r,
            funding_rate: dec!(0.04),
            term_days: 90,
            day_count_basis: 360,
            num_reuse_chains: 5,
            counterparty_risk_cost_bps: None,
            regulatory_framework: None,
        })
        .unwrap();

        assert!(
            out.collateral_velocity <= theoretical_max,
            "Velocity {} should be <= theoretical max {}",
            out.collateral_velocity,
            theoretical_max
        );
    }

    #[test]
    fn test_net_benefit_with_risk_cost() {
        let out = analyze_rehypothecation(&RehypothecationInput {
            total_collateral_received: dec!(10_000_000),
            rehypothecation_limit_pct: dec!(0.80),
            collateral_reuse_rate: dec!(0.50),
            funding_rate: dec!(0.04),
            term_days: 360,
            day_count_basis: 360,
            num_reuse_chains: 3,
            counterparty_risk_cost_bps: Some(dec!(50)),
            regulatory_framework: None,
        })
        .unwrap();

        assert!(out.net_benefit < out.funding_benefit);
    }

    #[test]
    fn test_emir_no_rehypothecation() {
        let out = analyze_rehypothecation(&RehypothecationInput {
            total_collateral_received: dec!(10_000_000),
            rehypothecation_limit_pct: dec!(0.0),
            collateral_reuse_rate: dec!(0.50),
            funding_rate: dec!(0.04),
            term_days: 90,
            day_count_basis: 360,
            num_reuse_chains: 3,
            counterparty_risk_cost_bps: None,
            regulatory_framework: Some("EMIR".into()),
        })
        .unwrap();

        assert_eq!(out.max_rehypothecable, Decimal::ZERO);
        assert_eq!(out.funding_benefit, Decimal::ZERO);
    }

    #[test]
    fn test_rehypothecation_validation_negative_collateral() {
        let result = analyze_rehypothecation(&RehypothecationInput {
            total_collateral_received: dec!(-1_000_000),
            rehypothecation_limit_pct: dec!(0.80),
            collateral_reuse_rate: dec!(0.50),
            funding_rate: dec!(0.04),
            term_days: 90,
            day_count_basis: 360,
            num_reuse_chains: 3,
            counterparty_risk_cost_bps: None,
            regulatory_framework: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_rehypothecation_validation_zero_term() {
        let result = analyze_rehypothecation(&RehypothecationInput {
            total_collateral_received: dec!(1_000_000),
            rehypothecation_limit_pct: dec!(0.80),
            collateral_reuse_rate: dec!(0.50),
            funding_rate: dec!(0.04),
            term_days: 0,
            day_count_basis: 360,
            num_reuse_chains: 3,
            counterparty_risk_cost_bps: None,
            regulatory_framework: None,
        });
        assert!(result.is_err());
    }

    // -- Wrapper tests -----------------------------------------------------

    #[test]
    fn test_analyze_collateral_haircut_model() {
        let input = CollateralInput {
            model: CollateralModel::Haircut(HaircutInput {
                collateral_type: CollateralType::Treasury,
                credit_rating: CreditRating::AAA,
                remaining_maturity: dec!(2),
                price_volatility: dec!(0.03),
                market_liquidity_score: dec!(0.95),
                is_cross_currency: false,
                collateral_value: Some(dec!(1_000_000)),
            }),
        };
        let out = analyze_collateral(&input).unwrap();
        assert!(out.methodology.contains("Haircut"));
    }

    #[test]
    fn test_analyze_collateral_margin_call_model() {
        let input = CollateralInput {
            model: CollateralModel::MarginCall(MarginCallInput {
                initial_collateral_value: dec!(1_050_000),
                current_collateral_value: dec!(1_050_000),
                loan_amount: dec!(1_000_000),
                initial_margin_pct: dec!(1.05),
                maintenance_margin_pct: dec!(1.02),
                variation_margin_pct: dec!(0.01),
                haircut_pct: dec!(0.0),
            }),
        };
        let out = analyze_collateral(&input).unwrap();
        assert!(out.methodology.contains("Margin Call"));
    }

    #[test]
    fn test_analyze_collateral_rehypothecation_model() {
        let input = CollateralInput {
            model: CollateralModel::Rehypothecation(RehypothecationInput {
                total_collateral_received: dec!(10_000_000),
                rehypothecation_limit_pct: dec!(0.80),
                collateral_reuse_rate: dec!(0.50),
                funding_rate: dec!(0.04),
                term_days: 90,
                day_count_basis: 360,
                num_reuse_chains: 3,
                counterparty_risk_cost_bps: None,
                regulatory_framework: None,
            }),
        };
        let out = analyze_collateral(&input).unwrap();
        assert!(out.methodology.contains("Rehypothecation"));
    }

    #[test]
    fn test_wrapper_margin_call_warning() {
        let input = CollateralInput {
            model: CollateralModel::MarginCall(MarginCallInput {
                initial_collateral_value: dec!(1_050_000),
                current_collateral_value: dec!(800_000),
                loan_amount: dec!(1_000_000),
                initial_margin_pct: dec!(1.05),
                maintenance_margin_pct: dec!(1.02),
                variation_margin_pct: dec!(0.01),
                haircut_pct: dec!(0.0),
            }),
        };
        let out = analyze_collateral(&input).unwrap();
        assert!(
            out.warnings.iter().any(|w| w.contains("Margin call")),
            "Should warn about margin call"
        );
    }

    #[test]
    fn test_wrapper_high_haircut_warning() {
        let input = CollateralInput {
            model: CollateralModel::Haircut(HaircutInput {
                collateral_type: CollateralType::Equity,
                credit_rating: CreditRating::CCC,
                remaining_maturity: dec!(20),
                price_volatility: dec!(0.30),
                market_liquidity_score: dec!(0.1),
                is_cross_currency: true,
                collateral_value: None,
            }),
        };
        let out = analyze_collateral(&input).unwrap();
        assert!(out.warnings.iter().any(|w| w.contains("50%")));
    }

    #[test]
    fn test_wrapper_metadata_present() {
        let input = CollateralInput {
            model: CollateralModel::Haircut(HaircutInput {
                collateral_type: CollateralType::Treasury,
                credit_rating: CreditRating::AAA,
                remaining_maturity: dec!(2),
                price_volatility: dec!(0.03),
                market_liquidity_score: dec!(0.95),
                is_cross_currency: false,
                collateral_value: None,
            }),
        };
        let out = analyze_collateral(&input).unwrap();
        assert!(!out.metadata.version.is_empty());
        assert_eq!(out.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_serialization_roundtrip_haircut() {
        let out = calculate_haircut(&HaircutInput {
            collateral_type: CollateralType::Treasury,
            credit_rating: CreditRating::AA,
            remaining_maturity: dec!(5),
            price_volatility: dec!(0.08),
            market_liquidity_score: dec!(0.80),
            is_cross_currency: false,
            collateral_value: Some(dec!(1_000_000)),
        })
        .unwrap();

        let json = serde_json::to_string(&out).unwrap();
        let _: HaircutOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_serialization_roundtrip_margin() {
        let out = analyze_margin_call(&MarginCallInput {
            initial_collateral_value: dec!(1_050_000),
            current_collateral_value: dec!(1_050_000),
            loan_amount: dec!(1_000_000),
            initial_margin_pct: dec!(1.05),
            maintenance_margin_pct: dec!(1.02),
            variation_margin_pct: dec!(0.01),
            haircut_pct: dec!(0.02),
        })
        .unwrap();

        let json = serde_json::to_string(&out).unwrap();
        let _: MarginCallOutput = serde_json::from_str(&json).unwrap();
    }
}
