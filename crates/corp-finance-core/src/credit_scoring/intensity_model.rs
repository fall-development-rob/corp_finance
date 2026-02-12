//! Reduced-form intensity (hazard rate) model for credit risk.
//!
//! Covers:
//! 1. **Hazard Rate from CDS** -- λ = spread / (1 - recovery) for flat hazard.
//! 2. **Survival Probability** -- S(t) = exp(-λ*t) for flat; product for piecewise.
//! 3. **Term Structure** -- bootstrap piecewise hazard rates from CDS spreads.
//! 4. **Conditional Default Probability** -- P(default in [t1,t2]) = S(t1) - S(t2).
//! 5. **Expected Loss** -- EL = (1-R) * PD * exposure.
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

/// Natural logarithm via Taylor series.
#[allow(dead_code)]
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
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

/// A CDS spread observation at a given tenor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdsTenorSpread {
    /// Tenor in years (e.g. 1.0, 3.0, 5.0, 7.0, 10.0).
    pub tenor: Decimal,
    /// CDS spread in decimal (e.g. 0.01 = 100bps).
    pub spread: Decimal,
}

/// Input for intensity model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntensityModelInput {
    /// CDS spreads at various tenors.
    pub cds_spreads: Vec<CdsTenorSpread>,
    /// Recovery rate (decimal, e.g. 0.40 = 40%).
    pub recovery_rate: Decimal,
    /// Risk-free rate (annualized, decimal).
    pub risk_free_rate: Decimal,
    /// Exposure at default (for expected loss calculation).
    #[serde(default)]
    pub exposure: Decimal,
}

/// Per-tenor result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenorResult {
    /// Tenor in years.
    pub tenor: Decimal,
    /// Piecewise hazard rate for this interval.
    pub hazard_rate: Decimal,
    /// Survival probability to this tenor.
    pub survival_prob: Decimal,
    /// Cumulative probability of default to this tenor.
    pub cumulative_pd: Decimal,
    /// Conditional PD for this interval (from previous tenor to this tenor).
    pub conditional_pd: Decimal,
    /// Expected loss for this interval.
    pub expected_loss: Decimal,
}

/// Output of the intensity model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntensityModelOutput {
    /// Per-tenor results.
    pub tenor_results: Vec<TenorResult>,
    /// Flat hazard rate (from the shortest tenor CDS).
    pub flat_hazard_rate: Decimal,
    /// Total expected loss across all intervals.
    pub total_expected_loss: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate intensity model: hazard rates, survival probabilities, PDs, ELs.
pub fn calculate_intensity_model(
    input: &IntensityModelInput,
) -> CorpFinanceResult<IntensityModelOutput> {
    validate_intensity_input(input)?;

    let lgd = Decimal::ONE - input.recovery_rate;

    // Sort spreads by tenor
    let mut spreads = input.cds_spreads.clone();
    spreads.sort_by(|a, b| a.tenor.partial_cmp(&b.tenor).unwrap());

    // Flat hazard rate from the first (shortest) tenor
    let flat_hazard_rate = if lgd.is_zero() {
        Decimal::ZERO
    } else {
        spreads[0].spread / lgd
    };

    // Bootstrap piecewise hazard rates
    let mut tenor_results: Vec<TenorResult> = Vec::with_capacity(spreads.len());
    let mut prev_tenor = Decimal::ZERO;
    let mut cum_hazard = Decimal::ZERO;
    let mut total_el = Decimal::ZERO;

    for (i, cs) in spreads.iter().enumerate() {
        let dt = cs.tenor - prev_tenor;

        // Bootstrap: for the i-th tenor, the flat-equivalent hazard rate λ_flat_i = spread_i / LGD.
        // The piecewise hazard for interval [t_{i-1}, t_i] satisfies:
        // λ_flat_i * t_i = cum_hazard + λ_i * dt
        // => λ_i = (λ_flat_i * t_i - cum_hazard) / dt
        let lambda_flat = if lgd.is_zero() {
            Decimal::ZERO
        } else {
            cs.spread / lgd
        };

        let lambda_i = if dt.is_zero() {
            lambda_flat
        } else {
            let target_cum = lambda_flat * cs.tenor;
            let lambda = (target_cum - cum_hazard) / dt;
            if lambda < Decimal::ZERO {
                // Floor at zero for inverted curves
                Decimal::ZERO
            } else {
                lambda
            }
        };

        cum_hazard += lambda_i * dt;

        // Survival probability: S(t) = exp(-cumulative hazard)
        let survival_prob = decimal_exp(-cum_hazard);
        let cumulative_pd = Decimal::ONE - survival_prob;

        // Conditional PD for this interval: S(t_{i-1}) - S(t_i)
        let prev_survival = if i == 0 {
            Decimal::ONE
        } else {
            tenor_results[i - 1].survival_prob
        };
        let conditional_pd = prev_survival - survival_prob;

        // Expected loss for this interval
        let expected_loss = lgd * conditional_pd * input.exposure;
        total_el += expected_loss;

        prev_tenor = cs.tenor;

        tenor_results.push(TenorResult {
            tenor: cs.tenor,
            hazard_rate: lambda_i,
            survival_prob,
            cumulative_pd,
            conditional_pd,
            expected_loss,
        });
    }

    Ok(IntensityModelOutput {
        tenor_results,
        flat_hazard_rate,
        total_expected_loss: total_el,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_intensity_input(input: &IntensityModelInput) -> CorpFinanceResult<()> {
    if input.cds_spreads.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one CDS spread is required.".into(),
        ));
    }
    if input.recovery_rate < Decimal::ZERO || input.recovery_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "recovery_rate".into(),
            reason: "Recovery rate must be in [0, 1).".into(),
        });
    }
    for cs in &input.cds_spreads {
        if cs.tenor <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "tenor".into(),
                reason: "Tenor must be positive.".into(),
            });
        }
        if cs.spread < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "spread".into(),
                reason: "CDS spread cannot be negative.".into(),
            });
        }
    }
    if input.exposure < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "exposure".into(),
            reason: "Exposure cannot be negative.".into(),
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

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn base_input() -> IntensityModelInput {
        IntensityModelInput {
            cds_spreads: vec![
                CdsTenorSpread {
                    tenor: dec!(1),
                    spread: dec!(0.01),
                },
                CdsTenorSpread {
                    tenor: dec!(3),
                    spread: dec!(0.015),
                },
                CdsTenorSpread {
                    tenor: dec!(5),
                    spread: dec!(0.02),
                },
                CdsTenorSpread {
                    tenor: dec!(7),
                    spread: dec!(0.022),
                },
                CdsTenorSpread {
                    tenor: dec!(10),
                    spread: dec!(0.025),
                },
            ],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        }
    }

    #[test]
    fn test_flat_hazard_rate_calculation() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        // λ = 0.01 / (1 - 0.40) = 0.01 / 0.60 = 0.01666...
        let expected = dec!(0.01) / dec!(0.60);
        assert!(approx_eq(out.flat_hazard_rate, expected, dec!(0.0001)));
    }

    #[test]
    fn test_five_tenor_results() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        assert_eq!(out.tenor_results.len(), 5);
    }

    #[test]
    fn test_survival_prob_decreasing() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        for i in 1..out.tenor_results.len() {
            assert!(
                out.tenor_results[i].survival_prob <= out.tenor_results[i - 1].survival_prob,
                "Survival prob should be non-increasing"
            );
        }
    }

    #[test]
    fn test_cumulative_pd_increasing() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        for i in 1..out.tenor_results.len() {
            assert!(
                out.tenor_results[i].cumulative_pd >= out.tenor_results[i - 1].cumulative_pd,
                "Cumulative PD should be non-decreasing"
            );
        }
    }

    #[test]
    fn test_survival_plus_pd_equals_one() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        for tr in &out.tenor_results {
            assert!(approx_eq(
                tr.survival_prob + tr.cumulative_pd,
                Decimal::ONE,
                dec!(0.0001)
            ));
        }
    }

    #[test]
    fn test_conditional_pd_non_negative() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        for tr in &out.tenor_results {
            assert!(
                tr.conditional_pd >= Decimal::ZERO,
                "Conditional PD {} should be non-negative",
                tr.conditional_pd
            );
        }
    }

    #[test]
    fn test_conditional_pds_sum_to_cumulative() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        let sum: Decimal = out.tenor_results.iter().map(|t| t.conditional_pd).sum();
        let final_cum = out.tenor_results.last().unwrap().cumulative_pd;
        assert!(approx_eq(sum, final_cum, dec!(0.001)));
    }

    #[test]
    fn test_expected_loss_positive() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        assert!(out.total_expected_loss > Decimal::ZERO);
    }

    #[test]
    fn test_expected_loss_zero_with_zero_exposure() {
        let mut input = base_input();
        input.exposure = Decimal::ZERO;
        let out = calculate_intensity_model(&input).unwrap();
        assert_eq!(out.total_expected_loss, Decimal::ZERO);
    }

    #[test]
    fn test_hazard_rates_non_negative() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        for tr in &out.tenor_results {
            assert!(
                tr.hazard_rate >= Decimal::ZERO,
                "Hazard rate {} should be non-negative",
                tr.hazard_rate
            );
        }
    }

    #[test]
    fn test_single_tenor_flat_bootstrap() {
        let input = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(0.02),
            }],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        let out = calculate_intensity_model(&input).unwrap();
        assert_eq!(out.tenor_results.len(), 1);
        // Flat hazard = 0.02 / 0.60 = 0.0333...
        let expected_lambda = dec!(0.02) / dec!(0.60);
        assert!(approx_eq(
            out.tenor_results[0].hazard_rate,
            expected_lambda,
            dec!(0.001)
        ));
    }

    #[test]
    fn test_higher_spread_higher_pd() {
        let low = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(0.005),
            }],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        let high = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(0.05),
            }],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        let out_low = calculate_intensity_model(&low).unwrap();
        let out_high = calculate_intensity_model(&high).unwrap();
        assert!(out_high.tenor_results[0].cumulative_pd > out_low.tenor_results[0].cumulative_pd);
    }

    #[test]
    fn test_lower_recovery_higher_survival() {
        // Lower recovery => higher LGD => lower hazard from same spread => higher survival
        let low_rec = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(0.02),
            }],
            recovery_rate: dec!(0.20),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        let high_rec = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(0.02),
            }],
            recovery_rate: dec!(0.60),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        let out_low = calculate_intensity_model(&low_rec).unwrap();
        let out_high = calculate_intensity_model(&high_rec).unwrap();
        assert!(out_low.tenor_results[0].survival_prob > out_high.tenor_results[0].survival_prob);
    }

    #[test]
    fn test_reject_empty_spreads() {
        let input = IntensityModelInput {
            cds_spreads: vec![],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        assert!(calculate_intensity_model(&input).is_err());
    }

    #[test]
    fn test_reject_recovery_ge_one() {
        let input = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(0.02),
            }],
            recovery_rate: dec!(1.0),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        assert!(calculate_intensity_model(&input).is_err());
    }

    #[test]
    fn test_reject_negative_recovery() {
        let input = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(0.02),
            }],
            recovery_rate: dec!(-0.1),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        assert!(calculate_intensity_model(&input).is_err());
    }

    #[test]
    fn test_reject_negative_tenor() {
        let input = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(-1),
                spread: dec!(0.02),
            }],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        assert!(calculate_intensity_model(&input).is_err());
    }

    #[test]
    fn test_reject_negative_spread() {
        let input = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(-0.01),
            }],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        assert!(calculate_intensity_model(&input).is_err());
    }

    #[test]
    fn test_reject_negative_exposure() {
        let input = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: dec!(0.02),
            }],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(-100),
        };
        assert!(calculate_intensity_model(&input).is_err());
    }

    #[test]
    fn test_zero_spread_zero_default() {
        let input = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(5),
                spread: Decimal::ZERO,
            }],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        let out = calculate_intensity_model(&input).unwrap();
        assert_eq!(out.tenor_results[0].cumulative_pd, Decimal::ZERO);
        assert_eq!(out.tenor_results[0].survival_prob, Decimal::ONE);
    }

    #[test]
    fn test_unordered_tenors_sorted() {
        let input = IntensityModelInput {
            cds_spreads: vec![
                CdsTenorSpread {
                    tenor: dec!(10),
                    spread: dec!(0.025),
                },
                CdsTenorSpread {
                    tenor: dec!(1),
                    spread: dec!(0.01),
                },
                CdsTenorSpread {
                    tenor: dec!(5),
                    spread: dec!(0.02),
                },
            ],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        let out = calculate_intensity_model(&input).unwrap();
        assert_eq!(out.tenor_results[0].tenor, dec!(1));
        assert_eq!(out.tenor_results[1].tenor, dec!(5));
        assert_eq!(out.tenor_results[2].tenor, dec!(10));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let out = calculate_intensity_model(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: IntensityModelOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_first_tenor_survival_near_exp_neg_lambda_t() {
        let input = IntensityModelInput {
            cds_spreads: vec![CdsTenorSpread {
                tenor: dec!(1),
                spread: dec!(0.01),
            }],
            recovery_rate: dec!(0.40),
            risk_free_rate: dec!(0.03),
            exposure: dec!(1000000),
        };
        let out = calculate_intensity_model(&input).unwrap();
        let lambda = dec!(0.01) / dec!(0.60);
        let expected_surv = decimal_exp(-lambda * dec!(1));
        assert!(approx_eq(
            out.tenor_results[0].survival_prob,
            expected_surv,
            dec!(0.001)
        ));
    }
}
