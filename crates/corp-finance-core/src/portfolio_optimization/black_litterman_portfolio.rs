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

/// An investor view in the Black-Litterman framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum View {
    /// Absolute view: "Asset A will return X%."
    Absolute {
        asset_index: usize,
        expected_return: Decimal,
    },
    /// Relative view: "Asset A will outperform Asset B by X%."
    Relative {
        long_index: usize,
        short_index: usize,
        expected_return: Decimal,
    },
}

/// Input to Black-Litterman portfolio optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackLittermanInput {
    /// Asset identifiers.
    pub asset_names: Vec<String>,
    /// Market-capitalization equilibrium weights.
    pub market_cap_weights: Vec<Decimal>,
    /// N x N covariance matrix.
    pub covariance_matrix: Vec<Vec<Decimal>>,
    /// Annual risk-free rate.
    pub risk_free_rate: Decimal,
    /// Market risk-aversion coefficient (lambda, typically 2.5).
    pub risk_aversion: Decimal,
    /// Scaling factor for uncertainty in equilibrium returns (typically 0.05).
    pub tau: Decimal,
    /// Investor views.
    pub views: Vec<View>,
    /// Confidence in each view (0-1, higher = more confident).
    pub view_confidences: Vec<Decimal>,
}

/// A single asset weight with prior comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetWeightBL {
    pub name: String,
    pub weight: Decimal,
    pub market_weight: Decimal,
    pub tilt: Decimal,
}

/// Contribution of a single view to the posterior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewContribution {
    pub view_description: String,
    pub impact_on_return: Decimal,
    pub omega_ii: Decimal,
}

/// Output of Black-Litterman portfolio optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackLittermanOutput {
    /// Implied equilibrium returns: pi = lambda * Sigma * w_mkt.
    pub implied_equilibrium_returns: Vec<Decimal>,
    /// BL posterior combined returns.
    pub posterior_returns: Vec<Decimal>,
    /// Posterior covariance matrix.
    pub posterior_covariance: Vec<Vec<Decimal>>,
    /// Optimal weights with market-cap comparison.
    pub optimal_weights: Vec<AssetWeightBL>,
    /// Portfolio expected return.
    pub portfolio_return: Decimal,
    /// Portfolio standard deviation.
    pub portfolio_risk: Decimal,
    /// Sharpe ratio: (return - rf) / risk.
    pub sharpe_ratio: Decimal,
    /// Per-view contribution metrics.
    pub view_contributions: Vec<ViewContribution>,
    /// Tracking error vs. market portfolio.
    pub tracking_error_vs_market: Decimal,
    /// Information ratio: excess return / tracking error.
    pub information_ratio: Decimal,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Optimize a portfolio using the Black-Litterman framework.
///
/// Combines market equilibrium with investor views to produce posterior
/// expected returns and optimal weights.
pub fn optimize_black_litterman(
    input: &BlackLittermanInput,
) -> CorpFinanceResult<ComputationOutput<BlackLittermanOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    let n = input.asset_names.len();
    validate_input(input, n)?;

    let sigma = &input.covariance_matrix;
    let w_mkt = &input.market_cap_weights;
    let lambda = input.risk_aversion;
    let tau = input.tau;
    let rf = input.risk_free_rate;
    let k = input.views.len();

    // --- Step 1: Implied equilibrium returns: pi = lambda * Sigma * w_mkt ---
    let sigma_w = mat_vec_multiply(sigma, w_mkt);
    let pi: Vec<Decimal> = sigma_w.iter().map(|v| lambda * *v).collect();

    // --- Step 2: Build P (pick) matrix and Q vector from views ---
    let (p_mat, q_vec) = build_pick_matrix_and_q(&input.views, n, k);

    // --- Step 3: Compute tau * Sigma ---
    let tau_sigma = mat_scale(sigma, tau);

    // --- Step 4: Compute Omega (diagonal uncertainty of views) ---
    // Omega_ii = (1/confidence_i - 1) * (P * tau*Sigma * P')_ii
    // This is the "proportional to variance" approach.
    let p_tau_sigma = mat_multiply(&p_mat, &tau_sigma);
    let p_tau_sigma_pt = mat_multiply_transpose_right(&p_tau_sigma, &p_mat);

    let omega = build_omega(&p_tau_sigma_pt, &input.view_confidences, k)?;

    // --- Step 5: Posterior returns ---
    // mu_BL = [(tau*Sigma)^-1 + P'*Omega^-1*P]^-1 * [(tau*Sigma)^-1*pi + P'*Omega^-1*Q]
    let tau_sigma_inv = mat_inverse(&tau_sigma)?;

    let omega_inv = mat_inverse_diagonal(&omega)?;
    let pt = mat_transpose(&p_mat);

    // P' * Omega^-1
    let pt_omega_inv = mat_multiply(&pt, &omega_inv);
    // P' * Omega^-1 * P
    let pt_omega_inv_p = mat_multiply(&pt_omega_inv, &p_mat);
    // P' * Omega^-1 * Q
    let pt_omega_inv_q = mat_vec_multiply(&pt_omega_inv, &q_vec);

    // A = (tau*Sigma)^-1 + P'*Omega^-1*P
    let a_mat = mat_add(&tau_sigma_inv, &pt_omega_inv_p);
    let a_inv = mat_inverse(&a_mat)?;

    // b = (tau*Sigma)^-1 * pi + P'*Omega^-1*Q
    let tau_sigma_inv_pi = mat_vec_multiply(&tau_sigma_inv, &pi);
    let b_vec: Vec<Decimal> = tau_sigma_inv_pi
        .iter()
        .zip(pt_omega_inv_q.iter())
        .map(|(a, b)| *a + *b)
        .collect();

    let mu_bl = mat_vec_multiply(&a_inv, &b_vec);

    // --- Step 6: Posterior covariance: M = A^-1 ---
    let posterior_cov = a_inv.clone();

    // --- Step 7: Optimal weights: w* = (lambda * Sigma)^-1 * mu_BL ---
    let lambda_sigma = mat_scale(sigma, lambda);
    let lambda_sigma_inv = mat_inverse(&lambda_sigma)?;
    let w_opt = mat_vec_multiply(&lambda_sigma_inv, &mu_bl);

    // Normalize weights to sum to 1
    let w_sum: Decimal = w_opt.iter().copied().sum();
    let w_opt_norm: Vec<Decimal> = if w_sum.is_zero() {
        vec![Decimal::ONE / Decimal::from(n as i64); n]
    } else {
        w_opt.iter().map(|w| *w / w_sum).collect()
    };

    // --- Step 8: Portfolio metrics ---
    let port_ret = vec_dot(&w_opt_norm, &mu_bl);
    let port_risk = portfolio_std(&w_opt_norm, sigma);
    let sharpe = if port_risk.is_zero() {
        Decimal::ZERO
    } else {
        (port_ret - rf) / port_risk
    };

    // --- Step 9: Tracking error vs. market portfolio ---
    let diff_w: Vec<Decimal> = w_opt_norm
        .iter()
        .zip(w_mkt.iter())
        .map(|(a, b)| *a - *b)
        .collect();
    let tracking_error = portfolio_std(&diff_w, sigma);

    // --- Step 10: Information ratio ---
    let benchmark_ret = vec_dot(w_mkt, &mu_bl);
    let information_ratio = if tracking_error.is_zero() {
        Decimal::ZERO
    } else {
        (port_ret - benchmark_ret) / tracking_error
    };

    // --- Step 11: Build optimal weights output ---
    let optimal_weights: Vec<AssetWeightBL> = (0..n)
        .map(|i| {
            let tilt = w_opt_norm[i] - w_mkt[i];
            AssetWeightBL {
                name: input.asset_names[i].clone(),
                weight: w_opt_norm[i],
                market_weight: w_mkt[i],
                tilt,
            }
        })
        .collect();

    // --- Step 12: View contributions ---
    let view_contributions =
        compute_view_contributions(&input.views, &input.asset_names, &omega, &pi, &mu_bl, k);

    // --- Warnings ---
    for aw in &optimal_weights {
        if aw.weight > dec!(0.50) {
            warnings.push(format!(
                "Concentrated position: {} has weight {:.4}",
                aw.name, aw.weight
            ));
        }
        if aw.tilt.abs() > dec!(0.20) {
            warnings.push(format!(
                "Large tilt from market: {} tilt = {:.4}",
                aw.name, aw.tilt
            ));
        }
    }
    if tracking_error > dec!(0.10) {
        warnings.push(format!(
            "High tracking error vs. market: {:.4}",
            tracking_error
        ));
    }

    let output = BlackLittermanOutput {
        implied_equilibrium_returns: pi,
        posterior_returns: mu_bl,
        posterior_covariance: posterior_cov,
        optimal_weights,
        portfolio_return: port_ret,
        portfolio_risk: port_risk,
        sharpe_ratio: sharpe,
        view_contributions,
        tracking_error_vs_market: tracking_error,
        information_ratio,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Black-Litterman Portfolio Optimization",
        &serde_json::json!({
            "n_assets": n,
            "n_views": k,
            "risk_aversion": lambda.to_string(),
            "tau": tau.to_string(),
            "risk_free_rate": rf.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &BlackLittermanInput, n: usize) -> CorpFinanceResult<()> {
    if n == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "At least one asset required".into(),
        ));
    }

    if input.market_cap_weights.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_cap_weights".into(),
            reason: format!(
                "Expected {} weights but got {}",
                n,
                input.market_cap_weights.len()
            ),
        });
    }

    validate_covariance_matrix(&input.covariance_matrix, n)?;

    if input.views.len() != input.view_confidences.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "view_confidences".into(),
            reason: format!(
                "Number of views ({}) must match number of confidences ({})",
                input.views.len(),
                input.view_confidences.len()
            ),
        });
    }

    for (i, conf) in input.view_confidences.iter().enumerate() {
        if *conf <= Decimal::ZERO || *conf > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("view_confidences[{}]", i),
                reason: format!("Confidence must be in (0, 1], got {}", conf),
            });
        }
    }

    for (i, view) in input.views.iter().enumerate() {
        match view {
            View::Absolute { asset_index, .. } => {
                if *asset_index >= n {
                    return Err(CorpFinanceError::InvalidInput {
                        field: format!("views[{}]", i),
                        reason: format!("Asset index {} out of range (n={})", asset_index, n),
                    });
                }
            }
            View::Relative {
                long_index,
                short_index,
                ..
            } => {
                if *long_index >= n || *short_index >= n {
                    return Err(CorpFinanceError::InvalidInput {
                        field: format!("views[{}]", i),
                        reason: format!(
                            "Asset index out of range: long={}, short={}, n={}",
                            long_index, short_index, n
                        ),
                    });
                }
                if long_index == short_index {
                    return Err(CorpFinanceError::InvalidInput {
                        field: format!("views[{}]", i),
                        reason: "Relative view must reference two different assets".into(),
                    });
                }
            }
        }
    }

    if input.risk_aversion <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "risk_aversion".into(),
            reason: format!("Must be positive, got {}", input.risk_aversion),
        });
    }

    if input.tau <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "tau".into(),
            reason: format!("Must be positive, got {}", input.tau),
        });
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
// Black-Litterman construction helpers
// ---------------------------------------------------------------------------

/// Build the K x N pick matrix P and K-vector Q from views.
fn build_pick_matrix_and_q(
    views: &[View],
    n: usize,
    k: usize,
) -> (Vec<Vec<Decimal>>, Vec<Decimal>) {
    let mut p = vec![vec![Decimal::ZERO; n]; k];
    let mut q = vec![Decimal::ZERO; k];

    for (row, view) in views.iter().enumerate() {
        match view {
            View::Absolute {
                asset_index,
                expected_return,
            } => {
                p[row][*asset_index] = Decimal::ONE;
                q[row] = *expected_return;
            }
            View::Relative {
                long_index,
                short_index,
                expected_return,
            } => {
                p[row][*long_index] = Decimal::ONE;
                p[row][*short_index] = -Decimal::ONE;
                q[row] = *expected_return;
            }
        }
    }

    (p, q)
}

/// Build diagonal Omega matrix from view confidences.
/// Omega_ii = (1/confidence_i - 1) * (P * tau*Sigma * P')_ii
fn build_omega(
    p_tau_sigma_pt: &[Vec<Decimal>],
    confidences: &[Decimal],
    k: usize,
) -> CorpFinanceResult<Vec<Vec<Decimal>>> {
    let mut omega = vec![vec![Decimal::ZERO; k]; k];
    for i in 0..k {
        let variance_term = p_tau_sigma_pt[i][i];
        let conf = confidences[i];
        // Scale: low confidence => large omega (high uncertainty)
        // (1/c - 1) * variance; when c=1 => 0 uncertainty, c->0 => huge uncertainty
        let scale = (Decimal::ONE / conf) - Decimal::ONE;
        let omega_ii = scale * variance_term;
        // Guard: omega_ii must be positive
        if omega_ii < Decimal::ZERO {
            return Err(CorpFinanceError::FinancialImpossibility(format!(
                "Omega[{},{}] = {} is negative; check covariance/confidence",
                i, i, omega_ii
            )));
        }
        // If omega_ii is exactly zero (confidence = 1), set a tiny floor
        omega[i][i] = if omega_ii.is_zero() {
            dec!(0.0000000001)
        } else {
            omega_ii
        };
    }
    Ok(omega)
}

/// Compute per-view contributions to posterior returns.
fn compute_view_contributions(
    views: &[View],
    asset_names: &[String],
    omega: &[Vec<Decimal>],
    pi: &[Decimal],
    mu_bl: &[Decimal],
    k: usize,
) -> Vec<ViewContribution> {
    let mut contributions = Vec::with_capacity(k);
    for (i, view) in views.iter().enumerate() {
        let (desc, impact) = match view {
            View::Absolute {
                asset_index,
                expected_return,
            } => {
                let name = &asset_names[*asset_index];
                let desc = format!("{} absolute return = {:.4}", name, expected_return);
                let impact = mu_bl[*asset_index] - pi[*asset_index];
                (desc, impact)
            }
            View::Relative {
                long_index,
                short_index,
                expected_return,
            } => {
                let long_name = &asset_names[*long_index];
                let short_name = &asset_names[*short_index];
                let desc = format!(
                    "{} outperforms {} by {:.4}",
                    long_name, short_name, expected_return
                );
                let impact_long = mu_bl[*long_index] - pi[*long_index];
                let impact_short = mu_bl[*short_index] - pi[*short_index];
                (desc, impact_long - impact_short)
            }
        };
        contributions.push(ViewContribution {
            view_description: desc,
            impact_on_return: impact,
            omega_ii: omega[i][i],
        });
    }
    contributions
}

// ---------------------------------------------------------------------------
// Matrix helpers
// ---------------------------------------------------------------------------

/// Dot product of two vectors.
fn vec_dot(a: &[Decimal], b: &[Decimal]) -> Decimal {
    a.iter().zip(b.iter()).map(|(x, y)| *x * *y).sum()
}

/// Matrix-vector multiplication: result_i = sum_j mat[i][j] * v[j].
fn mat_vec_multiply(mat: &[Vec<Decimal>], v: &[Decimal]) -> Vec<Decimal> {
    mat.iter().map(|row| vec_dot(row, v)).collect()
}

/// Matrix-matrix multiplication: C = A * B.
#[allow(clippy::needless_range_loop)]
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

/// Matrix multiplication with transpose of right operand: C = A * B'.
#[allow(clippy::needless_range_loop)]
fn mat_multiply_transpose_right(a: &[Vec<Decimal>], b: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    let m = a.len();
    let n = b.len();
    let p = if m > 0 { a[0].len() } else { 0 };
    let mut c = vec![vec![Decimal::ZERO; n]; m];
    for i in 0..m {
        for j in 0..n {
            let mut sum = Decimal::ZERO;
            for k in 0..p {
                sum += a[i][k] * b[j][k];
            }
            c[i][j] = sum;
        }
    }
    c
}

/// Matrix transpose.
fn mat_transpose(mat: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    if mat.is_empty() {
        return Vec::new();
    }
    let m = mat.len();
    let n = mat[0].len();
    let mut t = vec![vec![Decimal::ZERO; m]; n];
    for i in 0..m {
        for j in 0..n {
            t[j][i] = mat[i][j];
        }
    }
    t
}

/// Element-wise matrix addition: C = A + B.
fn mat_add(a: &[Vec<Decimal>], b: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    a.iter()
        .zip(b.iter())
        .map(|(row_a, row_b)| {
            row_a
                .iter()
                .zip(row_b.iter())
                .map(|(x, y)| *x + *y)
                .collect()
        })
        .collect()
}

/// Scale every element of a matrix by a scalar.
fn mat_scale(mat: &[Vec<Decimal>], s: Decimal) -> Vec<Vec<Decimal>> {
    mat.iter()
        .map(|row| row.iter().map(|v| *v * s).collect())
        .collect()
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

/// Inverse of a diagonal matrix (much faster than general inverse).
fn mat_inverse_diagonal(mat: &[Vec<Decimal>]) -> CorpFinanceResult<Vec<Vec<Decimal>>> {
    let n = mat.len();
    let mut inv = vec![vec![Decimal::ZERO; n]; n];
    for i in 0..n {
        if mat[i][i].is_zero() {
            return Err(CorpFinanceError::DivisionByZero {
                context: format!("Diagonal element [{},{}] is zero", i, i),
            });
        }
        inv[i][i] = Decimal::ONE / mat[i][i];
    }
    Ok(inv)
}

/// Portfolio standard deviation: sqrt(w' * Sigma * w).
fn portfolio_std(w: &[Decimal], sigma: &[Vec<Decimal>]) -> Decimal {
    let sigma_w = mat_vec_multiply(sigma, w);
    let var = vec_dot(w, &sigma_w);
    sqrt_decimal(var)
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

    /// Standard 3-asset covariance matrix for tests.
    /// Asset 0: Equity (15% vol), Asset 1: Bonds (10% vol), Asset 2: Commodities (20% vol)
    /// Correlations: Eq-Bd=0.3, Eq-Cm=0.5, Bd-Cm=0.1
    fn three_asset_cov() -> Vec<Vec<Decimal>> {
        let v0 = dec!(0.15);
        let v1 = dec!(0.10);
        let v2 = dec!(0.20);
        let c01 = dec!(0.3) * v0 * v1;
        let c02 = dec!(0.5) * v0 * v2;
        let c12 = dec!(0.1) * v1 * v2;
        vec![
            vec![v0 * v0, c01, c02],
            vec![c01, v1 * v1, c12],
            vec![c02, c12, v2 * v2],
        ]
    }

    fn three_asset_names() -> Vec<String> {
        vec!["Equity".into(), "Bonds".into(), "Commodities".into()]
    }

    fn three_asset_market_weights() -> Vec<Decimal> {
        vec![dec!(0.50), dec!(0.30), dec!(0.20)]
    }

    fn base_input() -> BlackLittermanInput {
        BlackLittermanInput {
            asset_names: three_asset_names(),
            market_cap_weights: three_asset_market_weights(),
            covariance_matrix: three_asset_cov(),
            risk_free_rate: dec!(0.02),
            risk_aversion: dec!(2.5),
            tau: dec!(0.05),
            views: vec![],
            view_confidences: vec![],
        }
    }

    // ------------------------------------------------------------------
    // 1. Three assets, one absolute view
    // ------------------------------------------------------------------
    #[test]
    fn test_one_absolute_view() {
        let mut input = base_input();
        // View: Equity will return 12%
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.12),
        }];
        input.view_confidences = vec![dec!(0.5)];

        let result = optimize_black_litterman(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.optimal_weights.len(), 3);
        assert_eq!(out.posterior_returns.len(), 3);
        assert_eq!(out.view_contributions.len(), 1);
        assert!(out.portfolio_risk > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // 2. Three assets, one relative view
    // ------------------------------------------------------------------
    #[test]
    fn test_one_relative_view() {
        let mut input = base_input();
        // View: Equity outperforms Bonds by 5%
        input.views = vec![View::Relative {
            long_index: 0,
            short_index: 1,
            expected_return: dec!(0.05),
        }];
        input.view_confidences = vec![dec!(0.6)];

        let result = optimize_black_litterman(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.optimal_weights.len(), 3);
        assert_eq!(out.view_contributions.len(), 1);
        assert!(out.view_contributions[0]
            .view_description
            .contains("outperforms"));
    }

    // ------------------------------------------------------------------
    // 3. Multiple views (absolute + relative)
    // ------------------------------------------------------------------
    #[test]
    fn test_multiple_views() {
        let mut input = base_input();
        input.views = vec![
            View::Absolute {
                asset_index: 0,
                expected_return: dec!(0.12),
            },
            View::Relative {
                long_index: 2,
                short_index: 1,
                expected_return: dec!(0.03),
            },
        ];
        input.view_confidences = vec![dec!(0.5), dec!(0.7)];

        let result = optimize_black_litterman(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.view_contributions.len(), 2);
        assert_eq!(out.posterior_returns.len(), 3);
        assert_eq!(out.posterior_covariance.len(), 3);
        for row in &out.posterior_covariance {
            assert_eq!(row.len(), 3);
        }
    }

    // ------------------------------------------------------------------
    // 4. Implied equilibrium returns calculation
    // ------------------------------------------------------------------
    #[test]
    fn test_implied_equilibrium_returns() {
        let input = base_input();
        // With no views, we still compute pi = lambda * Sigma * w_mkt
        // but we need at least one view for the full BL model.
        // Add a neutral view with low confidence to test pi.
        let mut input_with_view = input.clone();
        input_with_view.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.05), // close to equilibrium
        }];
        input_with_view.view_confidences = vec![dec!(0.01)]; // very low confidence

        let result = optimize_black_litterman(&input_with_view).unwrap();
        let pi = &result.result.implied_equilibrium_returns;

        // pi = lambda * Sigma * w_mkt
        let sigma = &input_with_view.covariance_matrix;
        let w = &input_with_view.market_cap_weights;
        let lambda = input_with_view.risk_aversion;
        let sigma_w = mat_vec_multiply(sigma, w);
        let expected_pi: Vec<Decimal> = sigma_w.iter().map(|v| lambda * *v).collect();

        for i in 0..3 {
            assert!(
                (pi[i] - expected_pi[i]).abs() < dec!(0.0001),
                "pi[{}] = {}, expected {}",
                i,
                pi[i],
                expected_pi[i]
            );
        }
    }

    // ------------------------------------------------------------------
    // 5. Posterior returns tilt toward views
    // ------------------------------------------------------------------
    #[test]
    fn test_posterior_tilts_toward_view() {
        let mut input = base_input();
        // Strong absolute view: Equity returns 20% (much higher than equilibrium)
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.20),
        }];
        input.view_confidences = vec![dec!(0.8)];

        let result = optimize_black_litterman(&input).unwrap();
        let out = &result.result;

        // Posterior return for equity should be higher than equilibrium
        let eq_return = out.implied_equilibrium_returns[0];
        let post_return = out.posterior_returns[0];
        assert!(
            post_return > eq_return,
            "Posterior return {} should exceed equilibrium {} when view is bullish",
            post_return,
            eq_return
        );
    }

    // ------------------------------------------------------------------
    // 6. Tracking error and information ratio
    // ------------------------------------------------------------------
    #[test]
    fn test_tracking_error_and_ir() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.15),
        }];
        input.view_confidences = vec![dec!(0.7)];

        let result = optimize_black_litterman(&input).unwrap();
        let out = &result.result;

        // Tracking error should be non-negative
        assert!(
            out.tracking_error_vs_market >= Decimal::ZERO,
            "Tracking error should be non-negative: {}",
            out.tracking_error_vs_market
        );

        // If tracking error > 0 and there is excess return, IR should be computed
        if out.tracking_error_vs_market > Decimal::ZERO {
            // IR can be positive or negative; just ensure it is finite
            assert!(
                out.information_ratio.abs() < dec!(100),
                "Information ratio seems unreasonable: {}",
                out.information_ratio
            );
        }
    }

    // ------------------------------------------------------------------
    // 7. Weights sum to ~1.0
    // ------------------------------------------------------------------
    #[test]
    fn test_weights_sum_to_one() {
        let mut input = base_input();
        input.views = vec![
            View::Absolute {
                asset_index: 1,
                expected_return: dec!(0.06),
            },
            View::Relative {
                long_index: 0,
                short_index: 2,
                expected_return: dec!(0.04),
            },
        ];
        input.view_confidences = vec![dec!(0.5), dec!(0.5)];

        let result = optimize_black_litterman(&input).unwrap();
        let total: Decimal = result.result.optimal_weights.iter().map(|w| w.weight).sum();
        assert!(
            (total - Decimal::ONE).abs() < dec!(0.01),
            "Weights should sum to ~1.0, got {}",
            total
        );
    }

    // ------------------------------------------------------------------
    // 8. High confidence view strongly shifts returns
    // ------------------------------------------------------------------
    #[test]
    fn test_high_confidence_strong_shift() {
        let mut input_high = base_input();
        input_high.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.20),
        }];
        input_high.view_confidences = vec![dec!(0.99)];

        let mut input_low = base_input();
        input_low.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.20),
        }];
        input_low.view_confidences = vec![dec!(0.10)];

        let result_high = optimize_black_litterman(&input_high).unwrap();
        let result_low = optimize_black_litterman(&input_low).unwrap();

        let shift_high = (result_high.result.posterior_returns[0]
            - result_high.result.implied_equilibrium_returns[0])
            .abs();
        let shift_low = (result_low.result.posterior_returns[0]
            - result_low.result.implied_equilibrium_returns[0])
            .abs();

        assert!(
            shift_high > shift_low,
            "High confidence shift {} should exceed low confidence shift {}",
            shift_high,
            shift_low
        );
    }

    // ------------------------------------------------------------------
    // 9. Low confidence view barely shifts returns
    // ------------------------------------------------------------------
    #[test]
    fn test_low_confidence_minimal_shift() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.20),
        }];
        input.view_confidences = vec![dec!(0.01)]; // very low confidence

        let result = optimize_black_litterman(&input).unwrap();
        let out = &result.result;

        let shift = (out.posterior_returns[0] - out.implied_equilibrium_returns[0]).abs();
        // With 1% confidence, the shift should be very small
        assert!(
            shift < dec!(0.05),
            "Low confidence shift should be small, got {}",
            shift
        );
    }

    // ------------------------------------------------------------------
    // 10. Tilt = weight - market_weight
    // ------------------------------------------------------------------
    #[test]
    fn test_tilt_calculation() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0.5)];

        let result = optimize_black_litterman(&input).unwrap();
        for aw in &result.result.optimal_weights {
            let expected_tilt = aw.weight - aw.market_weight;
            assert!(
                (aw.tilt - expected_tilt).abs() < dec!(0.0001),
                "Tilt mismatch for {}: got {}, expected {}",
                aw.name,
                aw.tilt,
                expected_tilt
            );
        }
    }

    // ------------------------------------------------------------------
    // 11. Sharpe ratio consistency
    // ------------------------------------------------------------------
    #[test]
    fn test_sharpe_ratio_consistency() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 2,
            expected_return: dec!(0.08),
        }];
        input.view_confidences = vec![dec!(0.6)];

        let result = optimize_black_litterman(&input).unwrap();
        let out = &result.result;

        if out.portfolio_risk > Decimal::ZERO {
            let expected_sharpe =
                (out.portfolio_return - input.risk_free_rate) / out.portfolio_risk;
            assert!(
                (out.sharpe_ratio - expected_sharpe).abs() < dec!(0.001),
                "Sharpe mismatch: got {}, expected {}",
                out.sharpe_ratio,
                expected_sharpe
            );
        }
    }

    // ------------------------------------------------------------------
    // 12. Posterior covariance is square and correct dimension
    // ------------------------------------------------------------------
    #[test]
    fn test_posterior_covariance_shape() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0.5)];

        let result = optimize_black_litterman(&input).unwrap();
        let cov = &result.result.posterior_covariance;
        assert_eq!(cov.len(), 3);
        for row in cov {
            assert_eq!(row.len(), 3);
        }
    }

    // ------------------------------------------------------------------
    // 13. Methodology string
    // ------------------------------------------------------------------
    #[test]
    fn test_methodology() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0.5)];

        let result = optimize_black_litterman(&input).unwrap();
        assert_eq!(result.methodology, "Black-Litterman Portfolio Optimization");
    }

    // ------------------------------------------------------------------
    // 14. Metadata precision
    // ------------------------------------------------------------------
    #[test]
    fn test_metadata_precision() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0.5)];

        let result = optimize_black_litterman(&input).unwrap();
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // ------------------------------------------------------------------
    // 15. Relative view tilts weights correctly
    // ------------------------------------------------------------------
    #[test]
    fn test_relative_view_tilts_correctly() {
        let mut input = base_input();
        // View: Equity outperforms Commodities by 8% (strong relative view)
        input.views = vec![View::Relative {
            long_index: 0,
            short_index: 2,
            expected_return: dec!(0.08),
        }];
        input.view_confidences = vec![dec!(0.8)];

        let result = optimize_black_litterman(&input).unwrap();
        let out = &result.result;

        // Equity tilt should be positive (overweight), Commodities should be negative (underweight)
        let eq_tilt = out.optimal_weights[0].tilt;
        let cm_tilt = out.optimal_weights[2].tilt;
        assert!(
            eq_tilt > cm_tilt,
            "Equity tilt {} should exceed Commodities tilt {} for long-short view",
            eq_tilt,
            cm_tilt
        );
    }

    // ------------------------------------------------------------------
    // 16. Validation: empty assets
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_empty_assets() {
        let input = BlackLittermanInput {
            asset_names: vec![],
            market_cap_weights: vec![],
            covariance_matrix: vec![],
            risk_free_rate: dec!(0.02),
            risk_aversion: dec!(2.5),
            tau: dec!(0.05),
            views: vec![],
            view_confidences: vec![],
        };
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 17. Validation: mismatched weights length
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_mismatched_weights() {
        let mut input = base_input();
        input.market_cap_weights = vec![dec!(0.5), dec!(0.5)]; // 2 instead of 3
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0.5)];
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 18. Validation: view confidence out of range
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_confidence_out_of_range() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(1.5)]; // > 1
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 19. Validation: zero confidence
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_zero_confidence() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0)]; // 0 not allowed
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 20. Validation: view index out of range
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_view_index_out_of_range() {
        let mut input = base_input();
        input.views = vec![View::Absolute {
            asset_index: 5, // out of range
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0.5)];
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 21. Validation: relative view same asset
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_relative_same_asset() {
        let mut input = base_input();
        input.views = vec![View::Relative {
            long_index: 1,
            short_index: 1, // same as long
            expected_return: dec!(0.03),
        }];
        input.view_confidences = vec![dec!(0.5)];
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 22. Validation: negative risk aversion
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_negative_risk_aversion() {
        let mut input = base_input();
        input.risk_aversion = dec!(-1.0);
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0.5)];
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 23. Validation: zero tau
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_zero_tau() {
        let mut input = base_input();
        input.tau = Decimal::ZERO;
        input.views = vec![View::Absolute {
            asset_index: 0,
            expected_return: dec!(0.10),
        }];
        input.view_confidences = vec![dec!(0.5)];
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 24. Validation: views/confidences length mismatch
    // ------------------------------------------------------------------
    #[test]
    fn test_validation_views_confidences_mismatch() {
        let mut input = base_input();
        input.views = vec![
            View::Absolute {
                asset_index: 0,
                expected_return: dec!(0.10),
            },
            View::Absolute {
                asset_index: 1,
                expected_return: dec!(0.05),
            },
        ];
        input.view_confidences = vec![dec!(0.5)]; // only 1 confidence for 2 views
        assert!(optimize_black_litterman(&input).is_err());
    }

    // ------------------------------------------------------------------
    // 25. Matrix inverse correctness
    // ------------------------------------------------------------------
    #[test]
    fn test_matrix_inverse_identity() {
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
    // 26. Sqrt helper
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
    // 27. Dot product helper
    // ------------------------------------------------------------------
    #[test]
    fn test_vec_dot() {
        let a = vec![dec!(1), dec!(2), dec!(3)];
        let b = vec![dec!(4), dec!(5), dec!(6)];
        assert_eq!(vec_dot(&a, &b), dec!(32));
    }

    // ------------------------------------------------------------------
    // 28. Pick matrix construction - absolute view
    // ------------------------------------------------------------------
    #[test]
    fn test_pick_matrix_absolute() {
        let views = vec![View::Absolute {
            asset_index: 1,
            expected_return: dec!(0.07),
        }];
        let (p, q) = build_pick_matrix_and_q(&views, 3, 1);
        assert_eq!(p[0], vec![Decimal::ZERO, Decimal::ONE, Decimal::ZERO]);
        assert_eq!(q[0], dec!(0.07));
    }

    // ------------------------------------------------------------------
    // 29. Pick matrix construction - relative view
    // ------------------------------------------------------------------
    #[test]
    fn test_pick_matrix_relative() {
        let views = vec![View::Relative {
            long_index: 0,
            short_index: 2,
            expected_return: dec!(0.03),
        }];
        let (p, q) = build_pick_matrix_and_q(&views, 3, 1);
        assert_eq!(p[0], vec![Decimal::ONE, Decimal::ZERO, -Decimal::ONE]);
        assert_eq!(q[0], dec!(0.03));
    }

    // ------------------------------------------------------------------
    // 30. Matrix transpose
    // ------------------------------------------------------------------
    #[test]
    fn test_mat_transpose() {
        let m = vec![
            vec![dec!(1), dec!(2), dec!(3)],
            vec![dec!(4), dec!(5), dec!(6)],
        ];
        let t = mat_transpose(&m);
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].len(), 2);
        assert_eq!(t[0][0], dec!(1));
        assert_eq!(t[0][1], dec!(4));
        assert_eq!(t[2][1], dec!(6));
    }
}
