//! Beneish M-Score model for detecting earnings manipulation.
//!
//! Implements the 8-variable model from Messod D. Beneish (1999):
//! DSRI, GMI, AQI, SGI, DEPI, SGAI, LVGI, TATA.
//! M > -1.78 suggests likely manipulation.
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

/// Financial data for current and prior year, used to compute the Beneish M-Score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeneishInput {
    pub current_receivables: Decimal,
    pub prior_receivables: Decimal,
    pub current_revenue: Decimal,
    pub prior_revenue: Decimal,
    pub current_cogs: Decimal,
    pub prior_cogs: Decimal,
    pub current_total_assets: Decimal,
    pub prior_total_assets: Decimal,
    pub current_ppe: Decimal,
    pub prior_ppe: Decimal,
    pub current_depreciation: Decimal,
    pub prior_depreciation: Decimal,
    pub current_sga: Decimal,
    pub prior_sga: Decimal,
    pub current_total_debt: Decimal,
    pub prior_total_debt: Decimal,
    pub current_net_income: Decimal,
    pub current_cfo: Decimal,
}

/// Beneish M-Score results with all 8 component ratios.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeneishOutput {
    pub dsri: Decimal,
    pub gmi: Decimal,
    pub aqi: Decimal,
    pub sgi: Decimal,
    pub depi: Decimal,
    pub sgai: Decimal,
    pub lvgi: Decimal,
    pub tata: Decimal,
    pub m_score: Decimal,
    /// True when M-Score > -1.78, suggesting likely manipulation.
    pub manipulation_likely: bool,
}

// Coefficients
const INTERCEPT: Decimal = dec!(-4.84);
const C_DSRI: Decimal = dec!(0.920);
const C_GMI: Decimal = dec!(0.528);
const C_AQI: Decimal = dec!(0.404);
const C_SGI: Decimal = dec!(0.892);
const C_DEPI: Decimal = dec!(0.115);
const C_SGAI: Decimal = dec!(-0.172);
const C_TATA: Decimal = dec!(4.679);
const C_LVGI: Decimal = dec!(-0.327);
const THRESHOLD: Decimal = dec!(-1.78);

fn safe_div(num: Decimal, den: Decimal, ctx: &str) -> CorpFinanceResult<Decimal> {
    if den == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: ctx.to_string(),
        });
    }
    Ok(num / den)
}

fn validate_positive(val: Decimal, field: &str) -> CorpFinanceResult<()> {
    if val <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: field.into(),
            reason: "Must be positive".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the Beneish M-Score (8-variable model).
pub fn calculate_beneish_m_score(input: &BeneishInput) -> CorpFinanceResult<BeneishOutput> {
    validate_positive(input.current_revenue, "current_revenue")?;
    validate_positive(input.prior_revenue, "prior_revenue")?;
    validate_positive(input.current_total_assets, "current_total_assets")?;
    validate_positive(input.prior_total_assets, "prior_total_assets")?;

    // DSRI
    let recv_rev_cur = safe_div(input.current_receivables, input.current_revenue, "DSRI cur")?;
    let recv_rev_pri = safe_div(input.prior_receivables, input.prior_revenue, "DSRI pri")?;
    let dsri = safe_div(recv_rev_cur, recv_rev_pri, "DSRI ratio")?;

    // GMI
    let prior_gm = safe_div(
        input.prior_revenue - input.prior_cogs,
        input.prior_revenue,
        "GMI prior",
    )?;
    let current_gm = safe_div(
        input.current_revenue - input.current_cogs,
        input.current_revenue,
        "GMI current",
    )?;
    let gmi = safe_div(prior_gm, current_gm, "GMI ratio")?;

    // AQI: 1 - (PPE / TA) per period
    let aqi_cur = Decimal::ONE - safe_div(input.current_ppe, input.current_total_assets, "AQI")?;
    let aqi_pri = Decimal::ONE - safe_div(input.prior_ppe, input.prior_total_assets, "AQI")?;
    let aqi = safe_div(aqi_cur, aqi_pri, "AQI ratio")?;

    // SGI
    let sgi = safe_div(input.current_revenue, input.prior_revenue, "SGI")?;

    // DEPI
    let pri_dep = safe_div(
        input.prior_depreciation,
        input.prior_ppe + input.prior_depreciation,
        "DEPI prior",
    )?;
    let cur_dep = safe_div(
        input.current_depreciation,
        input.current_ppe + input.current_depreciation,
        "DEPI current",
    )?;
    let depi = safe_div(pri_dep, cur_dep, "DEPI ratio")?;

    // SGAI
    let sga_cur = safe_div(input.current_sga, input.current_revenue, "SGAI cur")?;
    let sga_pri = safe_div(input.prior_sga, input.prior_revenue, "SGAI pri")?;
    let sgai = safe_div(sga_cur, sga_pri, "SGAI ratio")?;

    // LVGI
    let lev_cur = safe_div(
        input.current_total_debt,
        input.current_total_assets,
        "LVGI cur",
    )?;
    let lev_pri = safe_div(input.prior_total_debt, input.prior_total_assets, "LVGI pri")?;
    let lvgi = safe_div(lev_cur, lev_pri, "LVGI ratio")?;

    // TATA
    let tata = safe_div(
        input.current_net_income - input.current_cfo,
        input.current_total_assets,
        "TATA",
    )?;

    // M-Score
    let m_score = INTERCEPT
        + C_DSRI * dsri
        + C_GMI * gmi
        + C_AQI * aqi
        + C_SGI * sgi
        + C_DEPI * depi
        + C_SGAI * sgai
        + C_TATA * tata
        + C_LVGI * lvgi;

    Ok(BeneishOutput {
        dsri,
        gmi,
        aqi,
        sgi,
        depi,
        sgai,
        lvgi,
        tata,
        m_score,
        manipulation_likely: m_score > THRESHOLD,
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn normal_input() -> BeneishInput {
        BeneishInput {
            current_receivables: dec!(100),
            prior_receivables: dec!(95),
            current_revenue: dec!(1000),
            prior_revenue: dec!(950),
            current_cogs: dec!(600),
            prior_cogs: dec!(570),
            current_total_assets: dec!(2000),
            prior_total_assets: dec!(1900),
            current_ppe: dec!(800),
            prior_ppe: dec!(770),
            current_depreciation: dec!(80),
            prior_depreciation: dec!(77),
            current_sga: dec!(150),
            prior_sga: dec!(143),
            current_total_debt: dec!(500),
            prior_total_debt: dec!(480),
            current_net_income: dec!(120),
            current_cfo: dec!(140),
        }
    }

    fn unity_input() -> BeneishInput {
        BeneishInput {
            current_receivables: dec!(100),
            prior_receivables: dec!(100),
            current_revenue: dec!(1000),
            prior_revenue: dec!(1000),
            current_cogs: dec!(600),
            prior_cogs: dec!(600),
            current_total_assets: dec!(2000),
            prior_total_assets: dec!(2000),
            current_ppe: dec!(800),
            prior_ppe: dec!(800),
            current_depreciation: dec!(80),
            prior_depreciation: dec!(80),
            current_sga: dec!(150),
            prior_sga: dec!(150),
            current_total_debt: dec!(500),
            prior_total_debt: dec!(500),
            current_net_income: dec!(100),
            current_cfo: dec!(100),
        }
    }

    #[test]
    fn test_normal_company_not_flagged() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert!(!out.manipulation_likely, "M = {}", out.m_score);
    }

    #[test]
    fn test_dsri_calculation() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert_eq!(out.dsri, Decimal::ONE);
    }

    #[test]
    fn test_sgi_calculation() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert_eq!(out.sgi, dec!(1000) / dec!(950));
    }

    #[test]
    fn test_tata_calculation() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert_eq!(out.tata, dec!(-0.01));
    }

    #[test]
    fn test_gmi_stable_margin() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert_eq!(out.gmi, Decimal::ONE);
    }

    #[test]
    fn test_manipulation_flagged() {
        let mut input = normal_input();
        input.current_receivables = dec!(300);
        input.current_revenue = dec!(1600);
        input.current_cogs = dec!(1200);
        input.current_net_income = dec!(200);
        input.current_cfo = dec!(20);
        let out = calculate_beneish_m_score(&input).unwrap();
        assert!(out.manipulation_likely, "M = {}", out.m_score);
    }

    #[test]
    fn test_m_score_boundary_below() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert!(out.m_score < THRESHOLD);
    }

    #[test]
    fn test_zero_current_revenue_rejected() {
        let mut input = normal_input();
        input.current_revenue = Decimal::ZERO;
        match calculate_beneish_m_score(&input).unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "current_revenue"),
            e => panic!("Expected InvalidInput, got {e}"),
        }
    }

    #[test]
    fn test_zero_prior_revenue_rejected() {
        let mut input = normal_input();
        input.prior_revenue = Decimal::ZERO;
        match calculate_beneish_m_score(&input).unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "prior_revenue"),
            e => panic!("Expected InvalidInput, got {e}"),
        }
    }

    #[test]
    fn test_zero_current_assets_rejected() {
        let mut input = normal_input();
        input.current_total_assets = Decimal::ZERO;
        match calculate_beneish_m_score(&input).unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "current_total_assets")
            }
            e => panic!("Expected InvalidInput, got {e}"),
        }
    }

    #[test]
    fn test_zero_prior_assets_rejected() {
        let mut input = normal_input();
        input.prior_total_assets = Decimal::ZERO;
        match calculate_beneish_m_score(&input).unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "prior_total_assets"),
            e => panic!("Expected InvalidInput, got {e}"),
        }
    }

    #[test]
    fn test_negative_revenue_rejected() {
        let mut input = normal_input();
        input.current_revenue = dec!(-100);
        assert!(calculate_beneish_m_score(&input).is_err());
    }

    #[test]
    fn test_lvgi_stable_leverage() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert!(out.lvgi > dec!(0.98) && out.lvgi < dec!(1.01));
    }

    #[test]
    fn test_depi_stable_depreciation() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert!(out.depi > dec!(0.99) && out.depi < dec!(1.01));
    }

    #[test]
    fn test_sgai_stable() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert!(out.sgai > dec!(0.99) && out.sgai < dec!(1.01));
    }

    #[test]
    fn test_aqi_stable() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        assert!(out.aqi > dec!(1.0) && out.aqi < dec!(1.02));
    }

    #[test]
    fn test_high_receivables_growth_raises_dsri() {
        let mut input = normal_input();
        input.current_receivables = dec!(200);
        let out = calculate_beneish_m_score(&input).unwrap();
        assert!(out.dsri > dec!(1.5), "DSRI = {}", out.dsri);
    }

    #[test]
    fn test_margin_compression_raises_gmi() {
        let mut input = normal_input();
        input.current_cogs = dec!(750);
        let out = calculate_beneish_m_score(&input).unwrap();
        assert!(out.gmi > dec!(1.0));
    }

    #[test]
    fn test_high_accruals_raises_tata() {
        let mut input = normal_input();
        input.current_net_income = dec!(200);
        input.current_cfo = dec!(50);
        let out = calculate_beneish_m_score(&input).unwrap();
        assert!(out.tata > dec!(0.07));
    }

    #[test]
    fn test_all_ratios_one_yields_known_score() {
        let out = calculate_beneish_m_score(&unity_input()).unwrap();
        let expected =
            dec!(-4.84) + dec!(0.920) + dec!(0.528) + dec!(0.404) + dec!(0.892) + dec!(0.115)
                - dec!(0.172)
                - dec!(0.327);
        assert_eq!(out.m_score, expected);
        assert!(!out.manipulation_likely);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = normal_input();
        let json = serde_json::to_string(&input).unwrap();
        let deser: BeneishInput = serde_json::from_str(&json).unwrap();
        assert_eq!(
            calculate_beneish_m_score(&input).unwrap().m_score,
            calculate_beneish_m_score(&deser).unwrap().m_score,
        );
    }

    #[test]
    fn test_output_serialization() {
        let out = calculate_beneish_m_score(&normal_input()).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let deser: BeneishOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(out.m_score, deser.m_score);
        assert_eq!(out.manipulation_likely, deser.manipulation_likely);
    }
}
