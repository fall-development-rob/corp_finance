use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use statrs::distribution::{LogNormal, Normal, Triangular, Uniform};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{ComputationMetadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Helper: build ComputationOutput without requiring Decimal
// ---------------------------------------------------------------------------

fn with_metadata_f64<T: Serialize>(
    methodology: &str,
    assumptions: &impl Serialize,
    warnings: Vec<String>,
    elapsed_us: u64,
    result: T,
) -> ComputationOutput<T> {
    ComputationOutput {
        result,
        methodology: methodology.to_string(),
        assumptions: serde_json::to_value(assumptions).unwrap_or_default(),
        warnings,
        metadata: ComputationMetadata {
            version: env!("CARGO_PKG_VERSION").to_string(),
            computation_time_us: elapsed_us,
            precision: "ieee754_f64".to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Probability distribution specification for a Monte Carlo variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McDistribution {
    Normal { mean: f64, std_dev: f64 },
    LogNormal { mu: f64, sigma: f64 },
    Triangular { min: f64, mode: f64, max: f64 },
    Uniform { min: f64, max: f64 },
}

/// A single variable to simulate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McVariable {
    pub name: String,
    pub distribution: McDistribution,
}

/// Top-level input for a generic Monte Carlo simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloInput {
    /// Number of simulation paths (minimum 100).
    #[serde(default = "default_num_simulations")]
    pub num_simulations: u32,
    /// Optional seed for reproducibility.
    pub seed: Option<u64>,
    /// Variables to simulate.
    pub variables: Vec<McVariable>,
}

fn default_num_simulations() -> u32 {
    10_000
}

/// Percentile summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McPercentiles {
    pub p5: f64,
    pub p10: f64,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p95: f64,
}

/// A single histogram bin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBin {
    pub lower: f64,
    pub upper: f64,
    pub count: u32,
    pub frequency: f64,
}

/// Result statistics for one simulated variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McVariableResult {
    pub name: String,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub percentiles: McPercentiles,
    pub skewness: f64,
    pub kurtosis: f64,
    pub histogram: Vec<HistogramBin>,
}

/// Output of a generic Monte Carlo simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonteCarloOutput {
    pub num_simulations: u32,
    pub variables: Vec<McVariableResult>,
}

// ---------------------------------------------------------------------------
// DCF Monte Carlo types
// ---------------------------------------------------------------------------

/// Input for a Monte Carlo DCF valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McDcfInput {
    /// Base year free cash flow.
    pub base_fcf: f64,
    /// Number of projection years.
    pub projection_years: u32,
    /// Distribution for annual revenue growth rate.
    pub revenue_growth: McDistribution,
    /// Distribution for EBITDA margin.
    pub ebitda_margin: McDistribution,
    /// Distribution for the discount rate (WACC).
    pub wacc: McDistribution,
    /// Distribution for the terminal growth rate.
    pub terminal_growth: McDistribution,
    /// Capex as a percentage of revenue (fixed).
    pub capex_pct: f64,
    /// Corporate tax rate.
    pub tax_rate: f64,
    /// Number of simulation paths (minimum 100).
    #[serde(default = "default_num_simulations")]
    pub num_simulations: u32,
    /// Optional seed for reproducibility.
    pub seed: Option<u64>,
}

/// Probability that EV exceeds a given threshold.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdProbability {
    pub threshold: f64,
    pub probability: f64,
}

/// Output of a Monte Carlo DCF simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McDcfOutput {
    /// Percentile summary of simulated enterprise values.
    pub enterprise_values: McPercentiles,
    /// Mean enterprise value across all valid simulations.
    pub ev_mean: f64,
    /// Standard deviation of enterprise values.
    pub ev_std_dev: f64,
    /// Probability that EV exceeds selected thresholds.
    pub probability_above: Vec<ThresholdProbability>,
    /// 90% confidence interval (P5 to P95).
    pub implied_ev_range: (f64, f64),
    /// Number of valid simulations actually used.
    pub simulation_count: u32,
}

// ---------------------------------------------------------------------------
// Sampling
// ---------------------------------------------------------------------------

/// Sample a single value from the given distribution using the provided RNG.
fn sample(rng: &mut StdRng, dist: &McDistribution) -> CorpFinanceResult<f64> {
    match dist {
        McDistribution::Normal { mean, std_dev } => {
            let n = Normal::new(*mean, *std_dev).map_err(|e| CorpFinanceError::InvalidInput {
                field: "distribution".into(),
                reason: format!("Invalid Normal parameters: {e}"),
            })?;
            Ok(rng.sample(n))
        }
        McDistribution::LogNormal { mu, sigma } => {
            let ln = LogNormal::new(*mu, *sigma).map_err(|e| CorpFinanceError::InvalidInput {
                field: "distribution".into(),
                reason: format!("Invalid LogNormal parameters: {e}"),
            })?;
            Ok(rng.sample(ln))
        }
        McDistribution::Triangular { min, mode, max } => {
            let t =
                Triangular::new(*min, *max, *mode).map_err(|e| CorpFinanceError::InvalidInput {
                    field: "distribution".into(),
                    reason: format!("Invalid Triangular parameters: {e}"),
                })?;
            Ok(rng.sample(t))
        }
        McDistribution::Uniform { min, max } => {
            let u = Uniform::new(*min, *max).map_err(|e| CorpFinanceError::InvalidInput {
                field: "distribution".into(),
                reason: format!("Invalid Uniform parameters: {e}"),
            })?;
            Ok(rng.sample(u))
        }
    }
}

// ---------------------------------------------------------------------------
// Statistics helpers
// ---------------------------------------------------------------------------

/// Compute the percentile value from a **sorted** slice using linear interpolation.
fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    assert!(!sorted.is_empty());
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = p / 100.0 * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let frac = rank - lower as f64;
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}

/// Build a histogram with `num_bins` equal-width bins.
fn build_histogram(sorted: &[f64], num_bins: usize) -> Vec<HistogramBin> {
    let min_val = sorted[0];
    let max_val = sorted[sorted.len() - 1];

    // Handle case where all values are the same
    if (max_val - min_val).abs() < f64::EPSILON {
        return vec![HistogramBin {
            lower: min_val,
            upper: max_val,
            count: sorted.len() as u32,
            frequency: 1.0,
        }];
    }

    let bin_width = (max_val - min_val) / num_bins as f64;
    let n = sorted.len() as f64;

    let mut bins: Vec<HistogramBin> = (0..num_bins)
        .map(|i| {
            let lower = min_val + i as f64 * bin_width;
            let upper = if i == num_bins - 1 {
                max_val
            } else {
                min_val + (i + 1) as f64 * bin_width
            };
            HistogramBin {
                lower,
                upper,
                count: 0,
                frequency: 0.0,
            }
        })
        .collect();

    for &val in sorted {
        let mut idx = ((val - min_val) / bin_width).floor() as usize;
        if idx >= num_bins {
            idx = num_bins - 1;
        }
        bins[idx].count += 1;
    }

    for bin in &mut bins {
        bin.frequency = bin.count as f64 / n;
    }

    bins
}

/// Compute descriptive statistics for a mutable slice of f64 values.
/// The slice will be sorted in place.
fn compute_statistics(values: &mut [f64], name: &str) -> McVariableResult {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len() as f64;

    let mean = values.iter().sum::<f64>() / n;

    let median = if values.len().is_multiple_of(2) {
        let mid = values.len() / 2;
        (values[mid - 1] + values[mid]) / 2.0
    } else {
        values[values.len() / 2]
    };

    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
    let std_dev = variance.sqrt();

    let min = values[0];
    let max = values[values.len() - 1];

    let percentiles = McPercentiles {
        p5: percentile_sorted(values, 5.0),
        p10: percentile_sorted(values, 10.0),
        p25: percentile_sorted(values, 25.0),
        p50: percentile_sorted(values, 50.0),
        p75: percentile_sorted(values, 75.0),
        p90: percentile_sorted(values, 90.0),
        p95: percentile_sorted(values, 95.0),
    };

    // Skewness (population)
    let skewness = if std_dev > f64::EPSILON {
        values
            .iter()
            .map(|v| ((v - mean) / std_dev).powi(3))
            .sum::<f64>()
            / n
    } else {
        0.0
    };

    // Excess kurtosis (population)
    let kurtosis = if std_dev > f64::EPSILON {
        values
            .iter()
            .map(|v| ((v - mean) / std_dev).powi(4))
            .sum::<f64>()
            / n
            - 3.0
    } else {
        0.0
    };

    let histogram = build_histogram(values, 20);

    McVariableResult {
        name: name.to_string(),
        mean,
        median,
        std_dev,
        min,
        max,
        percentiles,
        skewness,
        kurtosis,
        histogram,
    }
}

// ---------------------------------------------------------------------------
// Public API: generic Monte Carlo simulation
// ---------------------------------------------------------------------------

/// Run a generic Monte Carlo simulation over the specified variables.
///
/// Each variable is independently sampled from its distribution for
/// `num_simulations` paths. Returns per-variable statistics including
/// mean, median, standard deviation, percentiles, skewness, kurtosis,
/// and a 20-bin histogram.
pub fn run_monte_carlo_simulation(
    input: &MonteCarloInput,
) -> CorpFinanceResult<ComputationOutput<MonteCarloOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // Validation
    if input.num_simulations < 100 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_simulations".into(),
            reason: "Must be at least 100".into(),
        });
    }
    if input.variables.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one variable is required".into(),
        ));
    }

    let mut rng = match input.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    let n = input.num_simulations as usize;
    let mut variable_results = Vec::with_capacity(input.variables.len());

    for var in &input.variables {
        let mut samples = Vec::with_capacity(n);
        for _ in 0..n {
            samples.push(sample(&mut rng, &var.distribution)?);
        }
        variable_results.push(compute_statistics(&mut samples, &var.name));
    }

    let output = MonteCarloOutput {
        num_simulations: input.num_simulations,
        variables: variable_results,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata_f64(
        "Monte Carlo Simulation",
        &serde_json::json!({
            "num_simulations": input.num_simulations,
            "seed": input.seed,
            "variables": input.variables.iter().map(|v| &v.name).collect::<Vec<_>>(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Public API: Monte Carlo DCF
// ---------------------------------------------------------------------------

/// Run a Monte Carlo DCF valuation.
///
/// For each simulation path, revenue growth, EBITDA margin, WACC, and
/// terminal growth are sampled from their distributions. Free cash flows
/// are projected and discounted. Paths where terminal growth >= WACC are
/// skipped as financial impossibilities.
pub fn run_monte_carlo_dcf(
    input: &McDcfInput,
) -> CorpFinanceResult<ComputationOutput<McDcfOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // Validation
    if input.num_simulations < 100 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_simulations".into(),
            reason: "Must be at least 100".into(),
        });
    }
    if input.projection_years < 1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "projection_years".into(),
            reason: "Must be at least 1".into(),
        });
    }

    let mut rng = match input.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };

    let n = input.num_simulations as usize;
    let mut ev_values: Vec<f64> = Vec::with_capacity(n);
    let mut skipped: u32 = 0;

    for _ in 0..n {
        let g = sample(&mut rng, &input.revenue_growth)?;
        let margin = sample(&mut rng, &input.ebitda_margin)?;
        let wacc = sample(&mut rng, &input.wacc)?;
        let tg = sample(&mut rng, &input.terminal_growth)?;

        // Skip financially impossible paths
        if tg >= wacc {
            skipped += 1;
            continue;
        }

        // Project FCFs
        let mut npv = 0.0_f64;
        let mut discount_factor = 1.0_f64;
        let mut last_fcf = 0.0_f64;

        for t in 1..=input.projection_years {
            let revenue_multiplier = (1.0 + g).powi(t as i32);
            let fcf = input.base_fcf * revenue_multiplier * margin;
            discount_factor /= 1.0 + wacc;
            npv += fcf * discount_factor;
            last_fcf = fcf;
        }

        // Terminal value
        let terminal_fcf = last_fcf * (1.0 + tg);
        let terminal_value = terminal_fcf / (wacc - tg);
        npv += terminal_value * discount_factor;

        ev_values.push(npv);
    }

    if ev_values.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "All simulations were skipped (terminal_growth >= wacc in every path)".into(),
        ));
    }

    if skipped > 0 {
        warnings.push(format!(
            "{skipped} of {} simulations skipped (terminal_growth >= wacc)",
            input.num_simulations
        ));
    }

    // Sort for percentile calculations
    ev_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let valid_n = ev_values.len() as f64;
    let ev_mean = ev_values.iter().sum::<f64>() / valid_n;
    let ev_variance = ev_values.iter().map(|v| (v - ev_mean).powi(2)).sum::<f64>() / valid_n;
    let ev_std_dev = ev_variance.sqrt();

    let enterprise_values = McPercentiles {
        p5: percentile_sorted(&ev_values, 5.0),
        p10: percentile_sorted(&ev_values, 10.0),
        p25: percentile_sorted(&ev_values, 25.0),
        p50: percentile_sorted(&ev_values, 50.0),
        p75: percentile_sorted(&ev_values, 75.0),
        p90: percentile_sorted(&ev_values, 90.0),
        p95: percentile_sorted(&ev_values, 95.0),
    };

    let implied_ev_range = (enterprise_values.p5, enterprise_values.p95);

    // Compute probability above common thresholds
    // Use quartile-based thresholds for generality
    let thresholds = vec![
        enterprise_values.p25,
        enterprise_values.p50,
        enterprise_values.p75,
        ev_mean,
    ];
    let probability_above: Vec<ThresholdProbability> = thresholds
        .into_iter()
        .map(|threshold| {
            let count_above = ev_values.iter().filter(|&&v| v > threshold).count();
            ThresholdProbability {
                threshold,
                probability: count_above as f64 / valid_n,
            }
        })
        .collect();

    let output = McDcfOutput {
        enterprise_values,
        ev_mean,
        ev_std_dev,
        probability_above,
        implied_ev_range,
        simulation_count: ev_values.len() as u32,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata_f64(
        "Monte Carlo DCF Valuation",
        &serde_json::json!({
            "base_fcf": input.base_fcf,
            "projection_years": input.projection_years,
            "capex_pct": input.capex_pct,
            "tax_rate": input.tax_rate,
            "num_simulations": input.num_simulations,
            "valid_simulations": ev_values.len(),
            "skipped_simulations": skipped,
            "seed": input.seed,
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u64 = 42;

    fn normal_var(name: &str, mean: f64, std_dev: f64) -> McVariable {
        McVariable {
            name: name.into(),
            distribution: McDistribution::Normal { mean, std_dev },
        }
    }

    fn basic_input() -> MonteCarloInput {
        MonteCarloInput {
            num_simulations: 10_000,
            seed: Some(SEED),
            variables: vec![normal_var("revenue_growth", 0.05, 0.02)],
        }
    }

    // --- Generic simulation tests ---

    #[test]
    fn test_basic_simulation_runs() {
        let result = run_monte_carlo_simulation(&basic_input()).unwrap();
        assert_eq!(result.result.num_simulations, 10_000);
        assert_eq!(result.result.variables.len(), 1);
    }

    #[test]
    fn test_seeded_reproducibility() {
        let input = basic_input();
        let r1 = run_monte_carlo_simulation(&input).unwrap();
        let r2 = run_monte_carlo_simulation(&input).unwrap();
        assert_eq!(r1.result.variables[0].mean, r2.result.variables[0].mean);
        assert_eq!(r1.result.variables[0].median, r2.result.variables[0].median);
        assert_eq!(
            r1.result.variables[0].std_dev,
            r2.result.variables[0].std_dev
        );
    }

    #[test]
    fn test_normal_distribution_statistics() {
        let input = MonteCarloInput {
            num_simulations: 50_000,
            seed: Some(SEED),
            variables: vec![normal_var("test", 100.0, 10.0)],
        };
        let result = run_monte_carlo_simulation(&input).unwrap();
        let v = &result.result.variables[0];

        // Mean should be close to 100
        assert!((v.mean - 100.0).abs() < 0.5, "mean={}", v.mean);
        // Std dev should be close to 10
        assert!((v.std_dev - 10.0).abs() < 0.5, "std_dev={}", v.std_dev);
        // Skewness should be close to 0 for normal
        assert!(v.skewness.abs() < 0.1, "skewness={}", v.skewness);
        // Excess kurtosis should be close to 0 for normal
        assert!(v.kurtosis.abs() < 0.1, "kurtosis={}", v.kurtosis);
    }

    #[test]
    fn test_lognormal_distribution() {
        let input = MonteCarloInput {
            num_simulations: 10_000,
            seed: Some(SEED),
            variables: vec![McVariable {
                name: "asset_price".into(),
                distribution: McDistribution::LogNormal {
                    mu: 0.0,
                    sigma: 0.5,
                },
            }],
        };
        let result = run_monte_carlo_simulation(&input).unwrap();
        let v = &result.result.variables[0];

        // LogNormal(0, 0.5) has mean = exp(0 + 0.25/2) = exp(0.125) ~ 1.133
        let expected_mean = (0.0_f64 + 0.5_f64 * 0.5 / 2.0).exp();
        assert!(
            (v.mean - expected_mean).abs() < 0.05,
            "mean={}, expected={}",
            v.mean,
            expected_mean
        );
        // All values must be positive
        assert!(v.min > 0.0);
        // Should be positively skewed
        assert!(v.skewness > 0.0, "skewness={}", v.skewness);
    }

    #[test]
    fn test_triangular_distribution() {
        let input = MonteCarloInput {
            num_simulations: 10_000,
            seed: Some(SEED),
            variables: vec![McVariable {
                name: "growth".into(),
                distribution: McDistribution::Triangular {
                    min: 0.0,
                    mode: 0.05,
                    max: 0.10,
                },
            }],
        };
        let result = run_monte_carlo_simulation(&input).unwrap();
        let v = &result.result.variables[0];

        // Triangular mean = (min + mode + max) / 3
        let expected_mean = (0.0 + 0.05 + 0.10) / 3.0;
        assert!(
            (v.mean - expected_mean).abs() < 0.005,
            "mean={}, expected={}",
            v.mean,
            expected_mean
        );
        assert!(v.min >= 0.0);
        assert!(v.max <= 0.10);
    }

    #[test]
    fn test_uniform_distribution() {
        let input = MonteCarloInput {
            num_simulations: 10_000,
            seed: Some(SEED),
            variables: vec![McVariable {
                name: "rate".into(),
                distribution: McDistribution::Uniform {
                    min: 0.03,
                    max: 0.07,
                },
            }],
        };
        let result = run_monte_carlo_simulation(&input).unwrap();
        let v = &result.result.variables[0];

        // Uniform mean = (min + max) / 2
        let expected_mean = (0.03 + 0.07) / 2.0;
        assert!(
            (v.mean - expected_mean).abs() < 0.005,
            "mean={}, expected={}",
            v.mean,
            expected_mean
        );
        assert!(v.min >= 0.03);
        assert!(v.max <= 0.07);
    }

    #[test]
    fn test_multiple_variables() {
        let input = MonteCarloInput {
            num_simulations: 1_000,
            seed: Some(SEED),
            variables: vec![
                normal_var("var_a", 10.0, 2.0),
                normal_var("var_b", 50.0, 5.0),
                McVariable {
                    name: "var_c".into(),
                    distribution: McDistribution::Uniform { min: 0.0, max: 1.0 },
                },
            ],
        };
        let result = run_monte_carlo_simulation(&input).unwrap();
        assert_eq!(result.result.variables.len(), 3);
        assert_eq!(result.result.variables[0].name, "var_a");
        assert_eq!(result.result.variables[1].name, "var_b");
        assert_eq!(result.result.variables[2].name, "var_c");
    }

    #[test]
    fn test_percentile_ordering() {
        let result = run_monte_carlo_simulation(&basic_input()).unwrap();
        let p = &result.result.variables[0].percentiles;
        assert!(p.p5 <= p.p10);
        assert!(p.p10 <= p.p25);
        assert!(p.p25 <= p.p50);
        assert!(p.p50 <= p.p75);
        assert!(p.p75 <= p.p90);
        assert!(p.p90 <= p.p95);
    }

    #[test]
    fn test_histogram_bin_count() {
        let result = run_monte_carlo_simulation(&basic_input()).unwrap();
        let h = &result.result.variables[0].histogram;
        assert_eq!(h.len(), 20);
    }

    #[test]
    fn test_histogram_total_count() {
        let result = run_monte_carlo_simulation(&basic_input()).unwrap();
        let h = &result.result.variables[0].histogram;
        let total: u32 = h.iter().map(|b| b.count).sum();
        assert_eq!(total, 10_000);
    }

    #[test]
    fn test_histogram_frequency_sums_to_one() {
        let result = run_monte_carlo_simulation(&basic_input()).unwrap();
        let h = &result.result.variables[0].histogram;
        let total_freq: f64 = h.iter().map(|b| b.frequency).sum();
        assert!(
            (total_freq - 1.0).abs() < 1e-10,
            "total_freq={}",
            total_freq
        );
    }

    #[test]
    fn test_min_simulations_validation() {
        let input = MonteCarloInput {
            num_simulations: 50,
            seed: Some(SEED),
            variables: vec![normal_var("x", 0.0, 1.0)],
        };
        assert!(run_monte_carlo_simulation(&input).is_err());
    }

    #[test]
    fn test_empty_variables_validation() {
        let input = MonteCarloInput {
            num_simulations: 100,
            seed: Some(SEED),
            variables: vec![],
        };
        assert!(run_monte_carlo_simulation(&input).is_err());
    }

    #[test]
    fn test_minimum_simulations_accepted() {
        let input = MonteCarloInput {
            num_simulations: 100,
            seed: Some(SEED),
            variables: vec![normal_var("x", 0.0, 1.0)],
        };
        let result = run_monte_carlo_simulation(&input).unwrap();
        assert_eq!(result.result.num_simulations, 100);
    }

    #[test]
    fn test_single_variable() {
        let input = MonteCarloInput {
            num_simulations: 500,
            seed: Some(SEED),
            variables: vec![normal_var("only_one", 42.0, 1.0)],
        };
        let result = run_monte_carlo_simulation(&input).unwrap();
        assert_eq!(result.result.variables.len(), 1);
        assert_eq!(result.result.variables[0].name, "only_one");
    }

    #[test]
    fn test_convergence_to_analytical_mean() {
        // With many simulations, the sample mean should converge to the true mean.
        let input = MonteCarloInput {
            num_simulations: 100_000,
            seed: Some(SEED),
            variables: vec![normal_var("converge", 50.0, 5.0)],
        };
        let result = run_monte_carlo_simulation(&input).unwrap();
        let v = &result.result.variables[0];
        assert!(
            (v.mean - 50.0).abs() < 0.1,
            "mean={} should be close to 50.0",
            v.mean
        );
    }

    #[test]
    fn test_metadata_precision_field() {
        let result = run_monte_carlo_simulation(&basic_input()).unwrap();
        assert_eq!(result.metadata.precision, "ieee754_f64");
    }

    // --- DCF Monte Carlo tests ---

    fn basic_dcf_input() -> McDcfInput {
        McDcfInput {
            base_fcf: 100.0,
            projection_years: 5,
            revenue_growth: McDistribution::Normal {
                mean: 0.05,
                std_dev: 0.02,
            },
            ebitda_margin: McDistribution::Normal {
                mean: 0.20,
                std_dev: 0.03,
            },
            wacc: McDistribution::Normal {
                mean: 0.10,
                std_dev: 0.01,
            },
            terminal_growth: McDistribution::Normal {
                mean: 0.025,
                std_dev: 0.005,
            },
            capex_pct: 0.05,
            tax_rate: 0.25,
            num_simulations: 10_000,
            seed: Some(SEED),
        }
    }

    #[test]
    fn test_dcf_simulation_runs() {
        let result = run_monte_carlo_dcf(&basic_dcf_input()).unwrap();
        let out = &result.result;
        assert!(out.simulation_count > 0);
        assert!(out.ev_mean > 0.0);
        assert!(out.ev_std_dev > 0.0);
    }

    #[test]
    fn test_dcf_seeded_reproducibility() {
        let input = basic_dcf_input();
        let r1 = run_monte_carlo_dcf(&input).unwrap();
        let r2 = run_monte_carlo_dcf(&input).unwrap();
        assert_eq!(r1.result.ev_mean, r2.result.ev_mean);
        assert_eq!(r1.result.simulation_count, r2.result.simulation_count);
    }

    #[test]
    fn test_dcf_implied_range() {
        let result = run_monte_carlo_dcf(&basic_dcf_input()).unwrap();
        let (low, high) = result.result.implied_ev_range;
        assert!(low < high, "P5={low} should be < P95={high}");
        assert!(low < result.result.ev_mean);
        assert!(high > result.result.ev_mean);
    }

    #[test]
    fn test_dcf_percentile_ordering() {
        let result = run_monte_carlo_dcf(&basic_dcf_input()).unwrap();
        let p = &result.result.enterprise_values;
        assert!(p.p5 <= p.p10);
        assert!(p.p10 <= p.p25);
        assert!(p.p25 <= p.p50);
        assert!(p.p50 <= p.p75);
        assert!(p.p75 <= p.p90);
        assert!(p.p90 <= p.p95);
    }

    #[test]
    fn test_dcf_min_simulations_validation() {
        let mut input = basic_dcf_input();
        input.num_simulations = 50;
        assert!(run_monte_carlo_dcf(&input).is_err());
    }

    #[test]
    fn test_dcf_min_projection_years_validation() {
        let mut input = basic_dcf_input();
        input.projection_years = 0;
        assert!(run_monte_carlo_dcf(&input).is_err());
    }

    #[test]
    fn test_dcf_probability_above_populated() {
        let result = run_monte_carlo_dcf(&basic_dcf_input()).unwrap();
        assert!(
            !result.result.probability_above.is_empty(),
            "probability_above should be populated"
        );
        for tp in &result.result.probability_above {
            assert!(
                (0.0..=1.0).contains(&tp.probability),
                "probability should be in [0,1], got {}",
                tp.probability
            );
        }
    }

    #[test]
    fn test_dcf_skips_impossible_paths() {
        // Use distributions where terminal_growth >= wacc is likely
        let input = McDcfInput {
            base_fcf: 100.0,
            projection_years: 5,
            revenue_growth: McDistribution::Normal {
                mean: 0.05,
                std_dev: 0.01,
            },
            ebitda_margin: McDistribution::Normal {
                mean: 0.20,
                std_dev: 0.01,
            },
            wacc: McDistribution::Normal {
                mean: 0.06,
                std_dev: 0.02,
            },
            terminal_growth: McDistribution::Normal {
                mean: 0.05,
                std_dev: 0.02,
            },
            capex_pct: 0.05,
            tax_rate: 0.25,
            num_simulations: 10_000,
            seed: Some(SEED),
        };
        let result = run_monte_carlo_dcf(&input).unwrap();
        // Some simulations should have been skipped
        assert!(
            result.result.simulation_count < 10_000,
            "Expected some skipped simulations, got count={}",
            result.result.simulation_count
        );
        // Check warning
        assert!(
            result.warnings.iter().any(|w| w.contains("skipped")),
            "Expected a warning about skipped simulations"
        );
    }
}
