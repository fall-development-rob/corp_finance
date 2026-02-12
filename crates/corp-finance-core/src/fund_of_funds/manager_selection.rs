//! Manager Selection and Due Diligence scoring model.
//!
//! Provides a quantitative + qualitative framework for evaluating
//! private equity fund managers:
//!
//! - **Performance scoring**: composite of TVPI, IRR, DPI quartile rankings
//! - **Persistence analysis**: correlation of consecutive fund performance
//! - **Alpha estimation**: fund IRR vs PME (excess return)
//! - **Dispersion analysis**: spread between top/bottom quartile
//! - **Qualitative scoring**: weighted factor assessment
//! - **Overall rating**: blended quant + qualitative score
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

/// A single fund in the manager's track record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundRecord {
    /// Fund name (e.g. "Fund III").
    pub name: String,
    /// Vintage year.
    pub vintage: u32,
    /// Net IRR to LPs.
    pub irr: Decimal,
    /// TVPI multiple.
    pub tvpi: Decimal,
    /// DPI multiple.
    pub dpi: Decimal,
    /// Public Market Equivalent ratio.
    pub pme: Decimal,
}

/// A qualitative assessment factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitativeFactor {
    /// Factor name (e.g. "Team", "Strategy", "Track Record").
    pub factor: String,
    /// Weight in the composite (decimal, should sum to 1.0 across all factors).
    pub weight: Decimal,
    /// Score from 1 to 5.
    pub score: Decimal,
}

/// Benchmark quartile boundaries for a given metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkQuartile {
    /// Metric name (e.g. "irr", "tvpi", "dpi").
    pub metric: String,
    /// First quartile (top 25%) threshold.
    pub q1: Decimal,
    /// Median threshold.
    pub median: Decimal,
    /// Third quartile (bottom 25%) threshold.
    pub q3: Decimal,
}

/// Input for manager selection analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerSelectionInput {
    /// Manager name.
    pub manager_name: String,
    /// Historical fund track record.
    pub funds: Vec<FundRecord>,
    /// Qualitative assessment factors.
    pub qualitative_scores: Vec<QualitativeFactor>,
    /// Benchmark quartile boundaries.
    pub benchmark_quartiles: Vec<BenchmarkQuartile>,
}

/// Quartile ranking for a specific fund and metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundQuartileRanking {
    /// Fund name.
    pub fund: String,
    /// Metric name.
    pub metric: String,
    /// Metric value.
    pub value: Decimal,
    /// Quartile (1 = top, 4 = bottom).
    pub quartile: u32,
}

/// Recommendation rating.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Recommendation {
    StrongBuy,
    Buy,
    Hold,
    Pass,
}

impl std::fmt::Display for Recommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Recommendation::StrongBuy => write!(f, "Strong Buy"),
            Recommendation::Buy => write!(f, "Buy"),
            Recommendation::Hold => write!(f, "Hold"),
            Recommendation::Pass => write!(f, "Pass"),
        }
    }
}

/// Output of the manager selection analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerSelectionOutput {
    /// Quantitative score (0-100).
    pub quantitative_score: Decimal,
    /// Qualitative score (0-100).
    pub qualitative_score: Decimal,
    /// Overall blended score (0-100).
    pub overall_score: Decimal,
    /// Quartile rankings for each fund and metric.
    pub fund_quartiles: Vec<FundQuartileRanking>,
    /// Persistence correlation between consecutive funds.
    pub persistence_correlation: Decimal,
    /// Average alpha (fund IRR - PME).
    pub average_alpha: Decimal,
    /// Return consistency measure (1 - coefficient of variation of IRRs).
    pub return_consistency: Decimal,
    /// Overall recommendation.
    pub recommendation: Recommendation,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Perform manager selection analysis.
pub fn analyze_manager_selection(
    input: &ManagerSelectionInput,
) -> CorpFinanceResult<ManagerSelectionOutput> {
    validate_manager_input(input)?;

    // 1. Quartile rankings
    let fund_quartiles = compute_quartile_rankings(&input.funds, &input.benchmark_quartiles);

    // 2. Quantitative score based on average quartile rankings.
    //    Q1 = 100, Q2 = 75, Q3 = 50, Q4 = 25.
    let quantitative_score = compute_quantitative_score(&fund_quartiles);

    // 3. Qualitative score: weighted sum of scores, normalized to 0-100.
    let qualitative_score = compute_qualitative_score(&input.qualitative_scores);

    // 4. Persistence correlation
    let persistence_correlation = compute_persistence(&input.funds);

    // 5. Average alpha
    let average_alpha = compute_average_alpha(&input.funds);

    // 6. Return consistency
    let return_consistency = compute_return_consistency(&input.funds);

    // 7. Overall score: 60% quantitative + 40% qualitative.
    let overall_score = dec!(0.60) * quantitative_score + dec!(0.40) * qualitative_score;

    // 8. Recommendation based on overall score.
    let recommendation = if overall_score >= dec!(80) {
        Recommendation::StrongBuy
    } else if overall_score >= dec!(60) {
        Recommendation::Buy
    } else if overall_score >= dec!(40) {
        Recommendation::Hold
    } else {
        Recommendation::Pass
    };

    Ok(ManagerSelectionOutput {
        quantitative_score,
        qualitative_score,
        overall_score,
        fund_quartiles,
        persistence_correlation,
        average_alpha,
        return_consistency,
        recommendation,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn compute_quartile_rankings(
    funds: &[FundRecord],
    benchmarks: &[BenchmarkQuartile],
) -> Vec<FundQuartileRanking> {
    let mut rankings = Vec::new();
    for fund in funds {
        for bm in benchmarks {
            let value = match bm.metric.as_str() {
                "irr" => fund.irr,
                "tvpi" => fund.tvpi,
                "dpi" => fund.dpi,
                _ => continue,
            };
            let quartile = if value >= bm.q1 {
                1
            } else if value >= bm.median {
                2
            } else if value >= bm.q3 {
                3
            } else {
                4
            };
            rankings.push(FundQuartileRanking {
                fund: fund.name.clone(),
                metric: bm.metric.clone(),
                value,
                quartile,
            });
        }
    }
    rankings
}

fn compute_quantitative_score(rankings: &[FundQuartileRanking]) -> Decimal {
    if rankings.is_empty() {
        return Decimal::ZERO;
    }
    let total: Decimal = rankings
        .iter()
        .map(|r| match r.quartile {
            1 => dec!(100),
            2 => dec!(75),
            3 => dec!(50),
            _ => dec!(25),
        })
        .sum();
    total / Decimal::from(rankings.len() as u32)
}

fn compute_qualitative_score(factors: &[QualitativeFactor]) -> Decimal {
    if factors.is_empty() {
        return Decimal::ZERO;
    }
    let total_weight: Decimal = factors.iter().map(|f| f.weight).sum();
    if total_weight.is_zero() {
        return Decimal::ZERO;
    }
    // Weighted average score on 1-5 scale, converted to 0-100.
    let weighted_sum: Decimal = factors.iter().map(|f| f.weight * f.score).sum();
    let avg_score = weighted_sum / total_weight; // 1-5 scale
                                                 // Convert to 0-100: (score - 1) / 4 * 100
    (avg_score - Decimal::ONE) / dec!(4) * dec!(100)
}

/// Pearson correlation between consecutive fund IRRs.
fn compute_persistence(funds: &[FundRecord]) -> Decimal {
    if funds.len() < 3 {
        return Decimal::ZERO;
    }

    // Sort by vintage
    let mut sorted = funds.to_vec();
    sorted.sort_by_key(|f| f.vintage);

    // Pairs: (fund_n IRR, fund_n+1 IRR)
    let n = sorted.len() - 1;
    if n == 0 {
        return Decimal::ZERO;
    }

    let xs: Vec<Decimal> = sorted[..n].iter().map(|f| f.irr).collect();
    let ys: Vec<Decimal> = sorted[1..].iter().map(|f| f.irr).collect();

    pearson_correlation(&xs, &ys)
}

fn pearson_correlation(xs: &[Decimal], ys: &[Decimal]) -> Decimal {
    let n = xs.len();
    if n == 0 {
        return Decimal::ZERO;
    }
    let n_dec = Decimal::from(n as u32);
    let sum_x: Decimal = xs.iter().copied().sum();
    let sum_y: Decimal = ys.iter().copied().sum();
    let mean_x = sum_x / n_dec;
    let mean_y = sum_y / n_dec;

    let mut cov = Decimal::ZERO;
    let mut var_x = Decimal::ZERO;
    let mut var_y = Decimal::ZERO;

    for i in 0..n {
        let dx = xs[i] - mean_x;
        let dy = ys[i] - mean_y;
        cov += dx * dy;
        var_x += dx * dx;
        var_y += dy * dy;
    }

    let denom_sq = var_x * var_y;
    if denom_sq.is_zero() {
        return Decimal::ZERO;
    }
    // sqrt via Newton's method
    let denom = decimal_sqrt(denom_sq);
    if denom.is_zero() {
        return Decimal::ZERO;
    }

    cov / denom
}

/// Newton's method square root (20 iterations).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let two = dec!(2);
    let mut guess = x / two;
    if guess.is_zero() {
        guess = Decimal::ONE;
    }
    for _ in 0..20 {
        let next = (guess + x / guess) / two;
        if (next - guess).abs() < dec!(0.0000000001) {
            return next;
        }
        guess = next;
    }
    guess
}

fn compute_average_alpha(funds: &[FundRecord]) -> Decimal {
    if funds.is_empty() {
        return Decimal::ZERO;
    }
    let total: Decimal = funds.iter().map(|f| f.irr - f.pme).sum();
    total / Decimal::from(funds.len() as u32)
}

fn compute_return_consistency(funds: &[FundRecord]) -> Decimal {
    if funds.len() < 2 {
        return Decimal::ONE;
    }
    let n = Decimal::from(funds.len() as u32);
    let mean: Decimal = funds.iter().map(|f| f.irr).sum::<Decimal>() / n;
    if mean.is_zero() {
        return Decimal::ZERO;
    }
    let variance: Decimal = funds
        .iter()
        .map(|f| {
            let diff = f.irr - mean;
            diff * diff
        })
        .sum::<Decimal>()
        / n;
    let std_dev = decimal_sqrt(variance);
    let cv = std_dev / mean.abs();
    // Consistency = 1 - CV, clamped to [0, 1]
    let consistency = Decimal::ONE - cv;
    if consistency < Decimal::ZERO {
        Decimal::ZERO
    } else if consistency > Decimal::ONE {
        Decimal::ONE
    } else {
        consistency
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_manager_input(input: &ManagerSelectionInput) -> CorpFinanceResult<()> {
    if input.funds.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one fund record is required.".into(),
        ));
    }
    for fund in &input.funds {
        if fund.tvpi < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "funds.tvpi".into(),
                reason: "TVPI cannot be negative.".into(),
            });
        }
        if fund.dpi < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "funds.dpi".into(),
                reason: "DPI cannot be negative.".into(),
            });
        }
    }
    for factor in &input.qualitative_scores {
        if factor.score < Decimal::ONE || factor.score > dec!(5) {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("qualitative_scores.{}", factor.factor),
                reason: "Qualitative scores must be in [1, 5].".into(),
            });
        }
        if factor.weight < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("qualitative_scores.{}.weight", factor.factor),
                reason: "Weights cannot be negative.".into(),
            });
        }
    }
    if input.benchmark_quartiles.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one benchmark quartile definition is required.".into(),
        ));
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

    fn default_input() -> ManagerSelectionInput {
        ManagerSelectionInput {
            manager_name: "Test Capital Partners".into(),
            funds: vec![
                FundRecord {
                    name: "Fund I".into(),
                    vintage: 2012,
                    irr: dec!(0.18),
                    tvpi: dec!(2.1),
                    dpi: dec!(1.9),
                    pme: dec!(0.12),
                },
                FundRecord {
                    name: "Fund II".into(),
                    vintage: 2015,
                    irr: dec!(0.22),
                    tvpi: dec!(2.5),
                    dpi: dec!(2.0),
                    pme: dec!(0.14),
                },
                FundRecord {
                    name: "Fund III".into(),
                    vintage: 2018,
                    irr: dec!(0.15),
                    tvpi: dec!(1.8),
                    dpi: dec!(1.2),
                    pme: dec!(0.10),
                },
                FundRecord {
                    name: "Fund IV".into(),
                    vintage: 2021,
                    irr: dec!(0.20),
                    tvpi: dec!(1.6),
                    dpi: dec!(0.8),
                    pme: dec!(0.11),
                },
            ],
            qualitative_scores: vec![
                QualitativeFactor {
                    factor: "Team".into(),
                    weight: dec!(0.30),
                    score: dec!(4),
                },
                QualitativeFactor {
                    factor: "Strategy".into(),
                    weight: dec!(0.25),
                    score: dec!(4),
                },
                QualitativeFactor {
                    factor: "Track Record".into(),
                    weight: dec!(0.20),
                    score: dec!(5),
                },
                QualitativeFactor {
                    factor: "Operations".into(),
                    weight: dec!(0.15),
                    score: dec!(3),
                },
                QualitativeFactor {
                    factor: "Terms".into(),
                    weight: dec!(0.10),
                    score: dec!(3),
                },
            ],
            benchmark_quartiles: vec![
                BenchmarkQuartile {
                    metric: "irr".into(),
                    q1: dec!(0.20),
                    median: dec!(0.14),
                    q3: dec!(0.08),
                },
                BenchmarkQuartile {
                    metric: "tvpi".into(),
                    q1: dec!(2.0),
                    median: dec!(1.5),
                    q3: dec!(1.2),
                },
                BenchmarkQuartile {
                    metric: "dpi".into(),
                    q1: dec!(1.5),
                    median: dec!(1.0),
                    q3: dec!(0.5),
                },
            ],
        }
    }

    #[test]
    fn test_manager_basic_output() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        assert!(out.quantitative_score > Decimal::ZERO);
        assert!(out.qualitative_score > Decimal::ZERO);
        assert!(out.overall_score > Decimal::ZERO);
    }

    #[test]
    fn test_manager_quartile_rankings_count() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        // 4 funds * 3 metrics = 12 rankings
        assert_eq!(out.fund_quartiles.len(), 12);
    }

    #[test]
    fn test_manager_quartile_values_1_to_4() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        for r in &out.fund_quartiles {
            assert!(
                r.quartile >= 1 && r.quartile <= 4,
                "Invalid quartile {}",
                r.quartile
            );
        }
    }

    #[test]
    fn test_manager_top_quartile_for_strong_irr() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        // Fund II has IRR 0.22 > Q1 threshold 0.20 => quartile 1
        let fund2_irr = out
            .fund_quartiles
            .iter()
            .find(|r| r.fund == "Fund II" && r.metric == "irr")
            .unwrap();
        assert_eq!(fund2_irr.quartile, 1);
    }

    #[test]
    fn test_manager_quantitative_score_in_range() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        assert!(
            out.quantitative_score >= Decimal::ZERO && out.quantitative_score <= dec!(100),
            "Quant score {} out of range",
            out.quantitative_score
        );
    }

    #[test]
    fn test_manager_qualitative_score_in_range() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        assert!(
            out.qualitative_score >= Decimal::ZERO && out.qualitative_score <= dec!(100),
            "Qual score {} out of range",
            out.qualitative_score
        );
    }

    #[test]
    fn test_manager_overall_score_is_blend() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        let expected = dec!(0.60) * out.quantitative_score + dec!(0.40) * out.qualitative_score;
        assert!(
            approx_eq(out.overall_score, expected, dec!(0.01)),
            "Overall {} != 60/40 blend {}",
            out.overall_score,
            expected
        );
    }

    #[test]
    fn test_manager_average_alpha_positive() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        // All funds have IRR > PME, so average alpha should be positive
        assert!(out.average_alpha > Decimal::ZERO);
    }

    #[test]
    fn test_manager_average_alpha_calculation() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        // Manual: (0.18-0.12 + 0.22-0.14 + 0.15-0.10 + 0.20-0.11) / 4
        //       = (0.06 + 0.08 + 0.05 + 0.09) / 4 = 0.28/4 = 0.07
        assert!(
            approx_eq(out.average_alpha, dec!(0.07), dec!(0.001)),
            "Average alpha {} should be ~0.07",
            out.average_alpha
        );
    }

    #[test]
    fn test_manager_persistence_with_enough_funds() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        // With 4 funds, persistence should be computable (non-zero possible)
        // The actual value depends on the pattern
        assert!(
            out.persistence_correlation >= dec!(-1) && out.persistence_correlation <= Decimal::ONE,
            "Persistence {} out of [-1, 1]",
            out.persistence_correlation
        );
    }

    #[test]
    fn test_manager_return_consistency_in_range() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        assert!(
            out.return_consistency >= Decimal::ZERO && out.return_consistency <= Decimal::ONE,
            "Consistency {} out of [0, 1]",
            out.return_consistency
        );
    }

    #[test]
    fn test_manager_recommendation_strong_buy() {
        // Create a strong manager: all top quartile + high qualitative
        let mut input = default_input();
        input.funds = vec![FundRecord {
            name: "Fund I".into(),
            vintage: 2018,
            irr: dec!(0.30),
            tvpi: dec!(3.0),
            dpi: dec!(2.5),
            pme: dec!(0.10),
        }];
        input.qualitative_scores = vec![QualitativeFactor {
            factor: "Team".into(),
            weight: dec!(1.0),
            score: dec!(5),
        }];
        let out = analyze_manager_selection(&input).unwrap();
        assert_eq!(out.recommendation, Recommendation::StrongBuy);
    }

    #[test]
    fn test_manager_recommendation_pass() {
        // Create a weak manager: all bottom quartile + low qualitative
        let mut input = default_input();
        input.funds = vec![FundRecord {
            name: "Fund I".into(),
            vintage: 2018,
            irr: dec!(0.02),
            tvpi: dec!(0.9),
            dpi: dec!(0.3),
            pme: dec!(0.10),
        }];
        input.qualitative_scores = vec![QualitativeFactor {
            factor: "Team".into(),
            weight: dec!(1.0),
            score: dec!(1),
        }];
        let out = analyze_manager_selection(&input).unwrap();
        assert_eq!(out.recommendation, Recommendation::Pass);
    }

    #[test]
    fn test_manager_single_fund_persistence_zero() {
        let mut input = default_input();
        input.funds = vec![input.funds[0].clone()];
        let out = analyze_manager_selection(&input).unwrap();
        // With only 1 fund, persistence correlation should be zero
        assert_eq!(out.persistence_correlation, Decimal::ZERO);
    }

    // -- Validation tests --

    #[test]
    fn test_reject_empty_funds() {
        let mut input = default_input();
        input.funds = vec![];
        assert!(analyze_manager_selection(&input).is_err());
    }

    #[test]
    fn test_reject_negative_tvpi() {
        let mut input = default_input();
        input.funds[0].tvpi = dec!(-1);
        assert!(analyze_manager_selection(&input).is_err());
    }

    #[test]
    fn test_reject_score_out_of_range() {
        let mut input = default_input();
        input.qualitative_scores[0].score = dec!(6);
        assert!(analyze_manager_selection(&input).is_err());
    }

    #[test]
    fn test_reject_score_below_one() {
        let mut input = default_input();
        input.qualitative_scores[0].score = Decimal::ZERO;
        assert!(analyze_manager_selection(&input).is_err());
    }

    #[test]
    fn test_reject_empty_benchmarks() {
        let mut input = default_input();
        input.benchmark_quartiles = vec![];
        assert!(analyze_manager_selection(&input).is_err());
    }

    #[test]
    fn test_reject_negative_weight() {
        let mut input = default_input();
        input.qualitative_scores[0].weight = dec!(-0.1);
        assert!(analyze_manager_selection(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let out = analyze_manager_selection(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: ManagerSelectionOutput = serde_json::from_str(&json).unwrap();
    }
}
