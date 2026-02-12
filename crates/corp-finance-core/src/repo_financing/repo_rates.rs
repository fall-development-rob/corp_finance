//! Institutional-grade repo rate analytics.
//!
//! Covers:
//! 1. **Repo rate calculation** -- purchase/repurchase prices, repo interest,
//!    effective rate, implied financing cost including margin.
//! 2. **Implied repo rate** -- back-out the financing rate from spot/forward
//!    bond prices, carry analysis.
//! 3. **Repo term structure** -- interpolated repo curve at standard tenors,
//!    implied forward repo rates, overnight-vs-term spreads, specialness.
//! 4. **Securities lending economics** -- fee income, cash collateral
//!    reinvestment, total return, intrinsic value.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Collateral type determines haircuts and specialness premiums.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CollateralType {
    Treasury,
    Agency,
    Corporate,
    Equity,
}

impl std::fmt::Display for CollateralType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollateralType::Treasury => write!(f, "Treasury"),
            CollateralType::Agency => write!(f, "Agency"),
            CollateralType::Corporate => write!(f, "Corporate"),
            CollateralType::Equity => write!(f, "Equity"),
        }
    }
}

// ---------------------------------------------------------------------------
// A) Repo Rate Calculation
// ---------------------------------------------------------------------------

/// Input for repo rate calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRateInput {
    /// Market value of the collateral security.
    pub collateral_value: Money,
    /// Repo rate (annualized, decimal: 0.05 = 5%).
    pub repo_rate: Rate,
    /// Term of the repo in calendar days.
    pub term_days: u32,
    /// Day-count basis denominator (e.g. 360 or 365).
    pub day_count_basis: u32,
    /// Haircut percentage (decimal: 0.02 = 2%).
    pub haircut_pct: Decimal,
    /// Initial margin requirement (decimal: 1.02 = 102%).
    pub initial_margin: Decimal,
    /// Accrued interest on the collateral security.
    pub accrued_interest: Money,
    /// Optional funding cost for net interest margin calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub funding_cost: Option<Rate>,
}

/// Output of the repo rate calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRateOutput {
    /// Purchase price = collateral_value * (1 - haircut).
    pub purchase_price: Money,
    /// Repurchase price = purchase_price * (1 + repo_rate * term/basis).
    pub repurchase_price: Money,
    /// Repo interest = repurchase_price - purchase_price.
    pub repo_interest: Money,
    /// Effective annualized rate.
    pub effective_rate: Rate,
    /// Implied financing cost including margin.
    pub implied_financing_cost: Rate,
    /// Net interest margin: repo_rate - funding_cost.
    pub net_interest_margin: Option<Rate>,
    /// Forward price = collateral * (1 + r*t/basis) - accrued.
    pub forward_price: Money,
}

/// Compute repo rate analytics.
pub fn calculate_repo_rate(input: &RepoRateInput) -> CorpFinanceResult<RepoRateOutput> {
    validate_repo_rate_input(input)?;

    let days = Decimal::from(input.term_days);
    let basis = Decimal::from(input.day_count_basis);
    let time_frac = days / basis;

    // Purchase price: collateral * (1 - haircut)
    let purchase_price = input.collateral_value * (Decimal::ONE - input.haircut_pct);

    // Repurchase price: purchase_price * (1 + repo_rate * t)
    let repurchase_price = purchase_price * (Decimal::ONE + input.repo_rate * time_frac);

    // Repo interest
    let repo_interest = repurchase_price - purchase_price;

    // Effective rate: annualized from actual interest
    let effective_rate = if purchase_price.is_zero() || days.is_zero() {
        Decimal::ZERO
    } else {
        (repo_interest / purchase_price) * (basis / days)
    };

    // Implied financing cost including margin
    let implied_financing_cost = if input.initial_margin.is_zero() {
        effective_rate
    } else {
        effective_rate * input.initial_margin
    };

    // Net interest margin
    let net_interest_margin = input.funding_cost.map(|fc| input.repo_rate - fc);

    // Forward price
    let forward_price = input.collateral_value * (Decimal::ONE + input.repo_rate * time_frac)
        - input.accrued_interest;

    Ok(RepoRateOutput {
        purchase_price,
        repurchase_price,
        repo_interest,
        effective_rate,
        implied_financing_cost,
        net_interest_margin,
        forward_price,
    })
}

fn validate_repo_rate_input(input: &RepoRateInput) -> CorpFinanceResult<()> {
    if input.collateral_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "collateral_value".into(),
            reason: "Collateral value must be positive.".into(),
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
    if input.haircut_pct < Decimal::ZERO || input.haircut_pct >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "haircut_pct".into(),
            reason: "Haircut must be in [0, 1).".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// B) Implied Repo Rate
// ---------------------------------------------------------------------------

/// Input for implied repo rate calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedRepoInput {
    /// Clean price of the bond in spot market.
    pub spot_clean_price: Money,
    /// Clean price of the bond in the forward/futures market.
    pub forward_clean_price: Money,
    /// Accrued interest at spot settlement.
    pub spot_accrued: Money,
    /// Accrued interest at forward settlement.
    pub forward_accrued: Money,
    /// Coupon income received during the holding period.
    pub coupon_income: Money,
    /// Term in calendar days.
    pub term_days: u32,
    /// Day-count basis denominator.
    pub day_count_basis: u32,
}

/// Output of the implied repo rate calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedRepoOutput {
    /// Implied repo rate (annualized).
    pub implied_repo_rate: Rate,
    /// Carry = coupon_income - financing_cost.
    pub carry: Money,
    /// Basis net of carry = forward - spot - carry.
    pub basis_net_of_carry: Money,
    /// Whether the trade has positive carry.
    pub is_positive_carry: bool,
    /// Financing cost = spot_dirty * implied_rate * t.
    pub financing_cost: Money,
}

/// Calculate the implied repo rate from spot and forward prices.
pub fn calculate_implied_repo(input: &ImpliedRepoInput) -> CorpFinanceResult<ImpliedRepoOutput> {
    validate_implied_repo_input(input)?;

    let days = Decimal::from(input.term_days);
    let basis = Decimal::from(input.day_count_basis);
    let time_frac = days / basis;

    let spot_dirty = input.spot_clean_price + input.spot_accrued;
    let forward_dirty = input.forward_clean_price + input.forward_accrued;

    // implied_repo = (forward_dirty + coupon_income - spot_dirty) / (spot_dirty * t)
    let numerator = forward_dirty + input.coupon_income - spot_dirty;
    let denominator = spot_dirty * time_frac;

    let implied_repo_rate = if denominator.is_zero() {
        Decimal::ZERO
    } else {
        numerator / denominator
    };

    // Financing cost = spot_dirty * implied_rate * time_frac
    let financing_cost = spot_dirty * implied_repo_rate * time_frac;

    // Carry = coupon_income - financing_cost
    let carry = input.coupon_income - financing_cost;

    // Basis net of carry
    let basis_net_of_carry = (forward_clean_diff(input)) - carry;

    let is_positive_carry = carry > Decimal::ZERO;

    Ok(ImpliedRepoOutput {
        implied_repo_rate,
        carry,
        basis_net_of_carry,
        is_positive_carry,
        financing_cost,
    })
}

fn forward_clean_diff(input: &ImpliedRepoInput) -> Money {
    input.forward_clean_price - input.spot_clean_price
}

fn validate_implied_repo_input(input: &ImpliedRepoInput) -> CorpFinanceResult<()> {
    if input.spot_clean_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_clean_price".into(),
            reason: "Spot clean price must be positive.".into(),
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
// C) Repo Term Structure
// ---------------------------------------------------------------------------

/// A single point on the repo term structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoTermPoint {
    /// Term in calendar days.
    pub term_days: u32,
    /// Repo rate at this tenor.
    pub rate: Rate,
}

/// Input for repo term structure analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoTermInput {
    /// Overnight (1-day) repo rate.
    pub overnight_rate: Rate,
    /// Observed term repo rates at various tenors.
    pub term_rates: Vec<RepoTermPoint>,
    /// Collateral type (affects specialness premium).
    pub collateral_type: CollateralType,
    /// General collateral (GC) rate for specialness calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gc_rate: Option<Rate>,
}

/// A point on the interpolated curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpolatedPoint {
    /// Tenor label (e.g. "1D", "1W").
    pub tenor_label: String,
    /// Term in calendar days.
    pub term_days: u32,
    /// Interpolated rate.
    pub rate: Rate,
}

/// Forward repo rate between two tenors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRepoRate {
    /// Start tenor label.
    pub start_tenor: String,
    /// End tenor label.
    pub end_tenor: String,
    /// Start day.
    pub start_days: u32,
    /// End day.
    pub end_days: u32,
    /// Implied forward rate.
    pub forward_rate: Rate,
}

/// Overnight vs. term spread at each tenor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermSpread {
    /// Tenor label.
    pub tenor_label: String,
    /// Term days.
    pub term_days: u32,
    /// Spread = term_rate - overnight_rate.
    pub spread: Rate,
}

/// Output of the repo term structure analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoTermOutput {
    /// Interpolated curve at standard tenors.
    pub interpolated_curve: Vec<InterpolatedPoint>,
    /// Implied forward repo rates between consecutive tenors.
    pub forward_repo_rates: Vec<ForwardRepoRate>,
    /// Overnight vs. term spread at each standard tenor.
    pub overnight_vs_term_spread: Vec<TermSpread>,
    /// Specialness premium (GC rate - collateral rate) if applicable.
    pub specialness_premium: Option<Rate>,
}

/// Standard tenors: 1D, 1W, 2W, 1M, 3M, 6M, 1Y.
const STANDARD_TENORS: [(u32, &str); 7] = [
    (1, "1D"),
    (7, "1W"),
    (14, "2W"),
    (30, "1M"),
    (90, "3M"),
    (180, "6M"),
    (360, "1Y"),
];

/// Analyse the repo term structure.
pub fn analyze_repo_term_structure(input: &RepoTermInput) -> CorpFinanceResult<RepoTermOutput> {
    validate_term_input(input)?;

    // Build a combined set of known points: overnight + term_rates
    let mut known_points: Vec<(u32, Decimal)> = Vec::with_capacity(input.term_rates.len() + 1);
    known_points.push((1, input.overnight_rate));
    for pt in &input.term_rates {
        known_points.push((pt.term_days, pt.rate));
    }
    known_points.sort_by_key(|p| p.0);
    known_points.dedup_by_key(|p| p.0);

    // Interpolate at standard tenors (linear interpolation)
    let interpolated_curve: Vec<InterpolatedPoint> = STANDARD_TENORS
        .iter()
        .map(|&(days, label)| {
            let rate = interpolate_rate(days, &known_points);
            InterpolatedPoint {
                tenor_label: label.to_string(),
                term_days: days,
                rate,
            }
        })
        .collect();

    // Implied forward rates between consecutive standard tenors
    // (1+r2*t2) = (1+r1*t1) * (1+f*dt) => f = [(1+r2*t2)/(1+r1*t1) - 1] / dt
    let basis = dec!(360);
    let forward_repo_rates: Vec<ForwardRepoRate> = interpolated_curve
        .windows(2)
        .map(|w| {
            let r1 = w[0].rate;
            let t1 = Decimal::from(w[0].term_days) / basis;
            let r2 = w[1].rate;
            let t2 = Decimal::from(w[1].term_days) / basis;
            let dt = t2 - t1;

            let fwd = if dt.is_zero() {
                r2
            } else {
                let compound1 = Decimal::ONE + r1 * t1;
                let compound2 = Decimal::ONE + r2 * t2;
                if compound1.is_zero() {
                    Decimal::ZERO
                } else {
                    ((compound2 / compound1) - Decimal::ONE) / dt
                }
            };

            ForwardRepoRate {
                start_tenor: w[0].tenor_label.clone(),
                end_tenor: w[1].tenor_label.clone(),
                start_days: w[0].term_days,
                end_days: w[1].term_days,
                forward_rate: fwd,
            }
        })
        .collect();

    // Overnight vs. term spread
    let overnight_vs_term_spread: Vec<TermSpread> = interpolated_curve
        .iter()
        .map(|pt| TermSpread {
            tenor_label: pt.tenor_label.clone(),
            term_days: pt.term_days,
            spread: pt.rate - input.overnight_rate,
        })
        .collect();

    // Specialness premium: GC rate - specific collateral rate
    // If collateral trades below GC, specialness is positive (the security is "on special")
    let specialness_premium = input.gc_rate.map(|gc| gc - input.overnight_rate);

    Ok(RepoTermOutput {
        interpolated_curve,
        forward_repo_rates,
        overnight_vs_term_spread,
        specialness_premium,
    })
}

/// Linear interpolation between known points.
fn interpolate_rate(target_days: u32, known: &[(u32, Decimal)]) -> Decimal {
    if known.is_empty() {
        return Decimal::ZERO;
    }
    if known.len() == 1 {
        return known[0].1;
    }

    let target = Decimal::from(target_days);

    // Before first point: flat extrapolation
    if target_days <= known[0].0 {
        return known[0].1;
    }
    // After last point: flat extrapolation
    if target_days >= known[known.len() - 1].0 {
        return known[known.len() - 1].1;
    }

    // Find bracketing points
    for i in 0..known.len() - 1 {
        let (d1, r1) = known[i];
        let (d2, r2) = known[i + 1];
        if target_days >= d1 && target_days <= d2 {
            if d1 == d2 {
                return r1;
            }
            let dd1 = Decimal::from(d1);
            let dd2 = Decimal::from(d2);
            let frac = (target - dd1) / (dd2 - dd1);
            return r1 + frac * (r2 - r1);
        }
    }

    known[known.len() - 1].1
}

fn validate_term_input(input: &RepoTermInput) -> CorpFinanceResult<()> {
    if input.term_rates.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one term rate point is required.".into(),
        ));
    }
    for pt in &input.term_rates {
        if pt.term_days == 0 {
            return Err(CorpFinanceError::InvalidInput {
                field: "term_days".into(),
                reason: "Term days must be positive.".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// D) Securities Lending Economics
// ---------------------------------------------------------------------------

/// Input for securities lending economics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecLendingInput {
    /// Market value of the security being lent.
    pub security_value: Money,
    /// Lending fee in basis points (annualized).
    pub lending_fee_bps: Decimal,
    /// Cash reinvestment rate (annualized, decimal).
    pub cash_reinvestment_rate: Rate,
    /// Collateral requirement as percentage of security value (e.g. 1.02 = 102%).
    pub collateral_pct: Decimal,
    /// Term in calendar days.
    pub term_days: u32,
    /// Day-count basis denominator.
    pub day_count_basis: u32,
    /// General collateral rate for intrinsic value calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gc_rate: Option<Rate>,
}

/// Output of the securities lending economics calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecLendingOutput {
    /// Lending fee income over the term.
    pub lending_fee_income: Money,
    /// Cash collateral received.
    pub cash_collateral: Money,
    /// Reinvestment income on cash collateral.
    pub reinvestment_income: Money,
    /// Total return = lending fee + reinvestment income.
    pub total_return: Money,
    /// Annualized total return as percentage of security value.
    pub annualized_return: Rate,
    /// Intrinsic value: lending fee bps vs GC rate spread.
    pub intrinsic_value: Option<Money>,
}

/// Calculate securities lending economics.
pub fn calculate_sec_lending(input: &SecLendingInput) -> CorpFinanceResult<SecLendingOutput> {
    validate_sec_lending_input(input)?;

    let days = Decimal::from(input.term_days);
    let basis = Decimal::from(input.day_count_basis);
    let time_frac = days / basis;
    let bps_divisor = dec!(10000);

    // Lending fee income
    let lending_fee_rate = input.lending_fee_bps / bps_divisor;
    let lending_fee_income = input.security_value * lending_fee_rate * time_frac;

    // Cash collateral
    let cash_collateral = input.security_value * input.collateral_pct;

    // Reinvestment income
    let reinvestment_income = cash_collateral * input.cash_reinvestment_rate * time_frac;

    // Total return
    let total_return = lending_fee_income + reinvestment_income;

    // Annualized return
    let annualized_return = if input.security_value.is_zero() || days.is_zero() {
        Decimal::ZERO
    } else {
        (total_return / input.security_value) * (basis / days)
    };

    // Intrinsic value: spread between lending fee and GC rate
    let intrinsic_value = input.gc_rate.map(|gc| {
        let spread = lending_fee_rate - gc;
        input.security_value * spread * time_frac
    });

    Ok(SecLendingOutput {
        lending_fee_income,
        cash_collateral,
        reinvestment_income,
        total_return,
        annualized_return,
        intrinsic_value,
    })
}

fn validate_sec_lending_input(input: &SecLendingInput) -> CorpFinanceResult<()> {
    if input.security_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "security_value".into(),
            reason: "Security value must be positive.".into(),
        });
    }
    if input.lending_fee_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "lending_fee_bps".into(),
            reason: "Lending fee cannot be negative.".into(),
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
    if input.collateral_pct < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "collateral_pct".into(),
            reason: "Collateral percentage cannot be negative.".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Wrapper: RepoAnalyticsInput / RepoAnalyticsOutput
// ---------------------------------------------------------------------------

/// Enum selecting the repo model to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepoModel {
    Rate(RepoRateInput),
    ImpliedRepo(ImpliedRepoInput),
    TermStructure(RepoTermInput),
    SecLending(SecLendingInput),
}

/// Top-level analytics input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoAnalyticsInput {
    pub model: RepoModel,
}

/// Top-level analytics output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RepoAnalyticsOutput {
    Rate(RepoRateOutput),
    ImpliedRepo(ImpliedRepoOutput),
    TermStructure(RepoTermOutput),
    SecLending(SecLendingOutput),
}

/// Unified entry point for all repo analytics.
pub fn analyze_repo(
    input: &RepoAnalyticsInput,
) -> CorpFinanceResult<ComputationOutput<RepoAnalyticsOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    let (result, methodology) = match &input.model {
        RepoModel::Rate(ref ri) => {
            if ri.haircut_pct > dec!(0.50) {
                warnings.push("Haircut exceeds 50% -- verify collateral quality.".into());
            }
            let out = calculate_repo_rate(ri)?;
            (RepoAnalyticsOutput::Rate(out), "Repo Rate Calculation")
        }
        RepoModel::ImpliedRepo(ref ii) => {
            let out = calculate_implied_repo(ii)?;
            if !out.is_positive_carry {
                warnings.push("Trade has negative carry.".into());
            }
            (
                RepoAnalyticsOutput::ImpliedRepo(out),
                "Implied Repo Rate (Cash-and-Carry)",
            )
        }
        RepoModel::TermStructure(ref ti) => {
            let out = analyze_repo_term_structure(ti)?;
            if let Some(sp) = out.specialness_premium {
                if sp > Decimal::ZERO {
                    warnings.push(format!(
                        "Collateral is on special: premium = {:.4}%.",
                        sp * dec!(100)
                    ));
                }
            }
            (
                RepoAnalyticsOutput::TermStructure(out),
                "Repo Term Structure Analysis",
            )
        }
        RepoModel::SecLending(ref si) => {
            let out = calculate_sec_lending(si)?;
            (
                RepoAnalyticsOutput::SecLending(out),
                "Securities Lending Economics",
            )
        }
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        methodology,
        &serde_json::json!({
            "model": methodology,
            "day_count": "Actual/Basis"
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

    // -- Repo Rate tests ---------------------------------------------------

    /// Helper: true if two Decimals are within `eps` of each other.
    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn test_repo_interest_equals_principal_times_rate_times_time() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        // With zero haircut: purchase = 1M, interest ~= 1M * 0.05 * 30/360 ~= 4166.67
        assert!(
            approx_eq(out.repo_interest, dec!(4166.67), dec!(1.0)),
            "repo_interest {} should be ~4166.67",
            out.repo_interest
        );
    }

    #[test]
    fn test_haircut_reduces_purchase_price() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 360,
            haircut_pct: dec!(0.02),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        assert_eq!(out.purchase_price, dec!(980_000));
        assert!(out.purchase_price < input.collateral_value);
    }

    #[test]
    fn test_repurchase_price_gt_purchase_price() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.03),
            term_days: 90,
            day_count_basis: 360,
            haircut_pct: dec!(0.05),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        assert!(out.repurchase_price > out.purchase_price);
    }

    #[test]
    fn test_effective_rate_equals_repo_rate_with_zero_haircut() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 90,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        assert_eq!(out.effective_rate, dec!(0.05));
    }

    #[test]
    fn test_net_interest_margin_calculation() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: Some(dec!(0.03)),
        };
        let out = calculate_repo_rate(&input).unwrap();
        assert_eq!(out.net_interest_margin, Some(dec!(0.02)));
    }

    #[test]
    fn test_net_interest_margin_none_when_no_funding_cost() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        assert!(out.net_interest_margin.is_none());
    }

    #[test]
    fn test_forward_price_calculation() {
        let input = RepoRateInput {
            collateral_value: dec!(100),
            repo_rate: dec!(0.05),
            term_days: 180,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: dec!(1.5),
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        // forward = 100 * (1 + 0.05 * 0.5) - 1.5 = 102.5 - 1.5 = 101.0
        assert_eq!(out.forward_price, dec!(101.0));
    }

    #[test]
    fn test_overnight_repo_one_day() {
        let input = RepoRateInput {
            collateral_value: dec!(10_000_000),
            repo_rate: dec!(0.0525),
            term_days: 1,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        // Interest ~= 10M * 0.0525 / 360 ~= 1458.33
        assert!(
            approx_eq(out.repo_interest, dec!(1458.33), dec!(1.0)),
            "overnight interest {} should be ~1458.33",
            out.repo_interest
        );
    }

    #[test]
    fn test_implied_financing_cost_with_margin() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.05),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        // effective_rate ~= 0.05, implied ~= 0.05 * 1.05 = 0.0525
        assert!(
            approx_eq(out.implied_financing_cost, dec!(0.0525), dec!(0.0001)),
            "implied_financing_cost {} should be ~0.0525",
            out.implied_financing_cost
        );
    }

    #[test]
    fn test_zero_haircut_purchase_equals_collateral() {
        let input = RepoRateInput {
            collateral_value: dec!(5_000_000),
            repo_rate: dec!(0.04),
            term_days: 60,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        assert_eq!(out.purchase_price, input.collateral_value);
    }

    // -- Validation tests --------------------------------------------------

    #[test]
    fn test_reject_zero_collateral() {
        let input = RepoRateInput {
            collateral_value: Decimal::ZERO,
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        assert!(calculate_repo_rate(&input).is_err());
    }

    #[test]
    fn test_reject_zero_term() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 0,
            day_count_basis: 360,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        assert!(calculate_repo_rate(&input).is_err());
    }

    #[test]
    fn test_reject_zero_basis() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 0,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        assert!(calculate_repo_rate(&input).is_err());
    }

    #[test]
    fn test_reject_haircut_ge_one() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 360,
            haircut_pct: dec!(1.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        assert!(calculate_repo_rate(&input).is_err());
    }

    // -- Implied Repo tests ------------------------------------------------

    #[test]
    fn test_implied_repo_recovers_known_rate() {
        // Set up: spot = 99, forward = 100, 90 days, 360 basis, no coupons
        // dirty_spot = 99 + 0 = 99, dirty_fwd = 100 + 0 = 100
        // implied = (100 - 99) / (99 * 90/360) = 1 / 24.75 = 0.040404...
        let input = ImpliedRepoInput {
            spot_clean_price: dec!(99),
            forward_clean_price: dec!(100),
            spot_accrued: Decimal::ZERO,
            forward_accrued: Decimal::ZERO,
            coupon_income: Decimal::ZERO,
            term_days: 90,
            day_count_basis: 360,
        };
        let out = calculate_implied_repo(&input).unwrap();
        // implied = 1 / (99 * 0.25) = 1 / 24.75
        let expected = Decimal::ONE / (dec!(99) * dec!(90) / dec!(360));
        assert_eq!(out.implied_repo_rate, expected);
    }

    #[test]
    fn test_positive_carry_when_coupon_exceeds_financing() {
        let input = ImpliedRepoInput {
            spot_clean_price: dec!(100),
            forward_clean_price: dec!(99),
            spot_accrued: dec!(0.5),
            forward_accrued: dec!(1.0),
            coupon_income: dec!(2.5),
            term_days: 180,
            day_count_basis: 360,
        };
        let out = calculate_implied_repo(&input).unwrap();
        assert!(out.is_positive_carry);
        assert!(out.carry > Decimal::ZERO);
    }

    #[test]
    fn test_negative_carry_when_no_coupon() {
        let input = ImpliedRepoInput {
            spot_clean_price: dec!(100),
            forward_clean_price: dec!(101),
            spot_accrued: Decimal::ZERO,
            forward_accrued: Decimal::ZERO,
            coupon_income: Decimal::ZERO,
            term_days: 90,
            day_count_basis: 360,
        };
        let out = calculate_implied_repo(&input).unwrap();
        // No coupon, forward > spot => positive financing cost, negative carry
        assert!(!out.is_positive_carry);
    }

    #[test]
    fn test_implied_repo_validation_zero_spot() {
        let input = ImpliedRepoInput {
            spot_clean_price: Decimal::ZERO,
            forward_clean_price: dec!(100),
            spot_accrued: Decimal::ZERO,
            forward_accrued: Decimal::ZERO,
            coupon_income: Decimal::ZERO,
            term_days: 90,
            day_count_basis: 360,
        };
        assert!(calculate_implied_repo(&input).is_err());
    }

    #[test]
    fn test_implied_repo_basis_net_of_carry() {
        let input = ImpliedRepoInput {
            spot_clean_price: dec!(100),
            forward_clean_price: dec!(100.5),
            spot_accrued: Decimal::ZERO,
            forward_accrued: Decimal::ZERO,
            coupon_income: dec!(1.0),
            term_days: 90,
            day_count_basis: 360,
        };
        let out = calculate_implied_repo(&input).unwrap();
        // basis_net_of_carry = (forward_clean - spot_clean) - carry
        let expected = (dec!(100.5) - dec!(100)) - out.carry;
        assert_eq!(out.basis_net_of_carry, expected);
    }

    // -- Term Structure tests ----------------------------------------------

    #[test]
    fn test_term_structure_seven_standard_tenors() {
        let input = RepoTermInput {
            overnight_rate: dec!(0.05),
            term_rates: vec![
                RepoTermPoint {
                    term_days: 30,
                    rate: dec!(0.051),
                },
                RepoTermPoint {
                    term_days: 90,
                    rate: dec!(0.052),
                },
                RepoTermPoint {
                    term_days: 360,
                    rate: dec!(0.055),
                },
            ],
            collateral_type: CollateralType::Treasury,
            gc_rate: None,
        };
        let out = analyze_repo_term_structure(&input).unwrap();
        assert_eq!(out.interpolated_curve.len(), 7);
    }

    #[test]
    fn test_term_structure_six_forward_rates() {
        let input = RepoTermInput {
            overnight_rate: dec!(0.05),
            term_rates: vec![
                RepoTermPoint {
                    term_days: 30,
                    rate: dec!(0.051),
                },
                RepoTermPoint {
                    term_days: 360,
                    rate: dec!(0.055),
                },
            ],
            collateral_type: CollateralType::Treasury,
            gc_rate: None,
        };
        let out = analyze_repo_term_structure(&input).unwrap();
        assert_eq!(out.forward_repo_rates.len(), 6); // 7 tenors - 1
    }

    #[test]
    fn test_forward_rates_consistent_with_spot() {
        // If the curve is flat, forward rates should equal spot rates
        let input = RepoTermInput {
            overnight_rate: dec!(0.05),
            term_rates: vec![
                RepoTermPoint {
                    term_days: 30,
                    rate: dec!(0.05),
                },
                RepoTermPoint {
                    term_days: 90,
                    rate: dec!(0.05),
                },
                RepoTermPoint {
                    term_days: 360,
                    rate: dec!(0.05),
                },
            ],
            collateral_type: CollateralType::Treasury,
            gc_rate: None,
        };
        let out = analyze_repo_term_structure(&input).unwrap();
        for fwd in &out.forward_repo_rates {
            // On a flat simple-interest curve, forwards approximate spot
            // but are not exactly equal due to convexity; tolerance ~2bps
            let diff = (fwd.forward_rate - dec!(0.05)).abs();
            assert!(
                diff < dec!(0.002),
                "Forward rate {} should be close to 0.05 on flat curve",
                fwd.forward_rate
            );
        }
    }

    #[test]
    fn test_specialness_premium_positive() {
        let input = RepoTermInput {
            overnight_rate: dec!(0.03),
            term_rates: vec![RepoTermPoint {
                term_days: 30,
                rate: dec!(0.031),
            }],
            collateral_type: CollateralType::Treasury,
            gc_rate: Some(dec!(0.05)),
        };
        let out = analyze_repo_term_structure(&input).unwrap();
        // GC 5% - ON 3% = 2% specialness: the collateral trades special
        assert_eq!(out.specialness_premium, Some(dec!(0.02)));
    }

    #[test]
    fn test_specialness_none_without_gc() {
        let input = RepoTermInput {
            overnight_rate: dec!(0.05),
            term_rates: vec![RepoTermPoint {
                term_days: 30,
                rate: dec!(0.051),
            }],
            collateral_type: CollateralType::Treasury,
            gc_rate: None,
        };
        let out = analyze_repo_term_structure(&input).unwrap();
        assert!(out.specialness_premium.is_none());
    }

    #[test]
    fn test_overnight_vs_term_spread_at_1d_is_zero() {
        let input = RepoTermInput {
            overnight_rate: dec!(0.05),
            term_rates: vec![RepoTermPoint {
                term_days: 30,
                rate: dec!(0.055),
            }],
            collateral_type: CollateralType::Treasury,
            gc_rate: None,
        };
        let out = analyze_repo_term_structure(&input).unwrap();
        // The 1D interpolated rate should equal overnight, so spread = 0
        assert_eq!(out.overnight_vs_term_spread[0].spread, Decimal::ZERO);
    }

    #[test]
    fn test_term_structure_validation_empty_rates() {
        let input = RepoTermInput {
            overnight_rate: dec!(0.05),
            term_rates: vec![],
            collateral_type: CollateralType::Treasury,
            gc_rate: None,
        };
        assert!(analyze_repo_term_structure(&input).is_err());
    }

    // -- Securities Lending tests ------------------------------------------

    #[test]
    fn test_sec_lending_fee_income() {
        let input = SecLendingInput {
            security_value: dec!(10_000_000),
            lending_fee_bps: dec!(50),
            cash_reinvestment_rate: dec!(0.04),
            collateral_pct: dec!(1.02),
            term_days: 30,
            day_count_basis: 360,
            gc_rate: None,
        };
        let out = calculate_sec_lending(&input).unwrap();
        // fee ~= 10M * 0.005 * (30/360) ~= 4166.67
        assert!(
            approx_eq(out.lending_fee_income, dec!(4166.67), dec!(1.0)),
            "lending_fee_income {} should be ~4166.67",
            out.lending_fee_income
        );
    }

    #[test]
    fn test_sec_lending_cash_collateral() {
        let input = SecLendingInput {
            security_value: dec!(10_000_000),
            lending_fee_bps: dec!(50),
            cash_reinvestment_rate: dec!(0.04),
            collateral_pct: dec!(1.02),
            term_days: 30,
            day_count_basis: 360,
            gc_rate: None,
        };
        let out = calculate_sec_lending(&input).unwrap();
        assert_eq!(out.cash_collateral, dec!(10_200_000));
    }

    #[test]
    fn test_sec_lending_reinvestment_income() {
        let input = SecLendingInput {
            security_value: dec!(10_000_000),
            lending_fee_bps: dec!(50),
            cash_reinvestment_rate: dec!(0.04),
            collateral_pct: dec!(1.02),
            term_days: 30,
            day_count_basis: 360,
            gc_rate: None,
        };
        let out = calculate_sec_lending(&input).unwrap();
        // reinvestment ~= 10.2M * 0.04 * 30/360 ~= 34000
        assert!(
            approx_eq(out.reinvestment_income, dec!(34000), dec!(1.0)),
            "reinvestment_income {} should be ~34000",
            out.reinvestment_income
        );
    }

    #[test]
    fn test_sec_lending_total_return_sum() {
        let input = SecLendingInput {
            security_value: dec!(5_000_000),
            lending_fee_bps: dec!(100),
            cash_reinvestment_rate: dec!(0.03),
            collateral_pct: dec!(1.05),
            term_days: 90,
            day_count_basis: 360,
            gc_rate: None,
        };
        let out = calculate_sec_lending(&input).unwrap();
        assert_eq!(
            out.total_return,
            out.lending_fee_income + out.reinvestment_income
        );
    }

    #[test]
    fn test_sec_lending_annualized_return() {
        let input = SecLendingInput {
            security_value: dec!(1_000_000),
            lending_fee_bps: dec!(200),
            cash_reinvestment_rate: dec!(0.0),
            collateral_pct: dec!(1.0),
            term_days: 360,
            day_count_basis: 360,
            gc_rate: None,
        };
        let out = calculate_sec_lending(&input).unwrap();
        // fee_income = 1M * 0.02 * 1.0 = 20000, reinvestment = 0
        // annualized = (20000/1M) * (360/360) = 0.02
        assert_eq!(out.annualized_return, dec!(0.02));
    }

    #[test]
    fn test_sec_lending_intrinsic_value() {
        let input = SecLendingInput {
            security_value: dec!(10_000_000),
            lending_fee_bps: dec!(50),
            cash_reinvestment_rate: dec!(0.04),
            collateral_pct: dec!(1.02),
            term_days: 90,
            day_count_basis: 360,
            gc_rate: Some(dec!(0.003)),
        };
        let out = calculate_sec_lending(&input).unwrap();
        // lending_rate = 50/10000 = 0.005
        // spread = 0.005 - 0.003 = 0.002
        // intrinsic = 10M * 0.002 * 90/360 = 5000
        let expected = dec!(10_000_000) * dec!(0.002) * dec!(90) / dec!(360);
        assert_eq!(out.intrinsic_value, Some(expected));
    }

    #[test]
    fn test_sec_lending_validation_zero_security() {
        let input = SecLendingInput {
            security_value: Decimal::ZERO,
            lending_fee_bps: dec!(50),
            cash_reinvestment_rate: dec!(0.04),
            collateral_pct: dec!(1.02),
            term_days: 30,
            day_count_basis: 360,
            gc_rate: None,
        };
        assert!(calculate_sec_lending(&input).is_err());
    }

    // -- Wrapper tests -----------------------------------------------------

    #[test]
    fn test_analyze_repo_rate_model() {
        let input = RepoAnalyticsInput {
            model: RepoModel::Rate(RepoRateInput {
                collateral_value: dec!(1_000_000),
                repo_rate: dec!(0.05),
                term_days: 30,
                day_count_basis: 360,
                haircut_pct: dec!(0.02),
                initial_margin: dec!(1.0),
                accrued_interest: Decimal::ZERO,
                funding_cost: None,
            }),
        };
        let out = analyze_repo(&input).unwrap();
        assert!(out.methodology.contains("Repo Rate"));
        match out.result {
            RepoAnalyticsOutput::Rate(ref r) => {
                assert!(r.purchase_price > Decimal::ZERO);
            }
            _ => panic!("Expected Rate variant"),
        }
    }

    #[test]
    fn test_analyze_repo_implied_model() {
        let input = RepoAnalyticsInput {
            model: RepoModel::ImpliedRepo(ImpliedRepoInput {
                spot_clean_price: dec!(99),
                forward_clean_price: dec!(100),
                spot_accrued: Decimal::ZERO,
                forward_accrued: Decimal::ZERO,
                coupon_income: Decimal::ZERO,
                term_days: 90,
                day_count_basis: 360,
            }),
        };
        let out = analyze_repo(&input).unwrap();
        assert!(out.methodology.contains("Implied Repo"));
    }

    #[test]
    fn test_analyze_repo_term_structure_model() {
        let input = RepoAnalyticsInput {
            model: RepoModel::TermStructure(RepoTermInput {
                overnight_rate: dec!(0.05),
                term_rates: vec![RepoTermPoint {
                    term_days: 90,
                    rate: dec!(0.052),
                }],
                collateral_type: CollateralType::Treasury,
                gc_rate: None,
            }),
        };
        let out = analyze_repo(&input).unwrap();
        assert!(out.methodology.contains("Term Structure"));
    }

    #[test]
    fn test_analyze_repo_sec_lending_model() {
        let input = RepoAnalyticsInput {
            model: RepoModel::SecLending(SecLendingInput {
                security_value: dec!(5_000_000),
                lending_fee_bps: dec!(50),
                cash_reinvestment_rate: dec!(0.04),
                collateral_pct: dec!(1.02),
                term_days: 30,
                day_count_basis: 360,
                gc_rate: None,
            }),
        };
        let out = analyze_repo(&input).unwrap();
        assert!(out.methodology.contains("Securities Lending"));
    }

    #[test]
    fn test_wrapper_high_haircut_warning() {
        let input = RepoAnalyticsInput {
            model: RepoModel::Rate(RepoRateInput {
                collateral_value: dec!(1_000_000),
                repo_rate: dec!(0.10),
                term_days: 30,
                day_count_basis: 360,
                haircut_pct: dec!(0.55),
                initial_margin: dec!(1.0),
                accrued_interest: Decimal::ZERO,
                funding_cost: None,
            }),
        };
        let out = analyze_repo(&input).unwrap();
        assert!(
            out.warnings.iter().any(|w| w.contains("50%")),
            "Should warn about high haircut"
        );
    }

    #[test]
    fn test_wrapper_negative_carry_warning() {
        let input = RepoAnalyticsInput {
            model: RepoModel::ImpliedRepo(ImpliedRepoInput {
                spot_clean_price: dec!(100),
                forward_clean_price: dec!(102),
                spot_accrued: Decimal::ZERO,
                forward_accrued: Decimal::ZERO,
                coupon_income: Decimal::ZERO,
                term_days: 90,
                day_count_basis: 360,
            }),
        };
        let out = analyze_repo(&input).unwrap();
        assert!(
            out.warnings.iter().any(|w| w.contains("negative carry")),
            "Should warn about negative carry"
        );
    }

    #[test]
    fn test_wrapper_metadata_present() {
        let input = RepoAnalyticsInput {
            model: RepoModel::Rate(RepoRateInput {
                collateral_value: dec!(1_000_000),
                repo_rate: dec!(0.05),
                term_days: 30,
                day_count_basis: 360,
                haircut_pct: dec!(0.0),
                initial_margin: dec!(1.0),
                accrued_interest: Decimal::ZERO,
                funding_cost: None,
            }),
        };
        let out = analyze_repo(&input).unwrap();
        assert!(!out.metadata.version.is_empty());
        assert_eq!(out.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_very_long_term_repo() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 365,
            day_count_basis: 365,
            haircut_pct: dec!(0.0),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        // 1 full year: interest = 1M * 0.05 * 1 = 50000
        assert_eq!(out.repo_interest, dec!(50_000));
    }

    #[test]
    fn test_serialization_roundtrip_repo_rate() {
        let input = RepoRateInput {
            collateral_value: dec!(1_000_000),
            repo_rate: dec!(0.05),
            term_days: 30,
            day_count_basis: 360,
            haircut_pct: dec!(0.02),
            initial_margin: dec!(1.0),
            accrued_interest: Decimal::ZERO,
            funding_cost: None,
        };
        let out = calculate_repo_rate(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: RepoRateOutput = serde_json::from_str(&json).unwrap();
    }
}
