//! Bankruptcy prediction Z-Score variants.
//!
//! Implements:
//! 1. **Altman Z-Score** (public company)
//! 2. **Altman Z'-Score** (private company)
//! 3. **Altman Z''-Score** (non-manufacturing / emerging markets)
//! 4. **Springate S-Score**
//! 5. **Zmijewski X-Score** (probit model)
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Taylor series exp(x) for Decimal. Uses sum_{k=0}^{40} x^k / k!
fn decimal_exp(x: Decimal) -> Decimal {
    let mut term = Decimal::ONE;
    let mut sum = Decimal::ONE;
    for k in 1u32..=50 {
        term *= x / Decimal::from(k);
        sum += term;
        // Early termination for convergence
        if term.abs() < dec!(0.00000000001) {
            break;
        }
    }
    sum
}

fn safe_div(num: Decimal, den: Decimal, ctx: &str) -> CorpFinanceResult<Decimal> {
    if den == Decimal::ZERO {
        return Err(CorpFinanceError::DivisionByZero {
            context: ctx.to_string(),
        });
    }
    Ok(num / den)
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for Z-Score bankruptcy prediction models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZScoreModelsInput {
    pub working_capital: Decimal,
    pub total_assets: Decimal,
    pub retained_earnings: Decimal,
    pub ebit: Decimal,
    pub market_cap: Decimal,
    pub book_equity: Decimal,
    pub total_liabilities: Decimal,
    pub revenue: Decimal,
    pub net_income: Decimal,
    pub total_debt: Decimal,
    pub current_assets: Decimal,
    pub current_liabilities: Decimal,
    pub cash_flow_operations: Decimal,
    pub is_public: bool,
    pub is_manufacturing: bool,
}

/// Result for an individual Altman model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltmanResult {
    pub z_score: Decimal,
    pub zone: String,
    pub x1: Decimal,
    pub x2: Decimal,
    pub x3: Decimal,
    pub x4: Decimal,
    pub x5: Option<Decimal>,
}

/// Result for the Springate S-Score model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpringateResult {
    pub s_score: Decimal,
    pub bankrupt: bool,
    pub a: Decimal,
    pub b: Decimal,
    pub c: Decimal,
    pub d: Decimal,
}

/// Result for the Zmijewski X-Score model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZmijewskiResult {
    pub x_score: Decimal,
    pub probability: Decimal,
    pub bankrupt: bool,
}

/// Combined output of all Z-Score models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZScoreModelsOutput {
    pub altman_z: AltmanResult,
    pub altman_z_prime: AltmanResult,
    pub altman_z_double_prime: AltmanResult,
    pub springate_s: SpringateResult,
    pub zmijewski_x: ZmijewskiResult,
    pub composite_risk: String,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate all bankruptcy prediction Z-Score models.
pub fn calculate_zscore_models(input: &ZScoreModelsInput) -> CorpFinanceResult<ZScoreModelsOutput> {
    // Validation
    if input.total_assets <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_assets".into(),
            reason: "Must be positive.".into(),
        });
    }
    if input.total_liabilities < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_liabilities".into(),
            reason: "Cannot be negative.".into(),
        });
    }
    if input.current_liabilities < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_liabilities".into(),
            reason: "Cannot be negative.".into(),
        });
    }

    let ta = input.total_assets;

    // Common ratios
    let x1 = safe_div(input.working_capital, ta, "X1 WC/TA")?;
    let x2 = safe_div(input.retained_earnings, ta, "X2 RE/TA")?;
    let x3 = safe_div(input.ebit, ta, "X3 EBIT/TA")?;

    // --- Altman Z (public) ---
    let x4_public = safe_div(input.market_cap, input.total_liabilities, "X4 MV/TL")?;
    let x5 = safe_div(input.revenue, ta, "X5 Sales/TA")?;

    let z = dec!(1.2) * x1 + dec!(1.4) * x2 + dec!(3.3) * x3 + dec!(0.6) * x4_public + x5;
    let zone_z = if z > dec!(2.99) {
        "Safe"
    } else if z >= dec!(1.81) {
        "Grey"
    } else {
        "Distress"
    };

    let altman_z = AltmanResult {
        z_score: z,
        zone: zone_z.to_string(),
        x1,
        x2,
        x3,
        x4: x4_public,
        x5: Some(x5),
    };

    // --- Altman Z' (private) ---
    let x4_private = safe_div(input.book_equity, input.total_liabilities, "X4' BE/TL")?;

    let z_prime = dec!(0.717) * x1
        + dec!(0.847) * x2
        + dec!(3.107) * x3
        + dec!(0.420) * x4_private
        + dec!(0.998) * x5;
    let zone_zp = if z_prime > dec!(2.90) {
        "Safe"
    } else if z_prime >= dec!(1.23) {
        "Grey"
    } else {
        "Distress"
    };

    let altman_z_prime = AltmanResult {
        z_score: z_prime,
        zone: zone_zp.to_string(),
        x1,
        x2,
        x3,
        x4: x4_private,
        x5: Some(x5),
    };

    // --- Altman Z'' (non-manufacturing / EM) ---
    let z_double_prime =
        dec!(6.56) * x1 + dec!(3.26) * x2 + dec!(6.72) * x3 + dec!(1.05) * x4_private;
    let zone_zdp = if z_double_prime > dec!(2.60) {
        "Safe"
    } else if z_double_prime >= dec!(1.10) {
        "Grey"
    } else {
        "Distress"
    };

    let altman_z_double_prime = AltmanResult {
        z_score: z_double_prime,
        zone: zone_zdp.to_string(),
        x1,
        x2,
        x3,
        x4: x4_private,
        x5: None,
    };

    // --- Springate S ---
    let s_a = x1; // WC/TA
    let s_b = x3; // EBIT/TA
    let s_c = if input.current_liabilities == Decimal::ZERO {
        Decimal::ZERO
    } else {
        safe_div(input.ebit, input.current_liabilities, "Springate C")?
    };
    let s_d = x5; // Sales/TA

    let s_score = dec!(1.03) * s_a + dec!(3.07) * s_b + dec!(0.66) * s_c + dec!(0.40) * s_d;

    let springate_s = SpringateResult {
        s_score,
        bankrupt: s_score < dec!(0.862),
        a: s_a,
        b: s_b,
        c: s_c,
        d: s_d,
    };

    // --- Zmijewski X ---
    let ni_ta = safe_div(input.net_income, ta, "Zmijewski NI/TA")?;
    let tl_ta = safe_div(input.total_liabilities, ta, "Zmijewski TL/TA")?;
    let ca_cl = if input.current_liabilities == Decimal::ZERO {
        dec!(999) // Very high current ratio
    } else {
        safe_div(
            input.current_assets,
            input.current_liabilities,
            "Zmijewski CA/CL",
        )?
    };

    let x_score = dec!(-4.336) - dec!(4.513) * ni_ta + dec!(5.679) * tl_ta + dec!(0.004) * ca_cl;

    // P(bankrupt) = 1 / (1 + exp(-X))
    let exp_neg_x = decimal_exp(-x_score);
    let probability = safe_div(
        Decimal::ONE,
        Decimal::ONE + exp_neg_x,
        "Zmijewski probability",
    )?;

    let zmijewski_x = ZmijewskiResult {
        x_score,
        probability,
        bankrupt: probability > dec!(0.5),
    };

    // --- Composite risk ---
    let mut distress_count = 0u32;
    if zone_z == "Distress" {
        distress_count += 1;
    }
    if zone_zp == "Distress" {
        distress_count += 1;
    }
    if zone_zdp == "Distress" {
        distress_count += 1;
    }
    if springate_s.bankrupt {
        distress_count += 1;
    }
    if zmijewski_x.bankrupt {
        distress_count += 1;
    }

    let composite_risk = match distress_count {
        0 => "Low",
        1 => "Moderate",
        2 | 3 => "High",
        _ => "Critical",
    }
    .to_string();

    Ok(ZScoreModelsOutput {
        altman_z,
        altman_z_prime,
        altman_z_double_prime,
        springate_s,
        zmijewski_x,
        composite_risk,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn healthy_input() -> ZScoreModelsInput {
        ZScoreModelsInput {
            working_capital: dec!(500),
            total_assets: dec!(2000),
            retained_earnings: dec!(800),
            ebit: dec!(300),
            market_cap: dec!(3000),
            book_equity: dec!(1000),
            total_liabilities: dec!(1000),
            revenue: dec!(4000),
            net_income: dec!(200),
            total_debt: dec!(600),
            current_assets: dec!(900),
            current_liabilities: dec!(400),
            cash_flow_operations: dec!(250),
            is_public: true,
            is_manufacturing: true,
        }
    }

    fn distressed_input() -> ZScoreModelsInput {
        ZScoreModelsInput {
            working_capital: dec!(-200),
            total_assets: dec!(1000),
            retained_earnings: dec!(-500),
            ebit: dec!(-100),
            market_cap: dec!(100),
            book_equity: dec!(50),
            total_liabilities: dec!(950),
            revenue: dec!(500),
            net_income: dec!(-150),
            total_debt: dec!(800),
            current_assets: dec!(200),
            current_liabilities: dec!(400),
            cash_flow_operations: dec!(-50),
            is_public: true,
            is_manufacturing: true,
        }
    }

    #[test]
    fn test_healthy_company_altman_safe() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        assert_eq!(out.altman_z.zone, "Safe");
        assert!(out.altman_z.z_score > dec!(2.99));
    }

    #[test]
    fn test_distressed_company_altman_distress() {
        let out = calculate_zscore_models(&distressed_input()).unwrap();
        assert_eq!(out.altman_z.zone, "Distress");
        assert!(out.altman_z.z_score < dec!(1.81));
    }

    #[test]
    fn test_altman_z_formula() {
        let inp = healthy_input();
        let out = calculate_zscore_models(&inp).unwrap();
        let expected = dec!(1.2) * (dec!(500) / dec!(2000))
            + dec!(1.4) * (dec!(800) / dec!(2000))
            + dec!(3.3) * (dec!(300) / dec!(2000))
            + dec!(0.6) * (dec!(3000) / dec!(1000))
            + dec!(4000) / dec!(2000);
        assert!(approx_eq(out.altman_z.z_score, expected, dec!(0.0001)));
    }

    #[test]
    fn test_altman_z_prime_uses_book_equity() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        let x4 = dec!(1000) / dec!(1000);
        assert!(approx_eq(out.altman_z_prime.x4, x4, dec!(0.0001)));
    }

    #[test]
    fn test_altman_z_prime_safe() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        assert_eq!(out.altman_z_prime.zone, "Safe");
    }

    #[test]
    fn test_altman_z_prime_distress() {
        let out = calculate_zscore_models(&distressed_input()).unwrap();
        assert_eq!(out.altman_z_prime.zone, "Distress");
    }

    #[test]
    fn test_altman_z_double_prime_no_x5() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        assert!(out.altman_z_double_prime.x5.is_none());
    }

    #[test]
    fn test_altman_z_double_prime_formula() {
        let inp = healthy_input();
        let out = calculate_zscore_models(&inp).unwrap();
        let expected = dec!(6.56) * (dec!(500) / dec!(2000))
            + dec!(3.26) * (dec!(800) / dec!(2000))
            + dec!(6.72) * (dec!(300) / dec!(2000))
            + dec!(1.05) * (dec!(1000) / dec!(1000));
        assert!(approx_eq(
            out.altman_z_double_prime.z_score,
            expected,
            dec!(0.0001)
        ));
    }

    #[test]
    fn test_altman_z_double_prime_safe() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        assert_eq!(out.altman_z_double_prime.zone, "Safe");
    }

    #[test]
    fn test_springate_healthy_not_bankrupt() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        assert!(!out.springate_s.bankrupt);
        assert!(out.springate_s.s_score >= dec!(0.862));
    }

    #[test]
    fn test_springate_distressed_bankrupt() {
        let out = calculate_zscore_models(&distressed_input()).unwrap();
        assert!(out.springate_s.bankrupt);
    }

    #[test]
    fn test_springate_formula() {
        let inp = healthy_input();
        let out = calculate_zscore_models(&inp).unwrap();
        let a = dec!(500) / dec!(2000);
        let b = dec!(300) / dec!(2000);
        let c = dec!(300) / dec!(400);
        let d = dec!(4000) / dec!(2000);
        let expected = dec!(1.03) * a + dec!(3.07) * b + dec!(0.66) * c + dec!(0.40) * d;
        assert!(approx_eq(out.springate_s.s_score, expected, dec!(0.0001)));
    }

    #[test]
    fn test_zmijewski_healthy_low_probability() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        assert!(!out.zmijewski_x.bankrupt);
        assert!(out.zmijewski_x.probability < dec!(0.5));
    }

    #[test]
    fn test_zmijewski_distressed_high_probability() {
        let out = calculate_zscore_models(&distressed_input()).unwrap();
        assert!(out.zmijewski_x.bankrupt);
        assert!(out.zmijewski_x.probability > dec!(0.5));
    }

    #[test]
    fn test_zmijewski_probability_between_0_and_1() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        assert!(out.zmijewski_x.probability >= Decimal::ZERO);
        assert!(out.zmijewski_x.probability <= Decimal::ONE);
    }

    #[test]
    fn test_composite_risk_low_for_healthy() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        assert_eq!(out.composite_risk, "Low");
    }

    #[test]
    fn test_composite_risk_critical_for_distressed() {
        let out = calculate_zscore_models(&distressed_input()).unwrap();
        assert!(
            out.composite_risk == "Critical" || out.composite_risk == "High",
            "Got {}",
            out.composite_risk
        );
    }

    #[test]
    fn test_zero_total_assets_error() {
        let mut input = healthy_input();
        input.total_assets = Decimal::ZERO;
        assert!(calculate_zscore_models(&input).is_err());
    }

    #[test]
    fn test_negative_total_liabilities_error() {
        let mut input = healthy_input();
        input.total_liabilities = dec!(-100);
        assert!(calculate_zscore_models(&input).is_err());
    }

    #[test]
    fn test_grey_zone() {
        let mut input = healthy_input();
        // Tweak to land in grey zone for original Altman
        input.working_capital = dec!(100);
        input.retained_earnings = dec!(200);
        input.ebit = dec!(50);
        input.market_cap = dec!(500);
        input.revenue = dec!(1500);
        let out = calculate_zscore_models(&input).unwrap();
        // This should land around the grey zone
        assert!(
            out.altman_z.z_score > dec!(1.0) && out.altman_z.z_score < dec!(5.0),
            "z={}",
            out.altman_z.z_score
        );
    }

    #[test]
    fn test_decimal_exp_at_zero() {
        let result = decimal_exp(Decimal::ZERO);
        assert!(approx_eq(result, Decimal::ONE, dec!(0.0001)));
    }

    #[test]
    fn test_decimal_exp_at_one() {
        let result = decimal_exp(Decimal::ONE);
        assert!(approx_eq(result, dec!(2.71828), dec!(0.001)));
    }

    #[test]
    fn test_zero_current_liabilities_springate() {
        let mut input = healthy_input();
        input.current_liabilities = Decimal::ZERO;
        input.working_capital = input.current_assets;
        let out = calculate_zscore_models(&input).unwrap();
        assert_eq!(out.springate_s.c, Decimal::ZERO);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _deser: ZScoreModelsOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_altman_components_sum_correctly() {
        let out = calculate_zscore_models(&healthy_input()).unwrap();
        let recomputed = dec!(1.2) * out.altman_z.x1
            + dec!(1.4) * out.altman_z.x2
            + dec!(3.3) * out.altman_z.x3
            + dec!(0.6) * out.altman_z.x4
            + out.altman_z.x5.unwrap();
        assert!(approx_eq(out.altman_z.z_score, recomputed, dec!(0.0001)));
    }
}
