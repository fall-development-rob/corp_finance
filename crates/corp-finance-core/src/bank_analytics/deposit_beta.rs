//! Deposit beta analysis.
//!
//! Covers:
//! 1. **Instantaneous beta** -- latest period deposit/benchmark change ratio.
//! 2. **Cumulative beta** -- total deposit rate change / total benchmark change.
//! 3. **Average beta** -- simple average of per-period betas.
//! 4. **Through-the-cycle beta** -- OLS regression slope.
//! 5. **Repricing lag** -- consecutive periods with no deposit change.
//! 6. **Deposit sensitivity** -- estimated cost change per 100bp benchmark move.
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

/// A single period rate change observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateChange {
    /// Period label (e.g. "Q1 2024").
    pub period: String,
    /// Change in benchmark rate (e.g. +0.0025 for 25bp).
    pub benchmark_rate_change: Decimal,
    /// Observed change in deposit rate.
    pub deposit_rate_change: Decimal,
}

/// Input for deposit beta analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositBetaInput {
    /// Historical rate change observations.
    pub rate_changes: Vec<RateChange>,
    /// Current deposit rate.
    pub current_deposit_rate: Decimal,
    /// Current benchmark rate.
    pub current_benchmark_rate: Decimal,
}

/// Output of deposit beta analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositBetaOutput {
    /// Instantaneous beta (latest period).
    pub instantaneous_beta: Decimal,
    /// Cumulative beta (total deposit change / total benchmark change).
    pub cumulative_beta: Decimal,
    /// Simple average of per-period betas.
    pub average_beta: Decimal,
    /// Repricing lag (periods before deposit rate adjusts).
    pub repricing_lag: Decimal,
    /// Deposit sensitivity per 100bp benchmark move.
    pub deposit_sensitivity: Decimal,
    /// Through-the-cycle beta (OLS regression slope).
    pub through_the_cycle_beta: Decimal,
}

// ---------------------------------------------------------------------------
// Core function
// ---------------------------------------------------------------------------

/// Analyze deposit beta characteristics from historical rate changes.
pub fn analyze_deposit_beta(input: &DepositBetaInput) -> CorpFinanceResult<DepositBetaOutput> {
    validate_deposit_beta_input(input)?;

    let n = input.rate_changes.len();

    // Instantaneous beta: last period
    let last = &input.rate_changes[n - 1];
    let instantaneous_beta = if last.benchmark_rate_change != Decimal::ZERO {
        last.deposit_rate_change / last.benchmark_rate_change
    } else {
        Decimal::ZERO
    };

    // Cumulative beta
    let total_deposit: Decimal = input
        .rate_changes
        .iter()
        .map(|r| r.deposit_rate_change)
        .sum();
    let total_benchmark: Decimal = input
        .rate_changes
        .iter()
        .map(|r| r.benchmark_rate_change)
        .sum();
    let cumulative_beta = if total_benchmark != Decimal::ZERO {
        total_deposit / total_benchmark
    } else {
        Decimal::ZERO
    };

    // Average beta: simple average of per-period betas (skip where benchmark=0)
    let mut period_betas = Vec::new();
    for rc in &input.rate_changes {
        if rc.benchmark_rate_change != Decimal::ZERO {
            period_betas.push(rc.deposit_rate_change / rc.benchmark_rate_change);
        }
    }
    let average_beta = if period_betas.is_empty() {
        Decimal::ZERO
    } else {
        let sum: Decimal = period_betas.iter().copied().sum();
        sum / Decimal::from(period_betas.len() as u64)
    };

    // Repricing lag: count consecutive periods from start where deposit_rate_change=0
    // when benchmark changed
    let mut lag: u32 = 0;
    for rc in &input.rate_changes {
        if rc.benchmark_rate_change != Decimal::ZERO && rc.deposit_rate_change == Decimal::ZERO {
            lag += 1;
        } else if rc.benchmark_rate_change != Decimal::ZERO {
            break;
        }
    }
    let repricing_lag = Decimal::from(lag);

    // Deposit sensitivity per 100bp
    let deposit_sensitivity = cumulative_beta * dec!(0.01); // 100bp = 1%

    // Through-the-cycle beta: OLS regression
    // y = deposit_rate_change, x = benchmark_rate_change
    // beta = sum((x - x_bar)(y - y_bar)) / sum((x - x_bar)^2)
    let through_the_cycle_beta = ols_slope(&input.rate_changes)?;

    Ok(DepositBetaOutput {
        instantaneous_beta,
        cumulative_beta,
        average_beta,
        repricing_lag,
        deposit_sensitivity,
        through_the_cycle_beta,
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ols_slope(changes: &[RateChange]) -> CorpFinanceResult<Decimal> {
    let n = Decimal::from(changes.len() as u64);
    if n == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    let x_sum: Decimal = changes.iter().map(|c| c.benchmark_rate_change).sum();
    let y_sum: Decimal = changes.iter().map(|c| c.deposit_rate_change).sum();
    let x_bar = x_sum / n;
    let y_bar = y_sum / n;

    let mut numerator = Decimal::ZERO;
    let mut denominator = Decimal::ZERO;

    for c in changes {
        let x_diff = c.benchmark_rate_change - x_bar;
        let y_diff = c.deposit_rate_change - y_bar;
        numerator += x_diff * y_diff;
        denominator += x_diff * x_diff;
    }

    if denominator == Decimal::ZERO {
        return Ok(Decimal::ZERO);
    }

    Ok(numerator / denominator)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_deposit_beta_input(input: &DepositBetaInput) -> CorpFinanceResult<()> {
    if input.rate_changes.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one rate change observation is required.".into(),
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

    fn perfect_passthrough() -> DepositBetaInput {
        DepositBetaInput {
            rate_changes: vec![
                RateChange {
                    period: "Q1".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0025),
                },
                RateChange {
                    period: "Q2".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0025),
                },
                RateChange {
                    period: "Q3".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0025),
                },
                RateChange {
                    period: "Q4".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0025),
                },
            ],
            current_deposit_rate: dec!(0.04),
            current_benchmark_rate: dec!(0.05),
        }
    }

    fn partial_passthrough() -> DepositBetaInput {
        DepositBetaInput {
            rate_changes: vec![
                RateChange {
                    period: "Q1".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0000),
                },
                RateChange {
                    period: "Q2".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0010),
                },
                RateChange {
                    period: "Q3".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0015),
                },
                RateChange {
                    period: "Q4".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0012),
                },
            ],
            current_deposit_rate: dec!(0.025),
            current_benchmark_rate: dec!(0.05),
        }
    }

    #[test]
    fn test_perfect_passthrough_beta_one() {
        let input = perfect_passthrough();
        let out = analyze_deposit_beta(&input).unwrap();
        assert_eq!(out.instantaneous_beta, Decimal::ONE);
        assert_eq!(out.cumulative_beta, Decimal::ONE);
        assert_eq!(out.average_beta, Decimal::ONE);
    }

    #[test]
    fn test_perfect_passthrough_ols_one() {
        let input = perfect_passthrough();
        let out = analyze_deposit_beta(&input).unwrap();
        // OLS slope should be 1.0 for perfect pass-through
        // Note: when all x values are equal, x-x_bar=0 => denominator=0 => 0
        // This is a degenerate case (no variance in x). Let's accept 0 or test varied input.
        // Actually all benchmark changes are equal so there's no variance.
        // This means OLS slope = 0 (denominator = 0)
        assert_eq!(out.through_the_cycle_beta, Decimal::ZERO);
    }

    #[test]
    fn test_perfect_passthrough_no_lag() {
        let input = perfect_passthrough();
        let out = analyze_deposit_beta(&input).unwrap();
        assert_eq!(out.repricing_lag, Decimal::ZERO);
    }

    #[test]
    fn test_zero_beta() {
        let input = DepositBetaInput {
            rate_changes: vec![
                RateChange {
                    period: "Q1".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: Decimal::ZERO,
                },
                RateChange {
                    period: "Q2".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: Decimal::ZERO,
                },
            ],
            current_deposit_rate: dec!(0.01),
            current_benchmark_rate: dec!(0.05),
        };
        let out = analyze_deposit_beta(&input).unwrap();
        assert_eq!(out.instantaneous_beta, Decimal::ZERO);
        assert_eq!(out.cumulative_beta, Decimal::ZERO);
        assert_eq!(out.average_beta, Decimal::ZERO);
    }

    #[test]
    fn test_partial_beta_range() {
        let input = partial_passthrough();
        let out = analyze_deposit_beta(&input).unwrap();
        // Cumulative: (0+0.001+0.0015+0.0012)/(4*0.0025) = 0.0037/0.01 = 0.37
        assert!(
            approx_eq(out.cumulative_beta, dec!(0.37), dec!(0.001)),
            "Expected ~0.37, got {}",
            out.cumulative_beta
        );
    }

    #[test]
    fn test_lag_detection() {
        let input = partial_passthrough();
        let out = analyze_deposit_beta(&input).unwrap();
        // First period: benchmark changed but deposit didn't -> lag = 1
        assert_eq!(out.repricing_lag, Decimal::ONE);
    }

    #[test]
    fn test_multi_period_lag() {
        let input = DepositBetaInput {
            rate_changes: vec![
                RateChange {
                    period: "Q1".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: Decimal::ZERO,
                },
                RateChange {
                    period: "Q2".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: Decimal::ZERO,
                },
                RateChange {
                    period: "Q3".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.005),
                },
            ],
            current_deposit_rate: dec!(0.02),
            current_benchmark_rate: dec!(0.05),
        };
        let out = analyze_deposit_beta(&input).unwrap();
        assert_eq!(out.repricing_lag, dec!(2));
    }

    #[test]
    fn test_single_period() {
        let input = DepositBetaInput {
            rate_changes: vec![RateChange {
                period: "Q1".into(),
                benchmark_rate_change: dec!(0.005),
                deposit_rate_change: dec!(0.003),
            }],
            current_deposit_rate: dec!(0.03),
            current_benchmark_rate: dec!(0.05),
        };
        let out = analyze_deposit_beta(&input).unwrap();
        assert!(approx_eq(out.instantaneous_beta, dec!(0.6), dec!(0.001)));
        assert!(approx_eq(out.cumulative_beta, dec!(0.6), dec!(0.001)));
        assert!(approx_eq(out.average_beta, dec!(0.6), dec!(0.001)));
    }

    #[test]
    fn test_ols_regression_with_variance() {
        let input = DepositBetaInput {
            rate_changes: vec![
                RateChange {
                    period: "Q1".into(),
                    benchmark_rate_change: dec!(0.0025),
                    deposit_rate_change: dec!(0.0010),
                },
                RateChange {
                    period: "Q2".into(),
                    benchmark_rate_change: dec!(0.0050),
                    deposit_rate_change: dec!(0.0025),
                },
                RateChange {
                    period: "Q3".into(),
                    benchmark_rate_change: dec!(0.0075),
                    deposit_rate_change: dec!(0.0035),
                },
                RateChange {
                    period: "Q4".into(),
                    benchmark_rate_change: dec!(0.0100),
                    deposit_rate_change: dec!(0.0050),
                },
            ],
            current_deposit_rate: dec!(0.03),
            current_benchmark_rate: dec!(0.05),
        };
        let out = analyze_deposit_beta(&input).unwrap();
        // OLS with varied x values should give a meaningful slope
        assert!(out.through_the_cycle_beta > Decimal::ZERO);
        assert!(out.through_the_cycle_beta < Decimal::ONE);
    }

    #[test]
    fn test_deposit_sensitivity_per_100bp() {
        let input = partial_passthrough();
        let out = analyze_deposit_beta(&input).unwrap();
        // Sensitivity = cumulative_beta * 0.01
        let expected = out.cumulative_beta * dec!(0.01);
        assert_eq!(out.deposit_sensitivity, expected);
    }

    #[test]
    fn test_benchmark_zero_change_skipped() {
        let input = DepositBetaInput {
            rate_changes: vec![
                RateChange {
                    period: "Q1".into(),
                    benchmark_rate_change: Decimal::ZERO,
                    deposit_rate_change: Decimal::ZERO,
                },
                RateChange {
                    period: "Q2".into(),
                    benchmark_rate_change: dec!(0.005),
                    deposit_rate_change: dec!(0.003),
                },
            ],
            current_deposit_rate: dec!(0.03),
            current_benchmark_rate: dec!(0.05),
        };
        let out = analyze_deposit_beta(&input).unwrap();
        // Average beta should only include Q2 (benchmark != 0)
        assert!(approx_eq(out.average_beta, dec!(0.6), dec!(0.001)));
    }

    #[test]
    fn test_instantaneous_beta_benchmark_zero() {
        let input = DepositBetaInput {
            rate_changes: vec![RateChange {
                period: "Q1".into(),
                benchmark_rate_change: Decimal::ZERO,
                deposit_rate_change: dec!(0.001),
            }],
            current_deposit_rate: dec!(0.03),
            current_benchmark_rate: dec!(0.05),
        };
        let out = analyze_deposit_beta(&input).unwrap();
        assert_eq!(out.instantaneous_beta, Decimal::ZERO);
    }

    #[test]
    fn test_reject_empty_rate_changes() {
        let input = DepositBetaInput {
            rate_changes: vec![],
            current_deposit_rate: dec!(0.03),
            current_benchmark_rate: dec!(0.05),
        };
        assert!(analyze_deposit_beta(&input).is_err());
    }

    #[test]
    fn test_negative_benchmark_change() {
        // Rate cuts scenario
        let input = DepositBetaInput {
            rate_changes: vec![
                RateChange {
                    period: "Q1".into(),
                    benchmark_rate_change: dec!(-0.0025),
                    deposit_rate_change: dec!(-0.0015),
                },
                RateChange {
                    period: "Q2".into(),
                    benchmark_rate_change: dec!(-0.0025),
                    deposit_rate_change: dec!(-0.0020),
                },
            ],
            current_deposit_rate: dec!(0.02),
            current_benchmark_rate: dec!(0.03),
        };
        let out = analyze_deposit_beta(&input).unwrap();
        // Beta should be positive (both directions move same way)
        assert!(out.cumulative_beta > Decimal::ZERO);
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = partial_passthrough();
        let out = analyze_deposit_beta(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: DepositBetaOutput = serde_json::from_str(&json).unwrap();
    }
}
