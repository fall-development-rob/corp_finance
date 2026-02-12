//! Logistic regression scorecard analytics.
//!
//! Covers:
//! 1. **Weight of Evidence (WoE)** -- ln(good_rate / bad_rate) per bin.
//! 2. **Information Value (IV)** -- predictive power of each variable.
//! 3. **Scorecard Points** -- transform WoE into additive score.
//! 4. **Gini Coefficient** -- 2*AUC - 1 from sorted predicted/actual pairs.
//! 5. **KS Statistic** -- max |CDF_good - CDF_bad| across thresholds.
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

/// Natural logarithm via Taylor series around 1. ln(x) for x > 0.
/// Uses the identity: ln(x) = 2 * sum_{k=0..N} (1/(2k+1)) * ((x-1)/(x+1))^(2k+1)
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    // Reduce: factor out powers of e ~ 2.718281828
    // Use ln(x) = ln(x/2^n) + n*ln(2) to bring x into [0.5, 2.0]
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
    // Taylor series: ln(val) = 2 * sum_{k=0..40} (1/(2k+1)) * ((val-1)/(val+1))^(2k+1)
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

/// A single bin in a WoE analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WoeBin {
    /// Lower boundary of the bin (inclusive).
    pub lower: Decimal,
    /// Upper boundary of the bin (exclusive, except last bin).
    pub upper: Decimal,
    /// Number of "good" (non-default) observations in this bin.
    pub good_count: u64,
    /// Number of "bad" (default) observations in this bin.
    pub bad_count: u64,
}

/// Input for scorecard calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorecardInput {
    /// WoE bins for the variable being scored.
    pub bins: Vec<WoeBin>,
    /// Target base score (e.g. 600).
    pub target_score: Decimal,
    /// Target odds (good:bad ratio at the target score, e.g. 50 means 50:1).
    pub target_odds: Decimal,
    /// Points to double the odds (PDO), e.g. 20.
    pub pdo: Decimal,
    /// Predicted-vs-actual pairs for Gini and KS (predicted probability, actual 0/1).
    pub predictions: Vec<PredictionPair>,
}

/// A single predicted/actual pair for discrimination metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionPair {
    /// Predicted probability of default.
    pub predicted: Decimal,
    /// Actual outcome: 0 = good, 1 = bad.
    pub actual: u8,
}

/// Per-bin WoE and IV detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinResult {
    /// Lower boundary.
    pub lower: Decimal,
    /// Upper boundary.
    pub upper: Decimal,
    /// Weight of Evidence for this bin.
    pub woe: Decimal,
    /// Information Value contribution for this bin.
    pub iv: Decimal,
    /// Scorecard points for this bin.
    pub points: Decimal,
}

/// IV strength classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IvStrength {
    Useless,
    Weak,
    Medium,
    Strong,
    Suspicious,
}

impl std::fmt::Display for IvStrength {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IvStrength::Useless => write!(f, "Useless"),
            IvStrength::Weak => write!(f, "Weak"),
            IvStrength::Medium => write!(f, "Medium"),
            IvStrength::Strong => write!(f, "Strong"),
            IvStrength::Suspicious => write!(f, "Suspicious"),
        }
    }
}

/// Output of the scorecard calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScorecardOutput {
    /// Per-bin results with WoE, IV, and scorecard points.
    pub bin_results: Vec<BinResult>,
    /// Total Information Value across all bins.
    pub total_iv: Decimal,
    /// IV strength classification.
    pub iv_strength: String,
    /// Gini coefficient (2*AUC - 1).
    pub gini: Decimal,
    /// KS statistic: max |CDF_good - CDF_bad|.
    pub ks_statistic: Decimal,
    /// Factor used in scorecard point calculation.
    pub factor: Decimal,
    /// Offset used in scorecard point calculation.
    pub offset: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Calculate scorecard analytics: WoE, IV, scorecard points, Gini, KS.
pub fn calculate_scorecard(input: &ScorecardInput) -> CorpFinanceResult<ScorecardOutput> {
    validate_scorecard_input(input)?;

    let ln2 = dec!(0.6931471805599453);

    // Factor and offset for scorecard points
    let factor = input.pdo / ln2;
    let offset = input.target_score - factor * decimal_ln(input.target_odds);
    let n_bins = Decimal::from(input.bins.len() as u64);

    // Total goods and bads
    let total_good: u64 = input.bins.iter().map(|b| b.good_count).sum();
    let total_bad: u64 = input.bins.iter().map(|b| b.bad_count).sum();
    let total_good_d = Decimal::from(total_good);
    let total_bad_d = Decimal::from(total_bad);

    let mut bin_results = Vec::with_capacity(input.bins.len());
    let mut total_iv = Decimal::ZERO;

    for bin in &input.bins {
        let good_rate = if total_good == 0 {
            Decimal::ZERO
        } else {
            Decimal::from(bin.good_count) / total_good_d
        };
        let bad_rate = if total_bad == 0 {
            Decimal::ZERO
        } else {
            Decimal::from(bin.bad_count) / total_bad_d
        };

        // Avoid division by zero / ln(0) by flooring rates at a small epsilon
        let eps = dec!(0.0001);
        let safe_good = if good_rate < eps { eps } else { good_rate };
        let safe_bad = if bad_rate < eps { eps } else { bad_rate };

        // WoE = ln(good_rate / bad_rate)
        let woe = decimal_ln(safe_good / safe_bad);

        // IV_i = (good_rate - bad_rate) * WoE
        let iv_i = (safe_good - safe_bad) * woe;

        // Scorecard points: -(WoE * factor + offset / n)
        let points = -(woe * factor + offset / n_bins);

        total_iv += iv_i;

        bin_results.push(BinResult {
            lower: bin.lower,
            upper: bin.upper,
            woe,
            iv: iv_i,
            points,
        });
    }

    // IV strength classification
    let iv_strength = classify_iv(total_iv);

    // Gini and KS from predictions
    let (gini, ks_statistic) = if input.predictions.is_empty() {
        (Decimal::ZERO, Decimal::ZERO)
    } else {
        let gini_val = calculate_gini(&input.predictions)?;
        let ks_val = calculate_ks(&input.predictions)?;
        (gini_val, ks_val)
    };

    Ok(ScorecardOutput {
        bin_results,
        total_iv,
        iv_strength: iv_strength.to_string(),
        gini,
        ks_statistic,
        factor,
        offset,
    })
}

// ---------------------------------------------------------------------------
// IV classification
// ---------------------------------------------------------------------------

fn classify_iv(iv: Decimal) -> IvStrength {
    if iv < dec!(0.02) {
        IvStrength::Useless
    } else if iv < dec!(0.1) {
        IvStrength::Weak
    } else if iv < dec!(0.3) {
        IvStrength::Medium
    } else if iv < dec!(0.5) {
        IvStrength::Strong
    } else {
        IvStrength::Suspicious
    }
}

// ---------------------------------------------------------------------------
// Gini = 2*AUC - 1
// ---------------------------------------------------------------------------

fn calculate_gini(predictions: &[PredictionPair]) -> CorpFinanceResult<Decimal> {
    // Sort by predicted descending
    let mut sorted: Vec<(Decimal, u8)> = predictions
        .iter()
        .map(|p| (p.predicted, p.actual))
        .collect();
    sorted.sort_by(|a, b| b.0.cmp(&a.0));

    let total_bad: u64 = sorted.iter().filter(|(_, a)| *a == 1).count() as u64;
    let total_good: u64 = sorted.iter().filter(|(_, a)| *a == 0).count() as u64;

    if total_bad == 0 || total_good == 0 {
        return Ok(Decimal::ZERO);
    }

    let total_bad_d = Decimal::from(total_bad);
    let total_good_d = Decimal::from(total_good);

    // AUC via trapezoidal: walk through sorted predictions
    let mut auc = Decimal::ZERO;
    let mut tp = Decimal::ZERO;
    let mut fp = Decimal::ZERO;
    let mut prev_tp = Decimal::ZERO;
    let mut prev_fp = Decimal::ZERO;

    let mut i = 0usize;
    while i < sorted.len() {
        let current_score = sorted[i].0;
        // Process all tied scores
        while i < sorted.len() && sorted[i].0 == current_score {
            if sorted[i].1 == 1 {
                tp += Decimal::ONE;
            } else {
                fp += Decimal::ONE;
            }
            i += 1;
        }
        // Trapezoidal rule
        let tpr = tp / total_bad_d;
        let fpr = fp / total_good_d;
        let prev_tpr = prev_tp / total_bad_d;
        let prev_fpr = prev_fp / total_good_d;
        auc += (fpr - prev_fpr) * (tpr + prev_tpr) / dec!(2);
        prev_tp = tp;
        prev_fp = fp;
    }

    let gini = dec!(2) * auc - Decimal::ONE;
    Ok(gini)
}

// ---------------------------------------------------------------------------
// KS statistic = max |CDF_good - CDF_bad|
// ---------------------------------------------------------------------------

fn calculate_ks(predictions: &[PredictionPair]) -> CorpFinanceResult<Decimal> {
    let mut sorted: Vec<(Decimal, u8)> = predictions
        .iter()
        .map(|p| (p.predicted, p.actual))
        .collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    let total_bad: u64 = sorted.iter().filter(|(_, a)| *a == 1).count() as u64;
    let total_good: u64 = sorted.iter().filter(|(_, a)| *a == 0).count() as u64;

    if total_bad == 0 || total_good == 0 {
        return Ok(Decimal::ZERO);
    }

    let total_bad_d = Decimal::from(total_bad);
    let total_good_d = Decimal::from(total_good);

    let mut cum_good = Decimal::ZERO;
    let mut cum_bad = Decimal::ZERO;
    let mut max_ks = Decimal::ZERO;

    for (_, actual) in &sorted {
        if *actual == 0 {
            cum_good += Decimal::ONE;
        } else {
            cum_bad += Decimal::ONE;
        }
        let cdf_good = cum_good / total_good_d;
        let cdf_bad = cum_bad / total_bad_d;
        let diff = (cdf_good - cdf_bad).abs();
        if diff > max_ks {
            max_ks = diff;
        }
    }

    Ok(max_ks)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_scorecard_input(input: &ScorecardInput) -> CorpFinanceResult<()> {
    if input.bins.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one bin is required.".into(),
        ));
    }
    if input.pdo <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pdo".into(),
            reason: "Points to double odds must be positive.".into(),
        });
    }
    if input.target_odds <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "target_odds".into(),
            reason: "Target odds must be positive.".into(),
        });
    }
    // Validate bin counts
    let total_good: u64 = input.bins.iter().map(|b| b.good_count).sum();
    let total_bad: u64 = input.bins.iter().map(|b| b.bad_count).sum();
    if total_good == 0 && total_bad == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "Bins must contain at least one observation.".into(),
        ));
    }
    // Validate predictions
    for p in &input.predictions {
        if p.actual > 1 {
            return Err(CorpFinanceError::InvalidInput {
                field: "actual".into(),
                reason: "Actual must be 0 or 1.".into(),
            });
        }
        if p.predicted < Decimal::ZERO || p.predicted > Decimal::ONE {
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

    fn sample_bins() -> Vec<WoeBin> {
        vec![
            WoeBin {
                lower: dec!(0),
                upper: dec!(30),
                good_count: 400,
                bad_count: 100,
            },
            WoeBin {
                lower: dec!(30),
                upper: dec!(60),
                good_count: 300,
                bad_count: 200,
            },
            WoeBin {
                lower: dec!(60),
                upper: dec!(100),
                good_count: 200,
                bad_count: 300,
            },
        ]
    }

    fn sample_predictions() -> Vec<PredictionPair> {
        vec![
            PredictionPair {
                predicted: dec!(0.1),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.2),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.3),
                actual: 1,
            },
            PredictionPair {
                predicted: dec!(0.4),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.5),
                actual: 1,
            },
            PredictionPair {
                predicted: dec!(0.6),
                actual: 1,
            },
            PredictionPair {
                predicted: dec!(0.7),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.8),
                actual: 1,
            },
            PredictionPair {
                predicted: dec!(0.9),
                actual: 1,
            },
            PredictionPair {
                predicted: dec!(0.05),
                actual: 0,
            },
        ]
    }

    fn base_input() -> ScorecardInput {
        ScorecardInput {
            bins: sample_bins(),
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: sample_predictions(),
        }
    }

    #[test]
    fn test_woe_positive_for_good_dominant_bin() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        // First bin: 400 good, 100 bad => good_rate > bad_rate => WoE > 0
        assert!(out.bin_results[0].woe > Decimal::ZERO);
    }

    #[test]
    fn test_woe_negative_for_bad_dominant_bin() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        // Third bin: 200 good, 300 bad => good_rate < bad_rate => WoE < 0
        assert!(out.bin_results[2].woe < Decimal::ZERO);
    }

    #[test]
    fn test_iv_per_bin_non_negative() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        // IV_i = (good_rate - bad_rate) * WoE. Sign of diff and WoE align => non-negative
        for br in &out.bin_results {
            assert!(
                br.iv >= Decimal::ZERO,
                "IV {} should be non-negative",
                br.iv
            );
        }
    }

    #[test]
    fn test_total_iv_is_sum_of_bin_iv() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        let sum: Decimal = out.bin_results.iter().map(|b| b.iv).sum();
        assert!(approx_eq(out.total_iv, sum, dec!(0.0001)));
    }

    #[test]
    fn test_iv_strength_classification_useless() {
        assert_eq!(classify_iv(dec!(0.01)), IvStrength::Useless);
    }

    #[test]
    fn test_iv_strength_classification_weak() {
        assert_eq!(classify_iv(dec!(0.05)), IvStrength::Weak);
    }

    #[test]
    fn test_iv_strength_classification_medium() {
        assert_eq!(classify_iv(dec!(0.15)), IvStrength::Medium);
    }

    #[test]
    fn test_iv_strength_classification_strong() {
        assert_eq!(classify_iv(dec!(0.35)), IvStrength::Strong);
    }

    #[test]
    fn test_iv_strength_classification_suspicious() {
        assert_eq!(classify_iv(dec!(0.55)), IvStrength::Suspicious);
    }

    #[test]
    fn test_factor_calculation() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        let ln2 = dec!(0.6931471805599453);
        let expected_factor = dec!(20) / ln2;
        assert!(approx_eq(out.factor, expected_factor, dec!(0.001)));
    }

    #[test]
    fn test_gini_between_minus_one_and_one() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        assert!(out.gini >= dec!(-1) && out.gini <= Decimal::ONE);
    }

    #[test]
    fn test_gini_perfect_discrimination() {
        // All bads have higher predicted than all goods
        let preds = vec![
            PredictionPair {
                predicted: dec!(0.1),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.2),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.3),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.8),
                actual: 1,
            },
            PredictionPair {
                predicted: dec!(0.9),
                actual: 1,
            },
        ];
        let input = ScorecardInput {
            bins: sample_bins(),
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: preds,
        };
        let out = calculate_scorecard(&input).unwrap();
        assert!(
            out.gini > dec!(0.9),
            "Perfect discrimination should give Gini near 1, got {}",
            out.gini
        );
    }

    #[test]
    fn test_ks_between_zero_and_one() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        assert!(out.ks_statistic >= Decimal::ZERO && out.ks_statistic <= Decimal::ONE);
    }

    #[test]
    fn test_ks_perfect_separation() {
        let preds = vec![
            PredictionPair {
                predicted: dec!(0.1),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.2),
                actual: 0,
            },
            PredictionPair {
                predicted: dec!(0.8),
                actual: 1,
            },
            PredictionPair {
                predicted: dec!(0.9),
                actual: 1,
            },
        ];
        let input = ScorecardInput {
            bins: sample_bins(),
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: preds,
        };
        let out = calculate_scorecard(&input).unwrap();
        assert_eq!(out.ks_statistic, Decimal::ONE);
    }

    #[test]
    fn test_scorecard_points_computed_for_each_bin() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        assert_eq!(out.bin_results.len(), 3);
    }

    #[test]
    fn test_reject_empty_bins() {
        let input = ScorecardInput {
            bins: vec![],
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: vec![],
        };
        assert!(calculate_scorecard(&input).is_err());
    }

    #[test]
    fn test_reject_zero_pdo() {
        let input = ScorecardInput {
            bins: sample_bins(),
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: Decimal::ZERO,
            predictions: vec![],
        };
        assert!(calculate_scorecard(&input).is_err());
    }

    #[test]
    fn test_reject_zero_target_odds() {
        let input = ScorecardInput {
            bins: sample_bins(),
            target_score: dec!(600),
            target_odds: Decimal::ZERO,
            pdo: dec!(20),
            predictions: vec![],
        };
        assert!(calculate_scorecard(&input).is_err());
    }

    #[test]
    fn test_reject_invalid_actual() {
        let input = ScorecardInput {
            bins: sample_bins(),
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: vec![PredictionPair {
                predicted: dec!(0.5),
                actual: 2,
            }],
        };
        assert!(calculate_scorecard(&input).is_err());
    }

    #[test]
    fn test_reject_prediction_out_of_range() {
        let input = ScorecardInput {
            bins: sample_bins(),
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: vec![PredictionPair {
                predicted: dec!(1.5),
                actual: 0,
            }],
        };
        assert!(calculate_scorecard(&input).is_err());
    }

    #[test]
    fn test_empty_predictions_give_zero_gini_ks() {
        let input = ScorecardInput {
            bins: sample_bins(),
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: vec![],
        };
        let out = calculate_scorecard(&input).unwrap();
        assert_eq!(out.gini, Decimal::ZERO);
        assert_eq!(out.ks_statistic, Decimal::ZERO);
    }

    #[test]
    fn test_decimal_ln_of_one() {
        let result = decimal_ln(Decimal::ONE);
        assert!(approx_eq(result, Decimal::ZERO, dec!(0.0001)));
    }

    #[test]
    fn test_decimal_ln_of_e() {
        let e_approx = dec!(2.718281828);
        let result = decimal_ln(e_approx);
        assert!(approx_eq(result, Decimal::ONE, dec!(0.001)));
    }

    #[test]
    fn test_woe_equal_bins_near_zero() {
        let bins = vec![
            WoeBin {
                lower: dec!(0),
                upper: dec!(50),
                good_count: 100,
                bad_count: 100,
            },
            WoeBin {
                lower: dec!(50),
                upper: dec!(100),
                good_count: 100,
                bad_count: 100,
            },
        ];
        let input = ScorecardInput {
            bins,
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: vec![],
        };
        let out = calculate_scorecard(&input).unwrap();
        for br in &out.bin_results {
            assert!(
                approx_eq(br.woe, Decimal::ZERO, dec!(0.01)),
                "Equal bins WoE should be ~0"
            );
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = base_input();
        let out = calculate_scorecard(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: ScorecardOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_all_good_no_bad_handles_gracefully() {
        let bins = vec![WoeBin {
            lower: dec!(0),
            upper: dec!(100),
            good_count: 500,
            bad_count: 0,
        }];
        let input = ScorecardInput {
            bins,
            target_score: dec!(600),
            target_odds: dec!(50),
            pdo: dec!(20),
            predictions: vec![],
        };
        let out = calculate_scorecard(&input).unwrap();
        assert_eq!(out.bin_results.len(), 1);
    }
}
