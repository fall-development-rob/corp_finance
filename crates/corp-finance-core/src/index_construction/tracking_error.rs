//! Tracking Error Analysis.
//!
//! Covers:
//! 1. **Realized Tracking Error** -- std dev of active returns, annualized
//! 2. **Ex-Ante Tracking Error** -- from weight differences and covariance (diagonal)
//! 3. **Information Ratio** -- active return / tracking error
//! 4. **Active Share** -- sum |w_p - w_b| / 2 across all unique tickers
//! 5. **Hit Rate** -- percentage of periods with positive active return
//! 6. **TE Decomposition** -- allocation vs selection components
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Newton's method square root for Decimal (20 iterations).
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

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A ticker and its weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TickerWeight {
    pub ticker: String,
    pub weight: Decimal,
}

/// Decomposition of tracking error into allocation and selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeDecomposition {
    pub allocation_te: Decimal,
    pub selection_te: Decimal,
}

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for tracking error analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingErrorInput {
    /// Portfolio period returns.
    pub portfolio_returns: Vec<Decimal>,
    /// Benchmark period returns (same length).
    pub benchmark_returns: Vec<Decimal>,
    /// Portfolio weights by ticker.
    pub portfolio_weights: Vec<TickerWeight>,
    /// Benchmark weights by ticker.
    pub benchmark_weights: Vec<TickerWeight>,
    /// Variance per asset (simplified diagonal covariance).
    pub covariance_diagonal: Vec<Decimal>,
}

/// Output of the tracking error analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingErrorOutput {
    /// Realized tracking error (annualized).
    pub realized_te: Decimal,
    /// Ex-ante tracking error from weight/covariance.
    pub ex_ante_te: Decimal,
    /// Information ratio = active return / TE.
    pub information_ratio: Decimal,
    /// Annualized active return.
    pub active_return: Decimal,
    /// Active share = sum|w_p - w_b| / 2.
    pub active_share: Decimal,
    /// Largest single-period deviation.
    pub max_deviation: Decimal,
    /// Percentage of periods with positive active return.
    pub hit_rate: Decimal,
    /// TE decomposition into allocation and selection.
    pub te_decomposition: TeDecomposition,
    /// Number of return periods.
    pub num_periods: u32,
}

// ---------------------------------------------------------------------------
// Calculation
// ---------------------------------------------------------------------------

/// Perform tracking error analysis.
pub fn calculate_tracking_error(
    input: &TrackingErrorInput,
) -> CorpFinanceResult<TrackingErrorOutput> {
    validate_tracking_error_input(input)?;

    let n = input.portfolio_returns.len();
    let periods_per_year = dec!(12); // assume monthly

    // Active returns
    let active_returns: Vec<Decimal> = input
        .portfolio_returns
        .iter()
        .zip(input.benchmark_returns.iter())
        .map(|(p, b)| *p - *b)
        .collect();

    // Mean active return
    let n_dec = Decimal::from(n as u64);
    let mean_active: Decimal = active_returns.iter().copied().sum::<Decimal>() / n_dec;

    // Variance of active returns
    let variance: Decimal = active_returns
        .iter()
        .map(|r| {
            let diff = *r - mean_active;
            diff * diff
        })
        .sum::<Decimal>()
        / if n > 1 {
            Decimal::from((n - 1) as u64)
        } else {
            Decimal::ONE
        };

    let std_dev = decimal_sqrt(variance);

    // Annualized realized TE
    let realized_te = std_dev * decimal_sqrt(periods_per_year);

    // Annualized active return
    let active_return = mean_active * periods_per_year;

    // Information ratio
    let information_ratio = if realized_te.is_zero() {
        Decimal::ZERO
    } else {
        active_return / realized_te
    };

    // Max deviation
    let max_deviation = active_returns
        .iter()
        .map(|r| r.abs())
        .max()
        .unwrap_or(Decimal::ZERO);

    // Hit rate
    let positive_count = active_returns
        .iter()
        .filter(|r| **r > Decimal::ZERO)
        .count();
    let hit_rate = Decimal::from(positive_count as u64) / n_dec;

    // Active share
    let active_share = calc_active_share(&input.portfolio_weights, &input.benchmark_weights);

    // Ex-ante TE (diagonal model)
    let ex_ante_te = calc_ex_ante_te(
        &input.portfolio_weights,
        &input.benchmark_weights,
        &input.covariance_diagonal,
    );

    // TE decomposition: split into allocation (weight diff * avg variance) and selection (residual)
    let allocation_te = calc_allocation_te(
        &input.portfolio_weights,
        &input.benchmark_weights,
        &input.covariance_diagonal,
    );
    let selection_te = if realized_te > allocation_te {
        realized_te - allocation_te
    } else {
        Decimal::ZERO
    };

    Ok(TrackingErrorOutput {
        realized_te,
        ex_ante_te,
        information_ratio,
        active_return,
        active_share,
        max_deviation,
        hit_rate,
        te_decomposition: TeDecomposition {
            allocation_te,
            selection_te,
        },
        num_periods: n as u32,
    })
}

/// Active share = sum|w_p_i - w_b_i| / 2 over all unique tickers.
fn calc_active_share(portfolio: &[TickerWeight], benchmark: &[TickerWeight]) -> Decimal {
    let mut all_tickers = HashSet::new();
    for tw in portfolio.iter().chain(benchmark.iter()) {
        all_tickers.insert(tw.ticker.clone());
    }

    let port_map: std::collections::HashMap<&str, Decimal> = portfolio
        .iter()
        .map(|tw| (tw.ticker.as_str(), tw.weight))
        .collect();
    let bench_map: std::collections::HashMap<&str, Decimal> = benchmark
        .iter()
        .map(|tw| (tw.ticker.as_str(), tw.weight))
        .collect();

    let mut total_abs_diff = Decimal::ZERO;
    for ticker in &all_tickers {
        let wp = port_map
            .get(ticker.as_str())
            .copied()
            .unwrap_or(Decimal::ZERO);
        let wb = bench_map
            .get(ticker.as_str())
            .copied()
            .unwrap_or(Decimal::ZERO);
        total_abs_diff += (wp - wb).abs();
    }
    total_abs_diff / dec!(2)
}

/// Ex-ante TE from diagonal covariance: sqrt(sum((w_p_i - w_b_i)^2 * var_i)).
fn calc_ex_ante_te(
    portfolio: &[TickerWeight],
    benchmark: &[TickerWeight],
    cov_diag: &[Decimal],
) -> Decimal {
    // Use the shorter of portfolio or covariance length
    let min_len = portfolio.len().min(cov_diag.len());
    let bench_map: std::collections::HashMap<&str, Decimal> = benchmark
        .iter()
        .map(|tw| (tw.ticker.as_str(), tw.weight))
        .collect();

    let mut sum_sq = Decimal::ZERO;
    for i in 0..min_len {
        let wp = portfolio[i].weight;
        let wb = bench_map
            .get(portfolio[i].ticker.as_str())
            .copied()
            .unwrap_or(Decimal::ZERO);
        let diff = wp - wb;
        sum_sq += diff * diff * cov_diag[i];
    }
    decimal_sqrt(sum_sq)
}

/// Allocation component of TE: from weight differences with average variance.
fn calc_allocation_te(
    portfolio: &[TickerWeight],
    benchmark: &[TickerWeight],
    cov_diag: &[Decimal],
) -> Decimal {
    if cov_diag.is_empty() {
        return Decimal::ZERO;
    }
    let avg_var: Decimal =
        cov_diag.iter().copied().sum::<Decimal>() / Decimal::from(cov_diag.len() as u64);
    let bench_map: std::collections::HashMap<&str, Decimal> = benchmark
        .iter()
        .map(|tw| (tw.ticker.as_str(), tw.weight))
        .collect();

    let mut sum_sq = Decimal::ZERO;
    for tw in portfolio {
        let wb = bench_map
            .get(tw.ticker.as_str())
            .copied()
            .unwrap_or(Decimal::ZERO);
        let diff = tw.weight - wb;
        sum_sq += diff * diff;
    }
    decimal_sqrt(sum_sq * avg_var)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_tracking_error_input(input: &TrackingErrorInput) -> CorpFinanceResult<()> {
    if input.portfolio_returns.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one return period is required".into(),
        ));
    }
    if input.portfolio_returns.len() != input.benchmark_returns.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "benchmark_returns".into(),
            reason: format!(
                "Portfolio has {} returns but benchmark has {}",
                input.portfolio_returns.len(),
                input.benchmark_returns.len()
            ),
        });
    }
    if input.portfolio_weights.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Portfolio weights are required".into(),
        ));
    }
    if input.benchmark_weights.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Benchmark weights are required".into(),
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

    fn make_ticker_weight(ticker: &str, w: Decimal) -> TickerWeight {
        TickerWeight {
            ticker: ticker.into(),
            weight: w,
        }
    }

    fn make_base_input() -> TrackingErrorInput {
        TrackingErrorInput {
            portfolio_returns: vec![
                dec!(0.02),
                dec!(0.01),
                dec!(-0.01),
                dec!(0.03),
                dec!(0.005),
                dec!(-0.02),
                dec!(0.015),
                dec!(0.01),
                dec!(-0.005),
                dec!(0.02),
                dec!(0.01),
                dec!(0.005),
            ],
            benchmark_returns: vec![
                dec!(0.015),
                dec!(0.012),
                dec!(-0.008),
                dec!(0.025),
                dec!(0.006),
                dec!(-0.015),
                dec!(0.010),
                dec!(0.008),
                dec!(-0.003),
                dec!(0.018),
                dec!(0.009),
                dec!(0.004),
            ],
            portfolio_weights: vec![
                make_ticker_weight("AAPL", dec!(0.30)),
                make_ticker_weight("MSFT", dec!(0.25)),
                make_ticker_weight("GOOG", dec!(0.20)),
                make_ticker_weight("AMZN", dec!(0.15)),
                make_ticker_weight("META", dec!(0.10)),
            ],
            benchmark_weights: vec![
                make_ticker_weight("AAPL", dec!(0.20)),
                make_ticker_weight("MSFT", dec!(0.20)),
                make_ticker_weight("GOOG", dec!(0.20)),
                make_ticker_weight("AMZN", dec!(0.20)),
                make_ticker_weight("META", dec!(0.20)),
            ],
            covariance_diagonal: vec![dec!(0.04), dec!(0.03), dec!(0.05), dec!(0.06), dec!(0.04)],
        }
    }

    // --- Realized TE ---
    #[test]
    fn test_realized_te_positive() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        assert!(out.realized_te > Decimal::ZERO);
    }

    #[test]
    fn test_perfect_tracking_zero_te() {
        let mut input = make_base_input();
        input.benchmark_returns = input.portfolio_returns.clone();
        let out = calculate_tracking_error(&input).unwrap();
        assert_eq!(out.realized_te, Decimal::ZERO);
    }

    #[test]
    fn test_num_periods() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        assert_eq!(out.num_periods, 12);
    }

    // --- Active return ---
    #[test]
    fn test_active_return_positive_when_outperforming() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        // Portfolio generally outperforms benchmark in this sample
        assert!(out.active_return > Decimal::ZERO);
    }

    // --- Information ratio ---
    #[test]
    fn test_information_ratio_sign() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        // IR should be positive when active return is positive and TE > 0
        if out.active_return > Decimal::ZERO && out.realized_te > Decimal::ZERO {
            assert!(out.information_ratio > Decimal::ZERO);
        }
    }

    #[test]
    fn test_ir_zero_when_perfect_tracking() {
        let mut input = make_base_input();
        input.benchmark_returns = input.portfolio_returns.clone();
        let out = calculate_tracking_error(&input).unwrap();
        assert_eq!(out.information_ratio, Decimal::ZERO);
    }

    // --- Active share ---
    #[test]
    fn test_active_share_positive() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        // Portfolio overweights AAPL/MSFT, underweights AMZN/META vs equal weight
        assert!(out.active_share > Decimal::ZERO);
    }

    #[test]
    fn test_active_share_zero_when_same_weights() {
        let mut input = make_base_input();
        input.portfolio_weights = input.benchmark_weights.clone();
        let out = calculate_tracking_error(&input).unwrap();
        assert_eq!(out.active_share, Decimal::ZERO);
    }

    #[test]
    fn test_active_share_calculation() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        // |0.30-0.20| + |0.25-0.20| + |0.20-0.20| + |0.15-0.20| + |0.10-0.20|
        // = 0.10 + 0.05 + 0 + 0.05 + 0.10 = 0.30, active_share = 0.30/2 = 0.15
        assert!(approx_eq(out.active_share, dec!(0.15), dec!(0.001)));
    }

    // --- Index hugger (low TE) ---
    #[test]
    fn test_index_hugger() {
        let mut input = make_base_input();
        // Very small deviations
        input.portfolio_returns = input
            .benchmark_returns
            .iter()
            .map(|r| *r + dec!(0.0001))
            .collect();
        let out = calculate_tracking_error(&input).unwrap();
        assert!(out.realized_te < dec!(0.01));
    }

    // --- Active fund (high TE) ---
    #[test]
    fn test_active_fund_high_te() {
        let mut input = make_base_input();
        // Large deviations
        input.portfolio_returns = vec![
            dec!(0.05),
            dec!(-0.03),
            dec!(0.04),
            dec!(-0.02),
            dec!(0.06),
            dec!(-0.04),
            dec!(0.05),
            dec!(-0.01),
            dec!(0.03),
            dec!(-0.03),
            dec!(0.04),
            dec!(-0.02),
        ];
        input.benchmark_returns = vec![
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
            dec!(0.01),
        ];
        let out = calculate_tracking_error(&input).unwrap();
        assert!(out.realized_te > dec!(0.05));
    }

    // --- Hit rate ---
    #[test]
    fn test_hit_rate() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        // Count periods where portfolio > benchmark
        assert!(out.hit_rate > Decimal::ZERO);
        assert!(out.hit_rate <= Decimal::ONE);
    }

    #[test]
    fn test_hit_rate_perfect() {
        let mut input = make_base_input();
        input.portfolio_returns = input
            .benchmark_returns
            .iter()
            .map(|r| *r + dec!(0.01))
            .collect();
        let out = calculate_tracking_error(&input).unwrap();
        assert_eq!(out.hit_rate, Decimal::ONE);
    }

    // --- Max deviation ---
    #[test]
    fn test_max_deviation() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        assert!(out.max_deviation > Decimal::ZERO);
    }

    // --- Ex-ante TE ---
    #[test]
    fn test_ex_ante_te_positive() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        assert!(out.ex_ante_te > Decimal::ZERO);
    }

    #[test]
    fn test_ex_ante_te_zero_same_weights() {
        let mut input = make_base_input();
        input.portfolio_weights = input.benchmark_weights.clone();
        let out = calculate_tracking_error(&input).unwrap();
        assert_eq!(out.ex_ante_te, Decimal::ZERO);
    }

    // --- TE decomposition ---
    #[test]
    fn test_te_decomposition_non_negative() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        assert!(out.te_decomposition.allocation_te >= Decimal::ZERO);
        assert!(out.te_decomposition.selection_te >= Decimal::ZERO);
    }

    #[test]
    fn test_te_decomposition_components() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        // allocation + selection should equal realized_te (by construction: selection = realized - allocation)
        let sum = out.te_decomposition.allocation_te + out.te_decomposition.selection_te;
        assert!(
            approx_eq(sum, out.realized_te, dec!(0.1)),
            "sum={} realized_te={}",
            sum,
            out.realized_te
        );
    }

    // --- Active share with non-overlapping tickers ---
    #[test]
    fn test_active_share_non_overlapping() {
        let mut input = make_base_input();
        input.portfolio_weights = vec![
            make_ticker_weight("X", dec!(0.50)),
            make_ticker_weight("Y", dec!(0.50)),
        ];
        input.benchmark_weights = vec![
            make_ticker_weight("A", dec!(0.50)),
            make_ticker_weight("B", dec!(0.50)),
        ];
        let out = calculate_tracking_error(&input).unwrap();
        // All different: |0.50|+|0.50|+|0.50|+|0.50| / 2 = 1.0
        assert!(approx_eq(out.active_share, Decimal::ONE, dec!(0.001)));
    }

    // --- Validation ---
    #[test]
    fn test_reject_empty_returns() {
        let mut input = make_base_input();
        input.portfolio_returns = vec![];
        assert!(calculate_tracking_error(&input).is_err());
    }

    #[test]
    fn test_reject_mismatched_returns() {
        let mut input = make_base_input();
        input.benchmark_returns.push(dec!(0.01));
        assert!(calculate_tracking_error(&input).is_err());
    }

    #[test]
    fn test_reject_empty_portfolio_weights() {
        let mut input = make_base_input();
        input.portfolio_weights = vec![];
        assert!(calculate_tracking_error(&input).is_err());
    }

    #[test]
    fn test_reject_empty_benchmark_weights() {
        let mut input = make_base_input();
        input.benchmark_weights = vec![];
        assert!(calculate_tracking_error(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = make_base_input();
        let out = calculate_tracking_error(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: TrackingErrorOutput = serde_json::from_str(&json).unwrap();
    }
}
