use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::{CorpFinanceError, CorpFinanceResult, types::*};
use super::metrics::CreditMetricsOutput;

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantTestInput {
    pub covenants: Vec<Covenant>,
    pub actuals: CreditMetricsOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Covenant {
    pub name: String,
    pub metric: CovenantMetric,
    pub threshold: Decimal,
    pub direction: CovenantDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CovenantMetric {
    NetDebtToEbitda,
    InterestCoverage,
    Dscr,
    DebtToEquity,
    MinCash,
    MaxCapex,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CovenantDirection {
    /// Actual must not exceed threshold.
    MaxOf,
    /// Actual must not fall below threshold.
    MinOf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantTestOutput {
    pub results: Vec<CovenantResult>,
    pub all_passing: bool,
    pub headroom_summary: Vec<CovenantHeadroom>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantResult {
    pub covenant: String,
    pub threshold: Decimal,
    pub actual: Decimal,
    pub passing: bool,
    pub headroom: Decimal,
    pub headroom_pct: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantHeadroom {
    pub covenant: String,
    pub headroom: Decimal,
    pub headroom_pct: Rate,
    pub passing: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Test a set of financial covenants against actual credit metrics and compute
/// headroom for each.
pub fn test_covenants(
    input: &CovenantTestInput,
) -> CorpFinanceResult<ComputationOutput<CovenantTestOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    if input.covenants.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one covenant must be provided.".into(),
        ));
    }

    let mut results: Vec<CovenantResult> = Vec::with_capacity(input.covenants.len());

    for cov in &input.covenants {
        let actual = match extract_metric(&input.actuals, &cov.metric) {
            Some(v) => v,
            None => {
                warnings.push(format!(
                    "Covenant '{}': metric {:?} not available in actuals; skipped.",
                    cov.name, cov.metric
                ));
                continue;
            }
        };

        let (passing, headroom) = match cov.direction {
            CovenantDirection::MaxOf => {
                let pass = actual <= cov.threshold;
                let hr = cov.threshold - actual;
                (pass, hr)
            }
            CovenantDirection::MinOf => {
                let pass = actual >= cov.threshold;
                let hr = actual - cov.threshold;
                (pass, hr)
            }
        };

        let headroom_pct = if cov.threshold.is_zero() {
            Decimal::ZERO
        } else {
            headroom / cov.threshold
        };

        results.push(CovenantResult {
            covenant: cov.name.clone(),
            threshold: cov.threshold,
            actual,
            passing,
            headroom,
            headroom_pct,
        });
    }

    let all_passing = results.iter().all(|r| r.passing);

    let headroom_summary: Vec<CovenantHeadroom> = results
        .iter()
        .map(|r| CovenantHeadroom {
            covenant: r.covenant.clone(),
            headroom: r.headroom,
            headroom_pct: r.headroom_pct,
            passing: r.passing,
        })
        .collect();

    let output = CovenantTestOutput {
        results,
        all_passing,
        headroom_summary,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "covenant_count": input.covenants.len(),
    });

    Ok(with_metadata(
        "Covenant Compliance Testing",
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Map a CovenantMetric enum variant to the corresponding value from
/// CreditMetricsOutput. Returns None for Custom metrics or metrics that
/// are not directly available.
fn extract_metric(actuals: &CreditMetricsOutput, metric: &CovenantMetric) -> Option<Decimal> {
    match metric {
        CovenantMetric::NetDebtToEbitda => Some(actuals.net_debt_to_ebitda),
        CovenantMetric::InterestCoverage => Some(actuals.interest_coverage),
        CovenantMetric::Dscr => Some(actuals.dscr),
        CovenantMetric::DebtToEquity => Some(actuals.debt_to_equity),
        CovenantMetric::MinCash => Some(actuals.cash_to_debt), // proxy: cash/debt ratio
        CovenantMetric::MaxCapex => Some(actuals.fcf), // proxy: FCF as capex indicator
        CovenantMetric::Custom(_) => None, // Custom metrics require external resolution
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::metrics::CreditRating;
    use rust_decimal_macros::dec;

    /// Build a sample CreditMetricsOutput for testing.
    fn sample_actuals() -> CreditMetricsOutput {
        CreditMetricsOutput {
            net_debt: dec!(420_000),
            net_debt_to_ebitda: dec!(2.1),
            total_debt_to_ebitda: dec!(2.5),
            debt_to_equity: dec!(1.25),
            debt_to_assets: dec!(0.4167),
            net_debt_to_ev: Some(dec!(0.3443)),
            interest_coverage: dec!(8),
            ebit_coverage: dec!(6),
            fixed_charge_coverage: Some(dec!(6)),
            dscr: dec!(5.6),
            ffo_to_debt: Some(dec!(0.34)),
            ocf_to_debt: dec!(0.36),
            fcf_to_debt: dec!(0.24),
            fcf: dec!(120_000),
            cash_conversion: dec!(0.9),
            current_ratio: dec!(2),
            quick_ratio: dec!(0.5333),
            cash_to_debt: dec!(0.16),
            implied_rating: CreditRating::AAA,
            rating_rationale: vec![],
        }
    }

    #[test]
    fn test_max_of_covenant_passing() {
        let input = CovenantTestInput {
            covenants: vec![Covenant {
                name: "Max Net Debt / EBITDA".into(),
                metric: CovenantMetric::NetDebtToEbitda,
                threshold: dec!(3.5),
                direction: CovenantDirection::MaxOf,
            }],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        let r = &result.result.results[0];
        assert!(r.passing);
        assert_eq!(r.actual, dec!(2.1));
        // headroom = 3.5 - 2.1 = 1.4
        assert_eq!(r.headroom, dec!(1.4));
        assert!(result.result.all_passing);
    }

    #[test]
    fn test_max_of_covenant_failing() {
        let input = CovenantTestInput {
            covenants: vec![Covenant {
                name: "Max Net Debt / EBITDA".into(),
                metric: CovenantMetric::NetDebtToEbitda,
                threshold: dec!(2.0),
                direction: CovenantDirection::MaxOf,
            }],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        let r = &result.result.results[0];
        assert!(!r.passing);
        // headroom = 2.0 - 2.1 = -0.1
        assert_eq!(r.headroom, dec!(-0.1));
        assert!(!result.result.all_passing);
    }

    #[test]
    fn test_min_of_covenant_passing() {
        let input = CovenantTestInput {
            covenants: vec![Covenant {
                name: "Min Interest Coverage".into(),
                metric: CovenantMetric::InterestCoverage,
                threshold: dec!(3.0),
                direction: CovenantDirection::MinOf,
            }],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        let r = &result.result.results[0];
        assert!(r.passing);
        // headroom = 8.0 - 3.0 = 5.0
        assert_eq!(r.headroom, dec!(5));
    }

    #[test]
    fn test_min_of_covenant_failing() {
        let input = CovenantTestInput {
            covenants: vec![Covenant {
                name: "Min Interest Coverage".into(),
                metric: CovenantMetric::InterestCoverage,
                threshold: dec!(10.0),
                direction: CovenantDirection::MinOf,
            }],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        let r = &result.result.results[0];
        assert!(!r.passing);
        // headroom = 8.0 - 10.0 = -2.0
        assert_eq!(r.headroom, dec!(-2));
    }

    #[test]
    fn test_multiple_covenants_mixed() {
        let input = CovenantTestInput {
            covenants: vec![
                Covenant {
                    name: "Max Net Debt / EBITDA".into(),
                    metric: CovenantMetric::NetDebtToEbitda,
                    threshold: dec!(3.5),
                    direction: CovenantDirection::MaxOf,
                },
                Covenant {
                    name: "Min Interest Coverage".into(),
                    metric: CovenantMetric::InterestCoverage,
                    threshold: dec!(10.0), // will fail
                    direction: CovenantDirection::MinOf,
                },
                Covenant {
                    name: "Min DSCR".into(),
                    metric: CovenantMetric::Dscr,
                    threshold: dec!(2.0),
                    direction: CovenantDirection::MinOf,
                },
            ],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        assert!(!result.result.all_passing);
        assert_eq!(result.result.results.len(), 3);
        assert!(result.result.results[0].passing); // leverage OK
        assert!(!result.result.results[1].passing); // coverage fails
        assert!(result.result.results[2].passing); // DSCR OK
    }

    #[test]
    fn test_headroom_pct_calculation() {
        let input = CovenantTestInput {
            covenants: vec![Covenant {
                name: "Max Net Debt / EBITDA".into(),
                metric: CovenantMetric::NetDebtToEbitda,
                threshold: dec!(3.5),
                direction: CovenantDirection::MaxOf,
            }],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        let r = &result.result.results[0];
        // headroom_pct = 1.4 / 3.5 = 0.4
        assert_eq!(r.headroom_pct, dec!(0.4));
    }

    #[test]
    fn test_headroom_summary_matches_results() {
        let input = CovenantTestInput {
            covenants: vec![
                Covenant {
                    name: "Lev".into(),
                    metric: CovenantMetric::NetDebtToEbitda,
                    threshold: dec!(3.5),
                    direction: CovenantDirection::MaxOf,
                },
                Covenant {
                    name: "Cov".into(),
                    metric: CovenantMetric::InterestCoverage,
                    threshold: dec!(3.0),
                    direction: CovenantDirection::MinOf,
                },
            ],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        assert_eq!(
            result.result.headroom_summary.len(),
            result.result.results.len()
        );
        for (summary, detail) in result
            .result
            .headroom_summary
            .iter()
            .zip(result.result.results.iter())
        {
            assert_eq!(summary.covenant, detail.covenant);
            assert_eq!(summary.headroom, detail.headroom);
            assert_eq!(summary.passing, detail.passing);
        }
    }

    #[test]
    fn test_custom_metric_skipped_with_warning() {
        let input = CovenantTestInput {
            covenants: vec![
                Covenant {
                    name: "Custom Metric".into(),
                    metric: CovenantMetric::Custom("ebitda_margin".into()),
                    threshold: dec!(0.20),
                    direction: CovenantDirection::MinOf,
                },
                Covenant {
                    name: "Min DSCR".into(),
                    metric: CovenantMetric::Dscr,
                    threshold: dec!(2.0),
                    direction: CovenantDirection::MinOf,
                },
            ],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        // Custom metric is skipped, only 1 result
        assert_eq!(result.result.results.len(), 1);
        assert!(result.warnings.iter().any(|w| w.contains("Custom")));
    }

    #[test]
    fn test_empty_covenants_rejected() {
        let input = CovenantTestInput {
            covenants: vec![],
            actuals: sample_actuals(),
        };
        let err = test_covenants(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {} // expected
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }

    #[test]
    fn test_debt_to_equity_covenant() {
        let input = CovenantTestInput {
            covenants: vec![Covenant {
                name: "Max D/E".into(),
                metric: CovenantMetric::DebtToEquity,
                threshold: dec!(2.0),
                direction: CovenantDirection::MaxOf,
            }],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        let r = &result.result.results[0];
        assert!(r.passing); // 1.25 <= 2.0
        assert_eq!(r.actual, dec!(1.25));
        assert_eq!(r.headroom, dec!(0.75));
    }

    #[test]
    fn test_all_passing_true_when_all_pass() {
        let input = CovenantTestInput {
            covenants: vec![
                Covenant {
                    name: "Max Lev".into(),
                    metric: CovenantMetric::NetDebtToEbitda,
                    threshold: dec!(5.0),
                    direction: CovenantDirection::MaxOf,
                },
                Covenant {
                    name: "Min Cov".into(),
                    metric: CovenantMetric::InterestCoverage,
                    threshold: dec!(2.0),
                    direction: CovenantDirection::MinOf,
                },
            ],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        assert!(result.result.all_passing);
    }

    #[test]
    fn test_metadata_populated() {
        let input = CovenantTestInput {
            covenants: vec![Covenant {
                name: "Test".into(),
                metric: CovenantMetric::Dscr,
                threshold: dec!(1.0),
                direction: CovenantDirection::MinOf,
            }],
            actuals: sample_actuals(),
        };
        let result = test_covenants(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
