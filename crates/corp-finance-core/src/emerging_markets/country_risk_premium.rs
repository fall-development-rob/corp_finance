//! Country Risk Premium (CRP) estimation for emerging markets.
//!
//! Implements:
//! 1. **Damodaran CRP** -- sovereign_spread x (equity_vol / bond_vol)
//! 2. **Rating-based premium** -- lookup from Moody's-style rating
//! 3. **Composite CRP** -- weighted blend of methods
//! 4. **Total cost of equity** -- risk-free + US ERP + CRP
//! 5. **Governance adjustment** -- World Bank governance indicator
//! 6. **Macro risk score** -- GDP, inflation, FX vol composite
//! 7. **Rating-implied default probability**
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

/// Input for country risk premium estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryRiskPremiumInput {
    /// Sovereign CDS or bond spread in basis points.
    pub sovereign_spread_bps: Decimal,
    /// Local equity market annualised volatility (e.g. 0.25 = 25%).
    pub equity_vol_local: Decimal,
    /// Local bond market annualised volatility (e.g. 0.10 = 10%).
    pub bond_vol_local: Decimal,
    /// Mature-market equity risk premium (e.g. 0.055 = 5.5%).
    pub us_equity_risk_premium: Decimal,
    /// Moody's-style rating string, e.g. "Baa2".
    pub country_rating: String,
    /// Real GDP growth rate (e.g. 0.04 = 4%).
    pub gdp_growth: Decimal,
    /// Inflation rate (e.g. 0.06 = 6%).
    pub inflation_rate: Decimal,
    /// Currency volatility vs USD (e.g. 0.12 = 12%).
    pub fx_volatility: Decimal,
    /// World Bank governance indicator (-2.5 to +2.5).
    pub governance_score: Decimal,
    /// Risk-free rate used for total cost of equity (e.g. 0.04 = 4%).
    pub risk_free_rate: Decimal,
}

/// Output from country risk premium estimation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CountryRiskPremiumOutput {
    /// Damodaran method: sovereign_spread x lambda.
    pub damodaran_crp: Decimal,
    /// Rating-based premium from lookup table.
    pub rating_based_premium: Decimal,
    /// Weighted composite CRP (50% Damodaran, 30% rating, 20% governance-adjusted).
    pub composite_crp: Decimal,
    /// Total cost of equity = risk_free + US ERP + composite CRP.
    pub total_cost_of_equity: Decimal,
    /// Lambda = equity_vol / bond_vol.
    pub lambda: Decimal,
    /// Governance adjustment component.
    pub governance_adjustment: Decimal,
    /// Macro risk score 0-100.
    pub macro_risk_score: Decimal,
    /// Rough implied default probability from rating.
    pub rating_implied_default_prob: Decimal,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map Moody's rating to (premium%, default_prob%).
fn rating_to_premium_and_pd(rating: &str) -> Option<(Decimal, Decimal)> {
    let (premium, pd) = match rating {
        "Aaa" => (dec!(0.0000), dec!(0.0001)),
        "Aa1" => (dec!(0.0050), dec!(0.0003)),
        "Aa2" => (dec!(0.0060), dec!(0.0005)),
        "Aa3" => (dec!(0.0070), dec!(0.0008)),
        "A1" => (dec!(0.0085), dec!(0.0010)),
        "A2" => (dec!(0.0100), dec!(0.0015)),
        "A3" => (dec!(0.0115), dec!(0.0020)),
        "Baa1" => (dec!(0.0150), dec!(0.0030)),
        "Baa2" => (dec!(0.0200), dec!(0.0050)),
        "Baa3" => (dec!(0.0250), dec!(0.0080)),
        "Ba1" => (dec!(0.0300), dec!(0.0120)),
        "Ba2" => (dec!(0.0375), dec!(0.0170)),
        "Ba3" => (dec!(0.0450), dec!(0.0250)),
        "B1" => (dec!(0.0550), dec!(0.0400)),
        "B2" => (dec!(0.0650), dec!(0.0550)),
        "B3" => (dec!(0.0750), dec!(0.0700)),
        "Caa1" => (dec!(0.0900), dec!(0.1000)),
        "Caa2" => (dec!(0.1050), dec!(0.1500)),
        "Caa3" => (dec!(0.1200), dec!(0.2000)),
        "Ca" => (dec!(0.1300), dec!(0.3000)),
        "C" => (dec!(0.1500), dec!(0.5000)),
        _ => return None,
    };
    Some((premium, pd))
}

/// Governance adjustment: map -2.5..+2.5 to a CRP adjustment.
/// Good governance (high score) reduces CRP; bad governance increases it.
/// Range roughly -2% to +3%.
fn governance_to_adjustment(score: Decimal) -> Decimal {
    // Linear: adjustment = -0.01 * score  (good governance lowers CRP)
    // Clamped score to [-2.5, 2.5]
    let clamped = if score < dec!(-2.5) {
        dec!(-2.5)
    } else if score > dec!(2.5) {
        dec!(2.5)
    } else {
        score
    };
    // At +2.5 -> -2.5%, at -2.5 -> +2.5%  => adjustment = -clamped / 100
    dec!(-1) * clamped / dec!(100)
}

/// Macro risk score (0-100) from GDP growth, inflation, FX vol.
/// Higher score = more macro risk.
fn compute_macro_risk_score(
    gdp_growth: Decimal,
    inflation_rate: Decimal,
    fx_volatility: Decimal,
) -> Decimal {
    // GDP component: low growth = risky.  Score = max(0, (5% - gdp_growth) / 5%) * 33
    let gdp_score = {
        let ratio = (dec!(0.05) - gdp_growth) / dec!(0.05);
        let clamped = if ratio < Decimal::ZERO {
            Decimal::ZERO
        } else if ratio > Decimal::ONE {
            Decimal::ONE
        } else {
            ratio
        };
        clamped * dec!(33)
    };

    // Inflation component: high inflation = risky.  Score = min(1, inflation / 15%) * 33
    let infl_score = {
        let ratio = inflation_rate / dec!(0.15);
        let clamped = if ratio < Decimal::ZERO {
            Decimal::ZERO
        } else if ratio > Decimal::ONE {
            Decimal::ONE
        } else {
            ratio
        };
        clamped * dec!(33)
    };

    // FX vol component: high FX vol = risky.  Score = min(1, fx_vol / 30%) * 34
    let fx_score = {
        let ratio = fx_volatility / dec!(0.30);
        let clamped = if ratio < Decimal::ZERO {
            Decimal::ZERO
        } else if ratio > Decimal::ONE {
            Decimal::ONE
        } else {
            ratio
        };
        clamped * dec!(34)
    };

    gdp_score + infl_score + fx_score
}

// ---------------------------------------------------------------------------
// Main function
// ---------------------------------------------------------------------------

/// Calculate emerging-market country risk premium using multiple methods.
pub fn calculate_country_risk_premium(
    input: &CountryRiskPremiumInput,
) -> CorpFinanceResult<CountryRiskPremiumOutput> {
    // Validation
    if input.bond_vol_local <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "bond_vol_local".to_string(),
            reason: "Bond volatility must be positive".to_string(),
        });
    }
    if input.equity_vol_local < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "equity_vol_local".to_string(),
            reason: "Equity volatility cannot be negative".to_string(),
        });
    }
    if input.sovereign_spread_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "sovereign_spread_bps".to_string(),
            reason: "Sovereign spread cannot be negative".to_string(),
        });
    }
    if input.governance_score < dec!(-2.5) || input.governance_score > dec!(2.5) {
        return Err(CorpFinanceError::InvalidInput {
            field: "governance_score".to_string(),
            reason: "Governance score must be between -2.5 and +2.5".to_string(),
        });
    }

    let (rating_premium, rating_pd) =
        rating_to_premium_and_pd(&input.country_rating).ok_or_else(|| {
            CorpFinanceError::InvalidInput {
                field: "country_rating".to_string(),
                reason: format!("Unknown rating: {}", input.country_rating),
            }
        })?;

    // 1. Damodaran CRP
    let lambda = input.equity_vol_local / input.bond_vol_local;
    let sovereign_spread_pct = input.sovereign_spread_bps / dec!(10000);
    let damodaran_crp = sovereign_spread_pct * lambda;

    // 2. Rating-based premium (already a decimal percentage)
    let rating_based_premium = rating_premium;

    // 3. Governance adjustment
    let governance_adjustment = governance_to_adjustment(input.governance_score);

    // 4. Composite CRP = 50% Damodaran + 30% rating + 20% governance-adjusted Damodaran
    let governance_adjusted = damodaran_crp + governance_adjustment;
    let governance_adj_clamped = if governance_adjusted < Decimal::ZERO {
        Decimal::ZERO
    } else {
        governance_adjusted
    };
    let composite_crp = dec!(0.50) * damodaran_crp
        + dec!(0.30) * rating_based_premium
        + dec!(0.20) * governance_adj_clamped;

    // 5. Total cost of equity
    let total_cost_of_equity = input.risk_free_rate + input.us_equity_risk_premium + composite_crp;

    // 6. Macro risk score
    let macro_risk_score =
        compute_macro_risk_score(input.gdp_growth, input.inflation_rate, input.fx_volatility);

    Ok(CountryRiskPremiumOutput {
        damodaran_crp,
        rating_based_premium,
        composite_crp,
        total_cost_of_equity,
        lambda,
        governance_adjustment,
        macro_risk_score,
        rating_implied_default_prob: rating_pd,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base_input() -> CountryRiskPremiumInput {
        CountryRiskPremiumInput {
            sovereign_spread_bps: dec!(250),
            equity_vol_local: dec!(0.25),
            bond_vol_local: dec!(0.10),
            us_equity_risk_premium: dec!(0.055),
            country_rating: "Baa2".to_string(),
            gdp_growth: dec!(0.04),
            inflation_rate: dec!(0.06),
            fx_volatility: dec!(0.12),
            governance_score: dec!(0.5),
            risk_free_rate: dec!(0.04),
        }
    }

    #[test]
    fn test_investment_grade_em() {
        let input = base_input();
        let out = calculate_country_risk_premium(&input).unwrap();
        // lambda = 0.25 / 0.10 = 2.5
        assert_eq!(out.lambda, dec!(2.5));
        // damodaran_crp = 250/10000 * 2.5 = 0.0625
        assert_eq!(out.damodaran_crp, dec!(0.0625));
        assert!(out.composite_crp > Decimal::ZERO);
        assert!(out.total_cost_of_equity > dec!(0.10));
    }

    #[test]
    fn test_frontier_market_high_spread() {
        let mut input = base_input();
        input.sovereign_spread_bps = dec!(800);
        input.country_rating = "B3".to_string();
        input.governance_score = dec!(-1.5);
        let out = calculate_country_risk_premium(&input).unwrap();
        // Higher spread => higher CRP
        assert!(out.damodaran_crp > dec!(0.15));
        assert!(out.rating_based_premium == dec!(0.0750));
    }

    #[test]
    fn test_damodaran_lambda() {
        let input = base_input();
        let out = calculate_country_risk_premium(&input).unwrap();
        assert_eq!(out.lambda, dec!(2.5));
    }

    #[test]
    fn test_lambda_one() {
        let mut input = base_input();
        input.equity_vol_local = dec!(0.10);
        input.bond_vol_local = dec!(0.10);
        let out = calculate_country_risk_premium(&input).unwrap();
        assert_eq!(out.lambda, Decimal::ONE);
        // damodaran_crp = spread_pct * 1 = spread_pct
        assert_eq!(out.damodaran_crp, dec!(0.025));
    }

    #[test]
    fn test_aaa_rating_zero_premium() {
        let mut input = base_input();
        input.country_rating = "Aaa".to_string();
        let out = calculate_country_risk_premium(&input).unwrap();
        assert_eq!(out.rating_based_premium, Decimal::ZERO);
        assert_eq!(out.rating_implied_default_prob, dec!(0.0001));
    }

    #[test]
    fn test_c_rating_max_premium() {
        let mut input = base_input();
        input.country_rating = "C".to_string();
        let out = calculate_country_risk_premium(&input).unwrap();
        assert_eq!(out.rating_based_premium, dec!(0.15));
        assert_eq!(out.rating_implied_default_prob, dec!(0.50));
    }

    #[test]
    fn test_governance_positive_reduces_crp() {
        let mut input = base_input();
        input.governance_score = dec!(2.0);
        let out = calculate_country_risk_premium(&input).unwrap();
        assert!(out.governance_adjustment < Decimal::ZERO);
    }

    #[test]
    fn test_governance_negative_increases_crp() {
        let mut input = base_input();
        input.governance_score = dec!(-2.0);
        let out = calculate_country_risk_premium(&input).unwrap();
        assert!(out.governance_adjustment > Decimal::ZERO);
    }

    #[test]
    fn test_governance_zero_neutral() {
        let mut input = base_input();
        input.governance_score = Decimal::ZERO;
        let out = calculate_country_risk_premium(&input).unwrap();
        assert_eq!(out.governance_adjustment, Decimal::ZERO);
    }

    #[test]
    fn test_macro_risk_score_high_inflation() {
        let mut input = base_input();
        input.inflation_rate = dec!(0.15); // 15% inflation
        let out = calculate_country_risk_premium(&input).unwrap();
        assert!(out.macro_risk_score > dec!(40));
    }

    #[test]
    fn test_macro_risk_score_high_growth_low_risk() {
        let mut input = base_input();
        input.gdp_growth = dec!(0.08);
        input.inflation_rate = dec!(0.02);
        input.fx_volatility = dec!(0.05);
        let out = calculate_country_risk_premium(&input).unwrap();
        assert!(out.macro_risk_score < dec!(25));
    }

    #[test]
    fn test_macro_risk_score_range() {
        let input = base_input();
        let out = calculate_country_risk_premium(&input).unwrap();
        assert!(out.macro_risk_score >= Decimal::ZERO);
        assert!(out.macro_risk_score <= dec!(100));
    }

    #[test]
    fn test_total_cost_of_equity_components() {
        let input = base_input();
        let out = calculate_country_risk_premium(&input).unwrap();
        // total = rf + us_erp + composite_crp
        let expected = input.risk_free_rate + input.us_equity_risk_premium + out.composite_crp;
        assert_eq!(out.total_cost_of_equity, expected);
    }

    #[test]
    fn test_composite_crp_weights() {
        let input = base_input();
        let out = calculate_country_risk_premium(&input).unwrap();
        // Composite must be between the min and max of its components
        let min_comp = out.damodaran_crp.min(out.rating_based_premium);
        let max_comp = out.damodaran_crp.max(out.rating_based_premium);
        // Allow governance adjustment to widen range slightly
        assert!(out.composite_crp >= min_comp - dec!(0.03));
        assert!(out.composite_crp <= max_comp + dec!(0.03));
    }

    #[test]
    fn test_zero_spread() {
        let mut input = base_input();
        input.sovereign_spread_bps = Decimal::ZERO;
        let out = calculate_country_risk_premium(&input).unwrap();
        assert_eq!(out.damodaran_crp, Decimal::ZERO);
    }

    #[test]
    fn test_invalid_bond_vol_zero() {
        let mut input = base_input();
        input.bond_vol_local = Decimal::ZERO;
        let err = calculate_country_risk_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_negative_spread() {
        let mut input = base_input();
        input.sovereign_spread_bps = dec!(-100);
        let err = calculate_country_risk_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_governance_out_of_range() {
        let mut input = base_input();
        input.governance_score = dec!(3.0);
        let err = calculate_country_risk_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_invalid_unknown_rating() {
        let mut input = base_input();
        input.country_rating = "XYZ".to_string();
        let err = calculate_country_risk_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }

    #[test]
    fn test_high_spread_frontier() {
        let mut input = base_input();
        input.sovereign_spread_bps = dec!(1200);
        input.equity_vol_local = dec!(0.40);
        input.bond_vol_local = dec!(0.15);
        input.country_rating = "Caa1".to_string();
        input.governance_score = dec!(-2.0);
        let out = calculate_country_risk_premium(&input).unwrap();
        assert!(out.total_cost_of_equity > dec!(0.25));
    }

    #[test]
    fn test_ba_ratings() {
        for (rating, expected) in [
            ("Ba1", dec!(0.0300)),
            ("Ba2", dec!(0.0375)),
            ("Ba3", dec!(0.0450)),
        ] {
            let mut input = base_input();
            input.country_rating = rating.to_string();
            let out = calculate_country_risk_premium(&input).unwrap();
            assert_eq!(out.rating_based_premium, expected);
        }
    }

    #[test]
    fn test_negative_equity_vol_rejected() {
        let mut input = base_input();
        input.equity_vol_local = dec!(-0.10);
        let err = calculate_country_risk_premium(&input).unwrap_err();
        assert!(matches!(err, CorpFinanceError::InvalidInput { .. }));
    }
}
