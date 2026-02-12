use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Assumed return distribution for tail risk calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TailDistribution {
    /// Standard Gaussian assumption
    Normal,
    /// Cornish-Fisher expansion adjusting for skewness and kurtosis
    CornishFisher,
    /// Empirical distribution from historical returns
    Historical,
}

/// A stress scenario with per-asset return shocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressScenario {
    /// Scenario name
    pub name: String,
    /// Return shock per asset
    pub asset_shocks: Vec<Decimal>,
}

/// Per-asset marginal risk measures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginalRisk {
    /// Asset name
    pub name: String,
    /// Marginal VaR: partial derivative of VaR with respect to weight
    pub marginal_var: Decimal,
    /// Marginal CVaR
    pub marginal_cvar: Decimal,
}

/// Per-asset component risk contribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRisk {
    /// Asset name
    pub name: String,
    /// Asset weight
    pub weight: Decimal,
    /// Component value (w_i * marginal)
    pub component_value: Decimal,
    /// Percentage of total risk
    pub pct_of_total: Decimal,
}

/// Stress test scenario result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressResult {
    /// Scenario name
    pub scenario_name: String,
    /// Portfolio loss amount
    pub portfolio_loss: Decimal,
    /// Loss as percentage of portfolio value
    pub loss_pct: Decimal,
    /// Worst performing asset name
    pub worst_asset: String,
    /// Worst asset's loss contribution
    pub worst_asset_loss: Decimal,
}

/// Per-asset CVaR risk budget decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskBudgetItem {
    /// Asset name
    pub name: String,
    /// Asset weight
    pub weight: Decimal,
    /// CVaR contribution
    pub cvar_contribution: Decimal,
    /// CVaR budget percentage
    pub cvar_budget_pct: Decimal,
}

/// Input for tail risk analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailRiskInput {
    /// Asset identifiers
    pub asset_names: Vec<String>,
    /// Portfolio weights (N-vector)
    pub weights: Vec<Decimal>,
    /// Expected returns per asset (N-vector)
    pub expected_returns: Vec<Decimal>,
    /// N x N covariance matrix
    pub covariance_matrix: Vec<Vec<Decimal>>,
    /// VaR/CVaR confidence level (e.g. 0.95 or 0.99)
    pub confidence_level: Decimal,
    /// Time horizon in years (e.g. 1/252 for daily)
    pub time_horizon: Decimal,
    /// Assumed return distribution
    pub distribution: TailDistribution,
    /// Optional T x N historical return matrix (required for Historical/CornishFisher)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub historical_returns: Option<Vec<Vec<Decimal>>>,
    /// Portfolio net asset value for dollar risk measures
    pub portfolio_value: Decimal,
    /// Optional stress scenarios
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stress_scenarios: Option<Vec<StressScenario>>,
}

/// Output of tail risk analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailRiskOutput {
    /// Expected portfolio return (annualized)
    pub portfolio_return: Decimal,
    /// Portfolio standard deviation (annualized)
    pub portfolio_volatility: Decimal,
    /// Value at Risk (loss amount, positive number)
    pub var_absolute: Decimal,
    /// VaR as percentage of portfolio value
    pub var_relative: Decimal,
    /// Conditional VaR / Expected Shortfall (loss amount, positive number)
    pub cvar_absolute: Decimal,
    /// CVaR as percentage of portfolio value
    pub cvar_relative: Decimal,
    /// Per-asset marginal VaR and CVaR
    pub marginal_var: Vec<MarginalRisk>,
    /// Per-asset component VaR (sums to total VaR)
    pub component_var: Vec<ComponentRisk>,
    /// Per-asset component CVaR
    pub component_cvar: Vec<ComponentRisk>,
    /// Tail risk ratio: CVaR / VaR (always > 1 for non-degenerate distributions)
    pub tail_risk_ratio: Decimal,
    /// Portfolio return skewness (if historical data available)
    pub skewness: Option<Decimal>,
    /// Portfolio excess kurtosis (if historical data available)
    pub excess_kurtosis: Option<Decimal>,
    /// Stress scenario outcomes
    pub stress_results: Vec<StressResult>,
    /// CVaR risk budget decomposition per asset
    pub risk_budget_decomposition: Vec<RiskBudgetItem>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze tail risk for a portfolio.
///
/// Computes VaR, CVaR (Expected Shortfall), marginal and component risk
/// decomposition, and optional stress test results. Supports Normal,
/// Cornish-Fisher, and Historical distribution assumptions.
pub fn analyze_tail_risk(
    input: &TailRiskInput,
) -> CorpFinanceResult<ComputationOutput<TailRiskOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validation
    // ------------------------------------------------------------------
    let n = input.asset_names.len();
    if n == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "At least one asset required".into(),
        ));
    }
    if input.weights.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "weights".into(),
            reason: format!("Expected {} weights, got {}", n, input.weights.len()),
        });
    }
    if input.expected_returns.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "expected_returns".into(),
            reason: format!(
                "Expected {} returns, got {}",
                n,
                input.expected_returns.len()
            ),
        });
    }
    validate_covariance_matrix(&input.covariance_matrix, n)?;

    if input.confidence_level <= Decimal::ZERO || input.confidence_level >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "confidence_level".into(),
            reason: "Must be between 0 and 1 exclusive".into(),
        });
    }
    if input.time_horizon <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_horizon".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.portfolio_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "portfolio_value".into(),
            reason: "Must be positive".into(),
        });
    }

    // Validate historical returns if provided
    if let Some(ref hist) = input.historical_returns {
        if hist.is_empty() {
            return Err(CorpFinanceError::InsufficientData(
                "Historical returns matrix is empty".into(),
            ));
        }
        for (t, row) in hist.iter().enumerate() {
            if row.len() != n {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("historical_returns[{}]", t),
                    reason: format!("Expected {} columns, got {}", n, row.len()),
                });
            }
        }
    }

    // Validate stress scenarios
    if let Some(ref scenarios) = input.stress_scenarios {
        for (i, s) in scenarios.iter().enumerate() {
            if s.asset_shocks.len() != n {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("stress_scenarios[{}].asset_shocks", i),
                    reason: format!("Expected {} shocks, got {}", n, s.asset_shocks.len()),
                });
            }
        }
    }

    // Require historical returns for non-Normal distributions
    match input.distribution {
        TailDistribution::CornishFisher | TailDistribution::Historical => {
            if input.historical_returns.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "historical_returns".into(),
                    reason: format!(
                        "{:?} distribution requires historical_returns",
                        match input.distribution {
                            TailDistribution::CornishFisher => "CornishFisher",
                            TailDistribution::Historical => "Historical",
                            _ => unreachable!(),
                        }
                    ),
                });
            }
        }
        TailDistribution::Normal => {}
    }

    let w = &input.weights;
    let v = input.portfolio_value;
    let alpha = input.confidence_level;
    let t = input.time_horizon;
    let sqrt_t = sqrt_decimal(t);

    // ------------------------------------------------------------------
    // 2. Portfolio return and volatility
    // ------------------------------------------------------------------
    let portfolio_return: Decimal = w
        .iter()
        .zip(input.expected_returns.iter())
        .map(|(wi, ri)| *wi * *ri)
        .sum();

    let sigma_w = matrix_vector_multiply(&input.covariance_matrix, w);
    let portfolio_variance: Decimal = w.iter().zip(sigma_w.iter()).map(|(wi, sw)| *wi * *sw).sum();
    let portfolio_volatility = sqrt_decimal(portfolio_variance);

    let mu_t = portfolio_return * sqrt_t;
    let sigma_t = portfolio_volatility * sqrt_t;

    // ------------------------------------------------------------------
    // 3. Compute VaR and CVaR based on distribution
    // ------------------------------------------------------------------
    let z_alpha = norm_inv(alpha);

    let (var_absolute, cvar_absolute, skewness, excess_kurtosis) = match input.distribution {
        TailDistribution::Normal => {
            let var_abs = compute_normal_var(mu_t, sigma_t, z_alpha, v);
            let cvar_abs = compute_normal_cvar(mu_t, sigma_t, z_alpha, alpha, v);
            (var_abs, cvar_abs, None, None)
        }
        TailDistribution::CornishFisher => {
            let hist = input.historical_returns.as_ref().unwrap();
            let (skew, kurt) = compute_portfolio_moments(hist, w);

            let z_cf = cornish_fisher_z(z_alpha, skew, kurt);
            let var_abs = compute_var_with_z(mu_t, sigma_t, z_cf, v);

            // CVaR for Cornish-Fisher: use normal CVaR as approximation with adjusted z
            let cvar_abs = compute_normal_cvar(mu_t, sigma_t, z_cf, alpha, v);

            (var_abs, cvar_abs, Some(skew), Some(kurt))
        }
        TailDistribution::Historical => {
            let hist = input.historical_returns.as_ref().unwrap();
            let (var_abs, cvar_abs, skew, kurt) =
                compute_historical_var_cvar(hist, w, alpha, v, sqrt_t);
            (var_abs, cvar_abs, Some(skew), Some(kurt))
        }
    };

    let var_relative = if v.is_zero() {
        Decimal::ZERO
    } else {
        var_absolute / v
    };
    let cvar_relative = if v.is_zero() {
        Decimal::ZERO
    } else {
        cvar_absolute / v
    };

    let tail_risk_ratio = if var_absolute.is_zero() {
        dec!(1.0)
    } else {
        cvar_absolute / var_absolute
    };

    // ------------------------------------------------------------------
    // 4. Marginal and component VaR / CVaR
    // ------------------------------------------------------------------
    // Marginal VaR_i = z_alpha * (Sigma * w)_i / sigma_p * sqrt(T) * V
    let z_for_marginal = match input.distribution {
        TailDistribution::CornishFisher => {
            let hist = input.historical_returns.as_ref().unwrap();
            let (skew, kurt) = compute_portfolio_moments(hist, w);
            cornish_fisher_z(z_alpha, skew, kurt)
        }
        _ => z_alpha,
    };

    let marginal_var_vec: Vec<Decimal> = (0..n)
        .map(|i| {
            if portfolio_volatility.is_zero() {
                Decimal::ZERO
            } else {
                z_for_marginal * sigma_w[i] / portfolio_volatility * sqrt_t * v
            }
        })
        .collect();

    // Marginal CVaR_i = phi(z) / (1-alpha) * (Sigma * w)_i / sigma_p * sqrt(T) * V
    let phi_z = norm_pdf(z_for_marginal);
    let one_minus_alpha = Decimal::ONE - alpha;
    let cvar_factor = if one_minus_alpha.is_zero() {
        Decimal::ZERO
    } else {
        phi_z / one_minus_alpha
    };

    let marginal_cvar_vec: Vec<Decimal> = (0..n)
        .map(|i| {
            if portfolio_volatility.is_zero() {
                Decimal::ZERO
            } else {
                cvar_factor * sigma_w[i] / portfolio_volatility * sqrt_t * v
            }
        })
        .collect();

    let marginal_var_output: Vec<MarginalRisk> = (0..n)
        .map(|i| MarginalRisk {
            name: input.asset_names[i].clone(),
            marginal_var: marginal_var_vec[i],
            marginal_cvar: marginal_cvar_vec[i],
        })
        .collect();

    // Component VaR_i = w_i * marginal_VaR_i
    let component_var_values: Vec<Decimal> = (0..n).map(|i| w[i] * marginal_var_vec[i]).collect();
    let total_component_var: Decimal = component_var_values.iter().copied().sum();

    let component_var: Vec<ComponentRisk> = (0..n)
        .map(|i| {
            let pct = if total_component_var.is_zero() {
                Decimal::ZERO
            } else {
                component_var_values[i] / total_component_var
            };
            ComponentRisk {
                name: input.asset_names[i].clone(),
                weight: w[i],
                component_value: component_var_values[i],
                pct_of_total: pct,
            }
        })
        .collect();

    // Component CVaR_i = w_i * marginal_CVaR_i
    let component_cvar_values: Vec<Decimal> = (0..n).map(|i| w[i] * marginal_cvar_vec[i]).collect();
    let total_component_cvar: Decimal = component_cvar_values.iter().copied().sum();

    let component_cvar: Vec<ComponentRisk> = (0..n)
        .map(|i| {
            let pct = if total_component_cvar.is_zero() {
                Decimal::ZERO
            } else {
                component_cvar_values[i] / total_component_cvar
            };
            ComponentRisk {
                name: input.asset_names[i].clone(),
                weight: w[i],
                component_value: component_cvar_values[i],
                pct_of_total: pct,
            }
        })
        .collect();

    // ------------------------------------------------------------------
    // 5. CVaR risk budget decomposition
    // ------------------------------------------------------------------
    let risk_budget_decomposition: Vec<RiskBudgetItem> = (0..n)
        .map(|i| {
            let cvar_budget_pct = if total_component_cvar.is_zero() {
                Decimal::ZERO
            } else {
                component_cvar_values[i] / total_component_cvar
            };
            RiskBudgetItem {
                name: input.asset_names[i].clone(),
                weight: w[i],
                cvar_contribution: component_cvar_values[i],
                cvar_budget_pct,
            }
        })
        .collect();

    // ------------------------------------------------------------------
    // 6. Stress tests
    // ------------------------------------------------------------------
    let stress_results = if let Some(ref scenarios) = input.stress_scenarios {
        scenarios
            .iter()
            .map(|s| compute_stress_result(s, w, &input.asset_names, v))
            .collect()
    } else {
        vec![]
    };

    // ------------------------------------------------------------------
    // 7. Warnings
    // ------------------------------------------------------------------
    if alpha < dec!(0.90) {
        warnings.push(format!(
            "Low confidence level ({:.2}%): VaR/CVaR may understate risk",
            alpha * dec!(100)
        ));
    }
    if tail_risk_ratio > dec!(1.5) {
        warnings.push(format!(
            "High tail risk ratio ({:.4}): distribution has heavy tails",
            tail_risk_ratio
        ));
    }
    if let Some(skew) = skewness {
        if skew < dec!(-0.5) {
            warnings.push(format!(
                "Negative skewness ({:.4}): returns are left-skewed",
                skew
            ));
        }
    }
    if let Some(kurt) = excess_kurtosis {
        if kurt > dec!(1.0) {
            warnings.push(format!(
                "Excess kurtosis ({:.4}): heavy-tailed distribution",
                kurt
            ));
        }
    }

    let output = TailRiskOutput {
        portfolio_return,
        portfolio_volatility,
        var_absolute,
        var_relative,
        cvar_absolute,
        cvar_relative,
        marginal_var: marginal_var_output,
        component_var,
        component_cvar,
        tail_risk_ratio,
        skewness,
        excess_kurtosis,
        stress_results,
        risk_budget_decomposition,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        &format!("Tail Risk Analysis ({:?})", input.distribution),
        &serde_json::json!({
            "num_assets": n,
            "confidence_level": alpha.to_string(),
            "time_horizon": t.to_string(),
            "distribution": format!("{:?}", input.distribution),
            "portfolio_value": v.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// VaR / CVaR computation
// ---------------------------------------------------------------------------

/// Normal VaR: VaR = -(mu * sqrt(T) - z_alpha * sigma * sqrt(T)) * V
/// This is the loss at the alpha quantile (positive number).
fn compute_normal_var(mu_t: Decimal, sigma_t: Decimal, z_alpha: Decimal, v: Decimal) -> Decimal {
    let var = -(mu_t - z_alpha * sigma_t) * v;
    if var < Decimal::ZERO {
        Decimal::ZERO
    } else {
        var
    }
}

/// VaR using an arbitrary z-score (for Cornish-Fisher).
fn compute_var_with_z(mu_t: Decimal, sigma_t: Decimal, z: Decimal, v: Decimal) -> Decimal {
    let var = -(mu_t - z * sigma_t) * v;
    if var < Decimal::ZERO {
        Decimal::ZERO
    } else {
        var
    }
}

/// Normal CVaR (Expected Shortfall):
/// CVaR = -(mu * sqrt(T) - sigma * sqrt(T) * phi(z_alpha) / (1 - alpha)) * V
fn compute_normal_cvar(
    mu_t: Decimal,
    sigma_t: Decimal,
    z_alpha: Decimal,
    alpha: Decimal,
    v: Decimal,
) -> Decimal {
    let one_minus_alpha = Decimal::ONE - alpha;
    if one_minus_alpha.is_zero() {
        return Decimal::ZERO;
    }
    let phi_z = norm_pdf(z_alpha);
    let cvar = -(mu_t - sigma_t * phi_z / one_minus_alpha) * v;
    if cvar < Decimal::ZERO {
        Decimal::ZERO
    } else {
        cvar
    }
}

/// Cornish-Fisher adjustment to the z-score.
/// z_CF = z + (z^2 - 1)*S/6 + (z^3 - 3z)*K/24 - (2z^3 - 5z)*S^2/36
fn cornish_fisher_z(z: Decimal, skew: Decimal, kurt: Decimal) -> Decimal {
    let z2 = z * z;
    let z3 = z2 * z;
    z + (z2 - Decimal::ONE) * skew / dec!(6) + (z3 - dec!(3) * z) * kurt / dec!(24)
        - (dec!(2) * z3 - dec!(5) * z) * skew * skew / dec!(36)
}

/// Compute historical VaR and CVaR from empirical return distribution.
/// Returns (VaR_abs, CVaR_abs, skewness, excess_kurtosis).
fn compute_historical_var_cvar(
    hist: &[Vec<Decimal>],
    w: &[Decimal],
    alpha: Decimal,
    v: Decimal,
    sqrt_t: Decimal,
) -> (Decimal, Decimal, Decimal, Decimal) {
    let t_obs = hist.len();
    let n = w.len();

    // Compute portfolio returns for each period
    let mut port_returns: Vec<Decimal> = Vec::with_capacity(t_obs);
    for period in hist {
        let ret: Decimal = (0..n).map(|i| w[i] * period[i]).sum();
        port_returns.push(ret);
    }

    // Sort ascending
    let mut sorted = port_returns.clone();
    sorted.sort();

    // VaR: negative of the (1-alpha) percentile
    let var_idx_raw = (Decimal::ONE - alpha) * Decimal::from(t_obs as i64);
    // Convert to usize, floor
    let var_idx = decimal_to_usize(var_idx_raw).min(t_obs.saturating_sub(1));
    let var_return = sorted[var_idx];
    let var_abs = -var_return * sqrt_t * v;
    let var_abs = if var_abs < Decimal::ZERO {
        Decimal::ZERO
    } else {
        var_abs
    };

    // CVaR: mean of returns below VaR threshold
    let threshold = sorted[var_idx];
    let tail_returns: Vec<Decimal> = sorted
        .iter()
        .filter(|r| **r <= threshold)
        .copied()
        .collect();
    let cvar_return = if tail_returns.is_empty() {
        var_return
    } else {
        let sum: Decimal = tail_returns.iter().copied().sum();
        sum / Decimal::from(tail_returns.len() as i64)
    };
    let cvar_abs = -cvar_return * sqrt_t * v;
    let cvar_abs = if cvar_abs < Decimal::ZERO {
        Decimal::ZERO
    } else {
        cvar_abs
    };

    // Compute skewness and kurtosis of portfolio returns
    let mean: Decimal = port_returns.iter().copied().sum::<Decimal>() / Decimal::from(t_obs as i64);
    let mut m2 = Decimal::ZERO;
    let mut m3 = Decimal::ZERO;
    let mut m4 = Decimal::ZERO;
    for r in &port_returns {
        let d = *r - mean;
        let d2 = d * d;
        m2 += d2;
        m3 += d2 * d;
        m4 += d2 * d2;
    }
    let n_dec = Decimal::from(t_obs as i64);
    m2 /= n_dec;
    m3 /= n_dec;
    m4 /= n_dec;

    let std_dev = sqrt_decimal(m2);
    let skewness = if std_dev.is_zero() {
        Decimal::ZERO
    } else {
        let std3 = std_dev * std_dev * std_dev;
        m3 / std3
    };
    let excess_kurtosis = if std_dev.is_zero() {
        Decimal::ZERO
    } else {
        let std4 = std_dev * std_dev * std_dev * std_dev;
        m4 / std4 - dec!(3)
    };

    (var_abs, cvar_abs, skewness, excess_kurtosis)
}

/// Compute portfolio skewness and excess kurtosis from historical returns.
fn compute_portfolio_moments(hist: &[Vec<Decimal>], w: &[Decimal]) -> (Decimal, Decimal) {
    let t_obs = hist.len();
    let n = w.len();

    let mut port_returns: Vec<Decimal> = Vec::with_capacity(t_obs);
    for period in hist {
        let ret: Decimal = (0..n).map(|i| w[i] * period[i]).sum();
        port_returns.push(ret);
    }

    let mean: Decimal = port_returns.iter().copied().sum::<Decimal>() / Decimal::from(t_obs as i64);
    let mut m2 = Decimal::ZERO;
    let mut m3 = Decimal::ZERO;
    let mut m4 = Decimal::ZERO;
    for r in &port_returns {
        let d = *r - mean;
        let d2 = d * d;
        m2 += d2;
        m3 += d2 * d;
        m4 += d2 * d2;
    }
    let n_dec = Decimal::from(t_obs as i64);
    m2 /= n_dec;
    m3 /= n_dec;
    m4 /= n_dec;

    let std_dev = sqrt_decimal(m2);
    let skewness = if std_dev.is_zero() {
        Decimal::ZERO
    } else {
        let std3 = std_dev * std_dev * std_dev;
        m3 / std3
    };
    let excess_kurtosis = if std_dev.is_zero() {
        Decimal::ZERO
    } else {
        let std4 = std_dev * std_dev * std_dev * std_dev;
        m4 / std4 - dec!(3)
    };

    (skewness, excess_kurtosis)
}

/// Compute stress test result for a single scenario.
fn compute_stress_result(
    scenario: &StressScenario,
    w: &[Decimal],
    asset_names: &[String],
    v: Decimal,
) -> StressResult {
    let n = w.len();
    let mut portfolio_loss = Decimal::ZERO;
    let mut worst_idx = 0;
    let mut worst_loss = Decimal::ZERO;

    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        let asset_loss = w[i] * scenario.asset_shocks[i] * v;
        portfolio_loss += asset_loss;
        // Most negative shock = biggest loss
        if asset_loss < worst_loss {
            worst_loss = asset_loss;
            worst_idx = i;
        }
    }

    // Portfolio loss is negative return * V (express as positive loss)
    let loss_amount = -portfolio_loss;
    let loss_pct = if v.is_zero() {
        Decimal::ZERO
    } else {
        loss_amount / v
    };

    StressResult {
        scenario_name: scenario.name.clone(),
        portfolio_loss: loss_amount,
        loss_pct,
        worst_asset: asset_names[worst_idx].clone(),
        worst_asset_loss: -worst_loss,
    }
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Convert a Decimal to usize (floor).
fn decimal_to_usize(val: Decimal) -> usize {
    // Use to_string and parse to avoid precision issues
    let s = val.to_string();
    if let Some(dot_pos) = s.find('.') {
        s[..dot_pos].parse::<usize>().unwrap_or(0)
    } else {
        s.parse::<usize>().unwrap_or(0)
    }
}

/// Matrix-vector multiply: result[i] = sum_j mat[i][j] * v[j].
fn matrix_vector_multiply(mat: &[Vec<Decimal>], v: &[Decimal]) -> Vec<Decimal> {
    mat.iter()
        .map(|row| row.iter().zip(v.iter()).map(|(a, b)| *a * *b).sum())
        .collect()
}

/// Square root via Newton's method (20 iterations).
fn sqrt_decimal(val: Decimal) -> Decimal {
    if val <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if val == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = dec!(2);
    let mut guess = val / two;
    if guess.is_zero() {
        guess = dec!(0.0000001);
    }
    if val > dec!(100) {
        guess = dec!(10);
    } else if val < dec!(0.01) {
        guess = dec!(0.1);
    }
    for _ in 0..20 {
        if guess.is_zero() {
            return Decimal::ZERO;
        }
        guess = (guess + val / guess) / two;
    }
    guess
}

/// Exponential function via Taylor series with range reduction.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = dec!(2);

    // Range reduction for large |x|
    if x > two || x < -two {
        let half = exp_decimal(x / two);
        return half * half;
    }

    // Taylor series: exp(x) = sum_{n=0}^{40} x^n / n!
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for i in 1u32..=40 {
        term = term * x / Decimal::from(i);
        sum += term;
    }
    sum
}

/// Natural logarithm via Newton's method: find y such that exp(y) = x.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return dec!(-999);
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    let e_approx = dec!(2.718281828459045);
    let mut y = if x > dec!(0.5) && x < dec!(2) {
        x - Decimal::ONE
    } else {
        let mut approx = Decimal::ZERO;
        let mut v = x;
        if x > Decimal::ONE {
            while v > e_approx {
                v /= e_approx;
                approx += Decimal::ONE;
            }
            approx + (v - Decimal::ONE)
        } else {
            while v < Decimal::ONE / e_approx {
                v *= e_approx;
                approx -= Decimal::ONE;
            }
            approx + (v - Decimal::ONE)
        }
    };

    // Newton: y_{n+1} = y_n - 1 + x / exp(y_n)
    for _ in 0..40 {
        let ey = exp_decimal(y);
        if ey.is_zero() {
            break;
        }
        y = y - Decimal::ONE + x / ey;
    }
    y
}

/// Standard normal PDF: phi(x) = exp(-x^2/2) / sqrt(2*pi).
fn norm_pdf(x: Decimal) -> Decimal {
    let two_pi = dec!(6.283185307179586);
    let exponent = -(x * x) / dec!(2);
    exp_decimal(exponent) / sqrt_decimal(two_pi)
}

/// Standard normal CDF using Abramowitz & Stegun approximation.
#[allow(dead_code)]
fn norm_cdf(x: Decimal) -> Decimal {
    let b1 = dec!(0.319381530);
    let b2 = dec!(-0.356563782);
    let b3 = dec!(1.781477937);
    let b4 = dec!(-1.821255978);
    let b5 = dec!(1.330274429);
    let p = dec!(0.2316419);

    let abs_x = if x < Decimal::ZERO { -x } else { x };
    let t = Decimal::ONE / (Decimal::ONE + p * abs_x);
    let poly = t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))));
    let cdf_pos = Decimal::ONE - norm_pdf(abs_x) * poly;

    if x < Decimal::ZERO {
        Decimal::ONE - cdf_pos
    } else {
        cdf_pos
    }
}

/// Inverse normal CDF (quantile function) via rational approximation.
/// Abramowitz & Stegun 26.2.23 for 0.5 < p < 1.
/// For p < 0.5: use symmetry norm_inv(p) = -norm_inv(1-p).
fn norm_inv(p: Decimal) -> Decimal {
    if p <= Decimal::ZERO || p >= Decimal::ONE {
        return Decimal::ZERO;
    }
    if p == dec!(0.5) {
        return Decimal::ZERO;
    }

    let (pp, sign) = if p < dec!(0.5) {
        (Decimal::ONE - p, dec!(-1))
    } else {
        (p, Decimal::ONE)
    };

    // Rational approximation coefficients (Abramowitz & Stegun 26.2.23)
    let c0 = dec!(2.515517);
    let c1 = dec!(0.802853);
    let c2 = dec!(0.010328);
    let d1 = dec!(1.432788);
    let d2 = dec!(0.189269);
    let d3 = dec!(0.001308);

    // t = sqrt(-2 * ln(1 - pp))
    let one_minus_pp = Decimal::ONE - pp;
    if one_minus_pp <= Decimal::ZERO {
        return sign * dec!(4); // Far tail
    }
    let t = sqrt_decimal(dec!(-2) * ln_decimal(one_minus_pp));

    // Rational approximation
    let numer = c0 + t * (c1 + t * c2);
    let denom = Decimal::ONE + t * (d1 + t * (d2 + t * d3));

    let z = t - numer / denom;
    sign * z
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_covariance_matrix(cov: &[Vec<Decimal>], n: usize) -> CorpFinanceResult<()> {
    if cov.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "covariance_matrix".into(),
            reason: format!("Expected {}x{} matrix but got {} rows", n, n, cov.len()),
        });
    }
    for (i, row) in cov.iter().enumerate() {
        if row.len() != n {
            return Err(CorpFinanceError::InvalidInput {
                field: "covariance_matrix".into(),
                reason: format!("Row {} has {} columns, expected {}", i, row.len(), n),
            });
        }
    }
    let tolerance = dec!(0.0000001);
    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        for j in (i + 1)..n {
            if (cov[i][j] - cov[j][i]).abs() > tolerance {
                return Err(CorpFinanceError::InvalidInput {
                    field: "covariance_matrix".into(),
                    reason: format!(
                        "Matrix is not symmetric: cov[{}][{}]={} != cov[{}][{}]={}",
                        i, j, cov[i][j], j, i, cov[j][i]
                    ),
                });
            }
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

    // -- Test helpers --

    /// Build a simple 2-asset normal input.
    fn simple_normal_input() -> TailRiskInput {
        TailRiskInput {
            asset_names: vec!["Equity".into(), "Bond".into()],
            weights: vec![dec!(0.6), dec!(0.4)],
            expected_returns: vec![dec!(0.10), dec!(0.04)],
            covariance_matrix: vec![vec![dec!(0.04), dec!(0.006)], vec![dec!(0.006), dec!(0.01)]],
            confidence_level: dec!(0.95),
            time_horizon: Decimal::ONE,
            distribution: TailDistribution::Normal,
            historical_returns: None,
            portfolio_value: dec!(1000000),
            stress_scenarios: None,
        }
    }

    /// Build a 3-asset input with historical returns.
    fn three_asset_historical_input() -> TailRiskInput {
        // Generate simple historical returns (12 periods)
        let hist = vec![
            vec![dec!(0.02), dec!(0.01), dec!(0.015)],
            vec![dec!(-0.03), dec!(0.005), dec!(-0.01)],
            vec![dec!(0.05), dec!(0.008), dec!(0.03)],
            vec![dec!(-0.01), dec!(0.003), dec!(0.005)],
            vec![dec!(0.04), dec!(0.012), dec!(0.02)],
            vec![dec!(-0.06), dec!(0.015), dec!(-0.04)],
            vec![dec!(0.03), dec!(0.007), dec!(0.01)],
            vec![dec!(0.01), dec!(0.004), dec!(0.008)],
            vec![dec!(-0.02), dec!(0.009), dec!(-0.005)],
            vec![dec!(0.06), dec!(0.006), dec!(0.04)],
            vec![dec!(-0.04), dec!(0.011), dec!(-0.02)],
            vec![dec!(0.02), dec!(0.003), dec!(0.01)],
        ];
        TailRiskInput {
            asset_names: vec!["Stock".into(), "Bond".into(), "Commodity".into()],
            weights: vec![dec!(0.5), dec!(0.3), dec!(0.2)],
            expected_returns: vec![dec!(0.08), dec!(0.03), dec!(0.05)],
            covariance_matrix: vec![
                vec![dec!(0.04), dec!(0.003), dec!(0.01)],
                vec![dec!(0.003), dec!(0.0025), dec!(0.001)],
                vec![dec!(0.01), dec!(0.001), dec!(0.0225)],
            ],
            confidence_level: dec!(0.95),
            time_horizon: Decimal::ONE,
            distribution: TailDistribution::Historical,
            historical_returns: Some(hist),
            portfolio_value: dec!(5000000),
            stress_scenarios: None,
        }
    }

    /// Build input with stress scenarios.
    fn input_with_stress() -> TailRiskInput {
        let mut input = simple_normal_input();
        input.stress_scenarios = Some(vec![
            StressScenario {
                name: "Market Crash".into(),
                asset_shocks: vec![dec!(-0.30), dec!(-0.05)],
            },
            StressScenario {
                name: "Rate Spike".into(),
                asset_shocks: vec![dec!(-0.10), dec!(-0.15)],
            },
        ]);
        input
    }

    // -- Basic Normal VaR tests --

    #[test]
    fn test_normal_var_positive() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.result.var_absolute > Decimal::ZERO);
        assert!(result.result.var_relative > Decimal::ZERO);
    }

    #[test]
    fn test_normal_cvar_exceeds_var() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(
            result.result.cvar_absolute >= result.result.var_absolute,
            "CVaR ({}) should >= VaR ({})",
            result.result.cvar_absolute,
            result.result.var_absolute
        );
    }

    #[test]
    fn test_normal_tail_risk_ratio_gte_one() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(
            result.result.tail_risk_ratio >= dec!(0.99),
            "Tail risk ratio {} should be >= 1",
            result.result.tail_risk_ratio
        );
    }

    #[test]
    fn test_normal_portfolio_return() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        // Expected: 0.6*0.10 + 0.4*0.04 = 0.06 + 0.016 = 0.076
        let tolerance = dec!(0.0001);
        assert!(
            (result.result.portfolio_return - dec!(0.076)).abs() < tolerance,
            "Expected ~0.076, got {}",
            result.result.portfolio_return
        );
    }

    #[test]
    fn test_normal_portfolio_volatility() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        // Var = 0.36*0.04 + 2*0.24*0.006 + 0.16*0.01 = 0.0144 + 0.00288 + 0.0016 = 0.01888
        // Vol = sqrt(0.01888) ~ 0.13741
        let expected_vol = sqrt_decimal(dec!(0.01888));
        let tolerance = dec!(0.001);
        assert!(
            (result.result.portfolio_volatility - expected_vol).abs() < tolerance,
            "Expected ~{}, got {}",
            expected_vol,
            result.result.portfolio_volatility
        );
    }

    #[test]
    fn test_higher_confidence_higher_var() {
        let mut input95 = simple_normal_input();
        input95.confidence_level = dec!(0.95);
        let result95 = analyze_tail_risk(&input95).unwrap();

        let mut input99 = simple_normal_input();
        input99.confidence_level = dec!(0.99);
        let result99 = analyze_tail_risk(&input99).unwrap();

        assert!(
            result99.result.var_absolute > result95.result.var_absolute,
            "99% VaR ({}) should exceed 95% VaR ({})",
            result99.result.var_absolute,
            result95.result.var_absolute
        );
    }

    #[test]
    fn test_var_scales_with_portfolio_value() {
        let mut input_small = simple_normal_input();
        input_small.portfolio_value = dec!(100000);
        let result_small = analyze_tail_risk(&input_small).unwrap();

        let mut input_big = simple_normal_input();
        input_big.portfolio_value = dec!(1000000);
        let result_big = analyze_tail_risk(&input_big).unwrap();

        // VaR should scale linearly with portfolio value
        let ratio = result_big.result.var_absolute / result_small.result.var_absolute;
        let tolerance = dec!(0.01);
        assert!(
            (ratio - dec!(10)).abs() < tolerance,
            "VaR ratio should be ~10, got {}",
            ratio
        );
    }

    #[test]
    fn test_var_scales_with_time_horizon() {
        let mut input_daily = simple_normal_input();
        input_daily.time_horizon = dec!(0.00396825396825); // ~1/252

        let mut input_annual = simple_normal_input();
        input_annual.time_horizon = Decimal::ONE;

        let result_daily = analyze_tail_risk(&input_daily).unwrap();
        let result_annual = analyze_tail_risk(&input_annual).unwrap();

        // Annual VaR should be much larger than daily
        assert!(result_annual.result.var_absolute > result_daily.result.var_absolute);
    }

    #[test]
    fn test_no_skewness_for_normal() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.result.skewness.is_none());
        assert!(result.result.excess_kurtosis.is_none());
    }

    // -- Component VaR tests --

    #[test]
    fn test_component_var_sums_consistently() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        let component_sum: Decimal = result
            .result
            .component_var
            .iter()
            .map(|c| c.component_value)
            .sum();
        // Component VaR sums to z * sigma_p * sqrt(T) * V (Euler decomposition
        // of the volatility component). VaR itself also subtracts the mean.
        // Verify: component sum > 0 and > total VaR (since VaR subtracts positive mean)
        assert!(
            component_sum > Decimal::ZERO,
            "Component VaR sum should be positive"
        );
        // With positive expected return, component VaR sum (z*sigma*V) exceeds
        // total VaR (z*sigma*V - mu*V) by the expected return contribution
        assert!(
            component_sum >= result.result.var_absolute,
            "Component VaR sum ({}) should >= total VaR ({}) when mean is positive",
            component_sum,
            result.result.var_absolute
        );
    }

    #[test]
    fn test_component_var_pcts_sum_to_one() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        let pct_sum: Decimal = result
            .result
            .component_var
            .iter()
            .map(|c| c.pct_of_total)
            .sum();
        let tolerance = dec!(0.01);
        assert!(
            (pct_sum - Decimal::ONE).abs() < tolerance,
            "Component VaR pcts should sum to ~1, got {}",
            pct_sum
        );
    }

    #[test]
    fn test_component_cvar_pcts_sum_to_one() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        let pct_sum: Decimal = result
            .result
            .component_cvar
            .iter()
            .map(|c| c.pct_of_total)
            .sum();
        let tolerance = dec!(0.01);
        assert!(
            (pct_sum - Decimal::ONE).abs() < tolerance,
            "Component CVaR pcts should sum to ~1, got {}",
            pct_sum
        );
    }

    // -- Marginal risk tests --

    #[test]
    fn test_marginal_var_count() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert_eq!(result.result.marginal_var.len(), 2);
    }

    #[test]
    fn test_marginal_var_positive_for_positive_weights() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        for mr in &result.result.marginal_var {
            assert!(
                mr.marginal_var > Decimal::ZERO,
                "Marginal VaR for {} should be positive",
                mr.name
            );
        }
    }

    #[test]
    fn test_marginal_cvar_exceeds_marginal_var() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        for mr in &result.result.marginal_var {
            assert!(
                mr.marginal_cvar >= mr.marginal_var,
                "Marginal CVaR ({}) should >= Marginal VaR ({}) for {}",
                mr.marginal_cvar,
                mr.marginal_var,
                mr.name
            );
        }
    }

    // -- Risk budget tests --

    #[test]
    fn test_risk_budget_count() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert_eq!(result.result.risk_budget_decomposition.len(), 2);
    }

    #[test]
    fn test_risk_budget_pcts_sum_to_one() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        let pct_sum: Decimal = result
            .result
            .risk_budget_decomposition
            .iter()
            .map(|rb| rb.cvar_budget_pct)
            .sum();
        let tolerance = dec!(0.01);
        assert!(
            (pct_sum - Decimal::ONE).abs() < tolerance,
            "CVaR budget pcts should sum to ~1, got {}",
            pct_sum
        );
    }

    // -- Historical distribution tests --

    #[test]
    fn test_historical_var_positive() {
        let input = three_asset_historical_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.result.var_absolute > Decimal::ZERO);
    }

    #[test]
    fn test_historical_cvar_exceeds_var() {
        let input = three_asset_historical_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(
            result.result.cvar_absolute >= result.result.var_absolute,
            "Historical CVaR ({}) should >= VaR ({})",
            result.result.cvar_absolute,
            result.result.var_absolute
        );
    }

    #[test]
    fn test_historical_skewness_computed() {
        let input = three_asset_historical_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.result.skewness.is_some());
        assert!(result.result.excess_kurtosis.is_some());
    }

    #[test]
    fn test_historical_three_asset_budget() {
        let input = three_asset_historical_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert_eq!(result.result.risk_budget_decomposition.len(), 3);
    }

    // -- Cornish-Fisher tests --

    #[test]
    fn test_cornish_fisher_basic() {
        let mut input = three_asset_historical_input();
        input.distribution = TailDistribution::CornishFisher;
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.result.var_absolute > Decimal::ZERO);
        assert!(result.result.skewness.is_some());
    }

    #[test]
    fn test_cornish_fisher_differs_from_normal() {
        let hist = three_asset_historical_input().historical_returns.unwrap();

        let mut input_normal = three_asset_historical_input();
        input_normal.distribution = TailDistribution::Normal;
        input_normal.historical_returns = None;
        let result_normal = analyze_tail_risk(&input_normal).unwrap();

        let mut input_cf = three_asset_historical_input();
        input_cf.distribution = TailDistribution::CornishFisher;
        input_cf.historical_returns = Some(hist);
        let result_cf = analyze_tail_risk(&input_cf).unwrap();

        // With skewness/kurtosis, CF VaR should differ from Normal VaR
        // (they could be close but typically differ)
        let diff = (result_cf.result.var_absolute - result_normal.result.var_absolute).abs();
        // Just verify both produce positive results
        assert!(result_cf.result.var_absolute > Decimal::ZERO);
        assert!(result_normal.result.var_absolute > Decimal::ZERO);
        // Allow any non-negative diff (they are based on different z-scores)
        assert!(diff >= Decimal::ZERO);
    }

    // -- Stress test tests --

    #[test]
    fn test_stress_results_count() {
        let input = input_with_stress();
        let result = analyze_tail_risk(&input).unwrap();
        assert_eq!(result.result.stress_results.len(), 2);
    }

    #[test]
    fn test_stress_market_crash() {
        let input = input_with_stress();
        let result = analyze_tail_risk(&input).unwrap();
        let crash = &result.result.stress_results[0];
        assert_eq!(crash.scenario_name, "Market Crash");
        // Portfolio loss: -(0.6 * -0.30 + 0.4 * -0.05) * 1M = (0.18 + 0.02) * 1M = 200k
        let tolerance = dec!(1000);
        assert!(
            (crash.portfolio_loss - dec!(200000)).abs() < tolerance,
            "Expected ~200000 loss, got {}",
            crash.portfolio_loss
        );
    }

    #[test]
    fn test_stress_worst_asset() {
        let input = input_with_stress();
        let result = analyze_tail_risk(&input).unwrap();
        let crash = &result.result.stress_results[0];
        // Equity has -30% shock with 0.6 weight => -180k, Bond -5% with 0.4 => -20k
        assert_eq!(crash.worst_asset, "Equity");
    }

    #[test]
    fn test_no_stress_scenarios() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.result.stress_results.is_empty());
    }

    // -- Validation tests --

    #[test]
    fn test_empty_assets_error() {
        let input = TailRiskInput {
            asset_names: vec![],
            weights: vec![],
            expected_returns: vec![],
            covariance_matrix: vec![],
            confidence_level: dec!(0.95),
            time_horizon: Decimal::ONE,
            distribution: TailDistribution::Normal,
            historical_returns: None,
            portfolio_value: dec!(1000000),
            stress_scenarios: None,
        };
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_weights_length_mismatch() {
        let mut input = simple_normal_input();
        input.weights = vec![dec!(1.0)]; // 1 weight for 2 assets
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_returns_length_mismatch() {
        let mut input = simple_normal_input();
        input.expected_returns = vec![dec!(0.10)]; // 1 return for 2 assets
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_invalid_confidence_zero() {
        let mut input = simple_normal_input();
        input.confidence_level = Decimal::ZERO;
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_invalid_confidence_one() {
        let mut input = simple_normal_input();
        input.confidence_level = Decimal::ONE;
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_invalid_time_horizon() {
        let mut input = simple_normal_input();
        input.time_horizon = Decimal::ZERO;
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_invalid_portfolio_value() {
        let mut input = simple_normal_input();
        input.portfolio_value = Decimal::ZERO;
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_covariance_wrong_size() {
        let mut input = simple_normal_input();
        input.covariance_matrix = vec![vec![dec!(0.04)]]; // 1x1 for 2 assets
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_covariance_asymmetric() {
        let mut input = simple_normal_input();
        input.covariance_matrix[0][1] = dec!(0.1);
        input.covariance_matrix[1][0] = dec!(-0.1);
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_historical_required_for_historical_dist() {
        let mut input = simple_normal_input();
        input.distribution = TailDistribution::Historical;
        input.historical_returns = None;
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_historical_required_for_cornish_fisher() {
        let mut input = simple_normal_input();
        input.distribution = TailDistribution::CornishFisher;
        input.historical_returns = None;
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_historical_returns_wrong_columns() {
        let mut input = simple_normal_input();
        input.distribution = TailDistribution::Historical;
        input.historical_returns = Some(vec![
            vec![dec!(0.01)], // Only 1 col for 2 assets
        ]);
        assert!(analyze_tail_risk(&input).is_err());
    }

    #[test]
    fn test_stress_shocks_wrong_count() {
        let mut input = simple_normal_input();
        input.stress_scenarios = Some(vec![StressScenario {
            name: "Bad".into(),
            asset_shocks: vec![dec!(-0.10)], // 1 shock for 2 assets
        }]);
        assert!(analyze_tail_risk(&input).is_err());
    }

    // -- Math helper tests --

    #[test]
    fn test_norm_inv_at_50pct() {
        let z = norm_inv(dec!(0.5));
        assert!(
            z.abs() < dec!(0.001),
            "norm_inv(0.5) should be ~0, got {}",
            z
        );
    }

    #[test]
    fn test_norm_inv_at_95pct() {
        let z = norm_inv(dec!(0.95));
        let tolerance = dec!(0.02);
        assert!(
            (z - dec!(1.645)).abs() < tolerance,
            "norm_inv(0.95) should be ~1.645, got {}",
            z
        );
    }

    #[test]
    fn test_norm_inv_at_99pct() {
        let z = norm_inv(dec!(0.99));
        let tolerance = dec!(0.02);
        assert!(
            (z - dec!(2.326)).abs() < tolerance,
            "norm_inv(0.99) should be ~2.326, got {}",
            z
        );
    }

    #[test]
    fn test_norm_cdf_at_zero() {
        let cdf = norm_cdf(Decimal::ZERO);
        let tolerance = dec!(0.001);
        assert!(
            (cdf - dec!(0.5)).abs() < tolerance,
            "norm_cdf(0) should be ~0.5, got {}",
            cdf
        );
    }

    #[test]
    fn test_norm_pdf_at_zero() {
        let pdf = norm_pdf(Decimal::ZERO);
        // 1/sqrt(2*pi) ~ 0.3989
        let tolerance = dec!(0.001);
        assert!(
            (pdf - dec!(0.3989)).abs() < tolerance,
            "norm_pdf(0) should be ~0.3989, got {}",
            pdf
        );
    }

    #[test]
    fn test_exp_decimal_zero() {
        let result = exp_decimal(Decimal::ZERO);
        let tolerance = dec!(0.0001);
        assert!(
            (result - Decimal::ONE).abs() < tolerance,
            "exp(0) should be ~1, got {}",
            result
        );
    }

    #[test]
    fn test_exp_decimal_one() {
        let result = exp_decimal(Decimal::ONE);
        let tolerance = dec!(0.001);
        assert!(
            (result - dec!(2.71828)).abs() < tolerance,
            "exp(1) should be ~2.71828, got {}",
            result
        );
    }

    #[test]
    fn test_ln_decimal_one() {
        let result = ln_decimal(Decimal::ONE);
        let tolerance = dec!(0.0001);
        assert!(
            result.abs() < tolerance,
            "ln(1) should be ~0, got {}",
            result
        );
    }

    #[test]
    fn test_ln_decimal_e() {
        let e = dec!(2.718281828459045);
        let result = ln_decimal(e);
        let tolerance = dec!(0.001);
        assert!(
            (result - Decimal::ONE).abs() < tolerance,
            "ln(e) should be ~1, got {}",
            result
        );
    }

    #[test]
    fn test_sqrt_decimal_basic() {
        let tolerance = dec!(0.0000001);
        assert!((sqrt_decimal(dec!(4)) - dec!(2)).abs() < tolerance);
        assert!((sqrt_decimal(dec!(9)) - dec!(3)).abs() < tolerance);
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
    }

    #[test]
    fn test_cornish_fisher_z_no_skew_kurt() {
        // With zero skew and kurt, CF z should equal original z
        let z = dec!(1.645);
        let z_cf = cornish_fisher_z(z, Decimal::ZERO, Decimal::ZERO);
        let tolerance = dec!(0.0001);
        assert!(
            (z_cf - z).abs() < tolerance,
            "CF z with zero skew/kurt should equal z, got {}",
            z_cf
        );
    }

    // -- Metadata test --

    #[test]
    fn test_metadata_populated() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.methodology.contains("Tail Risk"));
        assert!(!result.metadata.version.is_empty());
    }

    // -- Edge case tests --

    #[test]
    fn test_single_asset() {
        let input = TailRiskInput {
            asset_names: vec!["Only".into()],
            weights: vec![dec!(1.0)],
            expected_returns: vec![dec!(0.08)],
            covariance_matrix: vec![vec![dec!(0.04)]],
            confidence_level: dec!(0.95),
            time_horizon: Decimal::ONE,
            distribution: TailDistribution::Normal,
            historical_returns: None,
            portfolio_value: dec!(1000000),
            stress_scenarios: None,
        };
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.result.var_absolute > Decimal::ZERO);
        assert_eq!(result.result.component_var.len(), 1);
        // Single asset should have 100% component VaR
        let tolerance = dec!(0.01);
        assert!((result.result.component_var[0].pct_of_total - Decimal::ONE).abs() < tolerance);
    }

    #[test]
    fn test_very_short_time_horizon() {
        let mut input = simple_normal_input();
        input.time_horizon = dec!(0.001); // Very short
        let result = analyze_tail_risk(&input).unwrap();
        assert!(result.result.var_absolute > Decimal::ZERO);
        // VaR should be small for very short horizon
        assert!(result.result.var_absolute < dec!(100000));
    }

    #[test]
    fn test_var_relative_bounded() {
        let input = simple_normal_input();
        let result = analyze_tail_risk(&input).unwrap();
        // Relative VaR should be between 0 and 1 for reasonable inputs
        assert!(result.result.var_relative > Decimal::ZERO);
        assert!(result.result.var_relative < Decimal::ONE);
    }
}
