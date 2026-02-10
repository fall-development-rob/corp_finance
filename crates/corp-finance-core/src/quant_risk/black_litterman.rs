use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Type of investor view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ViewType {
    /// Absolute view on a single asset's return.
    Absolute,
    /// Relative view: one asset outperforms another by a spread.
    Relative,
}

/// An asset with its market-cap weight.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetWeight {
    pub name: String,
    pub weight: Decimal,
}

/// An investor view used in the Black-Litterman model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    /// Whether the view is absolute or relative.
    pub view_type: ViewType,
    /// Asset names involved in this view.
    pub assets: Vec<String>,
    /// Pick-matrix row: weights for each asset in `assets`.
    /// For an absolute view on one asset: \[1\].
    /// For a relative view "A outperforms B": \[1, -1\].
    pub asset_weights: Vec<Decimal>,
    /// Expected return expressed by this view.
    pub expected_return: Decimal,
    /// Confidence in the view, 0 to 1 (higher = more confident).
    pub confidence: Decimal,
}

/// Input to the Black-Litterman model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackLittermanInput {
    /// Market-cap weighted portfolio (weights must sum to ~1).
    pub market_cap_weights: Vec<AssetWeight>,
    /// NxN annualised covariance matrix (row-major).
    pub covariance_matrix: Vec<Vec<Decimal>>,
    /// Risk aversion coefficient (delta), typically around 2.5.
    pub risk_aversion: Decimal,
    /// Uncertainty scaling factor (tau), typically 0.025 - 0.05.
    pub tau: Decimal,
    /// Investor views.
    pub views: Vec<View>,
    /// Risk-free rate (annualised).
    pub risk_free_rate: Rate,
}

/// A named expected return for one asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetReturn {
    pub name: String,
    pub expected_return: Decimal,
}

/// Comparison of prior (equilibrium) vs posterior return for one asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnComparison {
    pub name: String,
    pub prior_return: Decimal,
    pub posterior_return: Decimal,
    pub shift: Decimal,
}

/// Output of the Black-Litterman model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackLittermanOutput {
    /// Implied equilibrium returns: Pi = delta * Sigma * w_mkt.
    pub equilibrium_returns: Vec<AssetReturn>,
    /// Posterior (BL combined) expected returns.
    pub posterior_returns: Vec<AssetReturn>,
    /// Optimal portfolio weights from mean-variance optimisation.
    pub optimal_weights: Vec<AssetWeight>,
    /// Prior vs posterior comparison per asset.
    pub prior_vs_posterior: Vec<ReturnComparison>,
    /// Portfolio expected return (w* dot E\[R\]).
    pub portfolio_expected_return: Decimal,
    /// Portfolio volatility: sqrt(w*' Sigma w*).
    pub portfolio_volatility: Decimal,
    /// Sharpe ratio: (E\[R_p\] - r_f) / vol_p.
    pub portfolio_sharpe: Decimal,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the Black-Litterman portfolio optimisation model.
///
/// When no views are provided the model returns the equilibrium (prior)
/// portfolio, which corresponds to the market-cap weighted portfolio under
/// the assumption that markets are efficient.
pub fn run_black_litterman(
    input: &BlackLittermanInput,
) -> CorpFinanceResult<ComputationOutput<BlackLittermanOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validation ---
    validate_input(input)?;

    let n = input.market_cap_weights.len();

    // Market-cap weight vector
    let w_mkt: Vec<Decimal> = input.market_cap_weights.iter().map(|a| a.weight).collect();
    let sigma = &input.covariance_matrix;
    let delta = input.risk_aversion;
    let tau = input.tau;

    // 1. Equilibrium returns: Pi = delta * Sigma * w_mkt
    let sigma_w = mat_vec_multiply(sigma, &w_mkt);
    let pi: Vec<Decimal> = sigma_w.iter().map(|v| delta * v).collect();

    let equilibrium_returns: Vec<AssetReturn> = input
        .market_cap_weights
        .iter()
        .enumerate()
        .map(|(i, a)| AssetReturn {
            name: a.name.clone(),
            expected_return: pi[i],
        })
        .collect();

    // If there are no views, posterior = equilibrium and weights = market-cap.
    let (posterior_vec, optimal_w) = if input.views.is_empty() {
        (pi.clone(), w_mkt.clone())
    } else {
        compute_posterior(n, &pi, sigma, delta, tau, input)?
    };

    // --- Build output structs ---
    let posterior_returns: Vec<AssetReturn> = input
        .market_cap_weights
        .iter()
        .enumerate()
        .map(|(i, a)| AssetReturn {
            name: a.name.clone(),
            expected_return: posterior_vec[i],
        })
        .collect();

    let optimal_weights: Vec<AssetWeight> = input
        .market_cap_weights
        .iter()
        .enumerate()
        .map(|(i, a)| AssetWeight {
            name: a.name.clone(),
            weight: optimal_w[i],
        })
        .collect();

    let prior_vs_posterior: Vec<ReturnComparison> = input
        .market_cap_weights
        .iter()
        .enumerate()
        .map(|(i, a)| ReturnComparison {
            name: a.name.clone(),
            prior_return: pi[i],
            posterior_return: posterior_vec[i],
            shift: posterior_vec[i] - pi[i],
        })
        .collect();

    // Portfolio expected return = w* . E[R]
    let portfolio_expected_return = vec_dot(&optimal_w, &posterior_vec);

    // Portfolio volatility = sqrt(w*' Sigma w*)
    let sigma_wopt = mat_vec_multiply(sigma, &optimal_w);
    let port_var = vec_dot(&optimal_w, &sigma_wopt);
    let portfolio_volatility = sqrt_decimal(port_var);

    // Sharpe ratio
    let portfolio_sharpe = if portfolio_volatility.is_zero() {
        Decimal::ZERO
    } else {
        (portfolio_expected_return - input.risk_free_rate) / portfolio_volatility
    };

    // --- Warnings ---
    for w in &optimal_weights {
        if w.weight > dec!(0.4) {
            warnings.push(format!(
                "Concentrated position: {} has weight {:.4}",
                w.name, w.weight
            ));
        }
        if w.weight < dec!(-0.1) {
            warnings.push(format!(
                "Short position: {} has weight {:.4}",
                w.name, w.weight
            ));
        }
    }
    if portfolio_volatility > dec!(0.3) {
        warnings.push(format!(
            "High portfolio volatility: {:.4}",
            portfolio_volatility
        ));
    }

    let output = BlackLittermanOutput {
        equilibrium_returns,
        posterior_returns,
        optimal_weights,
        prior_vs_posterior,
        portfolio_expected_return,
        portfolio_volatility,
        portfolio_sharpe,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Black-Litterman Portfolio Optimisation",
        &serde_json::json!({
            "n_assets": n,
            "n_views": input.views.len(),
            "risk_aversion": input.risk_aversion.to_string(),
            "tau": input.tau.to_string(),
            "risk_free_rate": input.risk_free_rate.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Core BL computation (views present)
// ---------------------------------------------------------------------------

/// Compute posterior returns and optimal weights when views are present.
fn compute_posterior(
    n: usize,
    pi: &[Decimal],
    sigma: &[Vec<Decimal>],
    delta: Decimal,
    tau: Decimal,
    input: &BlackLittermanInput,
) -> CorpFinanceResult<(Vec<Decimal>, Vec<Decimal>)> {
    let k = input.views.len();
    let asset_names: Vec<&str> = input
        .market_cap_weights
        .iter()
        .map(|a| a.name.as_str())
        .collect();

    // 2. Build P matrix (K x N), Q vector (K x 1)
    let mut p_mat: Vec<Vec<Decimal>> = vec![vec![Decimal::ZERO; n]; k];
    let mut q_vec: Vec<Decimal> = Vec::with_capacity(k);

    for (vi, view) in input.views.iter().enumerate() {
        for (ai, asset_name) in view.assets.iter().enumerate() {
            let col = asset_names
                .iter()
                .position(|nm| *nm == asset_name.as_str())
                .expect("asset validated");
            p_mat[vi][col] = view.asset_weights[ai];
        }
        q_vec.push(view.expected_return);
    }

    // tau * Sigma
    let tau_sigma = mat_scale(sigma, tau);

    // 3. Build Omega (K x K diagonal).
    // Omega_ii = (1/confidence - 1) * (P * tau*Sigma * P')_ii
    let p_tau_sigma = mat_multiply(&p_mat, &tau_sigma);
    let p_t = mat_transpose(&p_mat);
    let p_tau_sigma_pt = mat_multiply(&p_tau_sigma, &p_t);

    let mut omega: Vec<Vec<Decimal>> = vec![vec![Decimal::ZERO; k]; k];
    for i in 0..k {
        let conf = input.views[i].confidence;
        let scale = (Decimal::ONE / conf) - Decimal::ONE;
        omega[i][i] = scale * p_tau_sigma_pt[i][i];
    }

    // 4. Posterior returns:
    // E[R] = inv(inv(tau*Sigma) + P' inv(Omega) P)
    //        * (inv(tau*Sigma) * Pi + P' inv(Omega) Q)
    let tau_sigma_inv = mat_inverse(&tau_sigma)?;
    let omega_inv = mat_inverse(&omega)?;

    // P' * Omega_inv  (N x K)
    let pt_omega_inv = mat_multiply(&p_t, &omega_inv);

    // Left: inv(tau*Sigma) + P' Omega_inv P  (N x N)
    let pt_omega_inv_p = mat_multiply(&pt_omega_inv, &p_mat);
    let left = mat_add(&tau_sigma_inv, &pt_omega_inv_p);
    let left_inv = mat_inverse(&left)?;

    // Right: inv(tau*Sigma) * Pi + P' Omega_inv Q  (N x 1)
    let tau_sigma_inv_pi = mat_vec_multiply(&tau_sigma_inv, pi);
    let pt_omega_inv_q = mat_vec_multiply(&pt_omega_inv, &q_vec);
    let right: Vec<Decimal> = tau_sigma_inv_pi
        .iter()
        .zip(pt_omega_inv_q.iter())
        .map(|(a, b)| a + b)
        .collect();

    let posterior = mat_vec_multiply(&left_inv, &right);

    // 5. Optimal weights: w* = inv(delta * Sigma) * E[R]
    let delta_sigma = mat_scale(sigma, delta);
    let delta_sigma_inv = mat_inverse(&delta_sigma)?;
    let raw_weights = mat_vec_multiply(&delta_sigma_inv, &posterior);

    // 6. Normalise weights to sum to 1
    let w_sum: Decimal = raw_weights.iter().copied().sum();
    let opt_w = if w_sum.is_zero() {
        raw_weights
    } else {
        raw_weights.iter().map(|w| w / w_sum).collect()
    };

    Ok((posterior, opt_w))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &BlackLittermanInput) -> CorpFinanceResult<()> {
    let n = input.market_cap_weights.len();

    if n == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "At least one asset required".into(),
        ));
    }

    // tau > 0
    if input.tau <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "tau".into(),
            reason: "tau must be positive".into(),
        });
    }

    // risk_aversion > 0
    if input.risk_aversion <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_aversion".into(),
            reason: "risk_aversion must be positive".into(),
        });
    }

    // Covariance matrix must be NxN
    if input.covariance_matrix.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "covariance_matrix".into(),
            reason: format!(
                "Expected {}x{} matrix but got {} rows",
                n,
                n,
                input.covariance_matrix.len()
            ),
        });
    }
    for (i, row) in input.covariance_matrix.iter().enumerate() {
        if row.len() != n {
            return Err(CorpFinanceError::InvalidInput {
                field: "covariance_matrix".into(),
                reason: format!("Row {} has {} columns, expected {}", i, row.len(), n),
            });
        }
    }

    // Covariance matrix must be symmetric
    for i in 0..n {
        for j in (i + 1)..n {
            let diff = (input.covariance_matrix[i][j] - input.covariance_matrix[j][i]).abs();
            if diff > dec!(0.000001) {
                return Err(CorpFinanceError::InvalidInput {
                    field: "covariance_matrix".into(),
                    reason: format!(
                        "Not symmetric: [{},{}]={} != [{},{}]={}",
                        i, j, input.covariance_matrix[i][j], j, i, input.covariance_matrix[j][i]
                    ),
                });
            }
        }
    }

    // Market-cap weights must sum to ~1.0
    let weight_sum: Decimal = input.market_cap_weights.iter().map(|a| a.weight).sum();
    if (weight_sum - Decimal::ONE).abs() > dec!(0.01) {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_cap_weights".into(),
            reason: format!("Weights must sum to 1.0 (got {})", weight_sum),
        });
    }

    // Validate views
    let asset_names: Vec<&str> = input
        .market_cap_weights
        .iter()
        .map(|a| a.name.as_str())
        .collect();

    for (vi, view) in input.views.iter().enumerate() {
        if view.confidence <= Decimal::ZERO || view.confidence > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("views[{}].confidence", vi),
                reason: "Confidence must be in (0, 1]".into(),
            });
        }
        if view.assets.len() != view.asset_weights.len() {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("views[{}]", vi),
                reason: "assets and asset_weights must have the same length".into(),
            });
        }
        for asset_name in &view.assets {
            if !asset_names.contains(&asset_name.as_str()) {
                return Err(CorpFinanceError::InvalidInput {
                    field: format!("views[{}].assets", vi),
                    reason: format!("Unknown asset '{}'", asset_name),
                });
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix helpers (private)
// ---------------------------------------------------------------------------

/// Multiply two matrices: C = A * B.
/// A is (m x p), B is (p x n_cols), result is (m x n_cols).
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

/// Transpose a matrix.
fn mat_transpose(a: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    let m = a.len();
    if m == 0 {
        return Vec::new();
    }
    let n = a[0].len();
    let mut t = vec![vec![Decimal::ZERO; m]; n];
    for i in 0..m {
        for j in 0..n {
            t[j][i] = a[i][j];
        }
    }
    t
}

/// Invert a square matrix using Gauss-Jordan elimination with partial
/// pivoting. The range-loop pattern is deliberate for in-place row operations.
#[allow(clippy::needless_range_loop)]
fn mat_inverse(a: &[Vec<Decimal>]) -> CorpFinanceResult<Vec<Vec<Decimal>>> {
    let n = a.len();
    if n == 0 {
        return Ok(Vec::new());
    }

    // Augmented matrix [A | I]
    let mut aug: Vec<Vec<Decimal>> = Vec::with_capacity(n);
    for (i, a_row) in a.iter().enumerate() {
        let mut row = Vec::with_capacity(2 * n);
        row.extend_from_slice(a_row);
        for j in 0..n {
            row.push(if i == j { Decimal::ONE } else { Decimal::ZERO });
        }
        aug.push(row);
    }

    // Forward elimination with partial pivoting
    for col in 0..n {
        // Find pivot
        let mut max_val = aug[col][col].abs();
        let mut max_row = col;
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

        // Swap rows
        if max_row != col {
            aug.swap(col, max_row);
        }

        // Scale pivot row
        let pivot = aug[col][col];
        for cell in aug[col].iter_mut() {
            *cell /= pivot;
        }

        // Eliminate column in all other rows.
        // Clone the pivot row to avoid simultaneous borrow.
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

    // Extract the right half
    let inv: Vec<Vec<Decimal>> = aug.iter().map(|row| row[n..].to_vec()).collect();

    Ok(inv)
}

/// Multiply a matrix (m x n) by a vector (n x 1), returning (m x 1).
fn mat_vec_multiply(a: &[Vec<Decimal>], v: &[Decimal]) -> Vec<Decimal> {
    a.iter()
        .map(|row| row.iter().zip(v.iter()).map(|(a_ij, v_j)| a_ij * v_j).sum())
        .collect()
}

/// Dot product of two vectors.
fn vec_dot(a: &[Decimal], b: &[Decimal]) -> Decimal {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Element-wise addition of two matrices.
fn mat_add(a: &[Vec<Decimal>], b: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    a.iter()
        .zip(b.iter())
        .map(|(row_a, row_b)| row_a.iter().zip(row_b.iter()).map(|(x, y)| x + y).collect())
        .collect()
}

/// Scale every element of a matrix by a scalar.
fn mat_scale(a: &[Vec<Decimal>], s: Decimal) -> Vec<Vec<Decimal>> {
    a.iter()
        .map(|row| row.iter().map(|v| v * s).collect())
        .collect()
}

/// Square root via Newton's method (20 iterations).
fn sqrt_decimal(val: Decimal) -> Decimal {
    if val <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = val / dec!(2);
    if guess.is_zero() {
        guess = dec!(0.0001);
    }
    for _ in 0..20 {
        guess = (guess + val / guess) / dec!(2);
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

    // -- Helper builders --

    fn two_asset_input(views: Vec<View>) -> BlackLittermanInput {
        BlackLittermanInput {
            market_cap_weights: vec![
                AssetWeight {
                    name: "A".into(),
                    weight: dec!(0.6),
                },
                AssetWeight {
                    name: "B".into(),
                    weight: dec!(0.4),
                },
            ],
            covariance_matrix: vec![vec![dec!(0.04), dec!(0.006)], vec![dec!(0.006), dec!(0.09)]],
            risk_aversion: dec!(2.5),
            tau: dec!(0.05),
            views,
            risk_free_rate: dec!(0.02),
        }
    }

    fn three_asset_input(views: Vec<View>) -> BlackLittermanInput {
        BlackLittermanInput {
            market_cap_weights: vec![
                AssetWeight {
                    name: "Equity".into(),
                    weight: dec!(0.5),
                },
                AssetWeight {
                    name: "Bonds".into(),
                    weight: dec!(0.3),
                },
                AssetWeight {
                    name: "Commodities".into(),
                    weight: dec!(0.2),
                },
            ],
            covariance_matrix: vec![
                vec![dec!(0.0225), dec!(0.003), dec!(0.006)],
                vec![dec!(0.003), dec!(0.0016), dec!(0.001)],
                vec![dec!(0.006), dec!(0.001), dec!(0.0400)],
            ],
            risk_aversion: dec!(2.5),
            tau: dec!(0.025),
            views,
            risk_free_rate: dec!(0.02),
        }
    }

    // -- 1. Two-asset simple case (no views -> equilibrium) --

    #[test]
    fn test_two_asset_no_views() {
        let input = two_asset_input(vec![]);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        // Pi = delta * Sigma * w
        // Pi_A = 2.5 * (0.04*0.6 + 0.006*0.4) = 0.066
        // Pi_B = 2.5 * (0.006*0.6 + 0.09*0.4)  = 0.099
        assert_eq!(out.equilibrium_returns.len(), 2);
        assert_eq!(out.equilibrium_returns[0].expected_return, dec!(0.066));
        assert_eq!(out.equilibrium_returns[1].expected_return, dec!(0.099));

        // No views => posterior == equilibrium
        assert_eq!(
            out.posterior_returns[0].expected_return,
            out.equilibrium_returns[0].expected_return
        );
        assert_eq!(
            out.posterior_returns[1].expected_return,
            out.equilibrium_returns[1].expected_return
        );

        // Weights should be market-cap weights
        assert_eq!(out.optimal_weights[0].weight, dec!(0.6));
        assert_eq!(out.optimal_weights[1].weight, dec!(0.4));

        // Shift should be zero
        assert_eq!(out.prior_vs_posterior[0].shift, Decimal::ZERO);
        assert_eq!(out.prior_vs_posterior[1].shift, Decimal::ZERO);
    }

    // -- 2. Two-asset with absolute view --

    #[test]
    fn test_two_asset_absolute_view() {
        let views = vec![View {
            view_type: ViewType::Absolute,
            assets: vec!["A".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.10),
            confidence: dec!(0.8),
        }];
        let input = two_asset_input(views);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        // Equilibrium return for A is 0.066; view says 0.10.
        // Posterior should shift up toward the view.
        assert!(out.posterior_returns[0].expected_return > dec!(0.066));
        assert!(out.prior_vs_posterior[0].shift > Decimal::ZERO);
        // Optimal weight for A should increase relative to 0.6
        assert!(out.optimal_weights[0].weight > dec!(0.6));
    }

    // -- 3. Three-asset with absolute view --

    #[test]
    fn test_three_asset_absolute_view() {
        let views = vec![View {
            view_type: ViewType::Absolute,
            assets: vec!["Equity".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.08),
            confidence: dec!(0.7),
        }];
        let input = three_asset_input(views);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.equilibrium_returns.len(), 3);
        assert_eq!(out.posterior_returns.len(), 3);
        assert_eq!(out.optimal_weights.len(), 3);
        // The equity view (0.08) is higher than equilibrium
        let eq_ret = out.equilibrium_returns[0].expected_return;
        assert!(out.posterior_returns[0].expected_return > eq_ret);
    }

    // -- 4. Relative view: A outperforms B by 2% --

    #[test]
    fn test_relative_view() {
        let views = vec![View {
            view_type: ViewType::Relative,
            assets: vec!["A".into(), "B".into()],
            asset_weights: vec![dec!(1), dec!(-1)],
            expected_return: dec!(0.02),
            confidence: dec!(0.6),
        }];
        let input = two_asset_input(views);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        // equilibrium spread = 0.066 - 0.099 = -0.033
        // view says spread should be 0.02 => posterior spread moves toward 0.02
        let spread_prior =
            out.equilibrium_returns[0].expected_return - out.equilibrium_returns[1].expected_return;
        let spread_posterior =
            out.posterior_returns[0].expected_return - out.posterior_returns[1].expected_return;
        assert!(spread_posterior > spread_prior);
    }

    // -- 5. No views returns equilibrium --

    #[test]
    fn test_no_views_three_asset() {
        let input = three_asset_input(vec![]);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        for cmp in &out.prior_vs_posterior {
            assert_eq!(cmp.shift, Decimal::ZERO);
        }
        // Weights should be market cap
        assert_eq!(out.optimal_weights[0].weight, dec!(0.5));
        assert_eq!(out.optimal_weights[1].weight, dec!(0.3));
        assert_eq!(out.optimal_weights[2].weight, dec!(0.2));
    }

    // -- 6. High confidence vs low confidence --

    #[test]
    fn test_high_vs_low_confidence() {
        let make_input = |conf: Decimal| {
            two_asset_input(vec![View {
                view_type: ViewType::Absolute,
                assets: vec!["A".into()],
                asset_weights: vec![dec!(1)],
                expected_return: dec!(0.15),
                confidence: conf,
            }])
        };

        let high = run_black_litterman(&make_input(dec!(0.95))).unwrap();
        let low = run_black_litterman(&make_input(dec!(0.2))).unwrap();

        // Higher confidence => posterior closer to view => larger shift
        let high_shift = high.result.prior_vs_posterior[0].shift;
        let low_shift = low.result.prior_vs_posterior[0].shift;
        assert!(high_shift > low_shift);
    }

    // -- 7. Identity covariance matrix --

    #[test]
    fn test_identity_covariance() {
        let input = BlackLittermanInput {
            market_cap_weights: vec![
                AssetWeight {
                    name: "X".into(),
                    weight: dec!(0.5),
                },
                AssetWeight {
                    name: "Y".into(),
                    weight: dec!(0.5),
                },
            ],
            covariance_matrix: vec![vec![dec!(1), dec!(0)], vec![dec!(0), dec!(1)]],
            risk_aversion: dec!(2.5),
            tau: dec!(0.05),
            views: vec![],
            risk_free_rate: dec!(0.02),
        };
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        // Pi = delta * I * w = 2.5 * [0.5, 0.5] = [1.25, 1.25]
        assert_eq!(out.equilibrium_returns[0].expected_return, dec!(1.25));
        assert_eq!(out.equilibrium_returns[1].expected_return, dec!(1.25));
    }

    // -- 8. Validation: non-square covariance --

    #[test]
    fn test_validation_non_square_cov() {
        let mut input = two_asset_input(vec![]);
        input.covariance_matrix = vec![vec![dec!(0.04), dec!(0.006)]];
        assert!(run_black_litterman(&input).is_err());
    }

    // -- 9. Validation: asymmetric covariance --

    #[test]
    fn test_validation_asymmetric_cov() {
        let mut input = two_asset_input(vec![]);
        input.covariance_matrix = vec![vec![dec!(0.04), dec!(0.01)], vec![dec!(0.006), dec!(0.09)]];
        assert!(run_black_litterman(&input).is_err());
    }

    // -- 10. Validation: weights don't sum to 1 --

    #[test]
    fn test_validation_weights_sum() {
        let mut input = two_asset_input(vec![]);
        input.market_cap_weights[0].weight = dec!(0.5);
        input.market_cap_weights[1].weight = dec!(0.1);
        assert!(run_black_litterman(&input).is_err());
    }

    // -- 11. Validation: tau <= 0 --

    #[test]
    fn test_validation_tau() {
        let mut input = two_asset_input(vec![]);
        input.tau = dec!(-0.01);
        assert!(run_black_litterman(&input).is_err());
    }

    // -- 12. Validation: risk_aversion <= 0 --

    #[test]
    fn test_validation_risk_aversion() {
        let mut input = two_asset_input(vec![]);
        input.risk_aversion = Decimal::ZERO;
        assert!(run_black_litterman(&input).is_err());
    }

    // -- 13. Validation: unknown asset in view --

    #[test]
    fn test_validation_unknown_asset() {
        let views = vec![View {
            view_type: ViewType::Absolute,
            assets: vec!["Z".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.10),
            confidence: dec!(0.5),
        }];
        let input = two_asset_input(views);
        assert!(run_black_litterman(&input).is_err());
    }

    // -- 14. Validation: confidence out of range --

    #[test]
    fn test_validation_confidence_zero() {
        let views = vec![View {
            view_type: ViewType::Absolute,
            assets: vec!["A".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.10),
            confidence: Decimal::ZERO,
        }];
        let input = two_asset_input(views);
        assert!(run_black_litterman(&input).is_err());
    }

    // -- 15. Validation: assets/weights length mismatch --

    #[test]
    fn test_validation_view_length_mismatch() {
        let views = vec![View {
            view_type: ViewType::Relative,
            assets: vec!["A".into(), "B".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.02),
            confidence: dec!(0.5),
        }];
        let input = two_asset_input(views);
        assert!(run_black_litterman(&input).is_err());
    }

    // -- 16. Sharpe ratio is computed correctly --

    #[test]
    fn test_sharpe_ratio() {
        let input = two_asset_input(vec![]);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        assert!(out.portfolio_volatility > Decimal::ZERO);
        let expected_sharpe =
            (out.portfolio_expected_return - input.risk_free_rate) / out.portfolio_volatility;
        let diff = (out.portfolio_sharpe - expected_sharpe).abs();
        assert!(diff < dec!(0.0001));
    }

    // -- 17. Multiple views --

    #[test]
    fn test_multiple_views() {
        let views = vec![
            View {
                view_type: ViewType::Absolute,
                assets: vec!["Equity".into()],
                asset_weights: vec![dec!(1)],
                expected_return: dec!(0.10),
                confidence: dec!(0.7),
            },
            View {
                view_type: ViewType::Relative,
                assets: vec!["Equity".into(), "Bonds".into()],
                asset_weights: vec![dec!(1), dec!(-1)],
                expected_return: dec!(0.05),
                confidence: dec!(0.5),
            },
        ];
        let input = three_asset_input(views);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.posterior_returns.len(), 3);
        assert_eq!(out.optimal_weights.len(), 3);
        assert!(out.portfolio_volatility > Decimal::ZERO);
    }

    // -- 18. Weights normalise to 1 --

    #[test]
    fn test_weights_sum_to_one() {
        let views = vec![View {
            view_type: ViewType::Absolute,
            assets: vec!["A".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.12),
            confidence: dec!(0.9),
        }];
        let input = two_asset_input(views);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        let total: Decimal = out.optimal_weights.iter().map(|w| w.weight).sum();
        let diff = (total - Decimal::ONE).abs();
        assert!(
            diff < dec!(0.0001),
            "Weights should sum to ~1, got {}",
            total
        );
    }

    // -- 19. Methodology and metadata --

    #[test]
    fn test_metadata() {
        let input = two_asset_input(vec![]);
        let result = run_black_litterman(&input).unwrap();

        assert_eq!(result.methodology, "Black-Litterman Portfolio Optimisation");
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -- 20. Warning for concentrated position --

    #[test]
    fn test_warning_concentrated_position() {
        let views = vec![View {
            view_type: ViewType::Absolute,
            assets: vec!["A".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.50),
            confidence: dec!(0.99),
        }];
        let input = two_asset_input(views);
        let result = run_black_litterman(&input).unwrap();

        let a_weight = result.result.optimal_weights[0].weight;
        if a_weight > dec!(0.4) {
            let has_warning = result
                .warnings
                .iter()
                .any(|w| w.contains("Concentrated position"));
            assert!(has_warning);
        }
    }

    // -- 21. Portfolio volatility is non-negative --

    #[test]
    fn test_portfolio_volatility_non_negative() {
        let views = vec![View {
            view_type: ViewType::Absolute,
            assets: vec!["A".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.08),
            confidence: dec!(0.5),
        }];
        let input = two_asset_input(views);
        let result = run_black_litterman(&input).unwrap();

        assert!(result.result.portfolio_volatility >= Decimal::ZERO);
    }

    // -- 22. Equilibrium returns are always populated --

    #[test]
    fn test_equilibrium_always_populated() {
        let views = vec![View {
            view_type: ViewType::Absolute,
            assets: vec!["A".into()],
            asset_weights: vec![dec!(1)],
            expected_return: dec!(0.10),
            confidence: dec!(0.6),
        }];
        let input = two_asset_input(views);
        let result = run_black_litterman(&input).unwrap();
        let out = &result.result;

        // Equilibrium returns should not change based on views
        assert_eq!(out.equilibrium_returns[0].expected_return, dec!(0.066));
        assert_eq!(out.equilibrium_returns[1].expected_return, dec!(0.099));
    }
}
