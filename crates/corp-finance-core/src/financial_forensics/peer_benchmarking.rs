//! Peer benchmarking with percentile ranking.
//!
//! Compares a company's financial metrics against a set of peer companies,
//! calculating percentile ranks, z-scores, and composite ratings.
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

/// Newton square root for Decimal.
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess == Decimal::ZERO {
        guess = Decimal::ONE;
    }
    for _ in 0..30 {
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// A single named metric value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub metric_name: String,
    pub value: Decimal,
}

/// A company with its metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyMetrics {
    pub name: String,
    pub metrics: Vec<MetricValue>,
}

/// Input for peer benchmarking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkingInput {
    pub company: CompanyMetrics,
    pub peers: Vec<CompanyMetrics>,
    pub higher_is_better: Vec<String>,
    pub lower_is_better: Vec<String>,
}

/// Ranking result for a single metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricRanking {
    pub metric_name: String,
    pub company_value: Decimal,
    pub peer_median: Decimal,
    pub peer_mean: Decimal,
    pub peer_min: Decimal,
    pub peer_max: Decimal,
    pub percentile: Decimal,
    pub z_score: Decimal,
    pub relative_position: String,
}

/// Output of peer benchmarking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkingOutput {
    pub metric_rankings: Vec<MetricRanking>,
    pub composite_percentile: Decimal,
    pub composite_rating: String,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Perform peer benchmarking with percentile ranking.
pub fn calculate_peer_benchmarking(
    input: &PeerBenchmarkingInput,
) -> CorpFinanceResult<PeerBenchmarkingOutput> {
    // Validation
    if input.peers.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one peer company is required.".into(),
        ));
    }
    if input.company.metrics.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Company must have at least one metric.".into(),
        ));
    }

    let mut rankings = Vec::new();
    let mut percentile_sum = Decimal::ZERO;
    let mut percentile_count = 0u32;
    let mut strengths = Vec::new();
    let mut weaknesses = Vec::new();

    for metric in &input.company.metrics {
        let name = &metric.metric_name;
        let company_val = metric.value;

        // Collect peer values for this metric
        let peer_values: Vec<Decimal> = input
            .peers
            .iter()
            .filter_map(|p| {
                p.metrics
                    .iter()
                    .find(|m| m.metric_name == *name)
                    .map(|m| m.value)
            })
            .collect();

        if peer_values.is_empty() {
            continue;
        }

        // Statistics
        let n = peer_values.len();
        let peer_mean = peer_values.iter().copied().sum::<Decimal>() / Decimal::from(n as u64);
        let peer_min = peer_values.iter().copied().min().unwrap_or(Decimal::ZERO);
        let peer_max = peer_values.iter().copied().max().unwrap_or(Decimal::ZERO);

        // Median
        let mut sorted = peer_values.clone();
        sorted.sort();
        let peer_median = if n % 2 == 1 {
            sorted[n / 2]
        } else {
            (sorted[n / 2 - 1] + sorted[n / 2]) / dec!(2)
        };

        // Standard deviation
        let variance = peer_values
            .iter()
            .map(|&v| (v - peer_mean) * (v - peer_mean))
            .sum::<Decimal>()
            / Decimal::from(n as u64);
        let std_dev = decimal_sqrt(variance);

        // Z-score
        let z_score = if std_dev == Decimal::ZERO {
            Decimal::ZERO
        } else {
            (company_val - peer_mean) / std_dev
        };

        // Percentile: rank / (N+1) * 100
        // Combine company + peers, sort, find rank
        let mut all_values: Vec<Decimal> = peer_values.clone();
        all_values.push(company_val);
        all_values.sort();
        let total = all_values.len();

        // Find rank (1-based, average for ties)
        let mut rank_sum = Decimal::ZERO;
        let mut rank_count = 0u32;
        for (i, &v) in all_values.iter().enumerate() {
            if v == company_val {
                rank_sum += Decimal::from((i + 1) as u64);
                rank_count += 1;
            }
        }
        let avg_rank = if rank_count > 0 {
            rank_sum / Decimal::from(rank_count)
        } else {
            Decimal::ONE
        };

        let raw_percentile = avg_rank / Decimal::from((total + 1) as u64) * dec!(100);

        // Determine if this metric is lower_is_better
        let is_lower_better = input.lower_is_better.contains(name);
        let _is_higher_better = input.higher_is_better.contains(name);

        let percentile = if is_lower_better {
            dec!(100) - raw_percentile
        } else {
            raw_percentile
        };

        // Relative position
        let relative_position = if percentile >= dec!(75) {
            "Top Quartile"
        } else if percentile >= dec!(50) {
            "Above Median"
        } else if percentile >= dec!(25) {
            "Below Median"
        } else {
            "Bottom Quartile"
        }
        .to_string();

        // Strengths and weaknesses
        if percentile >= dec!(75) {
            strengths.push(name.clone());
        } else if percentile < dec!(25) {
            weaknesses.push(name.clone());
        }

        percentile_sum += percentile;
        percentile_count += 1;

        // Adjust z_score sign for lower_is_better
        let display_z = if is_lower_better { -z_score } else { z_score };

        rankings.push(MetricRanking {
            metric_name: name.clone(),
            company_value: company_val,
            peer_median,
            peer_mean,
            peer_min,
            peer_max,
            percentile,
            z_score: display_z,
            relative_position,
        });
    }

    if percentile_count == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "No matching metrics found between company and peers.".into(),
        ));
    }

    let composite_percentile = percentile_sum / Decimal::from(percentile_count);
    let composite_rating = if composite_percentile >= dec!(80) {
        "Leader"
    } else if composite_percentile >= dec!(60) {
        "Above Average"
    } else if composite_percentile >= dec!(40) {
        "Average"
    } else if composite_percentile >= dec!(20) {
        "Below Average"
    } else {
        "Laggard"
    }
    .to_string();

    Ok(PeerBenchmarkingOutput {
        metric_rankings: rankings,
        composite_percentile,
        composite_rating,
        strengths,
        weaknesses,
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

    fn make_metric(name: &str, value: Decimal) -> MetricValue {
        MetricValue {
            metric_name: name.to_string(),
            value,
        }
    }

    fn make_company(name: &str, metrics: Vec<(&str, Decimal)>) -> CompanyMetrics {
        CompanyMetrics {
            name: name.to_string(),
            metrics: metrics
                .into_iter()
                .map(|(n, v)| make_metric(n, v))
                .collect(),
        }
    }

    fn leader_input() -> PeerBenchmarkingInput {
        PeerBenchmarkingInput {
            company: make_company("Target", vec![("roe", dec!(0.25)), ("pe_ratio", dec!(10))]),
            peers: vec![
                make_company("Peer1", vec![("roe", dec!(0.10)), ("pe_ratio", dec!(20))]),
                make_company("Peer2", vec![("roe", dec!(0.12)), ("pe_ratio", dec!(25))]),
                make_company("Peer3", vec![("roe", dec!(0.08)), ("pe_ratio", dec!(30))]),
                make_company("Peer4", vec![("roe", dec!(0.15)), ("pe_ratio", dec!(22))]),
            ],
            higher_is_better: vec!["roe".to_string()],
            lower_is_better: vec!["pe_ratio".to_string()],
        }
    }

    fn laggard_input() -> PeerBenchmarkingInput {
        PeerBenchmarkingInput {
            company: make_company("Target", vec![("roe", dec!(0.02)), ("pe_ratio", dec!(50))]),
            peers: vec![
                make_company("Peer1", vec![("roe", dec!(0.10)), ("pe_ratio", dec!(15))]),
                make_company("Peer2", vec![("roe", dec!(0.15)), ("pe_ratio", dec!(12))]),
                make_company("Peer3", vec![("roe", dec!(0.20)), ("pe_ratio", dec!(18))]),
                make_company("Peer4", vec![("roe", dec!(0.12)), ("pe_ratio", dec!(14))]),
            ],
            higher_is_better: vec!["roe".to_string()],
            lower_is_better: vec!["pe_ratio".to_string()],
        }
    }

    #[test]
    fn test_leader_high_composite() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        assert!(
            out.composite_percentile >= dec!(75),
            "got {}",
            out.composite_percentile
        );
    }

    #[test]
    fn test_leader_rating() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        assert!(
            out.composite_rating == "Leader" || out.composite_rating == "Above Average",
            "got {}",
            out.composite_rating
        );
    }

    #[test]
    fn test_laggard_low_composite() {
        let out = calculate_peer_benchmarking(&laggard_input()).unwrap();
        assert!(
            out.composite_percentile < dec!(30),
            "got {}",
            out.composite_percentile
        );
    }

    #[test]
    fn test_laggard_has_weaknesses() {
        let out = calculate_peer_benchmarking(&laggard_input()).unwrap();
        assert!(!out.weaknesses.is_empty());
    }

    #[test]
    fn test_leader_has_strengths() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        assert!(!out.strengths.is_empty());
    }

    #[test]
    fn test_metric_ranking_count() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        assert_eq!(out.metric_rankings.len(), 2);
    }

    #[test]
    fn test_peer_median_calculation() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        let roe_rank = out
            .metric_rankings
            .iter()
            .find(|r| r.metric_name == "roe")
            .unwrap();
        // Peers: 0.08, 0.10, 0.12, 0.15 -> median = (0.10+0.12)/2 = 0.11
        assert!(approx_eq(roe_rank.peer_median, dec!(0.11), dec!(0.001)));
    }

    #[test]
    fn test_peer_mean_calculation() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        let roe_rank = out
            .metric_rankings
            .iter()
            .find(|r| r.metric_name == "roe")
            .unwrap();
        // Peers: (0.10+0.12+0.08+0.15)/4 = 0.1125
        assert!(approx_eq(roe_rank.peer_mean, dec!(0.1125), dec!(0.001)));
    }

    #[test]
    fn test_peer_min_max() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        let roe_rank = out
            .metric_rankings
            .iter()
            .find(|r| r.metric_name == "roe")
            .unwrap();
        assert_eq!(roe_rank.peer_min, dec!(0.08));
        assert_eq!(roe_rank.peer_max, dec!(0.15));
    }

    #[test]
    fn test_z_score_positive_for_above_mean() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        let roe_rank = out
            .metric_rankings
            .iter()
            .find(|r| r.metric_name == "roe")
            .unwrap();
        assert!(roe_rank.z_score > Decimal::ZERO);
    }

    #[test]
    fn test_z_score_negative_for_below_mean() {
        let out = calculate_peer_benchmarking(&laggard_input()).unwrap();
        let roe_rank = out
            .metric_rankings
            .iter()
            .find(|r| r.metric_name == "roe")
            .unwrap();
        assert!(roe_rank.z_score < Decimal::ZERO);
    }

    #[test]
    fn test_lower_is_better_inverts_percentile() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        let pe_rank = out
            .metric_rankings
            .iter()
            .find(|r| r.metric_name == "pe_ratio")
            .unwrap();
        // Company PE=10, peers are 20,25,30,22 — company is lowest (best)
        assert!(pe_rank.percentile > dec!(50));
    }

    #[test]
    fn test_relative_position_top_quartile() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        let roe_rank = out
            .metric_rankings
            .iter()
            .find(|r| r.metric_name == "roe")
            .unwrap();
        assert_eq!(roe_rank.relative_position, "Top Quartile");
    }

    #[test]
    fn test_relative_position_bottom_quartile() {
        let out = calculate_peer_benchmarking(&laggard_input()).unwrap();
        let roe_rank = out
            .metric_rankings
            .iter()
            .find(|r| r.metric_name == "roe")
            .unwrap();
        assert_eq!(roe_rank.relative_position, "Bottom Quartile");
    }

    #[test]
    fn test_single_peer() {
        let input = PeerBenchmarkingInput {
            company: make_company("Target", vec![("roe", dec!(0.15))]),
            peers: vec![make_company("Peer1", vec![("roe", dec!(0.10))])],
            higher_is_better: vec!["roe".to_string()],
            lower_is_better: vec![],
        };
        let out = calculate_peer_benchmarking(&input).unwrap();
        assert_eq!(out.metric_rankings.len(), 1);
    }

    #[test]
    fn test_single_metric() {
        let input = PeerBenchmarkingInput {
            company: make_company("Target", vec![("margin", dec!(0.20))]),
            peers: vec![
                make_company("P1", vec![("margin", dec!(0.10))]),
                make_company("P2", vec![("margin", dec!(0.30))]),
            ],
            higher_is_better: vec!["margin".to_string()],
            lower_is_better: vec![],
        };
        let out = calculate_peer_benchmarking(&input).unwrap();
        assert_eq!(out.metric_rankings.len(), 1);
        let rank = &out.metric_rankings[0];
        assert_eq!(rank.company_value, dec!(0.20));
    }

    #[test]
    fn test_empty_peers_error() {
        let input = PeerBenchmarkingInput {
            company: make_company("Target", vec![("roe", dec!(0.15))]),
            peers: vec![],
            higher_is_better: vec![],
            lower_is_better: vec![],
        };
        assert!(calculate_peer_benchmarking(&input).is_err());
    }

    #[test]
    fn test_empty_company_metrics_error() {
        let input = PeerBenchmarkingInput {
            company: CompanyMetrics {
                name: "Target".to_string(),
                metrics: vec![],
            },
            peers: vec![make_company("P1", vec![("roe", dec!(0.10))])],
            higher_is_better: vec![],
            lower_is_better: vec![],
        };
        assert!(calculate_peer_benchmarking(&input).is_err());
    }

    #[test]
    fn test_percentile_between_0_and_100() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        for rank in &out.metric_rankings {
            assert!(rank.percentile >= Decimal::ZERO && rank.percentile <= dec!(100));
        }
    }

    #[test]
    fn test_composite_percentile_between_0_and_100() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        assert!(out.composite_percentile >= Decimal::ZERO);
        assert!(out.composite_percentile <= dec!(100));
    }

    #[test]
    fn test_median_company_near_50_percentile() {
        let input = PeerBenchmarkingInput {
            company: make_company("Target", vec![("roe", dec!(0.12))]),
            peers: vec![
                make_company("P1", vec![("roe", dec!(0.08))]),
                make_company("P2", vec![("roe", dec!(0.10))]),
                make_company("P3", vec![("roe", dec!(0.14))]),
                make_company("P4", vec![("roe", dec!(0.16))]),
            ],
            higher_is_better: vec!["roe".to_string()],
            lower_is_better: vec![],
        };
        let out = calculate_peer_benchmarking(&input).unwrap();
        let roe_rank = &out.metric_rankings[0];
        assert!(
            roe_rank.percentile > dec!(30) && roe_rank.percentile < dec!(70),
            "percentile={}",
            roe_rank.percentile
        );
    }

    #[test]
    fn test_serialization_roundtrip() {
        let out = calculate_peer_benchmarking(&leader_input()).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _deser: PeerBenchmarkingOutput = serde_json::from_str(&json).unwrap();
    }
}
