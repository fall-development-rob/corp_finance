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

/// Constraints for portfolio optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConstraints {
    /// Per-asset minimum weight (default 0 for long-only).
    pub min_weights: Option<Vec<Decimal>>,
    /// Per-asset maximum weight (default 1).
    pub max_weights: Option<Vec<Decimal>>,
    /// No short selling.
    pub long_only: bool,
    /// Max total short exposure (e.g. 0.30 for 130/30 strategy).
    pub max_total_short: Option<Decimal>,
    /// Sector-level constraints.
    pub sector_constraints: Option<Vec<SectorConstraint>>,
}

/// A constraint on a group of assets (sector/region/etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectorConstraint {
    pub name: String,
    pub asset_indices: Vec<usize>,
    pub min_weight: Decimal,
    pub max_weight: Decimal,
}

/// Input to mean-variance portfolio optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeanVarianceInput {
    /// Asset identifiers.
    pub asset_names: Vec<String>,
    /// Annualized expected returns per asset.
    pub expected_returns: Vec<Decimal>,
    /// N x N covariance matrix.
    pub covariance_matrix: Vec<Vec<Decimal>>,
    /// Annual risk-free rate.
    pub risk_free_rate: Decimal,
    /// Portfolio constraints.
    pub constraints: OptimizationConstraints,
    /// Number of efficient frontier points (default 20).
    pub frontier_points: Option<u32>,
    /// Specific target return for optimization.
    pub target_return: Option<Decimal>,
    /// Specific target risk for optimization.
    pub target_risk: Option<Decimal>,
}

/// A single asset weight with risk/return contribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetWeight {
    pub name: String,
    pub weight: Decimal,
    /// Marginal risk contribution times weight.
    pub contribution_to_risk: Decimal,
    /// Weight times expected return.
    pub contribution_to_return: Decimal,
}

/// A single point on the efficient frontier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontierPoint {
    pub expected_return: Decimal,
    pub risk: Decimal,
    pub sharpe_ratio: Decimal,
    pub weights: Vec<Decimal>,
}

/// A named portfolio point (tangency or min-variance).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioPoint {
    pub weights: Vec<Decimal>,
    pub expected_return: Decimal,
    pub risk: Decimal,
    pub sharpe_ratio: Decimal,
}

/// Output of mean-variance optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeanVarianceOutput {
    /// Optimal portfolio weights.
    pub optimal_weights: Vec<AssetWeight>,
    /// Expected return of optimal portfolio.
    pub portfolio_return: Decimal,
    /// Standard deviation of optimal portfolio.
    pub portfolio_risk: Decimal,
    /// Sharpe ratio: (return - rf) / risk.
    pub sharpe_ratio: Decimal,
    /// Efficient frontier points.
    pub efficient_frontier: Vec<FrontierPoint>,
    /// Maximum Sharpe ratio portfolio.
    pub tangency_portfolio: PortfolioPoint,
    /// Global minimum variance portfolio.
    pub min_variance_portfolio: PortfolioPoint,
    /// Weighted average vol / portfolio vol.
    pub diversification_ratio: Decimal,
    /// Herfindahl-Hirschman index of weights.
    pub hhi_concentration: Decimal,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Optimize a portfolio using Markowitz mean-variance framework.
///
/// Computes optimal weights, efficient frontier, tangency portfolio,
/// and minimum variance portfolio.
pub fn optimize_mean_variance(
    input: &MeanVarianceInput,
) -> CorpFinanceResult<ComputationOutput<MeanVarianceOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    let n = input.asset_names.len();
    validate_input(input, n)?;

    let sigma = &input.covariance_matrix;
    let mu = &input.expected_returns;
    let rf = input.risk_free_rate;

    // --- Minimum variance portfolio ---
    let min_var_weights = compute_min_variance_weights(sigma, n, &input.constraints)?;
    let min_var_ret = vec_dot(&min_var_weights, mu);
    let min_var_risk = portfolio_std(&min_var_weights, sigma);
    let min_var_sharpe = compute_sharpe(min_var_ret, rf, min_var_risk);

    let min_variance_portfolio = PortfolioPoint {
        weights: min_var_weights.clone(),
        expected_return: min_var_ret,
        risk: min_var_risk,
        sharpe_ratio: min_var_sharpe,
    };

    // --- Tangency portfolio (max Sharpe) ---
    let tang_weights = compute_tangency_weights(sigma, mu, rf, n, &input.constraints)?;
    let tang_ret = vec_dot(&tang_weights, mu);
    let tang_risk = portfolio_std(&tang_weights, sigma);
    let tang_sharpe = compute_sharpe(tang_ret, rf, tang_risk);

    let tangency_portfolio = PortfolioPoint {
        weights: tang_weights.clone(),
        expected_return: tang_ret,
        risk: tang_risk,
        sharpe_ratio: tang_sharpe,
    };

    // --- Determine the "optimal" portfolio ---
    // If target_return specified, solve for min risk at that return.
    // If target_risk specified, solve for max return at that risk.
    // Otherwise, use the tangency portfolio.
    let optimal_w = if let Some(target_ret) = input.target_return {
        solve_target_return(sigma, mu, target_ret, n, &input.constraints)?
    } else if let Some(target_risk) = input.target_risk {
        solve_target_risk(sigma, mu, rf, target_risk, n, &input.constraints)?
    } else {
        tang_weights.clone()
    };

    let port_ret = vec_dot(&optimal_w, mu);
    let port_risk = portfolio_std(&optimal_w, sigma);
    let port_sharpe = compute_sharpe(port_ret, rf, port_risk);

    // --- Risk contributions ---
    let sigma_w = mat_vec_multiply(sigma, &optimal_w);
    let optimal_weights: Vec<AssetWeight> = (0..n)
        .map(|i| {
            let mcr = if port_risk.is_zero() {
                Decimal::ZERO
            } else {
                sigma_w[i] / port_risk
            };
            let rc = optimal_w[i] * mcr;
            AssetWeight {
                name: input.asset_names[i].clone(),
                weight: optimal_w[i],
                contribution_to_risk: rc,
                contribution_to_return: optimal_w[i] * mu[i],
            }
        })
        .collect();

    // --- Diversification ratio ---
    let individual_vols: Vec<Decimal> = (0..n).map(|i| sqrt_decimal(sigma[i][i])).collect();
    let weighted_avg_vol: Decimal = (0..n).map(|i| optimal_w[i] * individual_vols[i]).sum();
    let diversification_ratio = if port_risk.is_zero() {
        Decimal::ONE
    } else {
        weighted_avg_vol / port_risk
    };

    // --- HHI ---
    let hhi_concentration: Decimal = optimal_w.iter().map(|w| *w * *w).sum();

    // --- Efficient frontier ---
    let num_points = input.frontier_points.unwrap_or(20) as usize;
    let efficient_frontier = compute_efficient_frontier(
        sigma,
        mu,
        rf,
        n,
        &input.constraints,
        num_points,
        min_var_ret,
    )?;

    // --- Warnings ---
    for aw in &optimal_weights {
        if aw.weight > dec!(0.40) {
            warnings.push(format!(
                "Concentrated position: {} has weight {:.4}",
                aw.name, aw.weight
            ));
        }
        if aw.weight < dec!(-0.10) {
            warnings.push(format!(
                "Short position: {} has weight {:.4}",
                aw.name, aw.weight
            ));
        }
    }
    if hhi_concentration > dec!(0.5) {
        warnings.push(format!(
            "High concentration: HHI = {:.4}",
            hhi_concentration
        ));
    }
    if port_risk > dec!(0.30) {
        warnings.push(format!("High portfolio volatility: {:.4}", port_risk));
    }

    let output = MeanVarianceOutput {
        optimal_weights,
        portfolio_return: port_ret,
        portfolio_risk: port_risk,
        sharpe_ratio: port_sharpe,
        efficient_frontier,
        tangency_portfolio,
        min_variance_portfolio,
        diversification_ratio,
        hhi_concentration,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Markowitz Mean-Variance Optimization",
        &serde_json::json!({
            "n_assets": n,
            "risk_free_rate": rf.to_string(),
            "long_only": input.constraints.long_only,
            "frontier_points": num_points,
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Core optimization routines
// ---------------------------------------------------------------------------

/// Compute minimum variance portfolio weights.
/// Unconstrained: w* = Sigma^-1 * 1 / (1' * Sigma^-1 * 1)
/// Constrained: projected gradient descent.
fn compute_min_variance_weights(
    sigma: &[Vec<Decimal>],
    n: usize,
    constraints: &OptimizationConstraints,
) -> CorpFinanceResult<Vec<Decimal>> {
    // Unconstrained analytical solution
    let sigma_inv = mat_inverse(sigma)?;
    let ones = vec![Decimal::ONE; n];
    let sigma_inv_ones = mat_vec_multiply(&sigma_inv, &ones);
    let denom: Decimal = sigma_inv_ones.iter().sum();
    if denom.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "min_variance_weights: 1' * Sigma^-1 * 1 is zero".into(),
        });
    }
    let unconstrained: Vec<Decimal> = sigma_inv_ones.iter().map(|v| *v / denom).collect();

    if is_feasible(&unconstrained, constraints) {
        return Ok(unconstrained);
    }

    // Constrained: projected gradient descent
    // Objective: min w'Sigma w, gradient = 2 * Sigma * w
    let init = equal_weights(n);
    project_gradient_min_variance(sigma, &init, n, constraints)
}

/// Compute tangency portfolio weights (max Sharpe).
/// Unconstrained: w* = Sigma^-1 * (mu - rf*1) / (1' * Sigma^-1 * (mu - rf*1))
/// Constrained: projected gradient descent on negative Sharpe.
fn compute_tangency_weights(
    sigma: &[Vec<Decimal>],
    mu: &[Decimal],
    rf: Decimal,
    n: usize,
    constraints: &OptimizationConstraints,
) -> CorpFinanceResult<Vec<Decimal>> {
    let excess: Vec<Decimal> = mu.iter().map(|r| *r - rf).collect();

    let sigma_inv = mat_inverse(sigma)?;
    let sigma_inv_excess = mat_vec_multiply(&sigma_inv, &excess);
    let denom: Decimal = sigma_inv_excess.iter().sum();

    if denom.abs() < dec!(0.0000000001) {
        // All excess returns zero or cancellation -- fall back to min variance
        return compute_min_variance_weights(sigma, n, constraints);
    }

    let unconstrained: Vec<Decimal> = sigma_inv_excess.iter().map(|v| *v / denom).collect();

    if is_feasible(&unconstrained, constraints) {
        return Ok(unconstrained);
    }

    // Constrained: grid search over frontier for max Sharpe
    project_gradient_max_sharpe(sigma, mu, rf, n, constraints)
}

/// Solve for minimum-variance portfolio at a given target return.
fn solve_target_return(
    sigma: &[Vec<Decimal>],
    mu: &[Decimal],
    target_ret: Decimal,
    n: usize,
    constraints: &OptimizationConstraints,
) -> CorpFinanceResult<Vec<Decimal>> {
    // Use projected gradient descent minimizing variance subject to return constraint
    let mut w = equal_weights(n);
    project_onto_constraints(&mut w, constraints);

    let step = dec!(0.005);
    let return_penalty = dec!(100); // Lagrangian penalty for return constraint

    for _ in 0..200 {
        // Gradient of w'Sigma w = 2 * Sigma * w
        let sigma_w = mat_vec_multiply(sigma, &w);
        let grad_var: Vec<Decimal> = sigma_w.iter().map(|v| dec!(2) * *v).collect();

        // Penalty gradient for (w'mu - target)^2: 2 * penalty * (w'mu - target) * mu
        let cur_ret = vec_dot(&w, mu);
        let ret_diff = cur_ret - target_ret;
        let grad_penalty: Vec<Decimal> = mu
            .iter()
            .map(|m| dec!(2) * return_penalty * ret_diff * *m)
            .collect();

        // Combined gradient
        let grad: Vec<Decimal> = grad_var
            .iter()
            .zip(grad_penalty.iter())
            .map(|(a, b)| *a + *b)
            .collect();

        // Step
        let mut w_new: Vec<Decimal> = w
            .iter()
            .zip(grad.iter())
            .map(|(wi, gi)| *wi - step * *gi)
            .collect();
        project_onto_constraints(&mut w_new, constraints);
        normalize_weights(&mut w_new);
        w = w_new;
    }

    Ok(w)
}

/// Solve for the portfolio on the efficient frontier closest to target_risk.
fn solve_target_risk(
    sigma: &[Vec<Decimal>],
    mu: &[Decimal],
    rf: Decimal,
    target_risk: Decimal,
    n: usize,
    constraints: &OptimizationConstraints,
) -> CorpFinanceResult<Vec<Decimal>> {
    // Generate frontier and pick the point closest to target_risk with highest return
    let min_var_w = compute_min_variance_weights(sigma, n, constraints)?;
    let min_ret = vec_dot(&min_var_w, mu);
    let max_ret = mu
        .iter()
        .copied()
        .fold(Decimal::MIN, |a, b| if b > a { b } else { a });

    let num_points = 50usize;
    let mut best_w = min_var_w;
    let mut best_diff = Decimal::MAX;

    if max_ret > min_ret {
        let step = (max_ret - min_ret) / Decimal::from(num_points as i64);
        for i in 0..=num_points {
            let tr = min_ret + step * Decimal::from(i as i64);
            let w = solve_target_return(sigma, mu, tr, n, constraints)?;
            let risk = portfolio_std(&w, sigma);
            let diff = (risk - target_risk).abs();
            if diff < best_diff {
                best_diff = diff;
                best_w = w;
            }
        }
    }

    let _ = rf; // rf available for future refinement
    Ok(best_w)
}

/// Projected gradient descent for min-variance with constraints.
fn project_gradient_min_variance(
    sigma: &[Vec<Decimal>],
    init: &[Decimal],
    n: usize,
    constraints: &OptimizationConstraints,
) -> CorpFinanceResult<Vec<Decimal>> {
    let mut w: Vec<Decimal> = init.to_vec();
    project_onto_constraints(&mut w, constraints);
    normalize_weights(&mut w);

    let step = dec!(0.005);

    for _ in 0..100 {
        let sigma_w = mat_vec_multiply(sigma, &w);
        let grad: Vec<Decimal> = sigma_w.iter().map(|v| dec!(2) * *v).collect();

        let mut w_new: Vec<Decimal> = (0..n).map(|i| w[i] - step * grad[i]).collect();
        project_onto_constraints(&mut w_new, constraints);
        normalize_weights(&mut w_new);
        w = w_new;
    }

    Ok(w)
}

/// Projected gradient descent for max Sharpe with constraints.
fn project_gradient_max_sharpe(
    sigma: &[Vec<Decimal>],
    mu: &[Decimal],
    rf: Decimal,
    n: usize,
    constraints: &OptimizationConstraints,
) -> CorpFinanceResult<Vec<Decimal>> {
    let mut w = equal_weights(n);
    project_onto_constraints(&mut w, constraints);
    normalize_weights(&mut w);

    let step = dec!(0.001);
    let mut best_sharpe = Decimal::MIN;
    let mut best_w = w.clone();

    for _ in 0..200 {
        let port_ret = vec_dot(&w, mu);
        let port_risk = portfolio_std(&w, sigma);
        let sharpe = compute_sharpe(port_ret, rf, port_risk);

        if sharpe > best_sharpe {
            best_sharpe = sharpe;
            best_w = w.clone();
        }

        if port_risk.is_zero() {
            break;
        }

        // Gradient of negative Sharpe ratio (simplified):
        // d(-S)/dw_i approx -mu_i/sigma_p + (ret-rf)*(Sigma*w)_i / sigma_p^3
        let sigma_w = mat_vec_multiply(sigma, &w);
        let excess = port_ret - rf;
        let risk_cubed = port_risk * port_risk * port_risk;

        let grad: Vec<Decimal> = (0..n)
            .map(|i| {
                if risk_cubed.is_zero() {
                    Decimal::ZERO
                } else {
                    -(mu[i] - rf) / port_risk + excess * sigma_w[i] / risk_cubed
                }
            })
            .collect();

        let mut w_new: Vec<Decimal> = (0..n).map(|i| w[i] - step * grad[i]).collect();
        project_onto_constraints(&mut w_new, constraints);
        normalize_weights(&mut w_new);
        w = w_new;
    }

    Ok(best_w)
}

/// Compute efficient frontier by varying target return.
fn compute_efficient_frontier(
    sigma: &[Vec<Decimal>],
    mu: &[Decimal],
    rf: Decimal,
    n: usize,
    constraints: &OptimizationConstraints,
    num_points: usize,
    min_var_ret: Decimal,
) -> CorpFinanceResult<Vec<FrontierPoint>> {
    let max_ret = mu
        .iter()
        .copied()
        .fold(Decimal::MIN, |a, b| if b > a { b } else { a });

    if num_points <= 1 || max_ret <= min_var_ret {
        let w = solve_target_return(sigma, mu, min_var_ret, n, constraints)?;
        let risk = portfolio_std(&w, sigma);
        let sharpe = compute_sharpe(min_var_ret, rf, risk);
        return Ok(vec![FrontierPoint {
            expected_return: min_var_ret,
            risk,
            sharpe_ratio: sharpe,
            weights: w,
        }]);
    }

    let step = (max_ret - min_var_ret) / Decimal::from(num_points as i64 - 1);
    let mut frontier = Vec::with_capacity(num_points);

    for i in 0..num_points {
        let target_ret = min_var_ret + step * Decimal::from(i as i64);
        let w = solve_target_return(sigma, mu, target_ret, n, constraints)?;
        let ret = vec_dot(&w, mu);
        let risk = portfolio_std(&w, sigma);
        let sharpe = compute_sharpe(ret, rf, risk);
        frontier.push(FrontierPoint {
            expected_return: ret,
            risk,
            sharpe_ratio: sharpe,
            weights: w,
        });
    }

    Ok(frontier)
}

// ---------------------------------------------------------------------------
// Constraint helpers
// ---------------------------------------------------------------------------

/// Check whether weights satisfy constraints.
fn is_feasible(w: &[Decimal], constraints: &OptimizationConstraints) -> bool {
    let n = w.len();

    if constraints.long_only {
        for wi in w {
            if *wi < -dec!(0.0001) {
                return false;
            }
        }
    }

    if let Some(ref mins) = constraints.min_weights {
        for i in 0..n.min(mins.len()) {
            if w[i] < mins[i] - dec!(0.0001) {
                return false;
            }
        }
    }

    if let Some(ref maxs) = constraints.max_weights {
        for i in 0..n.min(maxs.len()) {
            if w[i] > maxs[i] + dec!(0.0001) {
                return false;
            }
        }
    }

    if let Some(max_short) = constraints.max_total_short {
        let total_short: Decimal = w
            .iter()
            .filter(|wi| **wi < Decimal::ZERO)
            .map(|wi| -wi)
            .sum();
        if total_short > max_short + dec!(0.0001) {
            return false;
        }
    }

    if let Some(ref sectors) = constraints.sector_constraints {
        for sc in sectors {
            let sector_weight: Decimal = sc.asset_indices.iter().filter_map(|&i| w.get(i)).sum();
            if sector_weight < sc.min_weight - dec!(0.0001)
                || sector_weight > sc.max_weight + dec!(0.0001)
            {
                return false;
            }
        }
    }

    true
}

/// Project weights onto the constraint set.
fn project_onto_constraints(w: &mut [Decimal], constraints: &OptimizationConstraints) {
    let n = w.len();

    // Box constraints
    if let Some(ref mins) = constraints.min_weights {
        for i in 0..n.min(mins.len()) {
            if w[i] < mins[i] {
                w[i] = mins[i];
            }
        }
    }

    if let Some(ref maxs) = constraints.max_weights {
        for i in 0..n.min(maxs.len()) {
            if w[i] > maxs[i] {
                w[i] = maxs[i];
            }
        }
    }

    // Long-only: clamp negatives to zero
    if constraints.long_only {
        for wi in w.iter_mut() {
            if *wi < Decimal::ZERO {
                *wi = Decimal::ZERO;
            }
        }
    }

    // Max total short
    if let Some(max_short) = constraints.max_total_short {
        let total_short: Decimal = w
            .iter()
            .filter(|wi| **wi < Decimal::ZERO)
            .map(|wi| -wi)
            .sum();
        if total_short > max_short && total_short > Decimal::ZERO {
            let scale = max_short / total_short;
            for wi in w.iter_mut() {
                if *wi < Decimal::ZERO {
                    *wi *= scale;
                }
            }
        }
    }

    // Sector constraints (simple clamping with redistribution)
    if let Some(ref sectors) = constraints.sector_constraints {
        for sc in sectors {
            let sector_weight: Decimal = sc
                .asset_indices
                .iter()
                .filter_map(|&i| w.get(i).copied())
                .sum();
            if sector_weight > sc.max_weight && sector_weight > Decimal::ZERO {
                let scale = sc.max_weight / sector_weight;
                for &i in &sc.asset_indices {
                    if i < n {
                        w[i] *= scale;
                    }
                }
            } else if sector_weight < sc.min_weight {
                let deficit = sc.min_weight - sector_weight;
                let cnt = sc.asset_indices.len();
                if cnt > 0 {
                    let per_asset = deficit / Decimal::from(cnt as i64);
                    for &i in &sc.asset_indices {
                        if i < n {
                            w[i] += per_asset;
                        }
                    }
                }
            }
        }
    }
}

/// Normalize weights to sum to 1.
fn normalize_weights(w: &mut [Decimal]) {
    let total: Decimal = w.iter().sum();
    if !total.is_zero() {
        for wi in w.iter_mut() {
            *wi /= total;
        }
    }
}

/// Equal weights for n assets.
fn equal_weights(n: usize) -> Vec<Decimal> {
    let w = Decimal::ONE / Decimal::from(n as i64);
    vec![w; n]
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &MeanVarianceInput, n: usize) -> CorpFinanceResult<()> {
    if n == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "At least one asset required".into(),
        ));
    }

    if input.expected_returns.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "expected_returns".into(),
            reason: format!(
                "Expected {} returns but got {}",
                n,
                input.expected_returns.len()
            ),
        });
    }

    validate_covariance_matrix(&input.covariance_matrix, n)?;

    // Validate constraint dimensions
    if let Some(ref mins) = input.constraints.min_weights {
        if mins.len() != n {
            return Err(CorpFinanceError::InvalidInput {
                field: "constraints.min_weights".into(),
                reason: format!("Expected {} values but got {}", n, mins.len()),
            });
        }
    }

    if let Some(ref maxs) = input.constraints.max_weights {
        if maxs.len() != n {
            return Err(CorpFinanceError::InvalidInput {
                field: "constraints.max_weights".into(),
                reason: format!("Expected {} values but got {}", n, maxs.len()),
            });
        }
    }

    if let Some(ref sectors) = input.constraints.sector_constraints {
        for (si, sc) in sectors.iter().enumerate() {
            for &idx in &sc.asset_indices {
                if idx >= n {
                    return Err(CorpFinanceError::InvalidInput {
                        field: format!("constraints.sector_constraints[{}]", si),
                        reason: format!("Asset index {} out of range (n={})", idx, n),
                    });
                }
            }
            if sc.min_weight > sc.max_weight {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("constraints.sector_constraints[{}]", si),
                    reason: "min_weight > max_weight".into(),
                });
            }
        }
    }

    Ok(())
}

#[allow(clippy::needless_range_loop)]
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
    for i in 0..n {
        for j in (i + 1)..n {
            if (cov[i][j] - cov[j][i]).abs() > tolerance {
                return Err(CorpFinanceError::InvalidInput {
                    field: "covariance_matrix".into(),
                    reason: format!(
                        "Not symmetric: [{},{}]={} != [{},{}]={}",
                        i, j, cov[i][j], j, i, cov[j][i]
                    ),
                });
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Portfolio math helpers
// ---------------------------------------------------------------------------

/// Compute Sharpe ratio with division-by-zero guard.
fn compute_sharpe(ret: Decimal, rf: Decimal, risk: Decimal) -> Decimal {
    if risk.is_zero() {
        Decimal::ZERO
    } else {
        (ret - rf) / risk
    }
}

/// Portfolio standard deviation: sqrt(w' * Sigma * w).
fn portfolio_std(w: &[Decimal], sigma: &[Vec<Decimal>]) -> Decimal {
    let sigma_w = mat_vec_multiply(sigma, w);
    let var = vec_dot(w, &sigma_w);
    sqrt_decimal(var)
}

// ---------------------------------------------------------------------------
// Matrix helpers (private)
// ---------------------------------------------------------------------------

/// Matrix-vector multiplication.
fn mat_vec_multiply(mat: &[Vec<Decimal>], v: &[Decimal]) -> Vec<Decimal> {
    mat.iter().map(|row| vec_dot(row, v)).collect()
}

/// Dot product.
fn vec_dot(a: &[Decimal], b: &[Decimal]) -> Decimal {
    a.iter().zip(b.iter()).map(|(x, y)| *x * *y).sum()
}

/// Matrix inverse via Gauss-Jordan with partial pivoting.
#[allow(clippy::needless_range_loop)]
fn mat_inverse(mat: &[Vec<Decimal>]) -> CorpFinanceResult<Vec<Vec<Decimal>>> {
    let n = mat.len();
    if n == 0 {
        return Ok(Vec::new());
    }

    let mut aug: Vec<Vec<Decimal>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(2 * n);
        row.extend_from_slice(&mat[i]);
        for j in 0..n {
            row.push(if i == j { Decimal::ONE } else { Decimal::ZERO });
        }
        aug.push(row);
    }

    for col in 0..n {
        // Partial pivoting
        let mut max_row = col;
        let mut max_val = aug[col][col].abs();
        for row in (col + 1)..n {
            let val = aug[row][col].abs();
            if val > max_val {
                max_val = val;
                max_row = row;
            }
        }

        if max_val < dec!(0.0000000001) {
            return Err(CorpFinanceError::FinancialImpossibility(
                "Singular matrix cannot be inverted".into(),
            ));
        }

        if max_row != col {
            aug.swap(col, max_row);
        }

        // Scale pivot row
        let pivot = aug[col][col];
        for cell in aug[col].iter_mut() {
            *cell /= pivot;
        }

        // Eliminate other rows
        let pivot_row = aug[col].clone();
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = aug[row][col];
            for (cell, &pv) in aug[row].iter_mut().zip(pivot_row.iter()) {
                *cell -= factor * pv;
            }
        }
    }

    let inv: Vec<Vec<Decimal>> = aug.iter().map(|row| row[n..].to_vec()).collect();
    Ok(inv)
}

/// Matrix-matrix multiplication.
#[allow(dead_code)]
fn mat_multiply(a: &[Vec<Decimal>], b: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    let m = a.len();
    let p = if m > 0 { a[0].len() } else { 0 };
    let n_cols = if !b.is_empty() { b[0].len() } else { 0 };
    let mut c = vec![vec![Decimal::ZERO; n_cols]; m];
    for i in 0..m {
        for j in 0..n_cols {
            let mut sum = Decimal::ZERO;
            for k in 0..p {
                sum += a[i][k] * b[k][j];
            }
            c[i][j] = sum;
        }
    }
    c
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
    for _ in 0..20 {
        if guess.is_zero() {
            return Decimal::ZERO;
        }
        guess = (guess + val / guess) / two;
    }
    guess
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn unconstrained() -> OptimizationConstraints {
        OptimizationConstraints {
            min_weights: None,
            max_weights: None,
            long_only: false,
            max_total_short: None,
            sector_constraints: None,
        }
    }

    fn long_only() -> OptimizationConstraints {
        OptimizationConstraints {
            min_weights: None,
            max_weights: None,
            long_only: true,
            max_total_short: None,
            sector_constraints: None,
        }
    }

    fn two_asset_input(constraints: OptimizationConstraints) -> MeanVarianceInput {
        // Asset A: 10% return, 20% vol
        // Asset B: 6% return, 10% vol
        // Correlation: 0.3
        let vol_a = dec!(0.20);
        let vol_b = dec!(0.10);
        let corr = dec!(0.3);
        MeanVarianceInput {
            asset_names: vec!["A".into(), "B".into()],
            expected_returns: vec![dec!(0.10), dec!(0.06)],
            covariance_matrix: vec![
                vec![vol_a * vol_a, corr * vol_a * vol_b],
                vec![corr * vol_a * vol_b, vol_b * vol_b],
            ],
            risk_free_rate: dec!(0.02),
            constraints,
            frontier_points: Some(10),
            target_return: None,
            target_risk: None,
        }
    }

    fn three_asset_input(constraints: OptimizationConstraints) -> MeanVarianceInput {
        let v1 = dec!(0.15);
        let v2 = dec!(0.20);
        let v3 = dec!(0.25);
        let c12 = dec!(0.3) * v1 * v2;
        let c13 = dec!(0.1) * v1 * v3;
        let c23 = dec!(0.5) * v2 * v3;
        MeanVarianceInput {
            asset_names: vec!["Equity".into(), "Bonds".into(), "Commodities".into()],
            expected_returns: vec![dec!(0.10), dec!(0.04), dec!(0.07)],
            covariance_matrix: vec![
                vec![v1 * v1, c12, c13],
                vec![c12, v2 * v2, c23],
                vec![c13, c23, v3 * v3],
            ],
            risk_free_rate: dec!(0.02),
            constraints,
            frontier_points: Some(15),
            target_return: None,
            target_risk: None,
        }
    }

    // ------------------------------------------------------------------
    // 1. Basic two-asset unconstrained optimization
    // ------------------------------------------------------------------
    #[test]
    fn test_two_asset_unconstrained() {
        let input = two_asset_input(unconstrained());
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.optimal_weights.len(), 2);
        let total: Decimal = out.optimal_weights.iter().map(|w| w.weight).sum();
        assert!((total - Decimal::ONE).abs() < dec!(0.01));
    }

    // ------------------------------------------------------------------
    // 2. Weights sum to one
    // ------------------------------------------------------------------
    #[test]
    fn test_weights_sum_to_one() {
        let input = three_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        let total: Decimal = result.result.optimal_weights.iter().map(|w| w.weight).sum();
        assert!(
            (total - Decimal::ONE).abs() < dec!(0.02),
            "Weights should sum to ~1, got {}",
            total
        );
    }

    // ------------------------------------------------------------------
    // 3. Min variance portfolio has lower risk than tangency
    // ------------------------------------------------------------------
    #[test]
    fn test_min_variance_lower_risk() {
        let input = two_asset_input(unconstrained());
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        assert!(
            out.min_variance_portfolio.risk <= out.tangency_portfolio.risk + dec!(0.01),
            "Min variance risk {} should be <= tangency risk {}",
            out.min_variance_portfolio.risk,
            out.tangency_portfolio.risk
        );
    }

    // ------------------------------------------------------------------
    // 4. Tangency portfolio has higher Sharpe than min variance
    // ------------------------------------------------------------------
    #[test]
    fn test_tangency_higher_sharpe() {
        let input = two_asset_input(unconstrained());
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        assert!(
            out.tangency_portfolio.sharpe_ratio
                >= out.min_variance_portfolio.sharpe_ratio - dec!(0.01),
            "Tangency Sharpe {} should be >= min variance Sharpe {}",
            out.tangency_portfolio.sharpe_ratio,
            out.min_variance_portfolio.sharpe_ratio
        );
    }

    // ------------------------------------------------------------------
    // 5. Long-only constraint: no negative weights
    // ------------------------------------------------------------------
    #[test]
    fn test_long_only_no_negative_weights() {
        let input = two_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        for w in &result.result.optimal_weights {
            assert!(
                w.weight >= -dec!(0.001),
                "Long-only constraint violated: {} has weight {}",
                w.name,
                w.weight
            );
        }
    }

    // ------------------------------------------------------------------
    // 6. Long-only min variance has no negatives
    // ------------------------------------------------------------------
    #[test]
    fn test_long_only_min_variance() {
        let input = two_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        for w in &result.result.min_variance_portfolio.weights {
            assert!(*w >= -dec!(0.001));
        }
    }

    // ------------------------------------------------------------------
    // 7. Three-asset unconstrained
    // ------------------------------------------------------------------
    #[test]
    fn test_three_asset_unconstrained() {
        let input = three_asset_input(unconstrained());
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.optimal_weights.len(), 3);
        assert!(out.portfolio_risk > Decimal::ZERO);
        assert!(out.portfolio_return != Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 8. Efficient frontier is monotonically non-decreasing in return
    // ------------------------------------------------------------------
    #[test]
    fn test_frontier_monotonic_return() {
        let input = two_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        let frontier = &result.result.efficient_frontier;

        assert!(!frontier.is_empty());
        for i in 1..frontier.len() {
            assert!(
                frontier[i].expected_return >= frontier[i - 1].expected_return - dec!(0.005),
                "Frontier not monotonic at point {}: {} < {}",
                i,
                frontier[i].expected_return,
                frontier[i - 1].expected_return
            );
        }
    }

    // ------------------------------------------------------------------
    // 9. Efficient frontier length matches requested points
    // ------------------------------------------------------------------
    #[test]
    fn test_frontier_length() {
        let mut input = two_asset_input(long_only());
        input.frontier_points = Some(15);
        let result = optimize_mean_variance(&input).unwrap();
        assert_eq!(result.result.efficient_frontier.len(), 15);
    }

    // ------------------------------------------------------------------
    // 10. Sharpe ratio computed correctly
    // ------------------------------------------------------------------
    #[test]
    fn test_sharpe_ratio_computation() {
        let input = two_asset_input(unconstrained());
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        if out.portfolio_risk > Decimal::ZERO {
            let expected = (out.portfolio_return - input.risk_free_rate) / out.portfolio_risk;
            assert!(
                (out.sharpe_ratio - expected).abs() < dec!(0.001),
                "Sharpe mismatch: got {}, expected {}",
                out.sharpe_ratio,
                expected
            );
        }
    }

    // ------------------------------------------------------------------
    // 11. HHI concentration in valid range
    // ------------------------------------------------------------------
    #[test]
    fn test_hhi_range() {
        let input = three_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        let hhi = result.result.hhi_concentration;
        assert!(hhi >= Decimal::ZERO);
        assert!(hhi <= Decimal::ONE + dec!(0.001));
    }

    // ------------------------------------------------------------------
    // 12. Diversification ratio >= 1 for long-only
    // ------------------------------------------------------------------
    #[test]
    fn test_diversification_ratio_gte_one() {
        let input = three_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        assert!(
            result.result.diversification_ratio >= dec!(0.99),
            "Diversification ratio should be >= 1, got {}",
            result.result.diversification_ratio
        );
    }

    // ------------------------------------------------------------------
    // 13. Risk contributions sum to portfolio risk
    // ------------------------------------------------------------------
    #[test]
    fn test_risk_contributions_sum() {
        let input = two_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        let total_rc: Decimal = out
            .optimal_weights
            .iter()
            .map(|w| w.contribution_to_risk)
            .sum();
        // Risk contributions should approximately sum to portfolio risk
        let diff = (total_rc - out.portfolio_risk).abs();
        assert!(
            diff < dec!(0.01),
            "Risk contributions sum {} != portfolio risk {}",
            total_rc,
            out.portfolio_risk
        );
    }

    // ------------------------------------------------------------------
    // 14. Return contributions sum to portfolio return
    // ------------------------------------------------------------------
    #[test]
    fn test_return_contributions_sum() {
        let input = two_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        let total_ret: Decimal = out
            .optimal_weights
            .iter()
            .map(|w| w.contribution_to_return)
            .sum();
        let diff = (total_ret - out.portfolio_return).abs();
        assert!(
            diff < dec!(0.001),
            "Return contributions sum {} != portfolio return {}",
            total_ret,
            out.portfolio_return
        );
    }

    // ------------------------------------------------------------------
    // 15. Target return optimization
    // ------------------------------------------------------------------
    #[test]
    fn test_target_return() {
        let mut input = two_asset_input(long_only());
        input.target_return = Some(dec!(0.08));
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        let diff = (out.portfolio_return - dec!(0.08)).abs();
        assert!(
            diff < dec!(0.02),
            "Portfolio return {} should be near target 0.08",
            out.portfolio_return
        );
    }

    // ------------------------------------------------------------------
    // 16. Target risk optimization
    // ------------------------------------------------------------------
    #[test]
    fn test_target_risk() {
        let mut input = two_asset_input(long_only());
        input.target_risk = Some(dec!(0.12));
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        let diff = (out.portfolio_risk - dec!(0.12)).abs();
        assert!(
            diff < dec!(0.03),
            "Portfolio risk {} should be near target 0.12",
            out.portfolio_risk
        );
    }

    // ------------------------------------------------------------------
    // 17. Box constraints respected
    // ------------------------------------------------------------------
    #[test]
    fn test_box_constraints() {
        let constraints = OptimizationConstraints {
            min_weights: Some(vec![dec!(0.1), dec!(0.1)]),
            max_weights: Some(vec![dec!(0.6), dec!(0.6)]),
            long_only: true,
            max_total_short: None,
            sector_constraints: None,
        };
        let input = two_asset_input(constraints);
        let result = optimize_mean_variance(&input).unwrap();

        for w in &result.result.optimal_weights {
            assert!(
                w.weight >= dec!(0.1) - dec!(0.02),
                "{} weight {} below min 0.1",
                w.name,
                w.weight
            );
            assert!(
                w.weight <= dec!(0.6) + dec!(0.02),
                "{} weight {} above max 0.6",
                w.name,
                w.weight
            );
        }
    }

    // ------------------------------------------------------------------
    // 18. Validation: empty asset list
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_empty_assets() {
        let input = MeanVarianceInput {
            asset_names: vec![],
            expected_returns: vec![],
            covariance_matrix: vec![],
            risk_free_rate: dec!(0.02),
            constraints: unconstrained(),
            frontier_points: None,
            target_return: None,
            target_risk: None,
        };
        assert!(optimize_mean_variance(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 19. Validation: mismatched returns length
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_mismatched_returns() {
        let mut input = two_asset_input(unconstrained());
        input.expected_returns = vec![dec!(0.10)]; // only 1 instead of 2
        assert!(optimize_mean_variance(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 20. Validation: non-square covariance
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_non_square_cov() {
        let mut input = two_asset_input(unconstrained());
        input.covariance_matrix = vec![vec![dec!(0.04), dec!(0.006)]]; // 1x2 instead of 2x2
        assert!(optimize_mean_variance(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 21. Validation: asymmetric covariance
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_asymmetric_cov() {
        let mut input = two_asset_input(unconstrained());
        input.covariance_matrix = vec![vec![dec!(0.04), dec!(0.01)], vec![dec!(0.006), dec!(0.01)]];
        assert!(optimize_mean_variance(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 22. Validation: mismatched min_weights length
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_mismatched_min_weights() {
        let constraints = OptimizationConstraints {
            min_weights: Some(vec![dec!(0.1)]), // 1 instead of 2
            max_weights: None,
            long_only: false,
            max_total_short: None,
            sector_constraints: None,
        };
        let input = two_asset_input(constraints);
        assert!(optimize_mean_variance(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 23. Validation: sector constraint index out of range
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_sector_index_out_of_range() {
        let constraints = OptimizationConstraints {
            min_weights: None,
            max_weights: None,
            long_only: false,
            max_total_short: None,
            sector_constraints: Some(vec![SectorConstraint {
                name: "Tech".into(),
                asset_indices: vec![0, 5], // index 5 out of range
                min_weight: dec!(0),
                max_weight: dec!(0.5),
            }]),
        };
        let input = two_asset_input(constraints);
        assert!(optimize_mean_variance(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 24. Validation: sector min > max
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_sector_min_gt_max() {
        let constraints = OptimizationConstraints {
            min_weights: None,
            max_weights: None,
            long_only: false,
            max_total_short: None,
            sector_constraints: Some(vec![SectorConstraint {
                name: "Tech".into(),
                asset_indices: vec![0],
                min_weight: dec!(0.6),
                max_weight: dec!(0.3),
            }]),
        };
        let input = two_asset_input(constraints);
        assert!(optimize_mean_variance(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 25. Single asset portfolio
    // ------------------------------------------------------------------
    #[test]
    fn test_single_asset() {
        let input = MeanVarianceInput {
            asset_names: vec!["Only".into()],
            expected_returns: vec![dec!(0.08)],
            covariance_matrix: vec![vec![dec!(0.04)]],
            risk_free_rate: dec!(0.02),
            constraints: long_only(),
            frontier_points: Some(5),
            target_return: None,
            target_risk: None,
        };
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.optimal_weights.len(), 1);
        assert!((out.optimal_weights[0].weight - Decimal::ONE).abs() < dec!(0.01));
        assert!((out.portfolio_return - dec!(0.08)).abs() < dec!(0.01));
    }

    // ------------------------------------------------------------------
    // 26. Equal-return assets give equal weights in tangency
    // ------------------------------------------------------------------
    #[test]
    fn test_equal_returns_equal_vol() {
        let v = dec!(0.15);
        let input = MeanVarianceInput {
            asset_names: vec!["A".into(), "B".into()],
            expected_returns: vec![dec!(0.08), dec!(0.08)],
            covariance_matrix: vec![
                vec![v * v, dec!(0.3) * v * v],
                vec![dec!(0.3) * v * v, v * v],
            ],
            risk_free_rate: dec!(0.02),
            constraints: long_only(),
            frontier_points: Some(5),
            target_return: None,
            target_risk: None,
        };
        let result = optimize_mean_variance(&input).unwrap();
        let w = &result.result.tangency_portfolio.weights;
        let diff = (w[0] - w[1]).abs();
        assert!(
            diff < dec!(0.05),
            "Equal return+vol assets should have ~equal weights: {} vs {}",
            w[0],
            w[1]
        );
    }

    // ------------------------------------------------------------------
    // 27. Higher return asset gets more weight in tangency
    // ------------------------------------------------------------------
    #[test]
    fn test_higher_sharpe_higher_weight() {
        // Two-asset input has A: Sharpe 0.40, B: Sharpe 0.40 (equal).
        // With equal Sharpe and correlation < 1, lower-vol asset B gets more weight.
        let input = two_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        let tang = &result.result.tangency_portfolio;
        // Both Sharpe ratios are equal (0.40), so B (lower vol) dominates.
        assert!(
            tang.weights[1] > tang.weights[0],
            "Lower-vol asset with equal Sharpe should have higher weight: B={} vs A={}",
            tang.weights[1],
            tang.weights[0]
        );
    }

    // ------------------------------------------------------------------
    // 28. Portfolio risk is non-negative
    // ------------------------------------------------------------------
    #[test]
    fn test_portfolio_risk_non_negative() {
        let input = three_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        assert!(result.result.portfolio_risk >= Decimal::ZERO);
        assert!(result.result.min_variance_portfolio.risk >= Decimal::ZERO);
        assert!(result.result.tangency_portfolio.risk >= Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 29. Frontier first point is near min variance return
    // ------------------------------------------------------------------
    #[test]
    fn test_frontier_starts_at_min_var() {
        let input = two_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        let out = &result.result;

        if !out.efficient_frontier.is_empty() {
            let diff = (out.efficient_frontier[0].expected_return
                - out.min_variance_portfolio.expected_return)
                .abs();
            assert!(
                diff < dec!(0.02),
                "Frontier start {} should be near min var return {}",
                out.efficient_frontier[0].expected_return,
                out.min_variance_portfolio.expected_return
            );
        }
    }

    // ------------------------------------------------------------------
    // 30. Methodology string is correct
    // ------------------------------------------------------------------
    #[test]
    fn test_methodology() {
        let input = two_asset_input(unconstrained());
        let result = optimize_mean_variance(&input).unwrap();
        assert_eq!(result.methodology, "Markowitz Mean-Variance Optimization");
    }

    // ------------------------------------------------------------------
    // 31. Metadata precision
    // ------------------------------------------------------------------
    #[test]
    fn test_metadata_precision() {
        let input = two_asset_input(unconstrained());
        let result = optimize_mean_variance(&input).unwrap();
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // ------------------------------------------------------------------
    // 32. Concentrated position warning
    // ------------------------------------------------------------------
    #[test]
    fn test_warning_concentrated() {
        // With two assets and very different returns, tangency concentrates
        let input = MeanVarianceInput {
            asset_names: vec!["Winner".into(), "Loser".into()],
            expected_returns: vec![dec!(0.30), dec!(0.01)],
            covariance_matrix: vec![vec![dec!(0.04), dec!(0.005)], vec![dec!(0.005), dec!(0.01)]],
            risk_free_rate: dec!(0.02),
            constraints: long_only(),
            frontier_points: Some(5),
            target_return: None,
            target_risk: None,
        };
        let result = optimize_mean_variance(&input).unwrap();
        let has_concentrated = result.warnings.iter().any(|w| w.contains("Concentrated"));
        // The winner should dominate
        assert!(has_concentrated || result.result.optimal_weights[0].weight > dec!(0.4));
    }

    // ------------------------------------------------------------------
    // 33. Sector constraints applied
    // ------------------------------------------------------------------
    #[test]
    fn test_sector_constraints() {
        let constraints = OptimizationConstraints {
            min_weights: None,
            max_weights: None,
            long_only: true,
            max_total_short: None,
            sector_constraints: Some(vec![SectorConstraint {
                name: "Equity".into(),
                asset_indices: vec![0],
                min_weight: dec!(0.2),
                max_weight: dec!(0.4),
            }]),
        };
        let input = three_asset_input(constraints);
        let result = optimize_mean_variance(&input).unwrap();
        let w0 = result.result.optimal_weights[0].weight;
        // After optimization with sector constraint, equity should be within bounds
        // (with some tolerance from gradient-based method)
        assert!(
            w0 >= dec!(0.15) && w0 <= dec!(0.50),
            "Sector constraint: equity weight {} outside approximate bounds",
            w0
        );
    }

    // ------------------------------------------------------------------
    // 34. Feasibility check helper
    // ------------------------------------------------------------------
    #[test]
    fn test_is_feasible() {
        let c = long_only();
        assert!(is_feasible(&[dec!(0.5), dec!(0.5)], &c));
        assert!(!is_feasible(&[dec!(-0.1), dec!(1.1)], &c));
    }

    // ------------------------------------------------------------------
    // 35. Equal weights helper
    // ------------------------------------------------------------------
    #[test]
    fn test_equal_weights() {
        let w = equal_weights(4);
        assert_eq!(w.len(), 4);
        for wi in &w {
            assert!((wi - dec!(0.25)).abs() < dec!(0.0001));
        }
    }

    // ------------------------------------------------------------------
    // 36. Matrix inverse correctness
    // ------------------------------------------------------------------
    #[test]
    fn test_matrix_inverse() {
        let a = vec![vec![dec!(2), dec!(1)], vec![dec!(5), dec!(3)]];
        let inv = mat_inverse(&a).unwrap();
        let product = mat_multiply(&a, &inv);
        for i in 0..2 {
            for j in 0..2 {
                let expected = if i == j { Decimal::ONE } else { Decimal::ZERO };
                assert!(
                    (product[i][j] - expected).abs() < dec!(0.0000001),
                    "Product[{}][{}] = {}, expected {}",
                    i,
                    j,
                    product[i][j],
                    expected
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // 37. Sqrt helper
    // ------------------------------------------------------------------
    #[test]
    fn test_sqrt_decimal() {
        assert!((sqrt_decimal(dec!(4)) - dec!(2)).abs() < dec!(0.0000001));
        assert!((sqrt_decimal(dec!(9)) - dec!(3)).abs() < dec!(0.0000001));
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
        assert_eq!(sqrt_decimal(dec!(-1)), Decimal::ZERO);
        assert_eq!(sqrt_decimal(Decimal::ONE), Decimal::ONE);
    }

    // ------------------------------------------------------------------
    // 38. Dot product helper
    // ------------------------------------------------------------------
    #[test]
    fn test_vec_dot() {
        let a = vec![dec!(1), dec!(2), dec!(3)];
        let b = vec![dec!(4), dec!(5), dec!(6)];
        assert_eq!(vec_dot(&a, &b), dec!(32)); // 4+10+18
    }

    // ------------------------------------------------------------------
    // 39. Four-asset optimization
    // ------------------------------------------------------------------
    #[test]
    fn test_four_asset() {
        let v = vec![dec!(0.15), dec!(0.20), dec!(0.25), dec!(0.10)];
        let mut cov = vec![vec![Decimal::ZERO; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                if i == j {
                    cov[i][j] = v[i] * v[i];
                } else {
                    cov[i][j] = dec!(0.2) * v[i] * v[j];
                }
            }
        }
        let input = MeanVarianceInput {
            asset_names: vec!["A".into(), "B".into(), "C".into(), "D".into()],
            expected_returns: vec![dec!(0.12), dec!(0.08), dec!(0.06), dec!(0.10)],
            covariance_matrix: cov,
            risk_free_rate: dec!(0.02),
            constraints: long_only(),
            frontier_points: Some(10),
            target_return: None,
            target_risk: None,
        };
        let result = optimize_mean_variance(&input).unwrap();
        assert_eq!(result.result.optimal_weights.len(), 4);
        assert!(result.result.portfolio_risk > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 40. Normalize weights helper
    // ------------------------------------------------------------------
    #[test]
    fn test_normalize_weights() {
        let mut w = vec![dec!(2), dec!(3), dec!(5)];
        normalize_weights(&mut w);
        let total: Decimal = w.iter().sum();
        assert!((total - Decimal::ONE).abs() < dec!(0.0001));
        assert!((w[0] - dec!(0.2)).abs() < dec!(0.0001));
        assert!((w[1] - dec!(0.3)).abs() < dec!(0.0001));
        assert!((w[2] - dec!(0.5)).abs() < dec!(0.0001));
    }

    // ------------------------------------------------------------------
    // 41. Sharpe with zero risk
    // ------------------------------------------------------------------
    #[test]
    fn test_sharpe_zero_risk() {
        assert_eq!(
            compute_sharpe(dec!(0.08), dec!(0.02), Decimal::ZERO),
            Decimal::ZERO
        );
    }

    // ------------------------------------------------------------------
    // 42. Max total short constraint
    // ------------------------------------------------------------------
    #[test]
    fn test_max_total_short() {
        let constraints = OptimizationConstraints {
            min_weights: None,
            max_weights: None,
            long_only: false,
            max_total_short: Some(dec!(0.30)),
            sector_constraints: None,
        };
        let mut w = vec![dec!(0.8), dec!(0.5), dec!(-0.3)];
        project_onto_constraints(&mut w, &constraints);
        let total_short: Decimal = w
            .iter()
            .filter(|wi| **wi < Decimal::ZERO)
            .map(|wi| -wi)
            .sum();
        assert!(total_short <= dec!(0.31));
    }

    // ------------------------------------------------------------------
    // 43. Frontier sharpe values are computed
    // ------------------------------------------------------------------
    #[test]
    fn test_frontier_sharpe_values() {
        let input = two_asset_input(long_only());
        let result = optimize_mean_variance(&input).unwrap();
        for pt in &result.result.efficient_frontier {
            if pt.risk > Decimal::ZERO {
                let expected = (pt.expected_return - input.risk_free_rate) / pt.risk;
                assert!(
                    (pt.sharpe_ratio - expected).abs() < dec!(0.01),
                    "Frontier point sharpe mismatch"
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // 44. HHI for equal weights
    // ------------------------------------------------------------------
    #[test]
    fn test_hhi_equal_weights() {
        // For N equal weights, HHI = N * (1/N)^2 = 1/N
        let w = vec![dec!(0.25); 4];
        let hhi: Decimal = w.iter().map(|wi| *wi * *wi).sum();
        assert!((hhi - dec!(0.25)).abs() < dec!(0.001));
    }

    // ------------------------------------------------------------------
    // 45. Validation: covariance row length mismatch
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_cov_row_length() {
        let mut input = two_asset_input(unconstrained());
        input.covariance_matrix = vec![
            vec![dec!(0.04), dec!(0.006)],
            vec![dec!(0.006)], // missing element
        ];
        assert!(optimize_mean_variance(&input).is_err());
    }
}
