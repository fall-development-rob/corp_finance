//! PIT vs TTC PD calibration using the Vasicek single-factor model.
//!
//! Covers:
//! 1. **Point-in-Time PD** -- adjust TTC PD for current macro conditions.
//! 2. **Through-the-Cycle PD** -- reverse mapping from PIT to TTC.
//! 3. **Basel IRB Asset Correlation** -- formula-based rho.
//! 4. **Central Tendency** -- long-run average vs current PD ratio.
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Exponential via Taylor series.
fn decimal_exp(x: Decimal) -> Decimal {
    let ln2 = dec!(0.6931471805599453);
    let n_raw = x / ln2;
    let n = if n_raw >= Decimal::ZERO {
        n_raw.floor()
    } else {
        n_raw.ceil() - Decimal::ONE
    };
    let r = x - n * ln2;

    let mut term = Decimal::ONE;
    let mut sum = Decimal::ONE;
    for i in 1u32..40 {
        term = term * r / Decimal::from(i);
        sum += term;
    }

    let n_i64 = n.to_string().parse::<i64>().unwrap_or(0);
    if n_i64 >= 0 {
        let mut pow2 = Decimal::ONE;
        for _ in 0..n_i64 {
            pow2 *= dec!(2);
        }
        sum * pow2
    } else {
        let mut pow2 = Decimal::ONE;
        for _ in 0..(-n_i64) {
            pow2 *= dec!(2);
        }
        sum / pow2
    }
}

/// Square root via Newton's method (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess.is_zero() {
        guess = Decimal::ONE;
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

/// Cumulative normal distribution (Abramowitz & Stegun).
fn norm_cdf(x: Decimal) -> Decimal {
    if x <= dec!(-10) {
        return Decimal::ZERO;
    }
    if x >= dec!(10) {
        return Decimal::ONE;
    }
    let is_neg = x < Decimal::ZERO;
    let abs_x = x.abs();

    let p = dec!(0.2316419);
    let b1 = dec!(0.319381530);
    let b2 = dec!(-0.356563782);
    let b3 = dec!(1.781477937);
    let b4 = dec!(-1.821255978);
    let b5 = dec!(1.330274429);

    let t = Decimal::ONE / (Decimal::ONE + p * abs_x);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    let sqrt_2pi = dec!(2.506628274631);
    let pdf = decimal_exp(-(abs_x * abs_x) / dec!(2)) / sqrt_2pi;

    let cdf = Decimal::ONE - pdf * (b1 * t + b2 * t2 + b3 * t3 + b4 * t4 + b5 * t5);

    if is_neg {
        Decimal::ONE - cdf
    } else {
        cdf
    }
}

/// Inverse normal (rational approximation + Newton refinement for precision).
fn norm_inv(p: Decimal) -> Decimal {
    if p <= Decimal::ZERO {
        return dec!(-10);
    }
    if p >= Decimal::ONE {
        return dec!(10);
    }

    let is_lower = p < dec!(0.5);
    let pp = if is_lower { p } else { Decimal::ONE - p };

    // Initial estimate: Abramowitz & Stegun 26.2.23
    let ln_pp = decimal_ln_positive(pp);
    let t = decimal_sqrt(dec!(-2) * ln_pp);

    let c0 = dec!(2.515517);
    let c1 = dec!(0.802853);
    let c2 = dec!(0.010328);
    let d1 = dec!(1.432788);
    let d2 = dec!(0.189269);
    let d3 = dec!(0.001308);

    let numerator = c0 + c1 * t + c2 * t * t;
    let denominator = Decimal::ONE + d1 * t + d2 * t * t + d3 * t * t * t;

    let mut result = t - numerator / denominator;
    if is_lower {
        result = -result;
    }

    // Newton refinement: x_{n+1} = x_n - (N(x_n) - p) / phi(x_n)
    let sqrt_2pi = dec!(2.506628274631);
    for _ in 0..3 {
        let cdf_val = norm_cdf(result);
        let pdf_val = decimal_exp(-(result * result) / dec!(2)) / sqrt_2pi;
        if pdf_val.is_zero() {
            break;
        }
        result -= (cdf_val - p) / pdf_val;
    }

    result
}

/// Natural log for positive values only (used in norm_inv).
fn decimal_ln_positive(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return dec!(-23); // floor
    }
    let ln2 = dec!(0.6931471805599453);
    let mut val = x;
    let mut adjust = Decimal::ZERO;
    while val > dec!(2.0) {
        val /= dec!(2);
        adjust += ln2;
    }
    while val < dec!(0.5) {
        val *= dec!(2);
        adjust -= ln2;
    }
    let z = (val - Decimal::ONE) / (val + Decimal::ONE);
    let z2 = z * z;
    let mut term = z;
    let mut sum = z;
    for k in 1u32..40 {
        term *= z2;
        let denom = Decimal::from(2 * k + 1);
        sum += term / denom;
    }
    dec!(2) * sum + adjust
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Direction of calibration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CalibrationDirection {
    /// Convert TTC PD to PIT PD.
    TtcToPit,
    /// Convert PIT PD to TTC PD.
    PitToTtc,
}

/// Input for PIT/TTC calibration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationInput {
    /// The PD to convert (either TTC or PIT, depending on direction).
    pub pd_input: Decimal,
    /// Optional asset correlation override. If None, Basel IRB formula is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_override: Option<Decimal>,
    /// Macro index (standardized, z-score). Positive = expansion, negative = contraction.
    pub macro_index: Decimal,
    /// Direction of calibration.
    pub direction: CalibrationDirection,
    /// Optional long-run average PD for central tendency calculation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_run_pd: Option<Decimal>,
}

/// Output of the calibration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalibrationOutput {
    /// Adjusted PD (PIT if direction=TtcToPit, TTC if direction=PitToTtc).
    pub pd_adjusted: Decimal,
    /// Asset correlation used.
    pub asset_correlation: Decimal,
    /// Central tendency ratio (long_run_pd / pd_input), if long_run_pd provided.
    pub central_tendency: Option<Decimal>,
    /// Calibration factor: pd_adjusted / pd_input.
    pub calibration_factor: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calibrate PD between PIT and TTC using Vasicek single-factor model.
pub fn calculate_calibration(input: &CalibrationInput) -> CorpFinanceResult<CalibrationOutput> {
    validate_calibration_input(input)?;

    let pd = input.pd_input;
    let z = input.macro_index;

    // Asset correlation
    let rho = match input.correlation_override {
        Some(r) => r,
        None => basel_irb_correlation(pd),
    };

    let sqrt_rho = decimal_sqrt(rho);
    let sqrt_one_minus_rho = decimal_sqrt(Decimal::ONE - rho);

    let pd_adjusted = match input.direction {
        CalibrationDirection::TtcToPit => {
            // PD_pit = N((N_inv(PD_ttc) - sqrt(rho)*z) / sqrt(1-rho))
            let n_inv_pd = norm_inv(pd);
            let numerator = n_inv_pd - sqrt_rho * z;
            let arg = if sqrt_one_minus_rho.is_zero() {
                Decimal::ZERO
            } else {
                numerator / sqrt_one_minus_rho
            };
            norm_cdf(arg)
        }
        CalibrationDirection::PitToTtc => {
            // PD_ttc: invert the Vasicek formula
            // N_inv(PD_pit) = (N_inv(PD_ttc) - sqrt(rho)*z) / sqrt(1-rho)
            // => N_inv(PD_ttc) = N_inv(PD_pit)*sqrt(1-rho) + sqrt(rho)*z
            let n_inv_pd = norm_inv(pd);
            let n_inv_ttc = n_inv_pd * sqrt_one_minus_rho + sqrt_rho * z;
            norm_cdf(n_inv_ttc)
        }
    };

    // Central tendency
    let central_tendency =
        input
            .long_run_pd
            .map(|lr| if pd.is_zero() { Decimal::ZERO } else { lr / pd });

    // Calibration factor
    let calibration_factor = if pd.is_zero() {
        Decimal::ZERO
    } else {
        pd_adjusted / pd
    };

    Ok(CalibrationOutput {
        pd_adjusted,
        asset_correlation: rho,
        central_tendency,
        calibration_factor,
    })
}

/// Basel IRB asset correlation formula.
/// rho = 0.12 * (1-exp(-50*PD))/(1-exp(-50)) + 0.24 * (1-(1-exp(-50*PD))/(1-exp(-50)))
fn basel_irb_correlation(pd: Decimal) -> Decimal {
    let exp_neg50 = decimal_exp(dec!(-50));
    let exp_neg50_pd = decimal_exp(dec!(-50) * pd);

    let denom = Decimal::ONE - exp_neg50;
    if denom.is_zero() {
        return dec!(0.12);
    }

    let factor = (Decimal::ONE - exp_neg50_pd) / denom;

    dec!(0.12) * factor + dec!(0.24) * (Decimal::ONE - factor)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_calibration_input(input: &CalibrationInput) -> CorpFinanceResult<()> {
    if input.pd_input < Decimal::ZERO || input.pd_input > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "pd_input".into(),
            reason: "PD must be in [0, 1].".into(),
        });
    }
    if let Some(rho) = input.correlation_override {
        if rho < Decimal::ZERO || rho >= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "correlation_override".into(),
                reason: "Asset correlation must be in [0, 1).".into(),
            });
        }
    }
    if input.macro_index < dec!(-10) || input.macro_index > dec!(10) {
        return Err(CorpFinanceError::InvalidInput {
            field: "macro_index".into(),
            reason: "Macro index (z-score) should be in [-10, 10].".into(),
        });
    }
    if let Some(lr) = input.long_run_pd {
        if lr < Decimal::ZERO || lr > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "long_run_pd".into(),
                reason: "Long-run PD must be in [0, 1].".into(),
            });
        }
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

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn base_input() -> CalibrationInput {
        CalibrationInput {
            pd_input: dec!(0.02),
            correlation_override: None,
            macro_index: Decimal::ZERO,
            direction: CalibrationDirection::TtcToPit,
            long_run_pd: None,
        }
    }

    #[test]
    fn test_zero_macro_index_zero_rho_no_change() {
        // When z=0 and rho=0, PIT should equal TTC exactly:
        // PD_pit = N(N_inv(PD_ttc) / sqrt(1)) = PD_ttc
        let input = CalibrationInput {
            pd_input: dec!(0.02),
            correlation_override: Some(Decimal::ZERO),
            macro_index: Decimal::ZERO,
            direction: CalibrationDirection::TtcToPit,
            long_run_pd: None,
        };
        let out = calculate_calibration(&input).unwrap();
        assert!(
            approx_eq(out.pd_adjusted, dec!(0.02), dec!(0.001)),
            "With z=0, rho=0, PD_pit {} should ≈ PD_ttc 0.02",
            out.pd_adjusted
        );
    }

    #[test]
    fn test_negative_macro_increases_pit_pd() {
        // In downturn (z < 0), PIT PD should exceed TTC PD
        let input = CalibrationInput {
            macro_index: dec!(-2.0),
            ..base_input()
        };
        let out = calculate_calibration(&input).unwrap();
        assert!(
            out.pd_adjusted > dec!(0.02),
            "In downturn, PIT PD {} should exceed TTC PD 0.02",
            out.pd_adjusted
        );
    }

    #[test]
    fn test_positive_macro_decreases_pit_pd() {
        // In expansion (z > 0), PIT PD should be below TTC PD
        let input = CalibrationInput {
            macro_index: dec!(2.0),
            ..base_input()
        };
        let out = calculate_calibration(&input).unwrap();
        assert!(
            out.pd_adjusted < dec!(0.02),
            "In expansion, PIT PD {} should be below TTC PD 0.02",
            out.pd_adjusted
        );
    }

    #[test]
    fn test_pit_to_ttc_reverses_ttc_to_pit() {
        let ttc_to_pit = CalibrationInput {
            pd_input: dec!(0.03),
            correlation_override: Some(dec!(0.15)),
            macro_index: dec!(-1.5),
            direction: CalibrationDirection::TtcToPit,
            long_run_pd: None,
        };
        let pit_result = calculate_calibration(&ttc_to_pit).unwrap();

        let pit_to_ttc = CalibrationInput {
            pd_input: pit_result.pd_adjusted,
            correlation_override: Some(dec!(0.15)),
            macro_index: dec!(-1.5),
            direction: CalibrationDirection::PitToTtc,
            long_run_pd: None,
        };
        let ttc_result = calculate_calibration(&pit_to_ttc).unwrap();
        assert!(
            approx_eq(ttc_result.pd_adjusted, dec!(0.03), dec!(0.002)),
            "Round-trip: TTC->PIT->TTC should recover original. Got {}",
            ttc_result.pd_adjusted
        );
    }

    #[test]
    fn test_correlation_override_used() {
        let input = CalibrationInput {
            correlation_override: Some(dec!(0.20)),
            ..base_input()
        };
        let out = calculate_calibration(&input).unwrap();
        assert_eq!(out.asset_correlation, dec!(0.20));
    }

    #[test]
    fn test_basel_correlation_high_pd() {
        // High PD => rho closer to 0.12
        let rho = basel_irb_correlation(dec!(0.20));
        assert!(
            rho < dec!(0.15),
            "High PD rho {} should be closer to 0.12",
            rho
        );
        assert!(rho > dec!(0.11));
    }

    #[test]
    fn test_basel_correlation_low_pd() {
        // Low PD => rho closer to 0.24
        let rho = basel_irb_correlation(dec!(0.001));
        assert!(
            rho > dec!(0.20),
            "Low PD rho {} should be closer to 0.24",
            rho
        );
    }

    #[test]
    fn test_basel_correlation_between_bounds() {
        for pd_bps in &[1u32, 10, 50, 100, 500, 1000, 2000] {
            let pd = Decimal::from(*pd_bps) / dec!(10000);
            let rho = basel_irb_correlation(pd);
            assert!(
                rho >= dec!(0.11) && rho <= dec!(0.25),
                "Correlation {} for PD {} should be in [0.12, 0.24]",
                rho,
                pd
            );
        }
    }

    #[test]
    fn test_central_tendency_calculated() {
        let input = CalibrationInput {
            long_run_pd: Some(dec!(0.03)),
            ..base_input()
        };
        let out = calculate_calibration(&input).unwrap();
        // central_tendency = 0.03 / 0.02 = 1.50
        assert!(out.central_tendency.is_some());
        assert!(approx_eq(
            out.central_tendency.unwrap(),
            dec!(1.5),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_central_tendency_none_without_long_run() {
        let input = base_input();
        let out = calculate_calibration(&input).unwrap();
        assert!(out.central_tendency.is_none());
    }

    #[test]
    fn test_calibration_factor_one_at_zero_z_zero_rho() {
        // With rho=0, the single-factor model collapses: PIT = TTC regardless of z
        let input = CalibrationInput {
            pd_input: dec!(0.02),
            correlation_override: Some(Decimal::ZERO),
            macro_index: Decimal::ZERO,
            direction: CalibrationDirection::TtcToPit,
            long_run_pd: None,
        };
        let out = calculate_calibration(&input).unwrap();
        assert!(
            approx_eq(out.calibration_factor, Decimal::ONE, dec!(0.05)),
            "Calibration factor {} should ≈ 1.0 with z=0, rho=0",
            out.calibration_factor
        );
    }

    #[test]
    fn test_calibration_factor_gt_one_downturn() {
        let input = CalibrationInput {
            macro_index: dec!(-2.0),
            ..base_input()
        };
        let out = calculate_calibration(&input).unwrap();
        assert!(out.calibration_factor > Decimal::ONE);
    }

    #[test]
    fn test_pd_adjusted_in_range() {
        let input = CalibrationInput {
            macro_index: dec!(-3.0),
            ..base_input()
        };
        let out = calculate_calibration(&input).unwrap();
        assert!(out.pd_adjusted >= Decimal::ZERO && out.pd_adjusted <= Decimal::ONE);
    }

    #[test]
    fn test_zero_pd_input() {
        let input = CalibrationInput {
            pd_input: Decimal::ZERO,
            ..base_input()
        };
        let out = calculate_calibration(&input).unwrap();
        assert!(out.pd_adjusted >= Decimal::ZERO);
    }

    #[test]
    fn test_reject_pd_out_of_range() {
        let input = CalibrationInput {
            pd_input: dec!(1.5),
            ..base_input()
        };
        assert!(calculate_calibration(&input).is_err());
    }

    #[test]
    fn test_reject_negative_pd() {
        let input = CalibrationInput {
            pd_input: dec!(-0.1),
            ..base_input()
        };
        assert!(calculate_calibration(&input).is_err());
    }

    #[test]
    fn test_reject_correlation_ge_one() {
        let input = CalibrationInput {
            correlation_override: Some(dec!(1.0)),
            ..base_input()
        };
        assert!(calculate_calibration(&input).is_err());
    }

    #[test]
    fn test_reject_macro_index_out_of_range() {
        let input = CalibrationInput {
            macro_index: dec!(15),
            ..base_input()
        };
        assert!(calculate_calibration(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_long_run_pd() {
        let input = CalibrationInput {
            long_run_pd: Some(dec!(2.0)),
            ..base_input()
        };
        assert!(calculate_calibration(&input).is_err());
    }

    #[test]
    fn test_norm_inv_of_half() {
        let val = norm_inv(dec!(0.5));
        assert!(approx_eq(val, Decimal::ZERO, dec!(0.01)));
    }

    #[test]
    fn test_norm_cdf_norm_inv_roundtrip() {
        let p = dec!(0.95);
        let z = norm_inv(p);
        let p2 = norm_cdf(z);
        assert!(approx_eq(p, p2, dec!(0.005)));
    }

    #[test]
    fn test_higher_correlation_amplifies_cycle() {
        let low_rho = CalibrationInput {
            correlation_override: Some(dec!(0.10)),
            macro_index: dec!(-2.0),
            ..base_input()
        };
        let high_rho = CalibrationInput {
            correlation_override: Some(dec!(0.30)),
            macro_index: dec!(-2.0),
            ..base_input()
        };
        let out_low = calculate_calibration(&low_rho).unwrap();
        let out_high = calculate_calibration(&high_rho).unwrap();
        assert!(
            out_high.pd_adjusted > out_low.pd_adjusted,
            "Higher rho {} should amplify downturn more than lower rho {}",
            out_high.pd_adjusted,
            out_low.pd_adjusted
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let out = calculate_calibration(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: CalibrationOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_pit_to_ttc_expansion_lowers_ttc() {
        // In expansion, PIT PD is lower than TTC. Converting PIT back to TTC
        // should give a higher number
        let input = CalibrationInput {
            pd_input: dec!(0.01), // low PIT PD observed during expansion
            correlation_override: Some(dec!(0.15)),
            macro_index: dec!(2.0), // expansion
            direction: CalibrationDirection::PitToTtc,
            long_run_pd: None,
        };
        let out = calculate_calibration(&input).unwrap();
        assert!(
            out.pd_adjusted > dec!(0.01),
            "TTC PD {} should exceed observed PIT PD in expansion",
            out.pd_adjusted
        );
    }
}
