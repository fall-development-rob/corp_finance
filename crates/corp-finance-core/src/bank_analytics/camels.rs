//! CAMELS rating system for bank examination.
//!
//! Covers:
//! 1. **Capital adequacy** -- CET1 ratio thresholds.
//! 2. **Asset quality** -- NPL ratio thresholds.
//! 3. **Management** -- Efficiency ratio thresholds.
//! 4. **Earnings** -- ROA thresholds.
//! 5. **Liquidity** -- LCR thresholds.
//! 6. **Sensitivity to market risk** -- Direct score input.
//! 7. **Composite rating** -- Simple average, rounded.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for CAMELS rating calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CamelsInput {
    // Capital
    /// Tier 1 (CET1) capital.
    pub tier1_capital: Decimal,
    /// Total capital (Tier 1 + Tier 2).
    pub total_capital: Decimal,
    /// Risk-weighted assets.
    pub risk_weighted_assets: Decimal,
    /// Leverage ratio (Tier 1 / total assets).
    pub leverage_ratio: Decimal,

    // Asset quality
    /// Non-performing loan ratio.
    pub npl_ratio: Decimal,
    /// Provision coverage ratio (provisions / NPL).
    pub provision_coverage: Decimal,
    /// Loan loss reserve ratio.
    pub loan_loss_reserve_ratio: Decimal,
    /// Classified assets ratio.
    pub classified_assets_ratio: Decimal,

    // Management
    /// Efficiency ratio (non-interest expense / revenue).
    pub efficiency_ratio: Decimal,
    /// Compliance score (0-100).
    pub compliance_score: Decimal,
    /// Board independence percentage.
    pub board_independence_pct: Decimal,

    // Earnings
    /// Return on assets.
    pub roa: Decimal,
    /// Return on equity.
    pub roe: Decimal,
    /// Net interest margin.
    pub nim: Decimal,
    /// Cost-to-income ratio.
    pub cost_income_ratio: Decimal,

    // Liquidity
    /// Liquidity coverage ratio (as decimal, e.g. 1.2 = 120%).
    pub lcr: Decimal,
    /// Net stable funding ratio (as decimal).
    pub nsfr: Decimal,
    /// Loan-to-deposit ratio.
    pub loan_to_deposit: Decimal,

    // Sensitivity
    /// Interest rate risk score (1-5, input directly).
    pub interest_rate_risk_score: Decimal,
    /// FX exposure as percentage of capital.
    pub fx_exposure_pct: Decimal,
    /// Duration gap (years).
    pub duration_gap: Decimal,
}

/// Output of CAMELS rating calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CamelsOutput {
    /// Capital adequacy rating (1-5).
    pub capital_rating: u8,
    /// Asset quality rating (1-5).
    pub asset_quality_rating: u8,
    /// Management rating (1-5).
    pub management_rating: u8,
    /// Earnings rating (1-5).
    pub earnings_rating: u8,
    /// Liquidity rating (1-5).
    pub liquidity_rating: u8,
    /// Sensitivity to market risk rating (1-5).
    pub sensitivity_rating: u8,
    /// Composite rating (1-5, rounded average).
    pub composite_rating: u8,
    /// Composite description.
    pub composite_description: String,
    /// List of flagged concerns.
    pub concerns: Vec<String>,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate CAMELS ratings from bank metrics.
pub fn calculate_camels(input: &CamelsInput) -> CorpFinanceResult<CamelsOutput> {
    validate_camels_input(input)?;

    let mut concerns = Vec::new();

    // Capital rating: CET1 ratio = tier1_capital / risk_weighted_assets
    let cet1_ratio = if input.risk_weighted_assets > Decimal::ZERO {
        input.tier1_capital / input.risk_weighted_assets
    } else {
        Decimal::ZERO
    };
    let capital_rating = rate_capital(cet1_ratio);
    if capital_rating >= 3 {
        concerns.push(format!(
            "Capital: CET1 ratio {:.2}% below well-capitalized threshold",
            cet1_ratio * dec!(100)
        ));
    }
    if input.leverage_ratio < dec!(0.04) {
        concerns.push(format!(
            "Capital: Leverage ratio {:.2}% below 4% minimum",
            input.leverage_ratio * dec!(100)
        ));
    }

    // Asset quality rating
    let asset_quality_rating = rate_asset_quality(input.npl_ratio);
    if asset_quality_rating >= 3 {
        concerns.push(format!(
            "Asset quality: NPL ratio {:.2}% elevated",
            input.npl_ratio * dec!(100)
        ));
    }
    if input.provision_coverage < dec!(1.0) {
        concerns.push(format!(
            "Asset quality: Provision coverage {:.1}% below 100%",
            input.provision_coverage * dec!(100)
        ));
    }

    // Management rating
    let management_rating = rate_management(input.efficiency_ratio);
    if input.compliance_score < dec!(70) {
        concerns.push(format!(
            "Management: Compliance score {} below acceptable threshold",
            input.compliance_score
        ));
    }
    if input.board_independence_pct < dec!(50) {
        concerns.push("Management: Board independence below 50%".into());
    }

    // Earnings rating
    let earnings_rating = rate_earnings(input.roa);
    if input.roe < dec!(0.08) {
        concerns.push(format!(
            "Earnings: ROE {:.2}% below 8% threshold",
            input.roe * dec!(100)
        ));
    }

    // Liquidity rating
    let liquidity_rating = rate_liquidity(input.lcr);
    if input.nsfr < Decimal::ONE {
        concerns.push(format!(
            "Liquidity: NSFR {:.1}% below 100% requirement",
            input.nsfr * dec!(100)
        ));
    }
    if input.loan_to_deposit > dec!(1.0) {
        concerns.push(format!(
            "Liquidity: Loan-to-deposit ratio {:.1}% exceeds 100%",
            input.loan_to_deposit * dec!(100)
        ));
    }

    // Sensitivity rating
    let sensitivity_rating = rate_sensitivity(input.interest_rate_risk_score);
    if input.fx_exposure_pct > dec!(0.25) {
        concerns.push(format!(
            "Sensitivity: FX exposure {:.1}% exceeds 25% of capital",
            input.fx_exposure_pct * dec!(100)
        ));
    }
    if input.duration_gap.abs() > dec!(3) {
        concerns.push(format!(
            "Sensitivity: Duration gap {:.1} years exceeds +/-3yr threshold",
            input.duration_gap
        ));
    }

    // Composite rating = simple average of 6 ratings, rounded
    let sum = Decimal::from(capital_rating)
        + Decimal::from(asset_quality_rating)
        + Decimal::from(management_rating)
        + Decimal::from(earnings_rating)
        + Decimal::from(liquidity_rating)
        + Decimal::from(sensitivity_rating);
    let avg = sum / dec!(6);
    // Round to nearest integer, clamped to 1-5
    let composite_raw = (avg + dec!(0.5)).floor();
    let composite_rating = composite_raw
        .to_string()
        .parse::<u8>()
        .unwrap_or(3)
        .clamp(1, 5);

    let composite_description = match composite_rating {
        1 => "Strong".to_string(),
        2 => "Satisfactory".to_string(),
        3 => "Fair".to_string(),
        4 => "Marginal".to_string(),
        _ => "Unsatisfactory".to_string(),
    };

    Ok(CamelsOutput {
        capital_rating,
        asset_quality_rating,
        management_rating,
        earnings_rating,
        liquidity_rating,
        sensitivity_rating,
        composite_rating,
        composite_description,
        concerns,
    })
}

// ---------------------------------------------------------------------------
// Rating helpers
// ---------------------------------------------------------------------------

fn rate_capital(cet1_ratio: Decimal) -> u8 {
    if cet1_ratio >= dec!(0.105) {
        1
    } else if cet1_ratio >= dec!(0.08) {
        2
    } else if cet1_ratio >= dec!(0.065) {
        3
    } else if cet1_ratio >= dec!(0.045) {
        4
    } else {
        5
    }
}

fn rate_asset_quality(npl_ratio: Decimal) -> u8 {
    if npl_ratio < dec!(0.01) {
        1
    } else if npl_ratio < dec!(0.03) {
        2
    } else if npl_ratio < dec!(0.05) {
        3
    } else if npl_ratio < dec!(0.08) {
        4
    } else {
        5
    }
}

fn rate_management(efficiency_ratio: Decimal) -> u8 {
    if efficiency_ratio < dec!(0.55) {
        1
    } else if efficiency_ratio < dec!(0.65) {
        2
    } else if efficiency_ratio < dec!(0.75) {
        3
    } else if efficiency_ratio < dec!(0.85) {
        4
    } else {
        5
    }
}

fn rate_earnings(roa: Decimal) -> u8 {
    if roa >= dec!(0.0125) {
        1
    } else if roa >= dec!(0.01) {
        2
    } else if roa >= dec!(0.0075) {
        3
    } else if roa >= dec!(0.005) {
        4
    } else {
        5
    }
}

fn rate_liquidity(lcr: Decimal) -> u8 {
    if lcr >= dec!(1.2) {
        1
    } else if lcr >= dec!(1.1) {
        2
    } else if lcr >= dec!(1.0) {
        3
    } else if lcr >= dec!(0.9) {
        4
    } else {
        5
    }
}

fn rate_sensitivity(score: Decimal) -> u8 {
    // Round to nearest integer, clamp 1-5
    let rounded = (score + dec!(0.5)).floor();
    rounded.to_string().parse::<u8>().unwrap_or(3).clamp(1, 5)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_camels_input(input: &CamelsInput) -> CorpFinanceResult<()> {
    if input.risk_weighted_assets < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_weighted_assets".into(),
            reason: "Risk-weighted assets cannot be negative.".into(),
        });
    }
    if input.tier1_capital < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "tier1_capital".into(),
            reason: "Tier 1 capital cannot be negative.".into(),
        });
    }
    if input.npl_ratio < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "npl_ratio".into(),
            reason: "NPL ratio cannot be negative.".into(),
        });
    }
    if input.efficiency_ratio < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "efficiency_ratio".into(),
            reason: "Efficiency ratio cannot be negative.".into(),
        });
    }
    if input.lcr < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "lcr".into(),
            reason: "LCR cannot be negative.".into(),
        });
    }
    if input.interest_rate_risk_score < Decimal::ONE || input.interest_rate_risk_score > dec!(5) {
        return Err(CorpFinanceError::InvalidInput {
            field: "interest_rate_risk_score".into(),
            reason: "Interest rate risk score must be between 1 and 5.".into(),
        });
    }
    if input.compliance_score < Decimal::ZERO || input.compliance_score > dec!(100) {
        return Err(CorpFinanceError::InvalidInput {
            field: "compliance_score".into(),
            reason: "Compliance score must be between 0 and 100.".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn well_capitalized_bank() -> CamelsInput {
        CamelsInput {
            tier1_capital: dec!(120_000_000),
            total_capital: dec!(150_000_000),
            risk_weighted_assets: dec!(1_000_000_000),
            leverage_ratio: dec!(0.08),
            npl_ratio: dec!(0.005),
            provision_coverage: dec!(2.0),
            loan_loss_reserve_ratio: dec!(0.02),
            classified_assets_ratio: dec!(0.01),
            efficiency_ratio: dec!(0.50),
            compliance_score: dec!(95),
            board_independence_pct: dec!(75),
            roa: dec!(0.015),
            roe: dec!(0.15),
            nim: dec!(0.035),
            cost_income_ratio: dec!(0.50),
            lcr: dec!(1.3),
            nsfr: dec!(1.2),
            loan_to_deposit: dec!(0.80),
            interest_rate_risk_score: dec!(1.0),
            fx_exposure_pct: dec!(0.05),
            duration_gap: dec!(1.0),
        }
    }

    fn troubled_bank() -> CamelsInput {
        CamelsInput {
            tier1_capital: dec!(30_000_000),
            total_capital: dec!(40_000_000),
            risk_weighted_assets: dec!(1_000_000_000),
            leverage_ratio: dec!(0.02),
            npl_ratio: dec!(0.12),
            provision_coverage: dec!(0.40),
            loan_loss_reserve_ratio: dec!(0.005),
            classified_assets_ratio: dec!(0.15),
            efficiency_ratio: dec!(0.92),
            compliance_score: dec!(40),
            board_independence_pct: dec!(30),
            roa: dec!(0.002),
            roe: dec!(0.03),
            nim: dec!(0.015),
            cost_income_ratio: dec!(0.92),
            lcr: dec!(0.7),
            nsfr: dec!(0.8),
            loan_to_deposit: dec!(1.20),
            interest_rate_risk_score: dec!(5.0),
            fx_exposure_pct: dec!(0.40),
            duration_gap: dec!(5.0),
        }
    }

    #[test]
    fn test_well_capitalized_all_ones() {
        let input = well_capitalized_bank();
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.capital_rating, 1);
        assert_eq!(out.asset_quality_rating, 1);
        assert_eq!(out.management_rating, 1);
        assert_eq!(out.earnings_rating, 1);
        assert_eq!(out.liquidity_rating, 1);
        assert_eq!(out.sensitivity_rating, 1);
        assert_eq!(out.composite_rating, 1);
        assert_eq!(out.composite_description, "Strong");
    }

    #[test]
    fn test_troubled_bank_all_fives() {
        let input = troubled_bank();
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.capital_rating, 5);
        assert_eq!(out.asset_quality_rating, 5);
        assert_eq!(out.management_rating, 5);
        assert_eq!(out.earnings_rating, 5);
        assert_eq!(out.liquidity_rating, 5);
        assert_eq!(out.sensitivity_rating, 5);
        assert_eq!(out.composite_rating, 5);
        assert_eq!(out.composite_description, "Unsatisfactory");
    }

    #[test]
    fn test_troubled_bank_has_concerns() {
        let input = troubled_bank();
        let out = calculate_camels(&input).unwrap();
        assert!(
            !out.concerns.is_empty(),
            "Troubled bank should have concerns"
        );
        assert!(
            out.concerns.len() >= 5,
            "Expected many concerns, got {}",
            out.concerns.len()
        );
    }

    #[test]
    fn test_well_capitalized_no_component_concerns() {
        let input = well_capitalized_bank();
        let out = calculate_camels(&input).unwrap();
        // A well-capitalized bank with all 1-ratings should have no concerns
        assert!(
            out.concerns.is_empty(),
            "Well-cap bank should have no concerns: {:?}",
            out.concerns
        );
    }

    #[test]
    fn test_capital_rating_boundary_10_5_pct() {
        let mut input = well_capitalized_bank();
        input.tier1_capital = dec!(105_000_000); // exactly 10.5%
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.capital_rating, 1);
    }

    #[test]
    fn test_capital_rating_boundary_just_below() {
        let mut input = well_capitalized_bank();
        input.tier1_capital = dec!(104_000_000); // 10.4%
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.capital_rating, 2);
    }

    #[test]
    fn test_capital_rating_8_pct() {
        let mut input = well_capitalized_bank();
        input.tier1_capital = dec!(80_000_000); // 8%
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.capital_rating, 2);
    }

    #[test]
    fn test_capital_rating_6_5_pct() {
        let mut input = well_capitalized_bank();
        input.tier1_capital = dec!(65_000_000); // 6.5%
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.capital_rating, 3);
    }

    #[test]
    fn test_capital_rating_4_5_pct() {
        let mut input = well_capitalized_bank();
        input.tier1_capital = dec!(45_000_000); // 4.5%
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.capital_rating, 4);
    }

    #[test]
    fn test_capital_rating_below_4_5_pct() {
        let mut input = well_capitalized_bank();
        input.tier1_capital = dec!(40_000_000); // 4.0%
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.capital_rating, 5);
    }

    #[test]
    fn test_npl_boundary_1_pct() {
        let mut input = well_capitalized_bank();
        input.npl_ratio = dec!(0.01); // exactly 1%
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.asset_quality_rating, 2);
    }

    #[test]
    fn test_npl_boundary_3_pct() {
        let mut input = well_capitalized_bank();
        input.npl_ratio = dec!(0.03);
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.asset_quality_rating, 3);
    }

    #[test]
    fn test_earnings_roa_boundary() {
        let mut input = well_capitalized_bank();
        input.roa = dec!(0.01); // exactly 1.0%
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.earnings_rating, 2);
    }

    #[test]
    fn test_liquidity_lcr_boundary() {
        let mut input = well_capitalized_bank();
        input.lcr = dec!(1.0);
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.liquidity_rating, 3);
    }

    #[test]
    fn test_sensitivity_rounding() {
        let mut input = well_capitalized_bank();
        input.interest_rate_risk_score = dec!(2.4);
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.sensitivity_rating, 2);
    }

    #[test]
    fn test_sensitivity_rounding_up() {
        let mut input = well_capitalized_bank();
        input.interest_rate_risk_score = dec!(2.6);
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.sensitivity_rating, 3);
    }

    #[test]
    fn test_mixed_ratings_composite() {
        let mut input = well_capitalized_bank();
        // Make capital=1, asset=3, mgmt=2, earnings=1, liq=1, sensitivity=2
        input.npl_ratio = dec!(0.04); // rating 3
        input.efficiency_ratio = dec!(0.60); // rating 2
        input.interest_rate_risk_score = dec!(2.0); // rating 2
        let out = calculate_camels(&input).unwrap();
        // Average: (1+3+2+1+1+2)/6 = 10/6 = 1.67, rounded to 2
        assert_eq!(out.composite_rating, 2);
        assert_eq!(out.composite_description, "Satisfactory");
    }

    #[test]
    fn test_composite_fair_rating() {
        let mut input = well_capitalized_bank();
        input.tier1_capital = dec!(70_000_000); // 7% -> rating 3
        input.npl_ratio = dec!(0.04); // rating 3
        input.efficiency_ratio = dec!(0.70); // rating 3
        input.roa = dec!(0.008); // rating 3
        input.lcr = dec!(1.05); // rating 3
        input.interest_rate_risk_score = dec!(3.0);
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.composite_rating, 3);
        assert_eq!(out.composite_description, "Fair");
    }

    #[test]
    fn test_composite_marginal_rating() {
        let mut input = well_capitalized_bank();
        input.tier1_capital = dec!(50_000_000); // 5% -> rating 4
        input.npl_ratio = dec!(0.06); // rating 4
        input.efficiency_ratio = dec!(0.80); // rating 4
        input.roa = dec!(0.006); // rating 4
        input.lcr = dec!(0.95); // rating 4
        input.interest_rate_risk_score = dec!(4.0);
        let out = calculate_camels(&input).unwrap();
        assert_eq!(out.composite_rating, 4);
        assert_eq!(out.composite_description, "Marginal");
    }

    #[test]
    fn test_concern_low_leverage() {
        let mut input = well_capitalized_bank();
        input.leverage_ratio = dec!(0.03);
        let out = calculate_camels(&input).unwrap();
        assert!(out.concerns.iter().any(|c| c.contains("Leverage ratio")));
    }

    #[test]
    fn test_concern_low_provision_coverage() {
        let mut input = well_capitalized_bank();
        input.provision_coverage = dec!(0.80);
        let out = calculate_camels(&input).unwrap();
        assert!(out
            .concerns
            .iter()
            .any(|c| c.contains("Provision coverage")));
    }

    #[test]
    fn test_concern_high_fx_exposure() {
        let mut input = well_capitalized_bank();
        input.fx_exposure_pct = dec!(0.35);
        let out = calculate_camels(&input).unwrap();
        assert!(out.concerns.iter().any(|c| c.contains("FX exposure")));
    }

    #[test]
    fn test_concern_large_duration_gap() {
        let mut input = well_capitalized_bank();
        input.duration_gap = dec!(4.5);
        let out = calculate_camels(&input).unwrap();
        assert!(out.concerns.iter().any(|c| c.contains("Duration gap")));
    }

    #[test]
    fn test_reject_negative_rwa() {
        let mut input = well_capitalized_bank();
        input.risk_weighted_assets = dec!(-100);
        assert!(calculate_camels(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_interest_rate_risk_score() {
        let mut input = well_capitalized_bank();
        input.interest_rate_risk_score = dec!(0.5);
        assert!(calculate_camels(&input).is_err());
    }

    #[test]
    fn test_reject_compliance_score_over_100() {
        let mut input = well_capitalized_bank();
        input.compliance_score = dec!(101);
        assert!(calculate_camels(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = well_capitalized_bank();
        let out = calculate_camels(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: CamelsOutput = serde_json::from_str(&json).unwrap();
    }
}
