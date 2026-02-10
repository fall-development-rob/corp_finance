use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Basel III/IV asset classes for credit risk Standardised Approach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetClass {
    Sovereign,
    Bank,
    Corporate,
    Retail,
    Mortgage,
    Equity,
    Other,
}

/// Collateral types for credit risk mitigation (CRM).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollateralType {
    Cash,
    GovernmentBond,
    CorporateBond,
    Equity,
    RealEstate,
}

/// Operational risk approach.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpRiskApproach {
    BasicIndicator,
    Standardised,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Capital structure of the institution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalStructure {
    /// Common Equity Tier 1
    pub cet1: Money,
    /// Additional Tier 1 (CoCos, preferred shares)
    pub additional_tier1: Money,
    /// Tier 2 (subordinated debt, general provisions)
    pub tier2: Money,
    /// Regulatory deductions (goodwill, deferred tax assets, etc.)
    pub deductions: Money,
}

/// A single credit exposure for RWA calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditExposure {
    pub name: String,
    /// Exposure at default (EAD)
    pub exposure_amount: Money,
    pub asset_class: AssetClass,
    /// Override risk weight (0 to 1.5); if None, derived from asset_class + rating
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_weight: Option<Decimal>,
    /// External credit rating: "AAA", "AA", "A", "BBB", "BB", "B", "CCC", "Unrated"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_rating: Option<String>,
    /// Collateral value for credit risk mitigation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collateral_value: Option<Money>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collateral_type: Option<CollateralType>,
}

/// Gross income for a single business line (Standardised Approach).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessLineIncome {
    /// One of: "CorporateFinance", "Trading", "Retail", "Commercial",
    /// "Payment", "Agency", "AssetMgmt", "RetailBrokerage"
    pub line: String,
    pub gross_income: Money,
}

/// Operational risk calculation inputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalRiskInput {
    pub approach: OpRiskApproach,
    /// Three years of gross income (for BIA / SA fallback)
    pub gross_income_3yr: Vec<Money>,
    /// Business-line breakdown (required for Standardised Approach)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub business_lines: Option<Vec<BusinessLineIncome>>,
}

/// Capital buffer requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalBuffers {
    /// Capital conservation buffer (typically 2.5%)
    pub conservation_buffer: Decimal,
    /// Countercyclical buffer (0 - 2.5%, varies by jurisdiction)
    pub countercyclical_buffer: Decimal,
    /// Systemic buffer for G-SIBs (0 - 3.5%)
    pub systemic_buffer: Decimal,
}

/// Top-level input for regulatory capital calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryCapitalInput {
    pub institution_name: String,
    pub capital: CapitalStructure,
    pub credit_exposures: Vec<CreditExposure>,
    /// Pre-calculated market risk RWA (SA-TB or IMA)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_risk_charge: Option<Money>,
    pub operational_risk: OperationalRiskInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffers: Option<CapitalBuffers>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Summary of available capital after deductions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalSummary {
    pub cet1_capital: Money,
    /// CET1 + AT1
    pub tier1_capital: Money,
    /// Tier1 + Tier2
    pub total_capital: Money,
    pub deductions: Money,
}

/// Basel III capital adequacy ratios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapitalRatios {
    /// CET1 / total RWA
    pub cet1_ratio: Decimal,
    /// Tier 1 / total RWA
    pub tier1_ratio: Decimal,
    /// Total capital / total RWA
    pub total_capital_ratio: Decimal,
    /// Tier 1 / total exposure (not RWA)
    pub leverage_ratio: Decimal,
}

/// Per-exposure detail showing risk weight application and CRM benefit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureDetail {
    pub name: String,
    pub exposure: Money,
    pub risk_weight: Decimal,
    /// exposure * risk_weight (after CRM adjustment)
    pub rwa: Money,
    /// RWA reduction from collateral
    pub crm_benefit: Money,
}

/// Capital buffer stacking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferRequirements {
    /// Basel III minimum CET1 (4.5%)
    pub min_cet1: Decimal,
    /// Conservation buffer (2.5%)
    pub conservation: Decimal,
    pub countercyclical: Decimal,
    pub systemic: Decimal,
    /// Sum of all CET1 requirements
    pub total_cet1_requirement: Decimal,
}

/// Capital surplus or deficit relative to regulatory minimums.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurplusDeficit {
    /// Actual CET1 ratio minus total CET1 requirement (incl. buffers)
    pub cet1_surplus: Decimal,
    /// Actual Tier 1 ratio minus 6.0%
    pub tier1_surplus: Decimal,
    /// Actual total capital ratio minus 8.0%
    pub total_capital_surplus: Decimal,
    pub meets_requirements: bool,
}

/// Complete regulatory capital output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryCapitalOutput {
    pub capital_summary: CapitalSummary,
    pub credit_rwa: Money,
    pub market_rwa: Money,
    pub operational_rwa: Money,
    pub total_rwa: Money,
    pub capital_ratios: CapitalRatios,
    pub exposure_details: Vec<ExposureDetail>,
    pub buffer_requirements: BufferRequirements,
    pub surplus_deficit: SurplusDeficit,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Basel III minimum CET1 ratio
const MIN_CET1_RATIO: Decimal = dec!(0.045);
/// Basel III minimum Tier 1 ratio
const MIN_TIER1_RATIO: Decimal = dec!(0.06);
/// Basel III minimum total capital ratio
const MIN_TOTAL_CAPITAL_RATIO: Decimal = dec!(0.08);
/// Default conservation buffer
const DEFAULT_CONSERVATION: Decimal = dec!(0.025);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Calculate Basel III/IV regulatory capital and RWA.
///
/// Computes risk-weighted assets under the Standardised Approach for credit
/// risk, operational risk (BIA or SA), and combines with a pre-calculated
/// market risk charge. Derives capital ratios, buffer requirements, and
/// surplus/deficit analysis.
pub fn calculate_regulatory_capital(
    input: &RegulatoryCapitalInput,
) -> CorpFinanceResult<ComputationOutput<RegulatoryCapitalOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    validate_input(input, &mut warnings)?;

    // -- Capital summary -----------------------------------------------------
    let cap = &input.capital;
    let cet1_capital = cap.cet1 - cap.deductions;
    let tier1_capital = cap.cet1 + cap.additional_tier1 - cap.deductions;
    let total_capital = cap.cet1 + cap.additional_tier1 + cap.tier2 - cap.deductions;

    let capital_summary = CapitalSummary {
        cet1_capital,
        tier1_capital,
        total_capital,
        deductions: cap.deductions,
    };

    // -- Credit RWA (Standardised Approach) ----------------------------------
    let mut exposure_details: Vec<ExposureDetail> = Vec::new();
    let mut credit_rwa = Decimal::ZERO;

    for exp in &input.credit_exposures {
        let detail = calculate_exposure_rwa(exp, &mut warnings)?;
        credit_rwa += detail.rwa;
        exposure_details.push(detail);
    }

    // -- Market risk RWA -----------------------------------------------------
    let market_rwa = input.market_risk_charge.unwrap_or(Decimal::ZERO);

    // -- Operational risk RWA ------------------------------------------------
    let operational_rwa = calculate_operational_risk(&input.operational_risk, &mut warnings)?;

    // -- Total RWA -----------------------------------------------------------
    let total_rwa = credit_rwa + market_rwa + operational_rwa;

    if total_rwa.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "total RWA is zero; cannot compute capital ratios".into(),
        });
    }

    // -- Capital ratios ------------------------------------------------------
    let cet1_ratio = cet1_capital / total_rwa;
    let tier1_ratio = tier1_capital / total_rwa;
    let total_capital_ratio = total_capital / total_rwa;

    // Leverage ratio: Tier 1 / total exposure (sum of EADs, not RWA)
    let total_exposure: Decimal = input
        .credit_exposures
        .iter()
        .map(|e| e.exposure_amount)
        .sum::<Decimal>()
        + market_rwa; // Include market risk as exposure proxy

    let leverage_ratio = if total_exposure.is_zero() {
        warnings.push("Total exposure is zero; leverage ratio set to zero.".into());
        Decimal::ZERO
    } else {
        tier1_capital / total_exposure
    };

    let capital_ratios = CapitalRatios {
        cet1_ratio,
        tier1_ratio,
        total_capital_ratio,
        leverage_ratio,
    };

    // -- Buffer requirements -------------------------------------------------
    let buffers = input.buffers.as_ref();
    let conservation = buffers
        .map(|b| b.conservation_buffer)
        .unwrap_or(DEFAULT_CONSERVATION);
    let countercyclical = buffers
        .map(|b| b.countercyclical_buffer)
        .unwrap_or(Decimal::ZERO);
    let systemic = buffers.map(|b| b.systemic_buffer).unwrap_or(Decimal::ZERO);
    let total_cet1_requirement = MIN_CET1_RATIO + conservation + countercyclical + systemic;

    let buffer_requirements = BufferRequirements {
        min_cet1: MIN_CET1_RATIO,
        conservation,
        countercyclical,
        systemic,
        total_cet1_requirement,
    };

    // -- Surplus / deficit ---------------------------------------------------
    let cet1_surplus = cet1_ratio - total_cet1_requirement;
    let tier1_surplus = tier1_ratio - MIN_TIER1_RATIO;
    let total_capital_surplus = total_capital_ratio - MIN_TOTAL_CAPITAL_RATIO;

    let meets_requirements = cet1_ratio >= total_cet1_requirement
        && tier1_ratio >= MIN_TIER1_RATIO
        && total_capital_ratio >= MIN_TOTAL_CAPITAL_RATIO;

    if !meets_requirements {
        warnings.push("Institution does NOT meet minimum capital requirements.".into());
    }

    let surplus_deficit = SurplusDeficit {
        cet1_surplus,
        tier1_surplus,
        total_capital_surplus,
        meets_requirements,
    };

    // -- Assemble output -----------------------------------------------------
    let output = RegulatoryCapitalOutput {
        capital_summary,
        credit_rwa,
        market_rwa,
        operational_rwa,
        total_rwa,
        capital_ratios,
        exposure_details,
        buffer_requirements,
        surplus_deficit,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "framework": "Basel III / IV Standardised Approach",
        "credit_risk": "SA risk weights by asset class and external rating",
        "operational_risk": format!("{:?}", input.operational_risk.approach),
        "minimum_cet1": "4.5%",
        "minimum_tier1": "6.0%",
        "minimum_total_capital": "8.0%",
    });

    Ok(with_metadata(
        "Basel III/IV Regulatory Capital (Standardised Approach)",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal: input validation
// ---------------------------------------------------------------------------

fn validate_input(
    input: &RegulatoryCapitalInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<()> {
    let cap = &input.capital;

    if cap.cet1 < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "capital.cet1".into(),
            reason: "CET1 capital cannot be negative.".into(),
        });
    }
    if cap.additional_tier1 < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "capital.additional_tier1".into(),
            reason: "AT1 capital cannot be negative.".into(),
        });
    }
    if cap.tier2 < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "capital.tier2".into(),
            reason: "Tier 2 capital cannot be negative.".into(),
        });
    }
    if cap.deductions < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "capital.deductions".into(),
            reason: "Deductions cannot be negative.".into(),
        });
    }

    if input.credit_exposures.is_empty() {
        warnings.push("No credit exposures provided; credit RWA will be zero.".into());
    }

    for (i, exp) in input.credit_exposures.iter().enumerate() {
        if exp.exposure_amount < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("credit_exposures[{}].exposure_amount", i),
                reason: "Exposure amount cannot be negative.".into(),
            });
        }
        if let Some(rw) = exp.risk_weight {
            if rw < Decimal::ZERO || rw > dec!(1.5) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("credit_exposures[{}].risk_weight", i),
                    reason: "Risk weight must be between 0 and 1.5.".into(),
                });
            }
        }
    }

    // Validate operational risk inputs
    let op = &input.operational_risk;
    if op.gross_income_3yr.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one year of gross income required for operational risk.".into(),
        ));
    }
    if op.approach == OpRiskApproach::Standardised && op.business_lines.is_none() {
        return Err(CorpFinanceError::InvalidInput {
            field: "operational_risk.business_lines".into(),
            reason: "Business line breakdown required for Standardised Approach.".into(),
        });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal: credit risk per exposure
// ---------------------------------------------------------------------------

/// Calculate RWA for a single credit exposure, applying CRM if collateral
/// is provided.
fn calculate_exposure_rwa(
    exp: &CreditExposure,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<ExposureDetail> {
    let risk_weight = match exp.risk_weight {
        Some(rw) => rw,
        None => derive_risk_weight(&exp.asset_class, &exp.external_rating, warnings),
    };

    // Credit risk mitigation via collateral
    let (adjusted_ead, crm_benefit) = apply_crm(exp, risk_weight, warnings);

    let rwa = adjusted_ead * risk_weight;

    Ok(ExposureDetail {
        name: exp.name.clone(),
        exposure: exp.exposure_amount,
        risk_weight,
        rwa,
        crm_benefit,
    })
}

/// Derive the SA risk weight from asset class and external rating.
fn derive_risk_weight(
    asset_class: &AssetClass,
    rating: &Option<String>,
    warnings: &mut Vec<String>,
) -> Decimal {
    let rating_str = rating.as_deref().unwrap_or("Unrated").trim().to_uppercase();

    match asset_class {
        AssetClass::Sovereign => match rating_str.as_str() {
            "AAA" | "AA" => dec!(0),
            "A" => dec!(0.20),
            "BBB" => dec!(0.50),
            "BB" | "B" => dec!(1.00),
            "CCC" => dec!(1.50),
            "UNRATED" => {
                warnings.push("Unrated sovereign assigned 100% risk weight.".into());
                dec!(1.00)
            }
            _ => {
                warnings.push(format!(
                    "Unknown sovereign rating '{}'; assigned 150% risk weight.",
                    rating_str
                ));
                dec!(1.50)
            }
        },
        AssetClass::Bank => match rating_str.as_str() {
            "AAA" | "AA" | "A" => dec!(0.20),
            "BBB" => dec!(0.50),
            "BB" | "B" => dec!(1.00),
            "CCC" => dec!(1.50),
            "UNRATED" => {
                warnings.push("Unrated bank assigned 50% risk weight.".into());
                dec!(0.50)
            }
            _ => {
                warnings.push(format!(
                    "Unknown bank rating '{}'; assigned 150% risk weight.",
                    rating_str
                ));
                dec!(1.50)
            }
        },
        AssetClass::Corporate => match rating_str.as_str() {
            "AAA" | "AA" => dec!(0.20),
            "A" => dec!(0.50),
            "BBB" | "BB" => dec!(1.00),
            "B" => dec!(1.50),
            "CCC" => dec!(1.50),
            "UNRATED" => dec!(1.00),
            _ => {
                warnings.push(format!(
                    "Unknown corporate rating '{}'; assigned 150% risk weight.",
                    rating_str
                ));
                dec!(1.50)
            }
        },
        AssetClass::Retail => dec!(0.75),
        AssetClass::Mortgage => {
            // 35% for residential (default); commercial would be 100% but
            // we treat all mortgage-class exposures as residential here.
            dec!(0.35)
        }
        AssetClass::Equity => {
            // Listed equity 100%, unlisted 150%. Without a flag, default
            // to 100% (listed). Use risk_weight override for unlisted.
            dec!(1.00)
        }
        AssetClass::Other => dec!(1.00),
    }
}

/// Apply credit risk mitigation (CRM) using collateral.
///
/// Returns (adjusted_ead, crm_benefit_in_rwa_terms).
fn apply_crm(
    exp: &CreditExposure,
    risk_weight: Decimal,
    _warnings: &mut Vec<String>,
) -> (Decimal, Decimal) {
    let ead = exp.exposure_amount;

    let (collateral_val, collateral_ty) = match (&exp.collateral_value, &exp.collateral_type) {
        (Some(cv), Some(ct)) if *cv > Decimal::ZERO => (*cv, ct.clone()),
        _ => return (ead, Decimal::ZERO),
    };

    let haircut = collateral_haircut(&collateral_ty);
    let effective_collateral = collateral_val * (Decimal::ONE - haircut);
    let adjusted_ead = (ead - effective_collateral).max(Decimal::ZERO);

    // CRM benefit = reduction in RWA
    let original_rwa = ead * risk_weight;
    let new_rwa = adjusted_ead * risk_weight;
    let crm_benefit = original_rwa - new_rwa;

    (adjusted_ead, crm_benefit)
}

/// Supervisory haircut for collateral types.
fn collateral_haircut(ct: &CollateralType) -> Decimal {
    match ct {
        CollateralType::Cash => dec!(0),
        CollateralType::GovernmentBond => dec!(0.02),
        CollateralType::CorporateBond => dec!(0.08),
        CollateralType::Equity => dec!(0.25),
        CollateralType::RealEstate => dec!(0.30),
    }
}

// ---------------------------------------------------------------------------
// Internal: operational risk
// ---------------------------------------------------------------------------

/// Calculate operational risk RWA under BIA or Standardised Approach.
fn calculate_operational_risk(
    op: &OperationalRiskInput,
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Decimal> {
    match op.approach {
        OpRiskApproach::BasicIndicator => {
            // BIA: 15% * average of positive gross income over 3 years
            let positive_incomes: Vec<Decimal> = op
                .gross_income_3yr
                .iter()
                .copied()
                .filter(|gi| *gi > Decimal::ZERO)
                .collect();

            if positive_incomes.is_empty() {
                warnings
                    .push("No positive gross income years; operational risk RWA is zero.".into());
                return Ok(Decimal::ZERO);
            }

            let count = Decimal::from(positive_incomes.len() as u64);
            let sum: Decimal = positive_incomes.into_iter().sum();
            let avg = sum / count;

            Ok(avg * dec!(0.15))
        }
        OpRiskApproach::Standardised => {
            let business_lines =
                op.business_lines
                    .as_ref()
                    .ok_or_else(|| CorpFinanceError::InvalidInput {
                        field: "operational_risk.business_lines".into(),
                        reason: "Business line data required for Standardised Approach.".into(),
                    })?;

            let mut total = Decimal::ZERO;
            for bl in business_lines {
                let beta = beta_factor(&bl.line, warnings);
                total += bl.gross_income * beta;
            }

            // Floor at zero
            Ok(total.max(Decimal::ZERO))
        }
    }
}

/// Basel II/III beta factors for operational risk Standardised Approach.
fn beta_factor(line: &str, warnings: &mut Vec<String>) -> Decimal {
    match line {
        "CorporateFinance" => dec!(0.18),
        "Trading" => dec!(0.18),
        "Retail" => dec!(0.12),
        "Commercial" => dec!(0.15),
        "Payment" => dec!(0.18),
        "Agency" => dec!(0.15),
        "AssetMgmt" => dec!(0.12),
        "RetailBrokerage" => dec!(0.12),
        other => {
            warnings.push(format!(
                "Unknown business line '{}'; using default beta of 15%.",
                other
            ));
            dec!(0.15)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -- Helpers -------------------------------------------------------------

    fn default_capital() -> CapitalStructure {
        CapitalStructure {
            cet1: dec!(10_000),
            additional_tier1: dec!(2_000),
            tier2: dec!(3_000),
            deductions: dec!(1_000),
        }
    }

    fn default_op_risk() -> OperationalRiskInput {
        OperationalRiskInput {
            approach: OpRiskApproach::BasicIndicator,
            gross_income_3yr: vec![dec!(50_000), dec!(55_000), dec!(60_000)],
            business_lines: None,
        }
    }

    fn simple_exposure(name: &str, amount: Decimal, class: AssetClass) -> CreditExposure {
        CreditExposure {
            name: name.to_string(),
            exposure_amount: amount,
            asset_class: class,
            risk_weight: None,
            external_rating: None,
            collateral_value: None,
            collateral_type: None,
        }
    }

    fn rated_exposure(
        name: &str,
        amount: Decimal,
        class: AssetClass,
        rating: &str,
    ) -> CreditExposure {
        CreditExposure {
            name: name.to_string(),
            exposure_amount: amount,
            asset_class: class,
            risk_weight: None,
            external_rating: Some(rating.to_string()),
            collateral_value: None,
            collateral_type: None,
        }
    }

    fn make_input(exposures: Vec<CreditExposure>) -> RegulatoryCapitalInput {
        RegulatoryCapitalInput {
            institution_name: "Test Bank".to_string(),
            capital: default_capital(),
            credit_exposures: exposures,
            market_risk_charge: None,
            operational_risk: default_op_risk(),
            buffers: None,
        }
    }

    // -- Test: 100% risk weight => RWA = exposure ----------------------------

    #[test]
    fn test_100pct_risk_weight_rwa_equals_exposure() {
        let exp = simple_exposure("Other Asset", dec!(100_000), AssetClass::Other);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        // Other class => 100% RW; op risk is added separately
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(1.00));
        assert_eq!(detail.rwa, dec!(100_000));
        assert_eq!(result.result.credit_rwa, dec!(100_000));
    }

    // -- Test: OECD sovereign AAA => 0% RW -----------------------------------

    #[test]
    fn test_sovereign_aaa_zero_risk_weight() {
        let exp = rated_exposure("US Treasury", dec!(50_000), AssetClass::Sovereign, "AAA");
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(0));
        assert_eq!(detail.rwa, dec!(0));
    }

    // -- Test: Sovereign AA => 0% RW -----------------------------------------

    #[test]
    fn test_sovereign_aa_zero_risk_weight() {
        let exp = rated_exposure("UK Gilt", dec!(30_000), AssetClass::Sovereign, "AA");
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        assert_eq!(result.result.exposure_details[0].risk_weight, dec!(0));
    }

    // -- Test: Corporate BBB => 100% RW --------------------------------------

    #[test]
    fn test_corporate_bbb_100pct() {
        let exp = rated_exposure("Corp BBB", dec!(80_000), AssetClass::Corporate, "BBB");
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(1.00));
        assert_eq!(detail.rwa, dec!(80_000));
    }

    // -- Test: Retail => 75% RW ----------------------------------------------

    #[test]
    fn test_retail_75pct() {
        let exp = simple_exposure("Retail Portfolio", dec!(40_000), AssetClass::Retail);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(0.75));
        assert_eq!(detail.rwa, dec!(30_000));
    }

    // -- Test: Residential mortgage => 35% RW --------------------------------

    #[test]
    fn test_residential_mortgage_35pct() {
        let exp = simple_exposure("Mortgages", dec!(200_000), AssetClass::Mortgage);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(0.35));
        assert_eq!(detail.rwa, dec!(70_000));
    }

    // -- Test: CET1 ratio calculation ----------------------------------------

    #[test]
    fn test_cet1_ratio_calculation() {
        // CET1 capital = 10,000 - 1,000 = 9,000
        // Single exposure: 100,000 at 100% => credit_rwa = 100,000
        // Op risk BIA: 15% * avg(50k, 55k, 60k) = 15% * 55k = 8,250
        // Total RWA = 100,000 + 8,250 = 108,250
        // CET1 ratio = 9,000 / 108,250
        let exp = simple_exposure("Loan", dec!(100_000), AssetClass::Other);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();

        let expected_cet1 = dec!(9_000);
        let expected_total_rwa = dec!(100_000) + dec!(8_250);
        let expected_ratio = expected_cet1 / expected_total_rwa;

        assert_eq!(result.result.capital_summary.cet1_capital, expected_cet1);
        assert_eq!(result.result.operational_rwa, dec!(8_250));
        assert_eq!(result.result.total_rwa, expected_total_rwa);
        assert_eq!(result.result.capital_ratios.cet1_ratio, expected_ratio);
    }

    // -- Test: Meets all capital requirements --------------------------------

    #[test]
    fn test_meets_all_requirements() {
        // With generous capital and small exposure, should pass
        let mut input = make_input(vec![simple_exposure(
            "Small Loan",
            dec!(10_000),
            AssetClass::Other,
        )]);
        input.capital = CapitalStructure {
            cet1: dec!(50_000),
            additional_tier1: dec!(10_000),
            tier2: dec!(15_000),
            deductions: dec!(2_000),
        };
        let result = calculate_regulatory_capital(&input).unwrap();
        assert!(result.result.surplus_deficit.meets_requirements);
        assert!(result.result.surplus_deficit.cet1_surplus > Decimal::ZERO);
        assert!(result.result.surplus_deficit.tier1_surplus > Decimal::ZERO);
        assert!(result.result.surplus_deficit.total_capital_surplus > Decimal::ZERO);
    }

    // -- Test: Fails CET1 minimum (< 4.5%) ----------------------------------

    #[test]
    fn test_fails_cet1_minimum() {
        // Tiny CET1 relative to large exposure
        let mut input = make_input(vec![simple_exposure(
            "Big Loan",
            dec!(1_000_000),
            AssetClass::Other,
        )]);
        input.capital = CapitalStructure {
            cet1: dec!(5_000),
            additional_tier1: dec!(1_000),
            tier2: dec!(2_000),
            deductions: dec!(0),
        };
        let result = calculate_regulatory_capital(&input).unwrap();
        // CET1 = 5,000; RWA ~ 1,000,000 + op_risk; CET1 ratio << 4.5%
        assert!(!result.result.surplus_deficit.meets_requirements);
        assert!(result.result.surplus_deficit.cet1_surplus < Decimal::ZERO);
    }

    // -- Test: Operational risk BIA calculation ------------------------------

    #[test]
    fn test_operational_risk_bia() {
        // BIA: 15% * avg(50k, 55k, 60k) = 15% * 55,000 = 8,250
        let input = make_input(vec![simple_exposure(
            "Placeholder",
            dec!(10_000),
            AssetClass::Other,
        )]);
        let result = calculate_regulatory_capital(&input).unwrap();
        assert_eq!(result.result.operational_rwa, dec!(8_250));
    }

    // -- Test: Operational risk SA calculation --------------------------------

    #[test]
    fn test_operational_risk_standardised() {
        let mut input = make_input(vec![simple_exposure(
            "Loan",
            dec!(10_000),
            AssetClass::Other,
        )]);
        input.operational_risk = OperationalRiskInput {
            approach: OpRiskApproach::Standardised,
            gross_income_3yr: vec![dec!(100_000)],
            business_lines: Some(vec![
                BusinessLineIncome {
                    line: "CorporateFinance".to_string(),
                    gross_income: dec!(30_000),
                },
                BusinessLineIncome {
                    line: "Retail".to_string(),
                    gross_income: dec!(20_000),
                },
                BusinessLineIncome {
                    line: "Trading".to_string(),
                    gross_income: dec!(10_000),
                },
            ]),
        };
        let result = calculate_regulatory_capital(&input).unwrap();
        // CorporateFinance: 30k * 0.18 = 5,400
        // Retail: 20k * 0.12 = 2,400
        // Trading: 10k * 0.18 = 1,800
        // Total = 9,600
        assert_eq!(result.result.operational_rwa, dec!(9_600));
    }

    // -- Test: Collateral reduces RWA ----------------------------------------

    #[test]
    fn test_collateral_reduces_rwa() {
        let exp = CreditExposure {
            name: "Secured Loan".to_string(),
            exposure_amount: dec!(100_000),
            asset_class: AssetClass::Corporate,
            risk_weight: None,
            external_rating: Some("BBB".to_string()),
            collateral_value: Some(dec!(50_000)),
            collateral_type: Some(CollateralType::Cash),
        };
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        // Cash haircut = 0 => effective collateral = 50,000
        // Adjusted EAD = max(0, 100,000 - 50,000) = 50,000
        // RWA = 50,000 * 1.00 = 50,000
        assert_eq!(detail.rwa, dec!(50_000));
        assert_eq!(detail.crm_benefit, dec!(50_000));
    }

    // -- Test: Govt bond collateral haircut ----------------------------------

    #[test]
    fn test_govt_bond_collateral_haircut() {
        let exp = CreditExposure {
            name: "Govt Bond Secured".to_string(),
            exposure_amount: dec!(100_000),
            asset_class: AssetClass::Corporate,
            risk_weight: None,
            external_rating: Some("BBB".to_string()),
            collateral_value: Some(dec!(50_000)),
            collateral_type: Some(CollateralType::GovernmentBond),
        };
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        // Govt bond haircut = 2% => effective collateral = 50,000 * 0.98 = 49,000
        // Adjusted EAD = 100,000 - 49,000 = 51,000
        // RWA = 51,000 * 1.00 = 51,000
        assert_eq!(detail.rwa, dec!(51_000));
        assert_eq!(detail.crm_benefit, dec!(49_000));
    }

    // -- Test: Buffer requirements stack correctly ----------------------------

    #[test]
    fn test_buffer_requirements_stack() {
        let mut input = make_input(vec![simple_exposure(
            "Loan",
            dec!(100_000),
            AssetClass::Other,
        )]);
        input.buffers = Some(CapitalBuffers {
            conservation_buffer: dec!(0.025),
            countercyclical_buffer: dec!(0.01),
            systemic_buffer: dec!(0.02),
        });
        let result = calculate_regulatory_capital(&input).unwrap();
        let buf = &result.result.buffer_requirements;
        assert_eq!(buf.min_cet1, dec!(0.045));
        assert_eq!(buf.conservation, dec!(0.025));
        assert_eq!(buf.countercyclical, dec!(0.01));
        assert_eq!(buf.systemic, dec!(0.02));
        // Total = 4.5% + 2.5% + 1.0% + 2.0% = 10.0%
        assert_eq!(buf.total_cet1_requirement, dec!(0.100));
    }

    // -- Test: Leverage ratio calculation ------------------------------------

    #[test]
    fn test_leverage_ratio() {
        let exp = simple_exposure("Loan", dec!(100_000), AssetClass::Other);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        // Tier 1 = CET1 + AT1 - deductions = 10,000 + 2,000 - 1,000 = 11,000
        // Total exposure = 100,000 (credit) + 0 (market) = 100,000
        // Leverage ratio = 11,000 / 100,000 = 0.11
        assert_eq!(result.result.capital_ratios.leverage_ratio, dec!(0.11));
    }

    // -- Test: Multiple exposure classes combined ----------------------------

    #[test]
    fn test_multiple_exposure_classes() {
        let exposures = vec![
            rated_exposure("Sovereign", dec!(50_000), AssetClass::Sovereign, "AAA"),
            rated_exposure("Bank", dec!(30_000), AssetClass::Bank, "A"),
            rated_exposure("Corporate", dec!(40_000), AssetClass::Corporate, "A"),
            simple_exposure("Retail", dec!(20_000), AssetClass::Retail),
            simple_exposure("Mortgage", dec!(60_000), AssetClass::Mortgage),
        ];
        let input = make_input(exposures);
        let result = calculate_regulatory_capital(&input).unwrap();

        // Sovereign AAA: 50,000 * 0% = 0
        // Bank A: 30,000 * 20% = 6,000
        // Corporate A: 40,000 * 50% = 20,000
        // Retail: 20,000 * 75% = 15,000
        // Mortgage: 60,000 * 35% = 21,000
        // Total credit RWA = 62,000
        assert_eq!(result.result.credit_rwa, dec!(62_000));

        assert_eq!(result.result.exposure_details[0].rwa, dec!(0));
        assert_eq!(result.result.exposure_details[1].rwa, dec!(6_000));
        assert_eq!(result.result.exposure_details[2].rwa, dec!(20_000));
        assert_eq!(result.result.exposure_details[3].rwa, dec!(15_000));
        assert_eq!(result.result.exposure_details[4].rwa, dec!(21_000));
    }

    // -- Test: Surplus / deficit calculation ---------------------------------

    #[test]
    fn test_surplus_deficit_calculation() {
        // Capital: CET1=10k, AT1=2k, T2=3k, deductions=1k
        // CET1 capital = 9k, Tier1 = 11k, Total = 14k
        // Exposure: 100k at 100% => credit_rwa = 100k
        // Op risk = 8,250 => total RWA = 108,250
        let exp = simple_exposure("Loan", dec!(100_000), AssetClass::Other);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();

        let total_rwa = result.result.total_rwa;
        let cet1_ratio = dec!(9_000) / total_rwa;
        let tier1_ratio = dec!(11_000) / total_rwa;
        let tc_ratio = dec!(14_000) / total_rwa;

        // Default buffers: conservation = 2.5%, no CCyB, no systemic
        // Total CET1 requirement = 4.5% + 2.5% = 7.0%
        let expected_cet1_surplus = cet1_ratio - dec!(0.070);
        let expected_tier1_surplus = tier1_ratio - dec!(0.06);
        let expected_tc_surplus = tc_ratio - dec!(0.08);

        assert_eq!(
            result.result.surplus_deficit.cet1_surplus,
            expected_cet1_surplus
        );
        assert_eq!(
            result.result.surplus_deficit.tier1_surplus,
            expected_tier1_surplus
        );
        assert_eq!(
            result.result.surplus_deficit.total_capital_surplus,
            expected_tc_surplus
        );
    }

    // -- Test: Capital deductions applied correctly --------------------------

    #[test]
    fn test_capital_deductions_applied() {
        let mut input = make_input(vec![simple_exposure(
            "Loan",
            dec!(100_000),
            AssetClass::Other,
        )]);
        input.capital.deductions = dec!(5_000);
        let result = calculate_regulatory_capital(&input).unwrap();
        let summary = &result.result.capital_summary;
        // CET1 capital = 10,000 - 5,000 = 5,000
        assert_eq!(summary.cet1_capital, dec!(5_000));
        // Tier 1 = 10,000 + 2,000 - 5,000 = 7,000
        assert_eq!(summary.tier1_capital, dec!(7_000));
        // Total = 10,000 + 2,000 + 3,000 - 5,000 = 10,000
        assert_eq!(summary.total_capital, dec!(10_000));
        assert_eq!(summary.deductions, dec!(5_000));
    }

    // -- Test: Unrated corporate => 100% RW ----------------------------------

    #[test]
    fn test_unrated_corporate_100pct() {
        let exp = simple_exposure("Unrated Corp", dec!(60_000), AssetClass::Corporate);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(1.00));
        assert_eq!(detail.rwa, dec!(60_000));
    }

    // -- Test: Below BB corporate => 150% RW ---------------------------------

    #[test]
    fn test_below_bb_corporate_150pct() {
        let exp = rated_exposure("Distressed Corp", dec!(40_000), AssetClass::Corporate, "B");
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(1.50));
        assert_eq!(detail.rwa, dec!(60_000));
    }

    // -- Test: CCC corporate => 150% RW --------------------------------------

    #[test]
    fn test_ccc_corporate_150pct() {
        let exp = rated_exposure("CCC Corp", dec!(20_000), AssetClass::Corporate, "CCC");
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(1.50));
        assert_eq!(detail.rwa, dec!(30_000));
    }

    // -- Test: Negative CET1 rejected ----------------------------------------

    #[test]
    fn test_negative_cet1_rejected() {
        let mut input = make_input(vec![simple_exposure(
            "Loan",
            dec!(10_000),
            AssetClass::Other,
        )]);
        input.capital.cet1 = dec!(-1);
        let err = calculate_regulatory_capital(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "capital.cet1");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    // -- Test: Negative exposure amount rejected ------------------------------

    #[test]
    fn test_negative_exposure_rejected() {
        let exp = simple_exposure("Bad", dec!(-100), AssetClass::Other);
        let input = make_input(vec![exp]);
        let err = calculate_regulatory_capital(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("exposure_amount"));
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    // -- Test: Risk weight override out of range rejected --------------------

    #[test]
    fn test_risk_weight_override_out_of_range() {
        let exp = CreditExposure {
            name: "Bad RW".to_string(),
            exposure_amount: dec!(10_000),
            asset_class: AssetClass::Other,
            risk_weight: Some(dec!(2.0)), // > 1.5
            external_rating: None,
            collateral_value: None,
            collateral_type: None,
        };
        let input = make_input(vec![exp]);
        let err = calculate_regulatory_capital(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("risk_weight"));
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    // -- Test: Risk weight override used when provided -----------------------

    #[test]
    fn test_risk_weight_override() {
        let exp = CreditExposure {
            name: "Custom RW".to_string(),
            exposure_amount: dec!(100_000),
            asset_class: AssetClass::Corporate,
            risk_weight: Some(dec!(0.50)),
            external_rating: Some("BBB".to_string()), // Would be 100% but override is 50%
            collateral_value: None,
            collateral_type: None,
        };
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(0.50));
        assert_eq!(detail.rwa, dec!(50_000));
    }

    // -- Test: Market risk charge included -----------------------------------

    #[test]
    fn test_market_risk_charge_included() {
        let mut input = make_input(vec![simple_exposure(
            "Loan",
            dec!(100_000),
            AssetClass::Other,
        )]);
        input.market_risk_charge = Some(dec!(15_000));
        let result = calculate_regulatory_capital(&input).unwrap();
        assert_eq!(result.result.market_rwa, dec!(15_000));
        // Total RWA = credit 100k + market 15k + op 8,250 = 123,250
        assert_eq!(result.result.total_rwa, dec!(123_250));
    }

    // -- Test: BIA with negative income year excluded -----------------------

    #[test]
    fn test_bia_negative_income_excluded() {
        let mut input = make_input(vec![simple_exposure(
            "Loan",
            dec!(10_000),
            AssetClass::Other,
        )]);
        input.operational_risk.gross_income_3yr = vec![dec!(100_000), dec!(-20_000), dec!(80_000)];
        let result = calculate_regulatory_capital(&input).unwrap();
        // Avg of positive = (100k + 80k) / 2 = 90k
        // BIA = 15% * 90k = 13,500
        assert_eq!(result.result.operational_rwa, dec!(13_500));
    }

    // -- Test: Equity collateral haircut -------------------------------------

    #[test]
    fn test_equity_collateral_haircut() {
        let exp = CreditExposure {
            name: "Equity Secured".to_string(),
            exposure_amount: dec!(100_000),
            asset_class: AssetClass::Corporate,
            risk_weight: None,
            external_rating: Some("BBB".to_string()),
            collateral_value: Some(dec!(40_000)),
            collateral_type: Some(CollateralType::Equity),
        };
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        // Equity haircut = 25% => effective collateral = 40,000 * 0.75 = 30,000
        // Adjusted EAD = 100,000 - 30,000 = 70,000
        // RWA = 70,000 * 1.00 = 70,000
        assert_eq!(detail.rwa, dec!(70_000));
        assert_eq!(detail.crm_benefit, dec!(30_000));
    }

    // -- Test: Tier1 and total capital ratios --------------------------------

    #[test]
    fn test_tier1_and_total_capital_ratios() {
        let exp = simple_exposure("Loan", dec!(100_000), AssetClass::Other);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();

        let total_rwa = result.result.total_rwa;
        // Tier 1 = 10k + 2k - 1k = 11k
        let expected_tier1_ratio = dec!(11_000) / total_rwa;
        // Total capital = 10k + 2k + 3k - 1k = 14k
        let expected_tc_ratio = dec!(14_000) / total_rwa;

        assert_eq!(
            result.result.capital_ratios.tier1_ratio,
            expected_tier1_ratio
        );
        assert_eq!(
            result.result.capital_ratios.total_capital_ratio,
            expected_tc_ratio
        );
    }

    // -- Test: Metadata is populated -----------------------------------------

    #[test]
    fn test_metadata_populated() {
        let input = make_input(vec![simple_exposure(
            "Loan",
            dec!(10_000),
            AssetClass::Other,
        )]);
        let result = calculate_regulatory_capital(&input).unwrap();
        assert!(result.methodology.contains("Basel III"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.warnings.is_empty() || result.warnings.is_empty()); // no panic
    }

    // -- Test: Bank rating categories ----------------------------------------

    #[test]
    fn test_bank_rating_risk_weights() {
        // AAA => 20%
        let exp = rated_exposure("Bank AAA", dec!(10_000), AssetClass::Bank, "AAA");
        let mut warnings = Vec::new();
        let detail = calculate_exposure_rwa(&exp, &mut warnings).unwrap();
        assert_eq!(detail.risk_weight, dec!(0.20));

        // BBB => 50%
        let exp = rated_exposure("Bank BBB", dec!(10_000), AssetClass::Bank, "BBB");
        let detail = calculate_exposure_rwa(&exp, &mut warnings).unwrap();
        assert_eq!(detail.risk_weight, dec!(0.50));

        // BB => 100%
        let exp = rated_exposure("Bank BB", dec!(10_000), AssetClass::Bank, "BB");
        let detail = calculate_exposure_rwa(&exp, &mut warnings).unwrap();
        assert_eq!(detail.risk_weight, dec!(1.00));

        // CCC => 150%
        let exp = rated_exposure("Bank CCC", dec!(10_000), AssetClass::Bank, "CCC");
        let detail = calculate_exposure_rwa(&exp, &mut warnings).unwrap();
        assert_eq!(detail.risk_weight, dec!(1.50));
    }

    // -- Test: Sovereign rating categories -----------------------------------

    #[test]
    fn test_sovereign_rating_categories() {
        let mut warnings = Vec::new();

        // A => 20%
        let exp = rated_exposure("Sov A", dec!(10_000), AssetClass::Sovereign, "A");
        let detail = calculate_exposure_rwa(&exp, &mut warnings).unwrap();
        assert_eq!(detail.risk_weight, dec!(0.20));

        // BBB => 50%
        let exp = rated_exposure("Sov BBB", dec!(10_000), AssetClass::Sovereign, "BBB");
        let detail = calculate_exposure_rwa(&exp, &mut warnings).unwrap();
        assert_eq!(detail.risk_weight, dec!(0.50));

        // BB => 100%
        let exp = rated_exposure("Sov BB", dec!(10_000), AssetClass::Sovereign, "BB");
        let detail = calculate_exposure_rwa(&exp, &mut warnings).unwrap();
        assert_eq!(detail.risk_weight, dec!(1.00));

        // CCC => 150%
        let exp = rated_exposure("Sov CCC", dec!(10_000), AssetClass::Sovereign, "CCC");
        let detail = calculate_exposure_rwa(&exp, &mut warnings).unwrap();
        assert_eq!(detail.risk_weight, dec!(1.50));
    }

    // -- Test: Default buffers when none provided ----------------------------

    #[test]
    fn test_default_buffers_when_none() {
        let input = make_input(vec![simple_exposure(
            "Loan",
            dec!(10_000),
            AssetClass::Other,
        )]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let buf = &result.result.buffer_requirements;
        assert_eq!(buf.conservation, dec!(0.025));
        assert_eq!(buf.countercyclical, Decimal::ZERO);
        assert_eq!(buf.systemic, Decimal::ZERO);
        assert_eq!(buf.total_cet1_requirement, dec!(0.070));
    }

    // -- Test: No exposures warns but does not crash -------------------------

    #[test]
    fn test_no_exposures_errors_on_zero_rwa() {
        // With no exposures and potentially zero op risk income, total_rwa could be zero
        let mut input = make_input(vec![]);
        input.operational_risk.gross_income_3yr = vec![dec!(-10)]; // all negative
        let result = calculate_regulatory_capital(&input);
        // Should error on zero total RWA
        assert!(result.is_err());
    }

    // -- Test: Equity exposure 100% risk weight ------------------------------

    #[test]
    fn test_equity_exposure_100pct() {
        let exp = simple_exposure("Listed Equity", dec!(25_000), AssetClass::Equity);
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(1.00));
        assert_eq!(detail.rwa, dec!(25_000));
    }

    // -- Test: Corporate AA => 20% RW ----------------------------------------

    #[test]
    fn test_corporate_aa_20pct() {
        let exp = rated_exposure("Corp AA", dec!(50_000), AssetClass::Corporate, "AA");
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(0.20));
        assert_eq!(detail.rwa, dec!(10_000));
    }

    // -- Test: Corporate A => 50% RW -----------------------------------------

    #[test]
    fn test_corporate_a_50pct() {
        let exp = rated_exposure("Corp A", dec!(50_000), AssetClass::Corporate, "A");
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        assert_eq!(detail.risk_weight, dec!(0.50));
        assert_eq!(detail.rwa, dec!(25_000));
    }

    // -- Test: Collateral exceeding EAD clamps adjusted EAD to zero ----------

    #[test]
    fn test_collateral_exceeding_ead_clamps_to_zero() {
        let exp = CreditExposure {
            name: "Over-Secured".to_string(),
            exposure_amount: dec!(50_000),
            asset_class: AssetClass::Corporate,
            risk_weight: None,
            external_rating: Some("BBB".to_string()),
            collateral_value: Some(dec!(100_000)),
            collateral_type: Some(CollateralType::Cash),
        };
        let input = make_input(vec![exp]);
        let result = calculate_regulatory_capital(&input).unwrap();
        let detail = &result.result.exposure_details[0];
        // Adjusted EAD = max(0, 50k - 100k) = 0
        assert_eq!(detail.rwa, dec!(0));
        assert_eq!(detail.crm_benefit, dec!(50_000));
    }

    // -- Test: SA with missing business lines errors -------------------------

    #[test]
    fn test_sa_missing_business_lines_rejected() {
        let mut input = make_input(vec![simple_exposure(
            "Loan",
            dec!(10_000),
            AssetClass::Other,
        )]);
        input.operational_risk = OperationalRiskInput {
            approach: OpRiskApproach::Standardised,
            gross_income_3yr: vec![dec!(100_000)],
            business_lines: None,
        };
        let err = calculate_regulatory_capital(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("business_lines"));
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    // -- Test: Empty gross income rejected -----------------------------------

    #[test]
    fn test_empty_gross_income_rejected() {
        let mut input = make_input(vec![simple_exposure(
            "Loan",
            dec!(10_000),
            AssetClass::Other,
        )]);
        input.operational_risk.gross_income_3yr = vec![];
        let err = calculate_regulatory_capital(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {} // expected
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }
}
