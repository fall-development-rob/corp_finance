//! Model validation metrics for credit scoring models.
//!
//! Covers:
//! 1. **AUC-ROC** -- trapezoidal integration of TPR vs FPR curve.
//! 2. **CAP Curve / Accuracy Ratio** -- cumulative accuracy profile.
//! 3. **Brier Score** -- decomposed into reliability, resolution, uncertainty.
//! 4. **Hosmer-Lemeshow** -- chi-squared goodness-of-fit test.
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

/// Incomplete upper gamma function approximation for chi-squared p-value.
/// P(X > x) for chi-squared with df degrees of freedom.
/// Uses a simple series expansion for small df (adequate for HL with df ~8-10).
fn chi_sq_p_value(x: Decimal, df: u32) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ONE;
    }
    // For chi-squared, use the regularized incomplete gamma function.
    // Approximate: P(X > x) = exp(-x/2) * sum_{k=0..50} (x/2)^k / Gamma(df/2+k+1) * Gamma(df/2)
    // Simpler: use Wilson-Hilferty approximation for chi-squared to normal
    // z = ((x/df)^(1/3) - (1 - 2/(9*df))) / sqrt(2/(9*df))
    // P(X > x) = 1 - N(z)
    if df == 0 {
        return Decimal::ONE;
    }
    let df_d = Decimal::from(df);
    let ratio = x / df_d;

    // Cube root via Newton's method
    let cbrt = newton_cbrt(ratio);

    let correction = Decimal::ONE - dec!(2) / (dec!(9) * df_d);
    let variance = dec!(2) / (dec!(9) * df_d);
    let std_dev = newton_sqrt(variance);

    if std_dev.is_zero() {
        return if x > df_d {
            Decimal::ZERO
        } else {
            Decimal::ONE
        };
    }

    let z = (cbrt - correction) / std_dev;

    // 1 - N(z)
    Decimal::ONE - norm_cdf_approx(z)
}

/// Cube root via Newton's method.
fn newton_cbrt(x: Decimal) -> Decimal {
    if x.is_zero() {
        return Decimal::ZERO;
    }
    let is_neg = x < Decimal::ZERO;
    let abs_x = x.abs();
    let mut guess = abs_x / dec!(3) + dec!(0.5);
    for _ in 0..30 {
        let g2 = guess * guess;
        if g2.is_zero() {
            break;
        }
        guess = (dec!(2) * guess + abs_x / g2) / dec!(3);
    }
    if is_neg {
        -guess
    } else {
        guess
    }
}

/// Square root via Newton's method (20 iterations).
fn newton_sqrt(x: Decimal) -> Decimal {
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

/// Cumulative normal approximation (Abramowitz & Stegun).
fn norm_cdf_approx(x: Decimal) -> Decimal {
    if x < dec!(-10) {
        return Decimal::ZERO;
    }
    if x > dec!(10) {
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

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single observation (predicted probability, actual 0/1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Predicted probability of default.
    pub predicted: Decimal,
    /// Actual outcome: 0 = good (no default), 1 = bad (default).
    pub actual: u8,
}

/// Input for model validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationInput {
    /// Observations (predicted + actual).
    pub observations: Vec<Observation>,
    /// Number of bins for Hosmer-Lemeshow test (typically 10).
    pub num_bins: u32,
}

/// Output of model validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOutput {
    /// Area Under the ROC Curve.
    pub auc_roc: Decimal,
    /// Accuracy Ratio (from CAP curve): AR = 2*area_under_cap - 1.
    pub accuracy_ratio: Decimal,
    /// Gini coefficient: 2*AUC - 1.
    pub gini: Decimal,
    /// Brier Score: (1/n) * sum((p_i - y_i)^2).
    pub brier_score: Decimal,
    /// Brier reliability component.
    pub brier_reliability: Decimal,
    /// Brier resolution component.
    pub brier_resolution: Decimal,
    /// Hosmer-Lemeshow chi-squared statistic.
    pub hl_statistic: Decimal,
    /// Approximate p-value for the HL test.
    pub hl_p_value: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate model validation metrics: AUC-ROC, AR, Gini, Brier, HL.
pub fn calculate_validation(input: &ValidationInput) -> CorpFinanceResult<ValidationOutput> {
    validate_input(input)?;

    // AUC-ROC
    let auc_roc = calculate_auc_roc(&input.observations)?;
    let gini = dec!(2) * auc_roc - Decimal::ONE;

    // Accuracy Ratio (CAP)
    let accuracy_ratio = calculate_accuracy_ratio(&input.observations)?;

    // Brier Score and decomposition
    let (brier_score, brier_reliability, brier_resolution) =
        calculate_brier(&input.observations, input.num_bins)?;

    // Hosmer-Lemeshow
    let (hl_statistic, hl_p_value) =
        calculate_hosmer_lemeshow(&input.observations, input.num_bins)?;

    Ok(ValidationOutput {
        auc_roc,
        accuracy_ratio,
        gini,
        brier_score,
        brier_reliability,
        brier_resolution,
        hl_statistic,
        hl_p_value,
    })
}

// ---------------------------------------------------------------------------
// AUC-ROC via trapezoidal integration
// ---------------------------------------------------------------------------

fn calculate_auc_roc(obs: &[Observation]) -> CorpFinanceResult<Decimal> {
    let total_pos: u64 = obs.iter().filter(|o| o.actual == 1).count() as u64;
    let total_neg: u64 = obs.iter().filter(|o| o.actual == 0).count() as u64;

    if total_pos == 0 || total_neg == 0 {
        return Ok(dec!(0.5));
    }

    let total_pos_d = Decimal::from(total_pos);
    let total_neg_d = Decimal::from(total_neg);

    // Sort by predicted descending
    let mut sorted: Vec<(Decimal, u8)> = obs.iter().map(|o| (o.predicted, o.actual)).collect();
    sorted.sort_by(|a, b| b.0.cmp(&a.0));

    let mut auc = Decimal::ZERO;
    let mut tp = Decimal::ZERO;
    let mut fp = Decimal::ZERO;
    let mut prev_tp = Decimal::ZERO;
    let mut prev_fp = Decimal::ZERO;

    let mut i = 0usize;
    while i < sorted.len() {
        let current_score = sorted[i].0;
        while i < sorted.len() && sorted[i].0 == current_score {
            if sorted[i].1 == 1 {
                tp += Decimal::ONE;
            } else {
                fp += Decimal::ONE;
            }
            i += 1;
        }
        let tpr = tp / total_pos_d;
        let fpr = fp / total_neg_d;
        let prev_tpr = prev_tp / total_pos_d;
        let prev_fpr = prev_fp / total_neg_d;
        auc += (fpr - prev_fpr) * (tpr + prev_tpr) / dec!(2);
        prev_tp = tp;
        prev_fp = fp;
    }

    Ok(auc)
}

// ---------------------------------------------------------------------------
// Accuracy Ratio (CAP curve)
// ---------------------------------------------------------------------------

fn calculate_accuracy_ratio(obs: &[Observation]) -> CorpFinanceResult<Decimal> {
    let n = obs.len();
    let total_bad: u64 = obs.iter().filter(|o| o.actual == 1).count() as u64;

    if total_bad == 0 || n == 0 {
        return Ok(Decimal::ZERO);
    }

    let total_bad_d = Decimal::from(total_bad);
    let n_d = Decimal::from(n as u64);

    // Sort by predicted descending (highest risk first)
    let mut sorted: Vec<(Decimal, u8)> = obs.iter().map(|o| (o.predicted, o.actual)).collect();
    sorted.sort_by(|a, b| b.0.cmp(&a.0));

    // CAP curve: cumulative fraction of bads captured vs fraction of population
    let mut cum_bad = Decimal::ZERO;
    let mut area_model = Decimal::ZERO;
    let mut prev_y = Decimal::ZERO;

    for (idx, (_, actual)) in sorted.iter().enumerate() {
        if *actual == 1 {
            cum_bad += Decimal::ONE;
        }
        let x = Decimal::from((idx + 1) as u64) / n_d;
        let y = cum_bad / total_bad_d;

        // Trapezoidal
        let dx = x - (Decimal::from(idx as u64) / n_d);
        area_model += dx * (y + prev_y) / dec!(2);
        prev_y = y;
    }

    // Perfect model area
    let bad_frac = total_bad_d / n_d;
    let area_perfect = Decimal::ONE - bad_frac / dec!(2);

    // Random model area = 0.5
    let area_random = dec!(0.5);

    let ar = if (area_perfect - area_random).is_zero() {
        Decimal::ZERO
    } else {
        (area_model - area_random) / (area_perfect - area_random)
    };

    Ok(ar)
}

// ---------------------------------------------------------------------------
// Brier Score and decomposition
// ---------------------------------------------------------------------------

fn calculate_brier(
    obs: &[Observation],
    num_bins: u32,
) -> CorpFinanceResult<(Decimal, Decimal, Decimal)> {
    let n = obs.len();
    let n_d = Decimal::from(n as u64);

    if n == 0 {
        return Ok((Decimal::ZERO, Decimal::ZERO, Decimal::ZERO));
    }

    // Brier score
    let mut brier_sum = Decimal::ZERO;
    let overall_mean: Decimal = obs
        .iter()
        .map(|o| Decimal::from(o.actual as u64))
        .sum::<Decimal>()
        / n_d;

    for o in obs {
        let diff = o.predicted - Decimal::from(o.actual as u64);
        brier_sum += diff * diff;
    }
    let brier_score = brier_sum / n_d;

    // Decomposition: group into bins by predicted probability
    let bins = num_bins.max(1);
    let bin_width = Decimal::ONE / Decimal::from(bins);

    let mut reliability = Decimal::ZERO;
    let mut resolution = Decimal::ZERO;

    for b in 0..bins {
        let lower = Decimal::from(b) * bin_width;
        let upper = if b == bins - 1 {
            Decimal::ONE + dec!(0.0001) // include 1.0
        } else {
            Decimal::from(b + 1) * bin_width
        };

        let bin_obs: Vec<&Observation> = obs
            .iter()
            .filter(|o| o.predicted >= lower && o.predicted < upper)
            .collect();

        let nk = bin_obs.len();
        if nk == 0 {
            continue;
        }
        let nk_d = Decimal::from(nk as u64);

        let mean_pred: Decimal = bin_obs.iter().map(|o| o.predicted).sum::<Decimal>() / nk_d;
        let mean_actual: Decimal = bin_obs
            .iter()
            .map(|o| Decimal::from(o.actual as u64))
            .sum::<Decimal>()
            / nk_d;

        // Reliability: (1/n) * n_k * (mean_pred - mean_actual)^2
        let diff = mean_pred - mean_actual;
        reliability += nk_d * diff * diff / n_d;

        // Resolution: (1/n) * n_k * (mean_actual - overall_mean)^2
        let diff2 = mean_actual - overall_mean;
        resolution += nk_d * diff2 * diff2 / n_d;
    }

    Ok((brier_score, reliability, resolution))
}

// ---------------------------------------------------------------------------
// Hosmer-Lemeshow test
// ---------------------------------------------------------------------------

fn calculate_hosmer_lemeshow(
    obs: &[Observation],
    num_bins: u32,
) -> CorpFinanceResult<(Decimal, Decimal)> {
    let n = obs.len();
    if n == 0 {
        return Ok((Decimal::ZERO, Decimal::ONE));
    }

    // Sort by predicted probability
    let mut sorted = obs.to_vec();
    sorted.sort_by(|a, b| a.predicted.cmp(&b.predicted));

    let bins = num_bins.max(2) as usize;
    let bin_size = n / bins;
    let remainder = n % bins;

    let mut hl_stat = Decimal::ZERO;
    let mut start = 0usize;
    let mut actual_bins = 0u32;

    for b in 0..bins {
        let extra = if b < remainder { 1 } else { 0 };
        let end = start + bin_size + extra;
        if start >= n || start >= end {
            break;
        }
        let bin_slice = &sorted[start..end];
        let nk = bin_slice.len();
        let nk_d = Decimal::from(nk as u64);

        let observed_events: Decimal = bin_slice
            .iter()
            .map(|o| Decimal::from(o.actual as u64))
            .sum();
        let expected_events: Decimal = bin_slice.iter().map(|o| o.predicted).sum();

        let observed_non: Decimal = nk_d - observed_events;
        let expected_non: Decimal = nk_d - expected_events;

        // HL component: (O - E)^2 / (E * (1 - E/n_k))
        // Simplified: (O1 - E1)^2 / E1 + (O0 - E0)^2 / E0
        if expected_events > Decimal::ZERO {
            let diff = observed_events - expected_events;
            hl_stat += diff * diff / expected_events;
        }
        if expected_non > Decimal::ZERO {
            let diff = observed_non - expected_non;
            hl_stat += diff * diff / expected_non;
        }

        actual_bins += 1;
        start = end;
    }

    // Degrees of freedom = num_groups - 2
    let df = if actual_bins > 2 { actual_bins - 2 } else { 1 };
    let p_value = chi_sq_p_value(hl_stat, df);

    Ok((hl_stat, p_value))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &ValidationInput) -> CorpFinanceResult<()> {
    if input.observations.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one observation is required.".into(),
        ));
    }
    if input.num_bins == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_bins".into(),
            reason: "Number of bins must be positive.".into(),
        });
    }
    for o in &input.observations {
        if o.actual > 1 {
            return Err(CorpFinanceError::InvalidInput {
                field: "actual".into(),
                reason: "Actual must be 0 or 1.".into(),
            });
        }
        if o.predicted < Decimal::ZERO || o.predicted > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "predicted".into(),
                reason: "Predicted probability must be in [0, 1].".into(),
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

    fn sample_obs() -> Vec<Observation> {
        vec![
            Observation {
                predicted: dec!(0.1),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.15),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.2),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.25),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.3),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.4),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.5),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.6),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.7),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.75),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.8),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.85),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.9),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.95),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.05),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.35),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.45),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.55),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.65),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.92),
                actual: 1,
            },
        ]
    }

    fn base_input() -> ValidationInput {
        ValidationInput {
            observations: sample_obs(),
            num_bins: 10,
        }
    }

    #[test]
    fn test_auc_between_zero_and_one() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(
            out.auc_roc >= Decimal::ZERO && out.auc_roc <= Decimal::ONE,
            "AUC {} should be in [0, 1]",
            out.auc_roc
        );
    }

    #[test]
    fn test_auc_above_random() {
        // Our sample has some discrimination power
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(
            out.auc_roc > dec!(0.5),
            "AUC {} should exceed random 0.5",
            out.auc_roc
        );
    }

    #[test]
    fn test_gini_equals_two_auc_minus_one() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        let expected_gini = dec!(2) * out.auc_roc - Decimal::ONE;
        assert_eq!(out.gini, expected_gini);
    }

    #[test]
    fn test_gini_between_minus_one_and_one() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(out.gini >= dec!(-1) && out.gini <= Decimal::ONE);
    }

    #[test]
    fn test_perfect_model_auc_one() {
        let obs = vec![
            Observation {
                predicted: dec!(0.1),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.2),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.3),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.8),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.9),
                actual: 1,
            },
        ];
        let input = ValidationInput {
            observations: obs,
            num_bins: 5,
        };
        let out = calculate_validation(&input).unwrap();
        assert!(
            approx_eq(out.auc_roc, Decimal::ONE, dec!(0.01)),
            "Perfect model AUC {} should be 1.0",
            out.auc_roc
        );
    }

    #[test]
    fn test_accuracy_ratio_between_minus_one_and_one() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(
            out.accuracy_ratio >= dec!(-1) && out.accuracy_ratio <= Decimal::ONE,
            "AR {} should be in [-1, 1]",
            out.accuracy_ratio
        );
    }

    #[test]
    fn test_accuracy_ratio_positive_for_good_model() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(out.accuracy_ratio > Decimal::ZERO);
    }

    #[test]
    fn test_brier_score_non_negative() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(out.brier_score >= Decimal::ZERO);
    }

    #[test]
    fn test_brier_score_max_one() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(out.brier_score <= Decimal::ONE);
    }

    #[test]
    fn test_brier_perfect_predictions() {
        let obs = vec![
            Observation {
                predicted: Decimal::ZERO,
                actual: 0,
            },
            Observation {
                predicted: Decimal::ZERO,
                actual: 0,
            },
            Observation {
                predicted: Decimal::ONE,
                actual: 1,
            },
            Observation {
                predicted: Decimal::ONE,
                actual: 1,
            },
        ];
        let input = ValidationInput {
            observations: obs,
            num_bins: 2,
        };
        let out = calculate_validation(&input).unwrap();
        assert_eq!(out.brier_score, Decimal::ZERO);
    }

    #[test]
    fn test_brier_worst_predictions() {
        let obs = vec![
            Observation {
                predicted: Decimal::ONE,
                actual: 0,
            },
            Observation {
                predicted: Decimal::ZERO,
                actual: 1,
            },
        ];
        let input = ValidationInput {
            observations: obs,
            num_bins: 2,
        };
        let out = calculate_validation(&input).unwrap();
        assert_eq!(out.brier_score, Decimal::ONE);
    }

    #[test]
    fn test_brier_reliability_non_negative() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(out.brier_reliability >= Decimal::ZERO);
    }

    #[test]
    fn test_brier_resolution_non_negative() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(out.brier_resolution >= Decimal::ZERO);
    }

    #[test]
    fn test_hl_statistic_non_negative() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(out.hl_statistic >= Decimal::ZERO);
    }

    #[test]
    fn test_hl_p_value_between_zero_and_one() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        assert!(
            out.hl_p_value >= Decimal::ZERO && out.hl_p_value <= Decimal::ONE,
            "HL p-value {} should be in [0, 1]",
            out.hl_p_value
        );
    }

    #[test]
    fn test_well_calibrated_model_low_hl() {
        // Predictions close to actual rates should have low HL statistic
        let obs = vec![
            Observation {
                predicted: dec!(0.5),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.5),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.5),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.5),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.5),
                actual: 1,
            },
            Observation {
                predicted: dec!(0.5),
                actual: 0,
            },
        ];
        let input = ValidationInput {
            observations: obs,
            num_bins: 2,
        };
        let out = calculate_validation(&input).unwrap();
        // This should be relatively well-calibrated
        assert!(out.hl_statistic < dec!(5.0));
    }

    #[test]
    fn test_reject_empty_observations() {
        let input = ValidationInput {
            observations: vec![],
            num_bins: 10,
        };
        assert!(calculate_validation(&input).is_err());
    }

    #[test]
    fn test_reject_zero_bins() {
        let input = ValidationInput {
            observations: sample_obs(),
            num_bins: 0,
        };
        assert!(calculate_validation(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_actual() {
        let input = ValidationInput {
            observations: vec![Observation {
                predicted: dec!(0.5),
                actual: 2,
            }],
            num_bins: 10,
        };
        assert!(calculate_validation(&input).is_err());
    }

    #[test]
    fn test_reject_predicted_out_of_range_high() {
        let input = ValidationInput {
            observations: vec![Observation {
                predicted: dec!(1.5),
                actual: 0,
            }],
            num_bins: 10,
        };
        assert!(calculate_validation(&input).is_err());
    }

    #[test]
    fn test_reject_predicted_out_of_range_low() {
        let input = ValidationInput {
            observations: vec![Observation {
                predicted: dec!(-0.1),
                actual: 0,
            }],
            num_bins: 10,
        };
        assert!(calculate_validation(&input).is_err());
    }

    #[test]
    fn test_all_same_class_auc_half() {
        let obs = vec![
            Observation {
                predicted: dec!(0.5),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.6),
                actual: 0,
            },
            Observation {
                predicted: dec!(0.7),
                actual: 0,
            },
        ];
        let input = ValidationInput {
            observations: obs,
            num_bins: 3,
        };
        let out = calculate_validation(&input).unwrap();
        assert_eq!(out.auc_roc, dec!(0.5));
    }

    #[test]
    fn test_single_observation() {
        let obs = vec![Observation {
            predicted: dec!(0.5),
            actual: 1,
        }];
        let input = ValidationInput {
            observations: obs,
            num_bins: 1,
        };
        let out = calculate_validation(&input).unwrap();
        assert!(out.brier_score >= Decimal::ZERO);
    }

    #[test]
    fn test_two_bins_hl() {
        let input = ValidationInput {
            observations: sample_obs(),
            num_bins: 2,
        };
        let out = calculate_validation(&input).unwrap();
        assert!(out.hl_statistic >= Decimal::ZERO);
    }

    #[test]
    fn test_large_num_bins_handled() {
        let input = ValidationInput {
            observations: sample_obs(),
            num_bins: 100,
        };
        let out = calculate_validation(&input).unwrap();
        assert!(out.auc_roc >= Decimal::ZERO);
    }

    #[test]
    fn test_chi_sq_p_value_zero_stat() {
        let p = chi_sq_p_value(Decimal::ZERO, 8);
        assert_eq!(p, Decimal::ONE);
    }

    #[test]
    fn test_chi_sq_p_value_large_stat() {
        let p = chi_sq_p_value(dec!(100), 8);
        assert!(p < dec!(0.01));
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let out = calculate_validation(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: ValidationOutput = serde_json::from_str(&json).unwrap();
    }
}
