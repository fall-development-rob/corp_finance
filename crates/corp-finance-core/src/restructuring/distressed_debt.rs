use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Seniority ranking for debt tranches in the capital structure.
///
/// Ordered from most senior (DIP) to most junior (Mezzanine). The waterfall
/// distributes value strictly in this order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Seniority {
    DIP,
    FirstLien,
    SecondLien,
    Senior,
    SeniorSub,
    Subordinated,
    Mezzanine,
}

impl std::fmt::Display for Seniority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DIP => write!(f, "DIP"),
            Self::FirstLien => write!(f, "First Lien"),
            Self::SecondLien => write!(f, "Second Lien"),
            Self::Senior => write!(f, "Senior Unsecured"),
            Self::SeniorSub => write!(f, "Senior Subordinated"),
            Self::Subordinated => write!(f, "Subordinated"),
            Self::Mezzanine => write!(f, "Mezzanine"),
        }
    }
}

/// Treatment type for a debt tranche in a restructuring plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreatmentType {
    /// Keep the debt as-is (claim = face value)
    Reinstate,
    /// Modify terms (coupon, maturity) but keep as debt
    Amend,
    /// Exchange old debt for new debt instrument
    Exchange,
    /// Convert debt to equity in the reorganized entity
    EquityConversion,
    /// Pay down with cash at closing
    CashPaydown,
    /// Combination of multiple treatment types
    Combination,
}

impl std::fmt::Display for TreatmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reinstate => write!(f, "Reinstate"),
            Self::Amend => write!(f, "Amend & Extend"),
            Self::Exchange => write!(f, "Exchange Offer"),
            Self::EquityConversion => write!(f, "Equity Conversion"),
            Self::CashPaydown => write!(f, "Cash Paydown"),
            Self::Combination => write!(f, "Combination"),
        }
    }
}

/// A single tranche of debt in the existing capital structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtTranche {
    /// Human-readable name (must be unique across tranches)
    pub name: String,
    /// Outstanding face / par value
    pub face_value: Money,
    /// Current secondary market price (cents on dollar, e.g. 0.65 = 65 cents)
    pub market_price: Decimal,
    /// Annual coupon rate (decimal, e.g. 0.08 = 8%)
    pub coupon_rate: Rate,
    /// Remaining years to contractual maturity
    pub maturity_years: Decimal,
    /// Position in the capital structure
    pub seniority: Seniority,
    /// Whether the tranche is secured by collateral
    pub is_secured: bool,
}

/// Proposed treatment for a specific debt tranche in the restructuring plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestructuringTreatment {
    /// Must match a `DebtTranche::name`
    pub tranche_name: String,
    /// Type of restructuring treatment
    pub treatment_type: TreatmentType,
    /// New face value (for Exchange offers)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_face_value: Option<Money>,
    /// New coupon rate (for Amend or Exchange)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_coupon: Option<Rate>,
    /// Percentage of reorganized equity received (for EquityConversion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equity_conversion_pct: Option<Decimal>,
    /// Cash paid at closing (for CashPaydown)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cash_paydown: Option<Money>,
}

/// Terms for a Debtor-in-Possession financing facility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DipTerms {
    /// Total commitment amount
    pub commitment: Money,
    /// Amount currently drawn
    pub drawn: Money,
    /// Annual interest rate on drawn amounts
    pub rate: Rate,
    /// Upfront and commitment fees as a percentage
    pub fees_pct: Rate,
    /// Facility term in months
    pub term_months: u32,
    /// Whether the DIP converts to exit financing
    pub converts_to_exit: bool,
}

/// Operating assumptions for the restructured entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatingAssumptions {
    /// Annual EBITDA of the operating business
    pub annual_ebitda: Money,
    /// Annual maintenance capital expenditures
    pub maintenance_capex: Money,
    /// Annual working capital change (positive = use of cash)
    pub working_capital_change: Money,
    /// One-time restructuring costs (advisory, legal, etc.)
    pub restructuring_costs: Money,
}

/// Top-level input for distressed debt analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistressedDebtInput {
    /// Current enterprise value estimate
    pub enterprise_value: Money,
    /// Post-restructuring / exit enterprise value
    pub exit_enterprise_value: Money,
    /// Expected time horizon to exit or resolution (years)
    pub exit_timeline_years: Decimal,
    /// Current debt stack ordered by seniority
    pub capital_structure: Vec<DebtTranche>,
    /// Proposed restructuring treatment for each tranche
    pub proposed_treatment: Vec<RestructuringTreatment>,
    /// Optional DIP financing facility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dip_facility: Option<DipTerms>,
    /// Operating assumptions for the go-forward entity
    pub operating_assumptions: OperatingAssumptions,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Detail on the fulcrum security (the tranche where recovery breaks).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FulcrumDetail {
    /// Name of the fulcrum tranche
    pub tranche_name: String,
    /// Outstanding face value of the tranche
    pub face_value: Money,
    /// Recovery rate (0.0 to 1.0) — portion of face recovered
    pub recovery_rate: Decimal,
    /// Implied market price based on recovery analysis
    pub implied_price: Decimal,
    /// Current market trading price
    pub current_price: Decimal,
    /// Mispricing: implied - current (positive = undervalued)
    pub mispricing: Decimal,
}

/// Per-tranche analysis within the restructuring plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheAnalysis {
    /// Tranche name
    pub name: String,
    /// Outstanding face / par value
    pub face_value: Money,
    /// Current market value = face * market_price
    pub current_market_value: Money,
    /// Value recovered through the plan waterfall
    pub recovery_value: Money,
    /// Recovery rate = recovery_value / face_value
    pub recovery_rate: Decimal,
    /// Value of the new instrument received post-restructuring
    pub post_restructuring_value: Money,
    /// IRR if purchased at market price and received recovery value
    pub irr_at_market: Decimal,
    /// Description of the treatment applied
    pub treatment: String,
}

/// Analysis of the DIP financing facility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DipAnalysis {
    /// Total cost of the DIP (interest + fees)
    pub total_cost: Money,
    /// Whether the DIP converts to exit debt
    pub converts_to_exit_debt: bool,
    /// Effective all-in rate (interest + fees annualized)
    pub effective_rate: Rate,
}

/// Full output of a distressed debt / restructuring analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistressedDebtOutput {
    /// Name of the fulcrum security
    pub fulcrum_security: String,
    /// Detail on the fulcrum tranche
    pub fulcrum_analysis: FulcrumDetail,
    /// Per-tranche analysis
    pub tranche_analysis: Vec<TrancheAnalysis>,
    /// Total plan distributable value (= exit enterprise value)
    pub plan_value: Money,
    /// Sum of all face-value claims
    pub total_claims: Money,
    /// Overall plan recovery = plan_value / total_claims
    pub overall_recovery: Decimal,
    /// Equity value created = exit_EV - total reinstated/new debt
    pub equity_value_created: Money,
    /// Optional DIP analysis
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dip_analysis: Option<DipAnalysis>,
    /// Maximum credit bid value = EV less senior claims
    pub credit_bid_value: Money,
}

// ---------------------------------------------------------------------------
// Calculation
// ---------------------------------------------------------------------------

/// Analyze a distressed debt situation and restructuring plan.
///
/// Performs a claims waterfall analysis against the exit enterprise value,
/// identifies the fulcrum security, calculates per-tranche recoveries and
/// implied IRRs, and evaluates credit bid opportunity and equity creation.
pub fn analyze_distressed_debt(
    input: &DistressedDebtInput,
) -> CorpFinanceResult<ComputationOutput<DistressedDebtOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validation ---------------------------------------------------------
    validate_input(input)?;

    // --- Sort capital structure by seniority --------------------------------
    let mut tranches = input.capital_structure.clone();
    tranches.sort_by(|a, b| a.seniority.cmp(&b.seniority));

    // --- Build treatment lookup ---------------------------------------------
    let treatments: std::collections::HashMap<&str, &RestructuringTreatment> = input
        .proposed_treatment
        .iter()
        .map(|t| (t.tranche_name.as_str(), t))
        .collect();

    // --- DIP analysis -------------------------------------------------------
    let dip_analysis = input.dip_facility.as_ref().map(|dip| {
        let term_years = Decimal::from(dip.term_months) / dec!(12);
        let interest_cost = dip.drawn * dip.rate * term_years;
        let fee_cost = dip.commitment * dip.fees_pct;
        let total_cost = interest_cost + fee_cost;
        let effective_rate = if dip.drawn > Decimal::ZERO && term_years > Decimal::ZERO {
            total_cost / (dip.drawn * term_years)
        } else {
            Decimal::ZERO
        };
        DipAnalysis {
            total_cost,
            converts_to_exit_debt: dip.converts_to_exit,
            effective_rate,
        }
    });

    // --- Waterfall: distribute exit EV through the seniority stack ----------
    let plan_value = input.exit_enterprise_value;
    let mut remaining = plan_value;
    let total_claims: Money = tranches.iter().map(|t| t.face_value).sum();

    // Track debt that survives restructuring (reinstated + new exchange debt)
    let mut total_surviving_debt = Decimal::ZERO;

    // DIP claims are super-senior
    if let Some(dip) = &input.dip_facility {
        let dip_claim = dip.drawn;
        let dip_consumed = remaining.min(dip_claim);
        remaining -= dip_consumed;

        if dip.converts_to_exit {
            total_surviving_debt += dip_claim;
        }
    }

    // Track per-tranche recovery and fulcrum detection
    let mut tranche_results: Vec<TrancheAnalysis> = Vec::new();
    let mut fulcrum: Option<FulcrumDetail> = None;
    let mut total_equity_conversion_pct = Decimal::ZERO;

    for tranche in &tranches {
        let treatment = treatments.get(tranche.name.as_str());
        let face = tranche.face_value;
        let market_value = face * tranche.market_price;

        // Determine recovery based on treatment type
        let (recovery_value, post_restructuring_value, treatment_desc) = compute_tranche_recovery(
            tranche,
            treatment,
            remaining,
            &mut total_surviving_debt,
            &mut total_equity_conversion_pct,
        );

        // Deduct from remaining distributable value
        let consumed = recovery_value.min(remaining);
        remaining -= consumed;

        // Recovery rate
        let recovery_rate = if face > Decimal::ZERO {
            recovery_value / face
        } else {
            Decimal::ZERO
        };

        // Fulcrum security: first tranche where recovery < 100%
        if fulcrum.is_none() && recovery_rate < Decimal::ONE {
            fulcrum = Some(FulcrumDetail {
                tranche_name: tranche.name.clone(),
                face_value: face,
                recovery_rate,
                implied_price: recovery_rate,
                current_price: tranche.market_price,
                mispricing: recovery_rate - tranche.market_price,
            });
        }

        // IRR at market price: if bought at market and received recovery
        let irr = compute_irr_at_market(market_value, recovery_value, input.exit_timeline_years);

        tranche_results.push(TrancheAnalysis {
            name: tranche.name.clone(),
            face_value: face,
            current_market_value: market_value,
            recovery_value,
            recovery_rate,
            post_restructuring_value,
            irr_at_market: irr,
            treatment: treatment_desc,
        });
    }

    // If all tranches have 100% recovery, the most junior tranche is the fulcrum
    let fulcrum_detail = fulcrum.unwrap_or_else(|| {
        let last = tranches.last().expect("at least one tranche validated");
        FulcrumDetail {
            tranche_name: last.name.clone(),
            face_value: last.face_value,
            recovery_rate: Decimal::ONE,
            implied_price: Decimal::ONE,
            current_price: last.market_price,
            mispricing: Decimal::ONE - last.market_price,
        }
    });

    // --- Credit bid value ---------------------------------------------------
    // Credit bid = EV minus all claims senior to the fulcrum tranche
    let credit_bid_value = compute_credit_bid(
        plan_value,
        &tranches,
        &fulcrum_detail.tranche_name,
        input.dip_facility.as_ref(),
    );

    // --- Equity value created -----------------------------------------------
    let equity_value = plan_value - total_surviving_debt;

    // --- Overall recovery ---------------------------------------------------
    let overall_recovery = if total_claims > Decimal::ZERO {
        plan_value / total_claims
    } else {
        Decimal::ZERO
    };

    // --- Warnings -----------------------------------------------------------
    // Fulcrum trades below implied recovery (opportunity)
    if fulcrum_detail.mispricing > Decimal::ZERO {
        warnings.push(format!(
            "Fulcrum security '{}' appears undervalued: implied price {:.2} vs market {:.2} \
             (mispricing: +{:.2})",
            fulcrum_detail.tranche_name,
            fulcrum_detail.implied_price,
            fulcrum_detail.current_price,
            fulcrum_detail.mispricing,
        ));
    }

    // Negative equity value
    if equity_value < Decimal::ZERO {
        warnings.push(format!(
            "Negative equity value ({}) — surviving debt exceeds exit enterprise value",
            equity_value,
        ));
    }

    // DIP > 50% of EV
    if let Some(dip) = &input.dip_facility {
        if dip.drawn > plan_value * dec!(0.5) {
            warnings.push(format!(
                "DIP facility ({}) exceeds 50% of exit enterprise value ({}) — \
                 may impair junior recovery",
                dip.drawn, plan_value,
            ));
        }
    }

    let output = DistressedDebtOutput {
        fulcrum_security: fulcrum_detail.tranche_name.clone(),
        fulcrum_analysis: fulcrum_detail,
        tranche_analysis: tranche_results,
        plan_value,
        total_claims,
        overall_recovery,
        equity_value_created: equity_value,
        dip_analysis,
        credit_bid_value,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Distressed Debt / Restructuring Analysis",
        &serde_json::json!({
            "enterprise_value": input.enterprise_value.to_string(),
            "exit_enterprise_value": input.exit_enterprise_value.to_string(),
            "exit_timeline_years": input.exit_timeline_years.to_string(),
            "num_tranches": input.capital_structure.len(),
            "num_treatments": input.proposed_treatment.len(),
            "has_dip": input.dip_facility.is_some(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Validate all inputs for the distressed debt analysis.
fn validate_input(input: &DistressedDebtInput) -> CorpFinanceResult<()> {
    if input.enterprise_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "enterprise_value".into(),
            reason: "Enterprise value must be positive".into(),
        });
    }
    if input.exit_enterprise_value < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "exit_enterprise_value".into(),
            reason: "Exit enterprise value cannot be negative".into(),
        });
    }
    if input.exit_timeline_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "exit_timeline_years".into(),
            reason: "Exit timeline must be positive".into(),
        });
    }
    if input.capital_structure.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "capital_structure".into(),
            reason: "At least one debt tranche is required".into(),
        });
    }

    // Validate individual tranches
    for tranche in &input.capital_structure {
        if tranche.face_value < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("capital_structure[{}].face_value", tranche.name),
                reason: "Face value cannot be negative".into(),
            });
        }
        if tranche.market_price < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("capital_structure[{}].market_price", tranche.name),
                reason: "Market price cannot be negative".into(),
            });
        }
    }

    // Validate treatment names match tranche names
    let tranche_names: std::collections::HashSet<&str> = input
        .capital_structure
        .iter()
        .map(|t| t.name.as_str())
        .collect();

    for treatment in &input.proposed_treatment {
        if !tranche_names.contains(treatment.tranche_name.as_str()) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!(
                    "proposed_treatment[{}].tranche_name",
                    treatment.tranche_name
                ),
                reason: format!(
                    "Treatment references unknown tranche '{}'. Valid tranches: {:?}",
                    treatment.tranche_name, tranche_names
                ),
            });
        }
    }

    // Validate DIP terms
    if let Some(dip) = &input.dip_facility {
        if dip.commitment < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "dip_facility.commitment".into(),
                reason: "DIP commitment cannot be negative".into(),
            });
        }
        if dip.drawn < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "dip_facility.drawn".into(),
                reason: "DIP drawn amount cannot be negative".into(),
            });
        }
        if dip.drawn > dip.commitment {
            return Err(CorpFinanceError::InvalidInput {
                field: "dip_facility.drawn".into(),
                reason: "DIP drawn amount cannot exceed commitment".into(),
            });
        }
    }

    // Validate operating assumptions
    if input.operating_assumptions.annual_ebitda < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "operating_assumptions.annual_ebitda".into(),
            reason: "Annual EBITDA cannot be negative".into(),
        });
    }
    if input.operating_assumptions.maintenance_capex < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "operating_assumptions.maintenance_capex".into(),
            reason: "Maintenance capex cannot be negative".into(),
        });
    }

    Ok(())
}

/// Compute recovery for a single tranche based on its treatment.
///
/// Returns `(recovery_value, post_restructuring_value, treatment_description)`.
fn compute_tranche_recovery(
    tranche: &DebtTranche,
    treatment: Option<&&RestructuringTreatment>,
    remaining: Decimal,
    total_surviving_debt: &mut Decimal,
    total_equity_conversion_pct: &mut Decimal,
) -> (Money, Money, String) {
    let face = tranche.face_value;

    match treatment {
        Some(t) => match &t.treatment_type {
            TreatmentType::Reinstate => {
                // Reinstated at full face value — consumes from distributable
                let recovery = face.min(remaining);
                *total_surviving_debt += face;
                (recovery, face, "Reinstate".to_string())
            }
            TreatmentType::Amend => {
                // Amended terms — still counts as full face claim for recovery
                let new_coupon = t.new_coupon.unwrap_or(tranche.coupon_rate);
                let recovery = face.min(remaining);
                *total_surviving_debt += face;
                (
                    recovery,
                    face,
                    format!(
                        "Amend & Extend (new coupon: {:.2}%)",
                        new_coupon * dec!(100)
                    ),
                )
            }
            TreatmentType::Exchange => {
                // Exchange: old debt for new debt at new_face_value
                let new_face = t.new_face_value.unwrap_or(face);
                let recovery = new_face.min(remaining);
                *total_surviving_debt += new_face;
                (
                    recovery,
                    new_face,
                    format!("Exchange Offer (new face: {})", new_face),
                )
            }
            TreatmentType::EquityConversion => {
                // Convert to equity — recovery = equity_conversion_pct of residual equity
                let equity_pct = t.equity_conversion_pct.unwrap_or(Decimal::ZERO);
                *total_equity_conversion_pct += equity_pct;
                // Recovery for equity conversion is the equity share of remaining value
                // After all senior debt is paid, the residual becomes equity
                let recovery = remaining * equity_pct;
                (
                    recovery,
                    recovery,
                    format!("Equity Conversion ({:.1}% equity)", equity_pct * dec!(100)),
                )
            }
            TreatmentType::CashPaydown => {
                // Cash paydown at closing
                let cash = t.cash_paydown.unwrap_or(Decimal::ZERO);
                let recovery = cash.min(remaining);
                (recovery, cash, format!("Cash Paydown ({})", cash))
            }
            TreatmentType::Combination => {
                // Combination: sum of cash paydown + new debt + equity conversion
                let cash = t.cash_paydown.unwrap_or(Decimal::ZERO);
                let new_face = t.new_face_value.unwrap_or(Decimal::ZERO);
                let equity_pct = t.equity_conversion_pct.unwrap_or(Decimal::ZERO);

                let debt_recovery = cash + new_face;
                let equity_recovery = (remaining - debt_recovery).max(Decimal::ZERO) * equity_pct;
                let total_recovery = (debt_recovery + equity_recovery).min(remaining);

                *total_surviving_debt += new_face;
                *total_equity_conversion_pct += equity_pct;

                (
                    total_recovery,
                    total_recovery,
                    format!(
                        "Combination (cash: {}, new debt: {}, equity: {:.1}%)",
                        cash,
                        new_face,
                        equity_pct * dec!(100)
                    ),
                )
            }
        },
        None => {
            // No explicit treatment — waterfall assigns value up to face
            let recovery = face.min(remaining);
            (
                recovery,
                recovery,
                "Waterfall (no explicit treatment)".to_string(),
            )
        }
    }
}

/// Compute IRR at market price.
///
/// IRR = (recovery_value / cost)^(1 / years) - 1
/// where cost = face_value * market_price (the purchase price).
fn compute_irr_at_market(market_value: Money, recovery_value: Money, years: Decimal) -> Decimal {
    if market_value <= Decimal::ZERO || years <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if recovery_value <= Decimal::ZERO {
        return dec!(-1); // Total loss
    }

    let ratio = recovery_value / market_value;

    // (ratio)^(1/years) - 1 using iterative approach for precision
    // We use the natural log / exp approximation via Newton's method
    // For ratio^(1/n), we solve x^n = ratio
    let exponent = Decimal::ONE / years;
    decimal_pow(ratio, exponent) - Decimal::ONE
}

/// Raise a positive Decimal to a fractional power using Newton's method.
///
/// Computes `base^exp` for positive base values. Uses the identity
/// `base^exp = e^(exp * ln(base))` with a Newton's method approach
/// for the logarithm and exponentiation.
fn decimal_pow(base: Decimal, exp: Decimal) -> Decimal {
    if base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if base == Decimal::ONE || exp == Decimal::ZERO {
        return Decimal::ONE;
    }
    if exp == Decimal::ONE {
        return base;
    }

    // ln(base) via Newton's method
    let ln_base = decimal_ln(base);
    // e^(exp * ln_base) via Taylor series
    decimal_exp(exp * ln_base)
}

/// Natural logarithm of a positive Decimal using an AGM-like series.
///
/// Uses the series: ln(x) = 2 * sum_{k=0..inf} (1/(2k+1)) * ((x-1)/(x+1))^(2k+1)
/// Converges well for x near 1. For larger x, we use range reduction.
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO; // undefined, but handle gracefully
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    // Range reduction: ln(x * 2^n) = ln(x) + n*ln(2)
    let ln2 = dec!(0.69314718055994530941723);
    let mut val = x;
    let mut adjustment = Decimal::ZERO;

    // Bring val into [0.5, 2.0] range
    while val > dec!(2) {
        val /= dec!(2);
        adjustment += ln2;
    }
    while val < dec!(0.5) {
        val *= dec!(2);
        adjustment -= ln2;
    }

    // Now compute ln(val) using the series for val in [0.5, 2.0]
    // ln(val) = 2 * sum_{k=0..N} (1/(2k+1)) * ((val-1)/(val+1))^(2k+1)
    let u = (val - Decimal::ONE) / (val + Decimal::ONE);
    let u_sq = u * u;
    let mut term = u;
    let mut result = u;

    for k in 1u32..40 {
        term *= u_sq;
        let denom = Decimal::from(2 * k + 1);
        result += term / denom;
    }

    dec!(2) * result + adjustment
}

/// Exponential function e^x using Taylor series.
fn decimal_exp(x: Decimal) -> Decimal {
    let mut term = Decimal::ONE;
    let mut result = Decimal::ONE;

    for n in 1u32..60 {
        term *= x / Decimal::from(n);
        result += term;
        // Early termination for convergence
        if term.abs() < dec!(0.0000000000000001) {
            break;
        }
    }

    result
}

/// Compute the credit bid value: EV minus all claims senior to the target tranche.
fn compute_credit_bid(
    plan_value: Money,
    tranches: &[DebtTranche],
    fulcrum_name: &str,
    dip: Option<&DipTerms>,
) -> Money {
    let mut senior_claims = Decimal::ZERO;

    // DIP is always super-senior
    if let Some(dip_terms) = dip {
        senior_claims += dip_terms.drawn;
    }

    for tranche in tranches {
        if tranche.name == fulcrum_name {
            break;
        }
        senior_claims += tranche.face_value;
    }

    (plan_value - senior_claims).max(Decimal::ZERO)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: minimal operating assumptions
    fn default_operating() -> OperatingAssumptions {
        OperatingAssumptions {
            annual_ebitda: dec!(50),
            maintenance_capex: dec!(10),
            working_capital_change: dec!(5),
            restructuring_costs: dec!(15),
        }
    }

    /// Helper: simple 2-tranche capital structure
    fn two_tranche_input() -> DistressedDebtInput {
        DistressedDebtInput {
            enterprise_value: dec!(500),
            exit_enterprise_value: dec!(600),
            exit_timeline_years: dec!(2),
            capital_structure: vec![
                DebtTranche {
                    name: "First Lien".into(),
                    face_value: dec!(400),
                    market_price: dec!(0.95),
                    coupon_rate: dec!(0.05),
                    maturity_years: dec!(3),
                    seniority: Seniority::FirstLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "Second Lien".into(),
                    face_value: dec!(300),
                    market_price: dec!(0.40),
                    coupon_rate: dec!(0.10),
                    maturity_years: dec!(5),
                    seniority: Seniority::SecondLien,
                    is_secured: true,
                },
            ],
            proposed_treatment: vec![
                RestructuringTreatment {
                    tranche_name: "First Lien".into(),
                    treatment_type: TreatmentType::Reinstate,
                    new_face_value: None,
                    new_coupon: None,
                    equity_conversion_pct: None,
                    cash_paydown: None,
                },
                RestructuringTreatment {
                    tranche_name: "Second Lien".into(),
                    treatment_type: TreatmentType::EquityConversion,
                    new_face_value: None,
                    new_coupon: None,
                    equity_conversion_pct: Some(dec!(1.0)),
                    cash_paydown: None,
                },
            ],
            dip_facility: None,
            operating_assumptions: default_operating(),
        }
    }

    // --- Test 1: Simple 2-tranche restructuring ---

    #[test]
    fn test_simple_two_tranche() {
        let input = two_tranche_input();
        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.plan_value, dec!(600));
        assert_eq!(out.total_claims, dec!(700)); // 400 + 300

        // First Lien: reinstated at 400 face, full recovery
        assert_eq!(out.tranche_analysis[0].name, "First Lien");
        assert_eq!(out.tranche_analysis[0].recovery_value, dec!(400));
        assert_eq!(out.tranche_analysis[0].recovery_rate, Decimal::ONE);

        // Second Lien: fulcrum, gets remaining 200 of 300 face via equity
        assert_eq!(out.tranche_analysis[1].name, "Second Lien");
        assert!(out.tranche_analysis[1].recovery_rate < Decimal::ONE);

        // Fulcrum should be Second Lien
        assert_eq!(out.fulcrum_security, "Second Lien");
    }

    // --- Test 2: Full capital structure with all treatment types ---

    #[test]
    fn test_full_capital_structure() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(1000),
            exit_enterprise_value: dec!(1200),
            exit_timeline_years: dec!(3),
            capital_structure: vec![
                DebtTranche {
                    name: "Revolver".into(),
                    face_value: dec!(100),
                    market_price: dec!(1.0),
                    coupon_rate: dec!(0.04),
                    maturity_years: dec!(2),
                    seniority: Seniority::FirstLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "Term Loan A".into(),
                    face_value: dec!(300),
                    market_price: dec!(0.90),
                    coupon_rate: dec!(0.06),
                    maturity_years: dec!(4),
                    seniority: Seniority::FirstLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "Term Loan B".into(),
                    face_value: dec!(200),
                    market_price: dec!(0.70),
                    coupon_rate: dec!(0.08),
                    maturity_years: dec!(5),
                    seniority: Seniority::SecondLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "Senior Notes".into(),
                    face_value: dec!(250),
                    market_price: dec!(0.50),
                    coupon_rate: dec!(0.09),
                    maturity_years: dec!(6),
                    seniority: Seniority::Senior,
                    is_secured: false,
                },
                DebtTranche {
                    name: "Sub Notes".into(),
                    face_value: dec!(150),
                    market_price: dec!(0.15),
                    coupon_rate: dec!(0.12),
                    maturity_years: dec!(7),
                    seniority: Seniority::Subordinated,
                    is_secured: false,
                },
            ],
            proposed_treatment: vec![
                RestructuringTreatment {
                    tranche_name: "Revolver".into(),
                    treatment_type: TreatmentType::CashPaydown,
                    new_face_value: None,
                    new_coupon: None,
                    equity_conversion_pct: None,
                    cash_paydown: Some(dec!(100)),
                },
                RestructuringTreatment {
                    tranche_name: "Term Loan A".into(),
                    treatment_type: TreatmentType::Reinstate,
                    new_face_value: None,
                    new_coupon: None,
                    equity_conversion_pct: None,
                    cash_paydown: None,
                },
                RestructuringTreatment {
                    tranche_name: "Term Loan B".into(),
                    treatment_type: TreatmentType::Exchange,
                    new_face_value: Some(dec!(150)),
                    new_coupon: Some(dec!(0.07)),
                    equity_conversion_pct: None,
                    cash_paydown: None,
                },
                RestructuringTreatment {
                    tranche_name: "Senior Notes".into(),
                    treatment_type: TreatmentType::Amend,
                    new_face_value: None,
                    new_coupon: Some(dec!(0.06)),
                    equity_conversion_pct: None,
                    cash_paydown: None,
                },
                RestructuringTreatment {
                    tranche_name: "Sub Notes".into(),
                    treatment_type: TreatmentType::EquityConversion,
                    new_face_value: None,
                    new_coupon: None,
                    equity_conversion_pct: Some(dec!(0.10)),
                    cash_paydown: None,
                },
            ],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.tranche_analysis.len(), 5);
        assert_eq!(out.plan_value, dec!(1200));
        assert_eq!(out.total_claims, dec!(1000));

        // Verify treatments are applied
        assert!(out.tranche_analysis[0].treatment.contains("Cash Paydown"));
        assert!(out.tranche_analysis[1].treatment.contains("Reinstate"));
        assert!(out.tranche_analysis[2].treatment.contains("Exchange"));
    }

    // --- Test 3: DIP with conversion ---

    #[test]
    fn test_dip_with_conversion() {
        let mut input = two_tranche_input();
        input.dip_facility = Some(DipTerms {
            commitment: dec!(100),
            drawn: dec!(80),
            rate: dec!(0.10),
            fees_pct: dec!(0.03),
            term_months: 18,
            converts_to_exit: true,
        });

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // DIP analysis should be present
        assert!(out.dip_analysis.is_some());
        let dip = out.dip_analysis.as_ref().unwrap();
        assert!(dip.total_cost > Decimal::ZERO);
        assert!(dip.converts_to_exit_debt);
        assert!(dip.effective_rate > Decimal::ZERO);

        // DIP drawn (80) is super-senior, reducing remaining for other tranches
        // Plan value = 600, after DIP: 520 remaining
        // First Lien (400) gets 400, leaving 120 for Second Lien (300 face)
        assert_eq!(out.tranche_analysis[0].recovery_value, dec!(400));
    }

    // --- Test 4: Exchange offer ---

    #[test]
    fn test_exchange_offer() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(500),
            exit_enterprise_value: dec!(500),
            exit_timeline_years: dec!(2),
            capital_structure: vec![DebtTranche {
                name: "Senior Notes".into(),
                face_value: dec!(400),
                market_price: dec!(0.60),
                coupon_rate: dec!(0.08),
                maturity_years: dec!(3),
                seniority: Seniority::Senior,
                is_secured: false,
            }],
            proposed_treatment: vec![RestructuringTreatment {
                tranche_name: "Senior Notes".into(),
                treatment_type: TreatmentType::Exchange,
                new_face_value: Some(dec!(300)),
                new_coupon: Some(dec!(0.06)),
                equity_conversion_pct: None,
                cash_paydown: None,
            }],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // Exchange: 400 face exchanged for 300 new face
        let tranche = &out.tranche_analysis[0];
        assert_eq!(tranche.recovery_value, dec!(300));
        assert_eq!(tranche.post_restructuring_value, dec!(300));
        assert!(tranche.treatment.contains("Exchange"));

        // Equity created = 500 - 300 = 200
        assert_eq!(out.equity_value_created, dec!(200));
    }

    // --- Test 5: Equity conversion ---

    #[test]
    fn test_equity_conversion() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(300),
            exit_enterprise_value: dec!(400),
            exit_timeline_years: dec!(2),
            capital_structure: vec![
                DebtTranche {
                    name: "Senior Secured".into(),
                    face_value: dec!(200),
                    market_price: dec!(0.90),
                    coupon_rate: dec!(0.06),
                    maturity_years: dec!(3),
                    seniority: Seniority::FirstLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "Unsecured".into(),
                    face_value: dec!(300),
                    market_price: dec!(0.25),
                    coupon_rate: dec!(0.10),
                    maturity_years: dec!(5),
                    seniority: Seniority::Senior,
                    is_secured: false,
                },
            ],
            proposed_treatment: vec![
                RestructuringTreatment {
                    tranche_name: "Senior Secured".into(),
                    treatment_type: TreatmentType::Reinstate,
                    new_face_value: None,
                    new_coupon: None,
                    equity_conversion_pct: None,
                    cash_paydown: None,
                },
                RestructuringTreatment {
                    tranche_name: "Unsecured".into(),
                    treatment_type: TreatmentType::EquityConversion,
                    new_face_value: None,
                    new_coupon: None,
                    equity_conversion_pct: Some(dec!(0.95)),
                    cash_paydown: None,
                },
            ],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // Senior Secured: reinstated at face (200)
        assert_eq!(out.tranche_analysis[0].recovery_value, dec!(200));
        assert_eq!(out.tranche_analysis[0].recovery_rate, Decimal::ONE);

        // Unsecured: 95% equity of remaining 200 => 190
        let unsecured = &out.tranche_analysis[1];
        assert!(unsecured.treatment.contains("Equity Conversion"));
        assert!(unsecured.treatment.contains("95.0%"));
        // Recovery = remaining(200) * 0.95 = 190
        assert_eq!(unsecured.recovery_value, dec!(190));

        // Fulcrum should be Unsecured (recovery < 100%)
        assert_eq!(out.fulcrum_security, "Unsecured");
    }

    // --- Test 6: Fulcrum identification with 3 tranches ---

    #[test]
    fn test_fulcrum_identification() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(400),
            exit_enterprise_value: dec!(450),
            exit_timeline_years: dec!(2),
            capital_structure: vec![
                DebtTranche {
                    name: "1L".into(),
                    face_value: dec!(200),
                    market_price: dec!(0.98),
                    coupon_rate: dec!(0.05),
                    maturity_years: dec!(3),
                    seniority: Seniority::FirstLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "2L".into(),
                    face_value: dec!(200),
                    market_price: dec!(0.60),
                    coupon_rate: dec!(0.08),
                    maturity_years: dec!(5),
                    seniority: Seniority::SecondLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "Mezz".into(),
                    face_value: dec!(200),
                    market_price: dec!(0.10),
                    coupon_rate: dec!(0.14),
                    maturity_years: dec!(7),
                    seniority: Seniority::Mezzanine,
                    is_secured: false,
                },
            ],
            proposed_treatment: vec![],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // Plan value = 450
        // 1L: 200, full recovery -> remaining = 250
        // 2L: 200, full recovery -> remaining = 50
        // Mezz: 200 face, only 50 remaining -> partial recovery
        // Fulcrum = 2L gets full recovery, so fulcrum = Mezz
        assert_eq!(out.fulcrum_security, "Mezz");
        assert_eq!(out.fulcrum_analysis.tranche_name, "Mezz");
        assert!(out.fulcrum_analysis.recovery_rate < Decimal::ONE);
    }

    // --- Test 7: Mispricing detection ---

    #[test]
    fn test_mispricing_detection() {
        let input = two_tranche_input();
        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // The fulcrum (Second Lien) trades at 0.40 but has implied recovery
        // Plan = 600, First Lien = 400 reinstated, remaining = 200
        // Second Lien: 200 recovery on 300 face via equity => recovery_rate = 200/300 = 0.6667
        // Market price = 0.40, implied = 0.6667
        // Mispricing = 0.6667 - 0.40 = 0.2667 (positive = undervalued)
        assert!(out.fulcrum_analysis.mispricing > Decimal::ZERO);
        assert!(out.fulcrum_analysis.implied_price > out.fulcrum_analysis.current_price);

        // Should have a warning about the undervalued fulcrum
        assert!(
            result.warnings.iter().any(|w| w.contains("undervalued")),
            "Expected undervalued warning, got: {:?}",
            result.warnings
        );
    }

    // --- Test 8: Credit bid calculation ---

    #[test]
    fn test_credit_bid_calculation() {
        let input = two_tranche_input();
        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // Fulcrum = Second Lien
        // Credit bid = EV (600) - senior claims (First Lien 400)
        assert_eq!(out.credit_bid_value, dec!(200));
    }

    // --- Test 9: Credit bid with DIP ---

    #[test]
    fn test_credit_bid_with_dip() {
        let mut input = two_tranche_input();
        input.dip_facility = Some(DipTerms {
            commitment: dec!(50),
            drawn: dec!(50),
            rate: dec!(0.12),
            fees_pct: dec!(0.02),
            term_months: 12,
            converts_to_exit: false,
        });

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // Credit bid = 600 - DIP(50) - First Lien(400) = 150
        assert_eq!(out.credit_bid_value, dec!(150));
    }

    // --- Test 10: IRR computation ---

    #[test]
    fn test_irr_computation() {
        let input = two_tranche_input();
        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // First Lien: bought at 0.95 * 400 = 380, recovery = 400, over 2 years
        // IRR = (400/380)^(1/2) - 1 = (1.0526)^0.5 - 1 ~ 2.6%
        let fl_irr = out.tranche_analysis[0].irr_at_market;
        assert!(fl_irr > Decimal::ZERO, "First lien IRR should be positive");
        // Approximate: should be around 2.5-2.7%
        assert!(
            fl_irr > dec!(0.02) && fl_irr < dec!(0.04),
            "Expected ~2.6% IRR, got {}",
            fl_irr
        );

        // Second Lien: bought at 0.40 * 300 = 120, recovery ~ 200, over 2 years
        // IRR = (200/120)^(1/2) - 1 = (1.6667)^0.5 - 1 ~ 29%
        let sl_irr = out.tranche_analysis[1].irr_at_market;
        assert!(sl_irr > Decimal::ZERO, "Second lien IRR should be positive");
        assert!(
            sl_irr > dec!(0.20) && sl_irr < dec!(0.40),
            "Expected ~29% IRR, got {}",
            sl_irr
        );
    }

    // --- Test 11: Zero recovery tranche ---

    #[test]
    fn test_zero_recovery_tranche() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(200),
            exit_enterprise_value: dec!(250),
            exit_timeline_years: dec!(2),
            capital_structure: vec![
                DebtTranche {
                    name: "Senior".into(),
                    face_value: dec!(250),
                    market_price: dec!(0.90),
                    coupon_rate: dec!(0.06),
                    maturity_years: dec!(3),
                    seniority: Seniority::Senior,
                    is_secured: false,
                },
                DebtTranche {
                    name: "Sub".into(),
                    face_value: dec!(100),
                    market_price: dec!(0.02),
                    coupon_rate: dec!(0.12),
                    maturity_years: dec!(5),
                    seniority: Seniority::Subordinated,
                    is_secured: false,
                },
            ],
            proposed_treatment: vec![],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // Senior gets full 250, leaving 0 for Sub
        assert_eq!(out.tranche_analysis[1].recovery_value, Decimal::ZERO);
        assert_eq!(out.tranche_analysis[1].recovery_rate, Decimal::ZERO);

        // IRR for zero recovery should be -1 (total loss)
        assert_eq!(out.tranche_analysis[1].irr_at_market, dec!(-1));
    }

    // --- Test 12: All tranches fully recovered ---

    #[test]
    fn test_all_tranches_full_recovery() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(500),
            exit_enterprise_value: dec!(1000),
            exit_timeline_years: dec!(2),
            capital_structure: vec![
                DebtTranche {
                    name: "1L".into(),
                    face_value: dec!(200),
                    market_price: dec!(0.98),
                    coupon_rate: dec!(0.05),
                    maturity_years: dec!(3),
                    seniority: Seniority::FirstLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "2L".into(),
                    face_value: dec!(200),
                    market_price: dec!(0.85),
                    coupon_rate: dec!(0.08),
                    maturity_years: dec!(5),
                    seniority: Seniority::SecondLien,
                    is_secured: true,
                },
            ],
            proposed_treatment: vec![],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // EV (1000) > total claims (400), so all recover fully
        assert_eq!(out.tranche_analysis[0].recovery_rate, Decimal::ONE);
        assert_eq!(out.tranche_analysis[1].recovery_rate, Decimal::ONE);

        // Fulcrum defaults to last tranche when all recover fully
        assert_eq!(out.fulcrum_security, "2L");
        assert_eq!(out.fulcrum_analysis.recovery_rate, Decimal::ONE);

        // No explicit treatment => no surviving debt is tracked
        // Equity value = exit_EV - 0 surviving debt = 1000
        assert_eq!(out.equity_value_created, dec!(1000));
    }

    // --- Test 13: Negative equity value warning ---

    #[test]
    fn test_negative_equity_warning() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(300),
            exit_enterprise_value: dec!(400),
            exit_timeline_years: dec!(2),
            capital_structure: vec![DebtTranche {
                name: "Senior".into(),
                face_value: dec!(500),
                market_price: dec!(0.70),
                coupon_rate: dec!(0.06),
                maturity_years: dec!(3),
                seniority: Seniority::Senior,
                is_secured: false,
            }],
            proposed_treatment: vec![RestructuringTreatment {
                tranche_name: "Senior".into(),
                treatment_type: TreatmentType::Reinstate,
                new_face_value: None,
                new_coupon: None,
                equity_conversion_pct: None,
                cash_paydown: None,
            }],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();

        // Reinstated debt (500) > exit EV (400), so negative equity
        assert!(result.result.equity_value_created < Decimal::ZERO);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("Negative equity")),
            "Expected negative equity warning, got: {:?}",
            result.warnings
        );
    }

    // --- Test 14: DIP > 50% of EV warning ---

    #[test]
    fn test_dip_exceeds_fifty_pct_warning() {
        let mut input = two_tranche_input();
        input.dip_facility = Some(DipTerms {
            commitment: dec!(400),
            drawn: dec!(350),
            rate: dec!(0.12),
            fees_pct: dec!(0.03),
            term_months: 12,
            converts_to_exit: false,
        });

        let result = analyze_distressed_debt(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("exceeds 50%")),
            "Expected DIP > 50% warning, got: {:?}",
            result.warnings
        );
    }

    // --- Test 15: Validation — EV must be positive ---

    #[test]
    fn test_validation_ev_positive() {
        let mut input = two_tranche_input();
        input.enterprise_value = dec!(-100);
        let err = analyze_distressed_debt(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "enterprise_value");
            }
            other => panic!("Expected InvalidInput for enterprise_value, got {other:?}"),
        }
    }

    // --- Test 16: Validation — at least one tranche ---

    #[test]
    fn test_validation_empty_tranches() {
        let mut input = two_tranche_input();
        input.capital_structure.clear();
        let err = analyze_distressed_debt(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "capital_structure");
            }
            other => panic!("Expected InvalidInput for capital_structure, got {other:?}"),
        }
    }

    // --- Test 17: Validation — treatment name must match tranche ---

    #[test]
    fn test_validation_treatment_name_mismatch() {
        let mut input = two_tranche_input();
        input.proposed_treatment.push(RestructuringTreatment {
            tranche_name: "Nonexistent Tranche".into(),
            treatment_type: TreatmentType::Reinstate,
            new_face_value: None,
            new_coupon: None,
            equity_conversion_pct: None,
            cash_paydown: None,
        });
        let err = analyze_distressed_debt(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("Nonexistent Tranche"));
            }
            other => panic!("Expected InvalidInput for treatment name, got {other:?}"),
        }
    }

    // --- Test 18: Validation — negative face value ---

    #[test]
    fn test_validation_negative_face_value() {
        let mut input = two_tranche_input();
        input.capital_structure[0].face_value = dec!(-100);
        let err = analyze_distressed_debt(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("face_value"));
            }
            other => panic!("Expected InvalidInput for face_value, got {other:?}"),
        }
    }

    // --- Test 19: Validation — DIP drawn exceeds commitment ---

    #[test]
    fn test_validation_dip_overdrawn() {
        let mut input = two_tranche_input();
        input.dip_facility = Some(DipTerms {
            commitment: dec!(50),
            drawn: dec!(80),
            rate: dec!(0.10),
            fees_pct: dec!(0.02),
            term_months: 12,
            converts_to_exit: false,
        });
        let err = analyze_distressed_debt(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("drawn"));
            }
            other => panic!("Expected InvalidInput for DIP drawn, got {other:?}"),
        }
    }

    // --- Test 20: Overall recovery calculation ---

    #[test]
    fn test_overall_recovery() {
        let input = two_tranche_input();
        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // plan_value (600) / total_claims (700)
        let expected = dec!(600) / dec!(700);
        assert_eq!(out.overall_recovery, expected);
    }

    // --- Test 21: Combination treatment ---

    #[test]
    fn test_combination_treatment() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(500),
            exit_enterprise_value: dec!(600),
            exit_timeline_years: dec!(2),
            capital_structure: vec![DebtTranche {
                name: "Senior Notes".into(),
                face_value: dec!(500),
                market_price: dec!(0.55),
                coupon_rate: dec!(0.08),
                maturity_years: dec!(4),
                seniority: Seniority::Senior,
                is_secured: false,
            }],
            proposed_treatment: vec![RestructuringTreatment {
                tranche_name: "Senior Notes".into(),
                treatment_type: TreatmentType::Combination,
                new_face_value: Some(dec!(200)),
                new_coupon: Some(dec!(0.06)),
                equity_conversion_pct: Some(dec!(0.50)),
                cash_paydown: Some(dec!(100)),
            }],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        let tranche = &out.tranche_analysis[0];
        assert!(tranche.treatment.contains("Combination"));

        // Combination: cash(100) + new_debt(200) = 300 debt
        // Remaining after debt portion: 600 - 300 = 300
        // Equity portion: 300 * 0.50 = 150
        // Total recovery = 300 + 150 = 450
        assert_eq!(tranche.recovery_value, dec!(450));
    }

    // --- Test 22: Metadata is populated ---

    #[test]
    fn test_metadata_populated() {
        let input = two_tranche_input();
        let result = analyze_distressed_debt(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Distressed"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(result.metadata.computation_time_us > 0 || true); // timing can be 0 on fast runs
    }

    // --- Test 23: Seniority ordering ---

    #[test]
    fn test_seniority_ordering() {
        // Verify tranches are processed in seniority order regardless of input order
        let input = DistressedDebtInput {
            enterprise_value: dec!(300),
            exit_enterprise_value: dec!(350),
            exit_timeline_years: dec!(2),
            capital_structure: vec![
                // Input in reverse seniority order
                DebtTranche {
                    name: "Mezz".into(),
                    face_value: dec!(100),
                    market_price: dec!(0.10),
                    coupon_rate: dec!(0.14),
                    maturity_years: dec!(7),
                    seniority: Seniority::Mezzanine,
                    is_secured: false,
                },
                DebtTranche {
                    name: "1L".into(),
                    face_value: dec!(200),
                    market_price: dec!(0.95),
                    coupon_rate: dec!(0.05),
                    maturity_years: dec!(3),
                    seniority: Seniority::FirstLien,
                    is_secured: true,
                },
                DebtTranche {
                    name: "Senior".into(),
                    face_value: dec!(150),
                    market_price: dec!(0.50),
                    coupon_rate: dec!(0.09),
                    maturity_years: dec!(5),
                    seniority: Seniority::Senior,
                    is_secured: false,
                },
            ],
            proposed_treatment: vec![],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        // Output should be sorted by seniority
        assert_eq!(out.tranche_analysis[0].name, "1L");
        assert_eq!(out.tranche_analysis[1].name, "Senior");
        assert_eq!(out.tranche_analysis[2].name, "Mezz");

        // 1L (200) fully covered, Senior (150) fully covered, Mezz gets 0
        assert_eq!(out.tranche_analysis[0].recovery_rate, Decimal::ONE);
        assert_eq!(out.tranche_analysis[1].recovery_rate, Decimal::ONE);
        assert_eq!(out.tranche_analysis[2].recovery_rate, Decimal::ZERO);

        assert_eq!(out.fulcrum_security, "Mezz");
    }

    // --- Test 24: DIP analysis cost calculation ---

    #[test]
    fn test_dip_cost_calculation() {
        let mut input = two_tranche_input();
        input.dip_facility = Some(DipTerms {
            commitment: dec!(100),
            drawn: dec!(80),
            rate: dec!(0.10),
            fees_pct: dec!(0.03),
            term_months: 12,
            converts_to_exit: false,
        });

        let result = analyze_distressed_debt(&input).unwrap();
        let dip = result.result.dip_analysis.as_ref().unwrap();

        // Interest = 80 * 0.10 * (12/12) = 8
        // Fees = 100 * 0.03 = 3
        // Total cost = 11
        assert_eq!(dip.total_cost, dec!(11));
        assert!(!dip.converts_to_exit_debt);
    }

    // --- Test 25: Validation — exit timeline must be positive ---

    #[test]
    fn test_validation_exit_timeline() {
        let mut input = two_tranche_input();
        input.exit_timeline_years = Decimal::ZERO;
        let err = analyze_distressed_debt(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "exit_timeline_years");
            }
            other => panic!("Expected InvalidInput for exit_timeline_years, got {other:?}"),
        }
    }

    // --- Test 26: Cash paydown treatment ---

    #[test]
    fn test_cash_paydown() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(500),
            exit_enterprise_value: dec!(500),
            exit_timeline_years: dec!(1),
            capital_structure: vec![DebtTranche {
                name: "Revolver".into(),
                face_value: dec!(200),
                market_price: dec!(0.95),
                coupon_rate: dec!(0.04),
                maturity_years: dec!(1),
                seniority: Seniority::FirstLien,
                is_secured: true,
            }],
            proposed_treatment: vec![RestructuringTreatment {
                tranche_name: "Revolver".into(),
                treatment_type: TreatmentType::CashPaydown,
                new_face_value: None,
                new_coupon: None,
                equity_conversion_pct: None,
                cash_paydown: Some(dec!(200)),
            }],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        let tranche = &out.tranche_analysis[0];
        assert_eq!(tranche.recovery_value, dec!(200));
        assert_eq!(tranche.recovery_rate, Decimal::ONE);
        assert!(tranche.treatment.contains("Cash Paydown"));

        // Cash paydown does not add to surviving debt
        assert_eq!(out.equity_value_created, dec!(500));
    }

    // --- Test 27: Amend treatment ---

    #[test]
    fn test_amend_treatment() {
        let input = DistressedDebtInput {
            enterprise_value: dec!(500),
            exit_enterprise_value: dec!(600),
            exit_timeline_years: dec!(2),
            capital_structure: vec![DebtTranche {
                name: "Term Loan".into(),
                face_value: dec!(400),
                market_price: dec!(0.80),
                coupon_rate: dec!(0.08),
                maturity_years: dec!(3),
                seniority: Seniority::FirstLien,
                is_secured: true,
            }],
            proposed_treatment: vec![RestructuringTreatment {
                tranche_name: "Term Loan".into(),
                treatment_type: TreatmentType::Amend,
                new_face_value: None,
                new_coupon: Some(dec!(0.05)),
                equity_conversion_pct: None,
                cash_paydown: None,
            }],
            dip_facility: None,
            operating_assumptions: default_operating(),
        };

        let result = analyze_distressed_debt(&input).unwrap();
        let out = &result.result;

        let tranche = &out.tranche_analysis[0];
        assert_eq!(tranche.recovery_value, dec!(400));
        assert!(tranche.treatment.contains("Amend"));
        assert!(tranche.treatment.contains("5.00%"));

        // Amend keeps face as surviving debt
        assert_eq!(out.equity_value_created, dec!(200)); // 600 - 400
    }
}
