use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

/// Frequency of return observations
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ReturnFrequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Annual,
}

impl ReturnFrequency {
    /// Number of periods in a year for annualisation
    pub fn periods_per_year(&self) -> Decimal {
        match self {
            ReturnFrequency::Daily => dec!(252),
            ReturnFrequency::Weekly => dec!(52),
            ReturnFrequency::Monthly => dec!(12),
            ReturnFrequency::Quarterly => dec!(4),
            ReturnFrequency::Annual => dec!(1),
        }
    }
}

/// Input for risk-adjusted return calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAdjustedInput {
    /// Periodic returns (as decimals, e.g. 0.05 = 5%)
    pub returns: Vec<Decimal>,
    /// Risk-free rate (annualised)
    pub risk_free_rate: Rate,
    /// Benchmark returns (same frequency as `returns`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub benchmark_returns: Option<Vec<Decimal>>,
    /// Observation frequency
    pub frequency: ReturnFrequency,
    /// Target return for Sortino ratio (annualised); defaults to risk_free_rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_return: Option<Rate>,
}

/// Output of risk-adjusted return calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAdjustedOutput {
    pub annualised_return: Rate,
    pub annualised_volatility: Rate,
    pub sharpe_ratio: Decimal,
    pub sortino_ratio: Decimal,
    pub calmar_ratio: Option<Decimal>,
    pub information_ratio: Option<Decimal>,
    pub treynor_ratio: Option<Decimal>,
    pub max_drawdown: Rate,
    pub downside_deviation: Rate,
    pub tracking_error: Option<Rate>,
    pub beta: Option<Decimal>,
    pub alpha: Option<Rate>,
}

/// Calculate risk-adjusted portfolio returns.
pub fn calculate_risk_adjusted_returns(
    input: &RiskAdjustedInput,
) -> CorpFinanceResult<ComputationOutput<RiskAdjustedOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let n = input.returns.len();
    if n < 2 {
        return Err(CorpFinanceError::InsufficientData(
            "At least 2 return observations required".into(),
        ));
    }

    let n_dec = Decimal::from(n as i64);
    let periods = input.frequency.periods_per_year();

    // Mean periodic return
    let sum: Decimal = input.returns.iter().sum();
    let mean_return = sum / n_dec;

    // Annualised return
    let annualised_return = mean_return * periods;

    // Variance and standard deviation (sample)
    let variance = sample_variance(&input.returns, mean_return);
    let std_dev = sqrt_decimal(variance);
    let annualised_volatility = std_dev * sqrt_decimal(periods);

    // Risk-free rate per period
    // Sharpe = (Rp - Rf) / sigma_p (annualised)
    let sharpe_ratio = if annualised_volatility.is_zero() {
        Decimal::ZERO
    } else {
        (annualised_return - input.risk_free_rate) / annualised_volatility
    };

    // Downside deviation (below target return per period)
    let target_per_period = input
        .target_return
        .unwrap_or(input.risk_free_rate)
        / periods;
    let downside_dev = downside_deviation(&input.returns, target_per_period);
    let annualised_downside = downside_dev * sqrt_decimal(periods);

    // Sortino = (Rp - Rf) / downside_deviation (annualised)
    let sortino_ratio = if annualised_downside.is_zero() {
        Decimal::ZERO
    } else {
        (annualised_return - input.risk_free_rate) / annualised_downside
    };

    // Max drawdown
    let max_dd = max_drawdown(&input.returns);

    // Calmar = annualised_return / |max_drawdown|
    let calmar_ratio = if max_dd.is_zero() {
        None
    } else {
        Some(annualised_return / max_dd.abs())
    };

    // Benchmark-dependent metrics
    let (information_ratio, tracking_error, beta, alpha, treynor_ratio) =
        if let Some(ref bench) = input.benchmark_returns {
            if bench.len() != n {
                return Err(CorpFinanceError::InvalidInput {
                    field: "benchmark_returns".into(),
                    reason: "Benchmark must have same length as returns".into(),
                });
            }

            let bench_mean: Decimal = bench.iter().sum::<Decimal>() / n_dec;

            // Excess returns over benchmark
            let excess: Vec<Decimal> = input
                .returns
                .iter()
                .zip(bench.iter())
                .map(|(r, b)| r - b)
                .collect();
            let excess_mean: Decimal = excess.iter().sum::<Decimal>() / n_dec;
            let te_var = sample_variance(&excess, excess_mean);
            let te = sqrt_decimal(te_var) * sqrt_decimal(periods);

            let ir = if te.is_zero() {
                None
            } else {
                Some((annualised_return - bench_mean * periods) / te)
            };

            // Beta = Cov(Rp, Rb) / Var(Rb)
            let cov = covariance(&input.returns, bench, mean_return, bench_mean);
            let bench_var = sample_variance(bench, bench_mean);
            let beta_val = if bench_var.is_zero() {
                None
            } else {
                Some(cov / bench_var)
            };

            // Alpha = Rp - [Rf + Beta * (Rb - Rf)] (annualised)
            let alpha_val = beta_val.map(|b| {
                annualised_return
                    - (input.risk_free_rate + b * (bench_mean * periods - input.risk_free_rate))
            });

            // Treynor = (Rp - Rf) / Beta (annualised)
            let treynor = beta_val.and_then(|b| {
                if b.is_zero() {
                    None
                } else {
                    Some((annualised_return - input.risk_free_rate) / b)
                }
            });

            (ir, Some(te), beta_val, alpha_val, treynor)
        } else {
            (None, None, None, None, None)
        };

    let output = RiskAdjustedOutput {
        annualised_return,
        annualised_volatility,
        sharpe_ratio,
        sortino_ratio,
        calmar_ratio,
        information_ratio,
        treynor_ratio,
        max_drawdown: max_dd,
        downside_deviation: annualised_downside,
        tracking_error,
        beta,
        alpha,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Risk-Adjusted Returns (Sharpe, Sortino, Calmar, Information Ratio, Treynor, Alpha/Beta)",
        &serde_json::json!({
            "observations": n,
            "frequency": format!("{:?}", input.frequency),
            "risk_free_rate": input.risk_free_rate.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Sample variance (n-1 denominator)
fn sample_variance(data: &[Decimal], mean: Decimal) -> Decimal {
    let n = data.len();
    if n < 2 {
        return Decimal::ZERO;
    }
    let sum_sq: Decimal = data.iter().map(|x| (x - mean) * (x - mean)).sum();
    sum_sq / Decimal::from((n - 1) as i64)
}

/// Downside deviation: std dev of returns below target
fn downside_deviation(returns: &[Decimal], target: Decimal) -> Decimal {
    let n = returns.len();
    if n == 0 {
        return Decimal::ZERO;
    }
    let sum_sq: Decimal = returns
        .iter()
        .map(|r| {
            let diff = r - target;
            if diff < Decimal::ZERO {
                diff * diff
            } else {
                Decimal::ZERO
            }
        })
        .sum();
    sqrt_decimal(sum_sq / Decimal::from(n as i64))
}

/// Maximum drawdown from a return series
fn max_drawdown(returns: &[Decimal]) -> Rate {
    let mut cumulative = Decimal::ONE;
    let mut peak = Decimal::ONE;
    let mut max_dd = Decimal::ZERO;

    for r in returns {
        cumulative *= Decimal::ONE + r;
        if cumulative > peak {
            peak = cumulative;
        }
        if !peak.is_zero() {
            let dd = (peak - cumulative) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// Covariance between two series (sample, n-1)
fn covariance(x: &[Decimal], y: &[Decimal], x_mean: Decimal, y_mean: Decimal) -> Decimal {
    let n = x.len();
    if n < 2 {
        return Decimal::ZERO;
    }
    let sum: Decimal = x
        .iter()
        .zip(y.iter())
        .map(|(xi, yi)| (xi - x_mean) * (yi - y_mean))
        .sum();
    sum / Decimal::from((n - 1) as i64)
}

/// Integer square root approximation via Decimal::sqrt()
fn sqrt_decimal(val: Decimal) -> Decimal {
    if val <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    val.sqrt().unwrap_or(Decimal::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_returns() -> Vec<Decimal> {
        vec![
            dec!(0.05),
            dec!(-0.02),
            dec!(0.03),
            dec!(0.01),
            dec!(-0.01),
            dec!(0.04),
            dec!(0.02),
            dec!(-0.03),
            dec!(0.06),
            dec!(0.01),
            dec!(-0.02),
            dec!(0.03),
        ]
    }

    #[test]
    fn test_basic_returns() {
        let input = RiskAdjustedInput {
            returns: sample_returns(),
            risk_free_rate: dec!(0.02),
            benchmark_returns: None,
            frequency: ReturnFrequency::Monthly,
            target_return: None,
        };
        let result = calculate_risk_adjusted_returns(&input).unwrap();
        let out = &result.result;

        // Mean monthly ~0.01417 => annualised ~0.17
        assert!(out.annualised_return > dec!(0.10));
        assert!(out.annualised_volatility > Decimal::ZERO);
        assert!(out.sharpe_ratio != Decimal::ZERO);
    }

    #[test]
    fn test_sharpe_direction() {
        // Higher returns with some volatility => higher Sharpe
        let high = RiskAdjustedInput {
            returns: vec![dec!(0.10), dec!(0.08), dec!(0.12), dec!(0.09)],
            risk_free_rate: dec!(0.02),
            benchmark_returns: None,
            frequency: ReturnFrequency::Monthly,
            target_return: None,
        };
        let low = RiskAdjustedInput {
            returns: vec![dec!(0.01), dec!(-0.01), dec!(0.02), dec!(0.00)],
            risk_free_rate: dec!(0.02),
            benchmark_returns: None,
            frequency: ReturnFrequency::Monthly,
            target_return: None,
        };
        let high_r = calculate_risk_adjusted_returns(&high).unwrap();
        let low_r = calculate_risk_adjusted_returns(&low).unwrap();
        assert!(high_r.result.sharpe_ratio > low_r.result.sharpe_ratio);
    }

    #[test]
    fn test_max_drawdown() {
        let returns = vec![dec!(0.10), dec!(-0.20), dec!(0.05), dec!(-0.10)];
        let dd = max_drawdown(&returns);
        // Peak after +10%: 1.1; trough after -20%: 0.88; dd=0.2
        // Then 0.88*1.05=0.924, 0.924*0.9=0.8316; dd from peak 1.1 = (1.1-0.8316)/1.1 ~0.244
        assert!(dd > dec!(0.20));
    }

    #[test]
    fn test_sortino_no_downside() {
        // All positive returns => downside dev = 0 => sortino = 0 (guarded)
        let input = RiskAdjustedInput {
            returns: vec![dec!(0.05), dec!(0.05), dec!(0.05)],
            risk_free_rate: dec!(0.00),
            benchmark_returns: None,
            frequency: ReturnFrequency::Monthly,
            target_return: Some(dec!(0.00)),
        };
        let result = calculate_risk_adjusted_returns(&input).unwrap();
        // With zero target, all returns are above target, so sortino guard gives 0
        assert_eq!(result.result.sortino_ratio, Decimal::ZERO);
    }

    #[test]
    fn test_with_benchmark() {
        let input = RiskAdjustedInput {
            returns: sample_returns(),
            risk_free_rate: dec!(0.02),
            benchmark_returns: Some(vec![
                dec!(0.04),
                dec!(-0.01),
                dec!(0.02),
                dec!(0.00),
                dec!(-0.02),
                dec!(0.03),
                dec!(0.01),
                dec!(-0.02),
                dec!(0.05),
                dec!(0.00),
                dec!(-0.01),
                dec!(0.02),
            ]),
            frequency: ReturnFrequency::Monthly,
            target_return: None,
        };
        let result = calculate_risk_adjusted_returns(&input).unwrap();
        let out = &result.result;
        assert!(out.beta.is_some());
        assert!(out.alpha.is_some());
        assert!(out.tracking_error.is_some());
        assert!(out.information_ratio.is_some());
        assert!(out.treynor_ratio.is_some());
    }

    #[test]
    fn test_insufficient_data() {
        let input = RiskAdjustedInput {
            returns: vec![dec!(0.05)],
            risk_free_rate: dec!(0.02),
            benchmark_returns: None,
            frequency: ReturnFrequency::Monthly,
            target_return: None,
        };
        assert!(calculate_risk_adjusted_returns(&input).is_err());
    }

    #[test]
    fn test_benchmark_length_mismatch() {
        let input = RiskAdjustedInput {
            returns: vec![dec!(0.05), dec!(0.03)],
            risk_free_rate: dec!(0.02),
            benchmark_returns: Some(vec![dec!(0.04)]),
            frequency: ReturnFrequency::Monthly,
            target_return: None,
        };
        assert!(calculate_risk_adjusted_returns(&input).is_err());
    }
}
