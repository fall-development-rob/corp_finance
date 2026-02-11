use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorExposure {
    pub factor_name: String,
    pub portfolio_exposure: Decimal,
    pub benchmark_exposure: Decimal,
    pub factor_return: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorAttributionInput {
    pub portfolio_name: String,
    pub portfolio_return: Decimal,
    pub benchmark_return: Decimal,
    pub factors: Vec<FactorExposure>,
    pub risk_free_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorContribution {
    pub factor_name: String,
    pub active_exposure: Decimal,
    pub factor_return: Decimal,
    pub return_contribution: Decimal,
    pub pct_of_active_return: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorAttributionOutput {
    pub portfolio_return: Decimal,
    pub benchmark_return: Decimal,
    pub active_return: Decimal,
    pub factor_contributions: Vec<FactorContribution>,
    pub total_factor_contribution: Decimal,
    pub residual_return: Decimal,
    pub residual_pct: Decimal,
    pub r_squared: Decimal,
    pub tracking_error_decomposition: TrackingErrorDecomp,
    pub methodology: String,
    pub assumptions: HashMap<String, String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingErrorDecomp {
    pub factor_tracking_error: Decimal,
    pub residual_tracking_error: Decimal,
    pub total_tracking_error: Decimal,
    pub factor_pct: Decimal,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Square root via Newton's method (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / Decimal::from(2);
    if guess == Decimal::ZERO {
        guess = Decimal::ONE;
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / Decimal::from(2);
    }
    guess
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Perform factor-based return attribution.
///
/// Decomposes active return into factor contributions and a residual (alpha).
/// Includes R-squared and a simplified tracking error decomposition.
pub fn factor_attribution(
    input: &FactorAttributionInput,
) -> CorpFinanceResult<FactorAttributionOutput> {
    let mut warnings = Vec::new();

    // Validate
    if input.factors.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "factors".into(),
            reason: "At least one factor is required".into(),
        });
    }

    let active_return = input.portfolio_return - input.benchmark_return;

    // Compute factor contributions
    let mut factor_contributions = Vec::with_capacity(input.factors.len());
    let mut total_factor_contribution = Decimal::ZERO;

    for f in &input.factors {
        let active_exposure = f.portfolio_exposure - f.benchmark_exposure;
        let return_contribution = active_exposure * f.factor_return;
        total_factor_contribution += return_contribution;

        let pct_of_active_return = if active_return != Decimal::ZERO {
            (return_contribution / active_return) * dec!(100)
        } else {
            Decimal::ZERO
        };

        factor_contributions.push(FactorContribution {
            factor_name: f.factor_name.clone(),
            active_exposure,
            factor_return: f.factor_return,
            return_contribution,
            pct_of_active_return,
        });
    }

    // Residual (alpha)
    let residual_return = active_return - total_factor_contribution;
    let residual_pct = if active_return != Decimal::ZERO {
        (residual_return / active_return) * dec!(100)
    } else {
        Decimal::ZERO
    };

    // R-squared: 1 - (residual^2 / active_return^2)
    let r_squared = if active_return != Decimal::ZERO {
        let ratio = (residual_return * residual_return) / (active_return * active_return);
        let r2 = Decimal::ONE - ratio;
        // Clamp to [0, 1]
        if r2 < Decimal::ZERO {
            Decimal::ZERO
        } else if r2 > Decimal::ONE {
            Decimal::ONE
        } else {
            r2
        }
    } else {
        Decimal::ZERO
    };

    // Tracking error decomposition (simplified single-period)
    // Factor TE = sqrt(sum of (active_exposure_i * factor_return_i)^2)
    let factor_te_sq: Decimal = input
        .factors
        .iter()
        .map(|f| {
            let ae = f.portfolio_exposure - f.benchmark_exposure;
            let contrib = ae * f.factor_return;
            contrib * contrib
        })
        .sum();
    let factor_te = decimal_sqrt(factor_te_sq);

    let residual_te = residual_return.abs();
    let total_te = decimal_sqrt(factor_te * factor_te + residual_te * residual_te);

    let factor_pct = if total_te > Decimal::ZERO {
        (factor_te / total_te) * dec!(100)
    } else {
        Decimal::ZERO
    };

    let te_decomp = TrackingErrorDecomp {
        factor_tracking_error: factor_te,
        residual_tracking_error: residual_te,
        total_tracking_error: total_te,
        factor_pct,
    };

    // Warnings
    if active_return != Decimal::ZERO {
        let residual_ratio = (residual_return / active_return).abs();
        if residual_ratio > dec!(0.50) {
            warnings.push(format!(
                "Large residual: {:.2}% of active return. Factor model may be incomplete.",
                residual_pct.abs()
            ));
        }
    }
    if r_squared < dec!(0.5) && active_return != Decimal::ZERO {
        warnings.push(format!(
            "Low R-squared ({:.4}). Factor model explains less than 50% of active return variance.",
            r_squared
        ));
    }

    let mut assumptions = HashMap::new();
    assumptions.insert("model".into(), "Factor-based return attribution".into());
    assumptions.insert(
        "tracking_error".into(),
        "Simplified single-period decomposition".into(),
    );
    assumptions.insert(
        "r_squared".into(),
        "Cross-sectional single-period proxy".into(),
    );

    Ok(FactorAttributionOutput {
        portfolio_return: input.portfolio_return,
        benchmark_return: input.benchmark_return,
        active_return,
        factor_contributions,
        total_factor_contribution,
        residual_return,
        residual_pct,
        r_squared,
        tracking_error_decomposition: te_decomp,
        methodology: "Factor-based return attribution with tracking error decomposition".into(),
        assumptions,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_factor(name: &str, pe: Decimal, be: Decimal, fr: Decimal) -> FactorExposure {
        FactorExposure {
            factor_name: name.into(),
            portfolio_exposure: pe,
            benchmark_exposure: be,
            factor_return: fr,
        }
    }

    fn basic_3_factor_input() -> FactorAttributionInput {
        FactorAttributionInput {
            portfolio_name: "Test Portfolio".into(),
            portfolio_return: dec!(0.12),
            benchmark_return: dec!(0.08),
            factors: vec![
                make_factor("Market", dec!(1.10), dec!(1.00), dec!(0.06)),
                make_factor("Size", dec!(0.30), dec!(0.00), dec!(0.03)),
                make_factor("Value", dec!(-0.10), dec!(0.00), dec!(0.02)),
            ],
            risk_free_rate: dec!(0.02),
        }
    }

    // ---- Basic 3-factor decomposition ----

    #[test]
    fn test_basic_3_factor_active_return() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        assert_eq!(out.active_return, dec!(0.04));
    }

    #[test]
    fn test_basic_3_factor_contributions() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        // Market: (1.10-1.00)*0.06 = 0.006
        assert_eq!(out.factor_contributions[0].return_contribution, dec!(0.006));
        // Size: (0.30-0.00)*0.03 = 0.009
        assert_eq!(out.factor_contributions[1].return_contribution, dec!(0.009));
        // Value: (-0.10-0.00)*0.02 = -0.002
        assert_eq!(
            out.factor_contributions[2].return_contribution,
            dec!(-0.002)
        );
    }

    #[test]
    fn test_basic_3_factor_total_contribution() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        // 0.006 + 0.009 + (-0.002) = 0.013
        assert_eq!(out.total_factor_contribution, dec!(0.013));
    }

    #[test]
    fn test_basic_3_factor_residual() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        // residual = 0.04 - 0.013 = 0.027
        assert_eq!(out.residual_return, dec!(0.027));
    }

    #[test]
    fn test_factor_contributions_sum() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        let sum: Decimal = out
            .factor_contributions
            .iter()
            .map(|fc| fc.return_contribution)
            .sum();
        assert_eq!(sum, out.total_factor_contribution);
    }

    #[test]
    fn test_residual_plus_factors_equals_active() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        assert_eq!(
            out.total_factor_contribution + out.residual_return,
            out.active_return
        );
    }

    // ---- Single factor (CAPM-like) ----

    #[test]
    fn test_single_factor_capm() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.10),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Market", dec!(1.20), dec!(1.00), dec!(0.08))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // active = 0.02, market contrib = 0.20*0.08 = 0.016
        assert_eq!(out.factor_contributions[0].return_contribution, dec!(0.016));
        assert_eq!(out.residual_return, dec!(0.004));
    }

    // ---- Zero active return ----

    #[test]
    fn test_zero_active_return() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.08),
            benchmark_return: dec!(0.08),
            factors: vec![
                make_factor("Market", dec!(1.00), dec!(1.00), dec!(0.06)),
                make_factor("Size", dec!(0.00), dec!(0.00), dec!(0.03)),
            ],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        assert_eq!(out.active_return, Decimal::ZERO);
        assert_eq!(out.total_factor_contribution, Decimal::ZERO);
        assert_eq!(out.residual_return, Decimal::ZERO);
    }

    // ---- Large residual warning ----

    #[test]
    fn test_large_residual_warning() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.15),
            benchmark_return: dec!(0.05),
            factors: vec![make_factor("Market", dec!(1.05), dec!(1.00), dec!(0.02))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // active = 0.10, factor contrib = 0.05*0.02 = 0.001, residual = 0.099
        assert!(out.warnings.iter().any(|w| w.contains("Large residual")));
    }

    // ---- Low R-squared warning ----

    #[test]
    fn test_low_r_squared_warning() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.20),
            benchmark_return: dec!(0.05),
            factors: vec![make_factor("Market", dec!(1.02), dec!(1.00), dec!(0.01))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // active = 0.15, factor = 0.02*0.01 = 0.0002, residual = ~0.1498
        assert!(out.r_squared < dec!(0.5));
        assert!(out.warnings.iter().any(|w| w.contains("R-squared")));
    }

    // ---- Negative factor returns ----

    #[test]
    fn test_negative_factor_returns() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.05),
            benchmark_return: dec!(0.08),
            factors: vec![
                make_factor("Market", dec!(1.10), dec!(1.00), dec!(-0.04)),
                make_factor("Quality", dec!(0.20), dec!(0.10), dec!(0.03)),
            ],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // Market contrib: 0.10 * (-0.04) = -0.004 (negative)
        assert!(out.factor_contributions[0].return_contribution < Decimal::ZERO);
        assert_eq!(
            out.total_factor_contribution + out.residual_return,
            out.active_return
        );
    }

    // ---- Factor contribution signs ----

    #[test]
    fn test_positive_active_exposure_positive_return_positive_contribution() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.12),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Momentum", dec!(0.50), dec!(0.20), dec!(0.05))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        assert!(out.factor_contributions[0].return_contribution > Decimal::ZERO);
    }

    #[test]
    fn test_negative_active_exposure_positive_return_negative_contribution() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.06),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Momentum", dec!(0.10), dec!(0.30), dec!(0.05))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // active_exposure = -0.20, contrib = -0.20*0.05 = -0.01
        assert!(out.factor_contributions[0].return_contribution < Decimal::ZERO);
    }

    // ---- R-squared computation ----

    #[test]
    fn test_high_r_squared() {
        // Factor explains almost all of active return
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.12),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Market", dec!(1.50), dec!(1.00), dec!(0.08))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // active = 0.04, factor = 0.50*0.08 = 0.04, residual = 0
        assert_eq!(out.r_squared, Decimal::ONE);
    }

    #[test]
    fn test_r_squared_zero_when_no_active_return() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.08),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Market", dec!(1.00), dec!(1.00), dec!(0.05))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        assert_eq!(out.r_squared, Decimal::ZERO);
    }

    #[test]
    fn test_r_squared_clamped_non_negative() {
        // residual > active return => r_squared would be negative, clamped to 0
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.10),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Market", dec!(0.80), dec!(1.00), dec!(0.10))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // active = 0.02, factor = -0.20*0.10 = -0.02, residual = 0.04
        // r2 = 1 - (0.04^2/0.02^2) = 1 - 4 = -3 => clamped to 0
        assert_eq!(out.r_squared, Decimal::ZERO);
    }

    // ---- Tracking error decomposition ----

    #[test]
    fn test_tracking_error_decomp_structure() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        let te = &out.tracking_error_decomposition;
        // total TE = sqrt(factor_te^2 + residual_te^2)
        let expected_total = decimal_sqrt(
            te.factor_tracking_error * te.factor_tracking_error
                + te.residual_tracking_error * te.residual_tracking_error,
        );
        let diff = (te.total_tracking_error - expected_total).abs();
        assert!(diff < dec!(0.0000001));
    }

    #[test]
    fn test_tracking_error_factor_pct() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        let te = &out.tracking_error_decomposition;
        if te.total_tracking_error > Decimal::ZERO {
            let expected_pct = (te.factor_tracking_error / te.total_tracking_error) * dec!(100);
            let diff = (te.factor_pct - expected_pct).abs();
            assert!(diff < dec!(0.0000001));
        }
    }

    #[test]
    fn test_tracking_error_residual_is_abs_residual() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        assert_eq!(
            out.tracking_error_decomposition.residual_tracking_error,
            out.residual_return.abs()
        );
    }

    // ---- Factor with zero return ----

    #[test]
    fn test_factor_with_zero_return() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.10),
            benchmark_return: dec!(0.08),
            factors: vec![
                make_factor("Market", dec!(1.10), dec!(1.00), dec!(0.05)),
                make_factor("Dead", dec!(0.50), dec!(0.20), dec!(0.00)),
            ],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // Dead factor contribution = 0.30 * 0.00 = 0
        assert_eq!(
            out.factor_contributions[1].return_contribution,
            Decimal::ZERO
        );
    }

    // ---- Identical exposures ----

    #[test]
    fn test_identical_exposures_zero_contribution() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.10),
            benchmark_return: dec!(0.08),
            factors: vec![
                make_factor("Market", dec!(1.00), dec!(1.00), dec!(0.06)),
                make_factor("Size", dec!(0.20), dec!(0.20), dec!(0.03)),
            ],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        assert_eq!(out.total_factor_contribution, Decimal::ZERO);
        assert_eq!(out.residual_return, out.active_return);
    }

    // ---- Many factors (8+) ----

    #[test]
    fn test_many_factors() {
        let factors: Vec<FactorExposure> = (0..8)
            .map(|i| {
                make_factor(
                    &format!("F{}", i),
                    Decimal::from(i + 1) / dec!(10),
                    Decimal::from(i) / dec!(10),
                    dec!(0.01),
                )
            })
            .collect();
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.15),
            benchmark_return: dec!(0.08),
            factors,
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        assert_eq!(out.factor_contributions.len(), 8);
        let sum: Decimal = out
            .factor_contributions
            .iter()
            .map(|fc| fc.return_contribution)
            .sum();
        assert_eq!(sum, out.total_factor_contribution);
    }

    // ---- Pct of active return sums ----

    #[test]
    fn test_pct_of_active_return_sum() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        let pct_sum: Decimal = out
            .factor_contributions
            .iter()
            .map(|fc| fc.pct_of_active_return)
            .sum();
        // pct_sum + residual_pct should = 100
        let total = pct_sum + out.residual_pct;
        let diff = (total - dec!(100)).abs();
        assert!(diff < dec!(0.001));
    }

    // ---- Residual interpretation ----

    #[test]
    fn test_residual_positive_alpha() {
        // Portfolio outperforms beyond what factors explain
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.15),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Market", dec!(1.10), dec!(1.00), dec!(0.05))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // active = 0.07, factor = 0.10*0.05 = 0.005, residual = 0.065
        assert!(out.residual_return > Decimal::ZERO);
    }

    #[test]
    fn test_residual_negative_alpha() {
        // Portfolio underperforms relative to factor exposure
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.09),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Market", dec!(1.30), dec!(1.00), dec!(0.06))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // active = 0.01, factor = 0.30*0.06 = 0.018, residual = 0.01-0.018 = -0.008
        assert!(out.residual_return < Decimal::ZERO);
    }

    // ---- Validation ----

    #[test]
    fn test_empty_factors_error() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.10),
            benchmark_return: dec!(0.08),
            factors: vec![],
            risk_free_rate: dec!(0.02),
        };
        assert!(factor_attribution(&input).is_err());
    }

    // ---- Methodology and assumptions ----

    #[test]
    fn test_methodology_string() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        assert!(out.methodology.contains("Factor-based"));
    }

    #[test]
    fn test_assumptions_present() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        assert!(out.assumptions.contains_key("model"));
        assert!(out.assumptions.contains_key("tracking_error"));
    }

    // ---- No warnings when model fits well ----

    #[test]
    fn test_no_warnings_good_fit() {
        let input = FactorAttributionInput {
            portfolio_name: "P".into(),
            portfolio_return: dec!(0.12),
            benchmark_return: dec!(0.08),
            factors: vec![make_factor("Market", dec!(1.50), dec!(1.00), dec!(0.08))],
            risk_free_rate: dec!(0.02),
        };
        let out = factor_attribution(&input).unwrap();
        // factor = 0.50*0.08 = 0.04 = active return, residual = 0
        assert!(out.warnings.is_empty());
    }

    // ---- Active exposure computation ----

    #[test]
    fn test_active_exposure_values() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        assert_eq!(out.factor_contributions[0].active_exposure, dec!(0.10));
        assert_eq!(out.factor_contributions[1].active_exposure, dec!(0.30));
        assert_eq!(out.factor_contributions[2].active_exposure, dec!(-0.10));
    }

    // ---- Factor return stored correctly ----

    #[test]
    fn test_factor_return_in_output() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        assert_eq!(out.factor_contributions[0].factor_return, dec!(0.06));
        assert_eq!(out.factor_contributions[1].factor_return, dec!(0.03));
        assert_eq!(out.factor_contributions[2].factor_return, dec!(0.02));
    }

    // ---- Portfolio/benchmark return pass-through ----

    #[test]
    fn test_returns_pass_through() {
        let out = factor_attribution(&basic_3_factor_input()).unwrap();
        assert_eq!(out.portfolio_return, dec!(0.12));
        assert_eq!(out.benchmark_return, dec!(0.08));
    }

    // ---- Sqrt helper ----

    #[test]
    fn test_decimal_sqrt_basic() {
        let result = decimal_sqrt(Decimal::from(9));
        let diff = (result - Decimal::from(3)).abs();
        assert!(diff < dec!(0.0000001));
    }

    #[test]
    fn test_decimal_sqrt_zero() {
        assert_eq!(decimal_sqrt(Decimal::ZERO), Decimal::ZERO);
    }
}
