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

/// Per-factor risk contribution breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorRiskContribution {
    /// Factor name
    pub factor_name: String,
    /// Portfolio beta (exposure) to this factor
    pub factor_exposure: Decimal,
    /// Factor volatility (sqrt of diagonal of factor covariance)
    pub factor_volatility: Decimal,
    /// Absolute risk contribution from this factor
    pub risk_contribution: Decimal,
    /// Percentage of total risk attributable to this factor
    pub risk_budget_pct: Decimal,
    /// Marginal risk contribution
    pub marginal_risk: Decimal,
}

/// Per-asset factor exposure summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetFactorExposure {
    /// Asset name
    pub asset_name: String,
    /// Portfolio weight
    pub weight: Decimal,
    /// Sum of factor betas for this asset
    pub total_beta: Decimal,
    /// Factor betas for each factor
    pub factor_betas: Vec<Decimal>,
    /// Idiosyncratic risk (sqrt of specific variance)
    pub specific_risk: Decimal,
    /// Total risk contribution of this asset
    pub total_risk_contribution: Decimal,
}

/// Input for factor-based risk budgeting analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorRiskBudgetInput {
    /// Asset identifiers
    pub asset_names: Vec<String>,
    /// Current portfolio weights (N-vector, should sum to 1)
    pub weights: Vec<Decimal>,
    /// Factor identifiers
    pub factor_names: Vec<String>,
    /// N x K matrix of factor loadings (assets x factors)
    pub factor_loadings: Vec<Vec<Decimal>>,
    /// K x K factor covariance matrix
    pub factor_covariance: Vec<Vec<Decimal>>,
    /// N-vector of idiosyncratic variances
    pub specific_variances: Vec<Decimal>,
    /// Optional target risk budget per factor (should sum to 1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_budgets: Option<Vec<Decimal>>,
    /// Whether to solve for optimal weights matching target budgets
    pub rebalance: bool,
}

/// Output of factor-based risk budgeting analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorRiskBudgetOutput {
    /// Portfolio total volatility
    pub total_risk: Decimal,
    /// Risk from factor exposures
    pub systematic_risk: Decimal,
    /// Idiosyncratic risk
    pub specific_risk: Decimal,
    /// Systematic as percentage of total variance
    pub systematic_pct: Decimal,
    /// Per-factor risk contribution breakdown
    pub factor_risk_decomposition: Vec<FactorRiskContribution>,
    /// Per-asset factor exposure summary
    pub asset_factor_exposures: Vec<AssetFactorExposure>,
    /// Actual risk budget proportions
    pub active_risk_budgets: Vec<Decimal>,
    /// Distance from target budgets (if target specified)
    pub budget_tracking_error: Option<Decimal>,
    /// Rebalanced weights (if rebalance=true)
    pub suggested_weights: Option<Vec<Decimal>>,
    /// Proportion of variance explained by factors
    pub r_squared: Decimal,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze factor-based risk budgeting for a portfolio.
///
/// Decomposes portfolio risk into systematic (factor) and specific (idiosyncratic)
/// components, computes per-factor risk contributions, and optionally rebalances
/// to match target risk budgets.
pub fn analyze_factor_risk_budget(
    input: &FactorRiskBudgetInput,
) -> CorpFinanceResult<ComputationOutput<FactorRiskBudgetOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validation
    // ------------------------------------------------------------------
    let n = input.asset_names.len();
    let k = input.factor_names.len();

    if n == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "At least one asset required".into(),
        ));
    }
    if k == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "At least one factor required".into(),
        ));
    }
    if input.weights.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "weights".into(),
            reason: format!("Expected {} weights, got {}", n, input.weights.len()),
        });
    }
    if input.factor_loadings.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "factor_loadings".into(),
            reason: format!(
                "Expected {} rows (one per asset), got {}",
                n,
                input.factor_loadings.len()
            ),
        });
    }
    for (i, row) in input.factor_loadings.iter().enumerate() {
        if row.len() != k {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("factor_loadings[{}]", i),
                reason: format!("Expected {} columns (one per factor), got {}", k, row.len()),
            });
        }
    }
    validate_square_matrix(&input.factor_covariance, k, "factor_covariance")?;
    if input.specific_variances.len() != n {
        return Err(CorpFinanceError::InvalidInput {
            field: "specific_variances".into(),
            reason: format!(
                "Expected {} specific variances, got {}",
                n,
                input.specific_variances.len()
            ),
        });
    }
    for (i, sv) in input.specific_variances.iter().enumerate() {
        if *sv < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("specific_variances[{}]", i),
                reason: "Specific variance must be non-negative".into(),
            });
        }
    }

    if let Some(ref budgets) = input.risk_budgets {
        if budgets.len() != k {
            return Err(CorpFinanceError::InvalidInput {
                field: "risk_budgets".into(),
                reason: format!(
                    "Expected {} risk budgets (one per factor), got {}",
                    k,
                    budgets.len()
                ),
            });
        }
        let budget_sum: Decimal = budgets.iter().copied().sum();
        if (budget_sum - Decimal::ONE).abs() > dec!(0.01) {
            warnings.push(format!(
                "Target risk budgets sum to {} (expected 1.0)",
                budget_sum
            ));
        }
    }

    // Weights sum check
    let weight_sum: Decimal = input.weights.iter().copied().sum();
    if (weight_sum - Decimal::ONE).abs() > dec!(0.01) {
        warnings.push(format!(
            "Portfolio weights sum to {} (expected 1.0)",
            weight_sum
        ));
    }

    // ------------------------------------------------------------------
    // 2. Compute factor covariance: Sigma = B * Sigma_f * B' + D
    // ------------------------------------------------------------------
    // B = factor_loadings (N x K)
    // Sigma_f = factor_covariance (K x K)
    // B' = transpose of B (K x N)
    // D = diag(specific_variances) (N x N)

    let b = &input.factor_loadings;
    let sigma_f = &input.factor_covariance;
    let bt = matrix_transpose(b);

    // B * Sigma_f => N x K
    let b_sigma_f = matrix_multiply(b, sigma_f);
    // (B * Sigma_f) * B' => N x N
    let systematic_cov = matrix_multiply(&b_sigma_f, &bt);

    // Full covariance: Sigma = systematic_cov + D
    let mut full_cov = systematic_cov.clone();
    #[allow(clippy::needless_range_loop)]
    for i in 0..n {
        full_cov[i][i] += input.specific_variances[i];
    }

    // ------------------------------------------------------------------
    // 3. Portfolio variance decomposition
    // ------------------------------------------------------------------
    let w = &input.weights;
    let total_variance = portfolio_variance(w, &full_cov);
    let systematic_variance = portfolio_variance(w, &systematic_cov);

    // Specific variance = total - systematic (or w' D w)
    let specific_variance = if total_variance > systematic_variance {
        total_variance - systematic_variance
    } else {
        Decimal::ZERO
    };

    let total_risk = sqrt_decimal(total_variance);
    let systematic_risk = sqrt_decimal(systematic_variance);
    let specific_risk_val = sqrt_decimal(specific_variance);

    let systematic_pct = if total_variance.is_zero() {
        Decimal::ZERO
    } else {
        systematic_variance / total_variance
    };

    let r_squared = systematic_pct;

    // ------------------------------------------------------------------
    // 4. Factor risk decomposition
    // ------------------------------------------------------------------
    // Portfolio factor exposure: beta_p = B' * w (K x 1)
    let beta_p = matrix_vector_multiply(&bt, w);

    // Sigma_f * beta_p (K x 1)
    let sigma_f_beta_p = matrix_vector_multiply(sigma_f, &beta_p);

    // Factor k contribution to variance: beta_pk * (Sigma_f * beta_p)_k
    let factor_var_contributions: Vec<Decimal> =
        (0..k).map(|ki| beta_p[ki] * sigma_f_beta_p[ki]).collect();

    let factor_var_sum: Decimal = factor_var_contributions.iter().copied().sum();

    let factor_risk_decomposition: Vec<FactorRiskContribution> = (0..k)
        .map(|ki| {
            let factor_vol = sqrt_decimal(sigma_f[ki][ki]);
            let risk_contribution = factor_var_contributions[ki];
            let risk_budget_pct = if total_variance.is_zero() {
                Decimal::ZERO
            } else {
                risk_contribution / total_variance
            };
            // Marginal risk: derivative of portfolio vol w.r.t. beta_pk
            let marginal_risk = if total_risk.is_zero() {
                Decimal::ZERO
            } else {
                sigma_f_beta_p[ki] / total_risk
            };
            FactorRiskContribution {
                factor_name: input.factor_names[ki].clone(),
                factor_exposure: beta_p[ki],
                factor_volatility: factor_vol,
                risk_contribution,
                risk_budget_pct,
                marginal_risk,
            }
        })
        .collect();

    // Active risk budgets (actual proportions of systematic variance per factor)
    let active_risk_budgets: Vec<Decimal> = (0..k)
        .map(|ki| {
            if factor_var_sum.is_zero() {
                Decimal::ZERO
            } else {
                factor_var_contributions[ki] / factor_var_sum
            }
        })
        .collect();

    // ------------------------------------------------------------------
    // 5. Per-asset factor exposure summary
    // ------------------------------------------------------------------
    // Full covariance times weights: Sigma * w
    let sigma_w = matrix_vector_multiply(&full_cov, w);

    let asset_factor_exposures: Vec<AssetFactorExposure> = (0..n)
        .map(|i| {
            let factor_betas = input.factor_loadings[i].clone();
            let total_beta: Decimal = factor_betas.iter().copied().sum();
            let asset_specific_risk = sqrt_decimal(input.specific_variances[i]);
            let total_risk_contribution = if total_risk.is_zero() {
                Decimal::ZERO
            } else {
                w[i] * sigma_w[i] / total_risk
            };
            AssetFactorExposure {
                asset_name: input.asset_names[i].clone(),
                weight: w[i],
                total_beta,
                factor_betas,
                specific_risk: asset_specific_risk,
                total_risk_contribution,
            }
        })
        .collect();

    // ------------------------------------------------------------------
    // 6. Budget tracking error
    // ------------------------------------------------------------------
    let budget_tracking_error = input.risk_budgets.as_ref().map(|target_budgets| {
        let mut sse = Decimal::ZERO;
        for ki in 0..k {
            let diff = active_risk_budgets[ki] - target_budgets[ki];
            sse += diff * diff;
        }
        sqrt_decimal(sse)
    });

    // ------------------------------------------------------------------
    // 7. Rebalancing (gradient descent if enabled)
    // ------------------------------------------------------------------
    let suggested_weights = if input.rebalance {
        if let Some(ref target_budgets) = input.risk_budgets {
            Some(rebalance_to_target(
                input,
                n,
                k,
                target_budgets,
                &mut warnings,
            )?)
        } else {
            warnings.push("Rebalance requested but no target risk_budgets provided".into());
            None
        }
    } else {
        None
    };

    // ------------------------------------------------------------------
    // 8. Build output
    // ------------------------------------------------------------------
    if r_squared < dec!(0.5) {
        warnings.push(format!(
            "Low R-squared ({:.4}): factors explain less than 50% of portfolio variance",
            r_squared
        ));
    }

    let output = FactorRiskBudgetOutput {
        total_risk,
        systematic_risk,
        specific_risk: specific_risk_val,
        systematic_pct,
        factor_risk_decomposition,
        asset_factor_exposures,
        active_risk_budgets,
        budget_tracking_error,
        suggested_weights,
        r_squared,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Factor Risk Budgeting",
        &serde_json::json!({
            "num_assets": n,
            "num_factors": k,
            "rebalance": input.rebalance,
            "has_target_budgets": input.risk_budgets.is_some(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Rebalancing solver
// ---------------------------------------------------------------------------

/// Iterative gradient descent to match target risk budgets.
/// Minimizes sum((actual_budget_k - target_budget_k)^2) subject to
/// weights summing to 1 and in [0, 1] for long-only.
fn rebalance_to_target(
    input: &FactorRiskBudgetInput,
    n: usize,
    k: usize,
    target_budgets: &[Decimal],
    warnings: &mut Vec<String>,
) -> CorpFinanceResult<Vec<Decimal>> {
    let b = &input.factor_loadings;
    let sigma_f = &input.factor_covariance;
    let bt = matrix_transpose(b);

    // Precompute B * Sigma_f (N x K)
    let b_sigma_f = matrix_multiply(b, sigma_f);
    // Systematic covariance: (B * Sigma_f) * B' (N x N)
    let _systematic_cov = matrix_multiply(&b_sigma_f, &bt);

    // Start from current weights
    let mut weights = input.weights.clone();
    let learning_rate = dec!(0.01);

    for _iter in 0..50 {
        // Compute portfolio factor exposure: beta_p = B' * w
        let beta_p = matrix_vector_multiply(&bt, &weights);
        let sigma_f_beta_p = matrix_vector_multiply(sigma_f, &beta_p);

        // Factor variance contributions
        let factor_var_contributions: Vec<Decimal> =
            (0..k).map(|ki| beta_p[ki] * sigma_f_beta_p[ki]).collect();
        let factor_var_sum: Decimal = factor_var_contributions.iter().copied().sum();

        if factor_var_sum.is_zero() {
            break;
        }

        // Current budgets
        let current_budgets: Vec<Decimal> = factor_var_contributions
            .iter()
            .map(|c| *c / factor_var_sum)
            .collect();

        // Check convergence
        let mut sse = Decimal::ZERO;
        for ki in 0..k {
            let diff = current_budgets[ki] - target_budgets[ki];
            sse += diff * diff;
        }
        if sse < dec!(0.000001) {
            break;
        }

        // Gradient: for each weight w_i, compute d(objective)/d(w_i)
        // Using numerical gradient via factor decomposition
        // d(factor_var_k)/d(w_i) = 2 * B[i][k'] * (Sigma_f * beta_p)[k']
        // Approximate: adjust weights proportionally to factor exposure misalignment
        for i in 0..n {
            let mut grad = Decimal::ZERO;
            for ki in 0..k {
                let budget_error = current_budgets[ki] - target_budgets[ki];
                // d(contribution_k)/d(w_i) via chain rule on beta_p
                // beta_p_k = sum_j B[j][k] * w[j], so d(beta_p_k)/d(w_i) = B[i][k]
                let d_beta_p_k = b[i][ki];
                // d(factor_var_k)/d(w_i) = d_beta_p_k * sigma_f_beta_p[k] + beta_p[k] * (Sigma_f * d_beta_p)[k]
                // Simplified: ~ 2 * d_beta_p_k * sigma_f_beta_p[k]
                let d_var_k = dec!(2) * d_beta_p_k * sigma_f_beta_p[ki];
                // Chain rule through normalization
                let d_budget_k = (d_var_k * factor_var_sum
                    - factor_var_contributions[ki] * d_var_k)
                    / (factor_var_sum * factor_var_sum);
                grad += dec!(2) * budget_error * d_budget_k;
            }
            weights[i] -= learning_rate * grad;
        }

        // Project: clamp to [0, 1] and normalize
        for w in &mut weights {
            if *w < Decimal::ZERO {
                *w = Decimal::ZERO;
            }
            if *w > Decimal::ONE {
                *w = Decimal::ONE;
            }
        }
        let total: Decimal = weights.iter().copied().sum();
        if !total.is_zero() {
            for w in &mut weights {
                *w /= total;
            }
        }
    }

    // Final convergence check
    let beta_p = matrix_vector_multiply(&bt, &weights);
    let sigma_f_beta_p = matrix_vector_multiply(sigma_f, &beta_p);
    let factor_var_contributions: Vec<Decimal> =
        (0..k).map(|ki| beta_p[ki] * sigma_f_beta_p[ki]).collect();
    let factor_var_sum: Decimal = factor_var_contributions.iter().copied().sum();
    if factor_var_sum > Decimal::ZERO {
        let mut sse = Decimal::ZERO;
        for ki in 0..k {
            let actual = factor_var_contributions[ki] / factor_var_sum;
            let diff = actual - target_budgets[ki];
            sse += diff * diff;
        }
        if sse > dec!(0.01) {
            warnings.push(format!(
                "Rebalancing did not fully converge (residual SSE: {:.6})",
                sse
            ));
        }
    }

    Ok(weights)
}

// ---------------------------------------------------------------------------
// Matrix helpers
// ---------------------------------------------------------------------------

/// Matrix multiply: C = A * B where A is M x P and B is P x N => C is M x N.
fn matrix_multiply(a: &[Vec<Decimal>], b: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    let m = a.len();
    if m == 0 {
        return vec![];
    }
    let p = a[0].len();
    let n_cols = if b.is_empty() { 0 } else { b[0].len() };

    let mut result = vec![vec![Decimal::ZERO; n_cols]; m];
    for i in 0..m {
        for j in 0..n_cols {
            let mut sum = Decimal::ZERO;
            for l in 0..p {
                sum += a[i][l] * b[l][j];
            }
            result[i][j] = sum;
        }
    }
    result
}

/// Matrix transpose: B[j][i] = A[i][j].
fn matrix_transpose(a: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    if a.is_empty() {
        return vec![];
    }
    let m = a.len();
    let n_cols = a[0].len();
    let mut result = vec![vec![Decimal::ZERO; m]; n_cols];
    for i in 0..m {
        for j in 0..n_cols {
            result[j][i] = a[i][j];
        }
    }
    result
}

/// Matrix-vector multiply: result[i] = sum_j mat[i][j] * v[j].
fn matrix_vector_multiply(mat: &[Vec<Decimal>], v: &[Decimal]) -> Vec<Decimal> {
    mat.iter()
        .map(|row| row.iter().zip(v.iter()).map(|(a, b)| *a * *b).sum())
        .collect()
}

/// Portfolio variance: w' * Sigma * w.
fn portfolio_variance(w: &[Decimal], cov: &[Vec<Decimal>]) -> Decimal {
    let sigma_w = matrix_vector_multiply(cov, w);
    w.iter().zip(sigma_w.iter()).map(|(wi, sw)| *wi * *sw).sum()
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

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_square_matrix(
    mat: &[Vec<Decimal>],
    expected: usize,
    name: &str,
) -> CorpFinanceResult<()> {
    if mat.len() != expected {
        return Err(CorpFinanceError::InvalidInput {
            field: name.into(),
            reason: format!(
                "Expected {}x{} matrix but got {} rows",
                expected,
                expected,
                mat.len()
            ),
        });
    }
    for (i, row) in mat.iter().enumerate() {
        if row.len() != expected {
            return Err(CorpFinanceError::InvalidInput {
                field: name.into(),
                reason: format!("Row {} has {} columns, expected {}", i, row.len(), expected),
            });
        }
    }
    // Symmetry check
    let tolerance = dec!(0.0000001);
    #[allow(clippy::needless_range_loop)]
    for i in 0..expected {
        for j in (i + 1)..expected {
            if (mat[i][j] - mat[j][i]).abs() > tolerance {
                return Err(CorpFinanceError::InvalidInput {
                    field: name.into(),
                    reason: format!(
                        "Matrix is not symmetric: {}[{}][{}]={} != {}[{}][{}]={}",
                        name, i, j, mat[i][j], name, j, i, mat[j][i]
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

    /// Build a simple 2-asset, 1-factor input.
    fn simple_1f_input() -> FactorRiskBudgetInput {
        FactorRiskBudgetInput {
            asset_names: vec!["Stock".into(), "Bond".into()],
            weights: vec![dec!(0.6), dec!(0.4)],
            factor_names: vec!["Market".into()],
            factor_loadings: vec![
                vec![dec!(1.2)], // Stock beta
                vec![dec!(0.3)], // Bond beta
            ],
            factor_covariance: vec![vec![dec!(0.04)]], // Market variance = 4%
            specific_variances: vec![dec!(0.01), dec!(0.005)],
            risk_budgets: None,
            rebalance: false,
        }
    }

    /// Build a 3-asset, 2-factor input.
    fn two_factor_input() -> FactorRiskBudgetInput {
        FactorRiskBudgetInput {
            asset_names: vec!["EquityA".into(), "EquityB".into(), "FixedIncome".into()],
            weights: vec![dec!(0.4), dec!(0.35), dec!(0.25)],
            factor_names: vec!["Market".into(), "Size".into()],
            factor_loadings: vec![
                vec![dec!(1.0), dec!(0.5)],  // EquityA
                vec![dec!(1.2), dec!(-0.3)], // EquityB
                vec![dec!(0.2), dec!(0.1)],  // FixedIncome
            ],
            factor_covariance: vec![vec![dec!(0.04), dec!(0.005)], vec![dec!(0.005), dec!(0.02)]],
            specific_variances: vec![dec!(0.01), dec!(0.015), dec!(0.003)],
            risk_budgets: None,
            rebalance: false,
        }
    }

    /// Build a 4-asset, 3-factor (Fama-French-like) input.
    fn three_factor_input() -> FactorRiskBudgetInput {
        FactorRiskBudgetInput {
            asset_names: vec![
                "LargeCap".into(),
                "SmallCap".into(),
                "Value".into(),
                "Growth".into(),
            ],
            weights: vec![dec!(0.3), dec!(0.2), dec!(0.25), dec!(0.25)],
            factor_names: vec!["MKT".into(), "SMB".into(), "HML".into()],
            factor_loadings: vec![
                vec![dec!(1.0), dec!(-0.2), dec!(0.1)],  // LargeCap
                vec![dec!(1.1), dec!(0.8), dec!(0.0)],   // SmallCap
                vec![dec!(0.9), dec!(0.0), dec!(0.7)],   // Value
                vec![dec!(1.2), dec!(-0.1), dec!(-0.5)], // Growth
            ],
            factor_covariance: vec![
                vec![dec!(0.04), dec!(0.002), dec!(0.001)],
                vec![dec!(0.002), dec!(0.025), dec!(-0.003)],
                vec![dec!(0.001), dec!(-0.003), dec!(0.02)],
            ],
            specific_variances: vec![dec!(0.008), dec!(0.015), dec!(0.01), dec!(0.012)],
            risk_budgets: None,
            rebalance: false,
        }
    }

    // -- Basic functionality tests --

    #[test]
    fn test_single_factor_basic() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        assert!(out.total_risk > Decimal::ZERO);
        assert!(out.systematic_risk > Decimal::ZERO);
        assert!(out.specific_risk > Decimal::ZERO);
        assert!(out.systematic_risk <= out.total_risk);
    }

    #[test]
    fn test_single_factor_r_squared() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        assert!(out.r_squared >= Decimal::ZERO);
        assert!(out.r_squared <= Decimal::ONE);
        assert_eq!(out.r_squared, out.systematic_pct);
    }

    #[test]
    fn test_single_factor_decomposition_count() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.factor_risk_decomposition.len(), 1);
        assert_eq!(out.factor_risk_decomposition[0].factor_name, "Market");
    }

    #[test]
    fn test_single_factor_exposure() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        // Portfolio beta = 0.6 * 1.2 + 0.4 * 0.3 = 0.72 + 0.12 = 0.84
        let expected_beta = dec!(0.84);
        let tolerance = dec!(0.0001);
        assert!(
            (out.factor_risk_decomposition[0].factor_exposure - expected_beta).abs() < tolerance,
            "Expected portfolio beta ~{}, got {}",
            expected_beta,
            out.factor_risk_decomposition[0].factor_exposure
        );
    }

    #[test]
    fn test_two_factor_basic() {
        let input = two_factor_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        assert!(out.total_risk > Decimal::ZERO);
        assert_eq!(out.factor_risk_decomposition.len(), 2);
        assert_eq!(out.asset_factor_exposures.len(), 3);
    }

    #[test]
    fn test_two_factor_budget_sums_to_one() {
        let input = two_factor_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        let budget_sum: Decimal = out.active_risk_budgets.iter().copied().sum();
        let tolerance = dec!(0.01);
        assert!(
            (budget_sum - Decimal::ONE).abs() < tolerance,
            "Active risk budgets should sum to ~1, got {}",
            budget_sum
        );
    }

    #[test]
    fn test_three_factor_basic() {
        let input = three_factor_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        assert!(out.total_risk > Decimal::ZERO);
        assert_eq!(out.factor_risk_decomposition.len(), 3);
        assert_eq!(out.asset_factor_exposures.len(), 4);
    }

    #[test]
    fn test_systematic_plus_specific_equals_total() {
        let input = two_factor_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        // In variance space: systematic_var + specific_var = total_var
        let sys_var = out.systematic_risk * out.systematic_risk;
        let spec_var = out.specific_risk * out.specific_risk;
        let total_var = out.total_risk * out.total_risk;

        let tolerance = dec!(0.0001);
        assert!(
            (sys_var + spec_var - total_var).abs() < tolerance,
            "sys_var({}) + spec_var({}) = {} != total_var({})",
            sys_var,
            spec_var,
            sys_var + spec_var,
            total_var
        );
    }

    #[test]
    fn test_factor_volatility_matches_diagonal() {
        let input = two_factor_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        let tolerance = dec!(0.0001);
        // Market factor: sqrt(0.04) = 0.2
        assert!((out.factor_risk_decomposition[0].factor_volatility - dec!(0.2)).abs() < tolerance);
        // Size factor: sqrt(0.02) ~ 0.1414
        let expected_size_vol = sqrt_decimal(dec!(0.02));
        assert!(
            (out.factor_risk_decomposition[1].factor_volatility - expected_size_vol).abs()
                < tolerance
        );
    }

    #[test]
    fn test_asset_exposures_weights_match_input() {
        let input = two_factor_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        for (i, ae) in out.asset_factor_exposures.iter().enumerate() {
            assert_eq!(ae.weight, input.weights[i]);
            assert_eq!(ae.asset_name, input.asset_names[i]);
        }
    }

    #[test]
    fn test_asset_total_beta() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        // Stock: total_beta = 1.2 (single factor)
        assert_eq!(out.asset_factor_exposures[0].total_beta, dec!(1.2));
        // Bond: total_beta = 0.3
        assert_eq!(out.asset_factor_exposures[1].total_beta, dec!(0.3));
    }

    #[test]
    fn test_asset_specific_risk() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        let tolerance = dec!(0.0001);
        // Stock specific: sqrt(0.01) = 0.1
        assert!((out.asset_factor_exposures[0].specific_risk - dec!(0.1)).abs() < tolerance);
    }

    #[test]
    fn test_no_budgets_no_tracking_error() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        assert!(result.result.budget_tracking_error.is_none());
    }

    #[test]
    fn test_no_rebalance_no_suggested_weights() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        assert!(result.result.suggested_weights.is_none());
    }

    // -- Budget tracking error tests --

    #[test]
    fn test_budget_tracking_error_with_target() {
        let mut input = two_factor_input();
        input.risk_budgets = Some(vec![dec!(0.7), dec!(0.3)]);
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        assert!(out.budget_tracking_error.is_some());
        assert!(out.budget_tracking_error.unwrap() >= Decimal::ZERO);
    }

    #[test]
    fn test_budget_tracking_error_perfect_match() {
        let mut input = two_factor_input();
        // First get actual budgets, then set them as target
        let result1 = analyze_factor_risk_budget(&input).unwrap();
        let actual = result1.result.active_risk_budgets.clone();

        input.risk_budgets = Some(actual);
        let result2 = analyze_factor_risk_budget(&input).unwrap();
        let tracking_err = result2.result.budget_tracking_error.unwrap();

        let tolerance = dec!(0.0001);
        assert!(
            tracking_err < tolerance,
            "Tracking error should be ~0 when target = actual, got {}",
            tracking_err
        );
    }

    // -- Rebalancing tests --

    #[test]
    fn test_rebalance_produces_weights() {
        let mut input = two_factor_input();
        input.risk_budgets = Some(vec![dec!(0.7), dec!(0.3)]);
        input.rebalance = true;
        let result = analyze_factor_risk_budget(&input).unwrap();
        assert!(result.result.suggested_weights.is_some());
    }

    #[test]
    fn test_rebalanced_weights_sum_to_one() {
        let mut input = two_factor_input();
        input.risk_budgets = Some(vec![dec!(0.7), dec!(0.3)]);
        input.rebalance = true;
        let result = analyze_factor_risk_budget(&input).unwrap();
        let weights = result.result.suggested_weights.unwrap();

        let total: Decimal = weights.iter().copied().sum();
        let tolerance = dec!(0.001);
        assert!(
            (total - Decimal::ONE).abs() < tolerance,
            "Rebalanced weights should sum to ~1, got {}",
            total
        );
    }

    #[test]
    fn test_rebalanced_weights_non_negative() {
        let mut input = two_factor_input();
        input.risk_budgets = Some(vec![dec!(0.7), dec!(0.3)]);
        input.rebalance = true;
        let result = analyze_factor_risk_budget(&input).unwrap();
        let weights = result.result.suggested_weights.unwrap();

        for w in &weights {
            assert!(*w >= Decimal::ZERO, "Weight should be >= 0, got {}", w);
        }
    }

    #[test]
    fn test_rebalance_without_budgets_gives_none() {
        let mut input = simple_1f_input();
        input.rebalance = true;
        // No risk_budgets set
        let result = analyze_factor_risk_budget(&input).unwrap();
        assert!(result.result.suggested_weights.is_none());
    }

    #[test]
    fn test_rebalance_three_factor() {
        let mut input = three_factor_input();
        input.risk_budgets = Some(vec![dec!(0.5), dec!(0.25), dec!(0.25)]);
        input.rebalance = true;
        let result = analyze_factor_risk_budget(&input).unwrap();
        assert!(result.result.suggested_weights.is_some());
        let w = result.result.suggested_weights.unwrap();
        assert_eq!(w.len(), 4);
    }

    // -- Marginal risk tests --

    #[test]
    fn test_marginal_risk_positive_for_positive_beta() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        assert!(
            out.factor_risk_decomposition[0].marginal_risk > Decimal::ZERO,
            "Marginal risk should be positive for positive factor exposure"
        );
    }

    #[test]
    fn test_risk_contribution_positive() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        for frc in &out.factor_risk_decomposition {
            assert!(
                frc.risk_contribution >= Decimal::ZERO,
                "Risk contribution for {} should be >= 0, got {}",
                frc.factor_name,
                frc.risk_contribution
            );
        }
    }

    // -- Validation tests --

    #[test]
    fn test_empty_assets_error() {
        let input = FactorRiskBudgetInput {
            asset_names: vec![],
            weights: vec![],
            factor_names: vec!["F1".into()],
            factor_loadings: vec![],
            factor_covariance: vec![vec![dec!(0.04)]],
            specific_variances: vec![],
            risk_budgets: None,
            rebalance: false,
        };
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_empty_factors_error() {
        let input = FactorRiskBudgetInput {
            asset_names: vec!["A".into()],
            weights: vec![dec!(1.0)],
            factor_names: vec![],
            factor_loadings: vec![vec![]],
            factor_covariance: vec![],
            specific_variances: vec![dec!(0.01)],
            risk_budgets: None,
            rebalance: false,
        };
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_weights_length_mismatch() {
        let mut input = simple_1f_input();
        input.weights = vec![dec!(0.5)]; // Only 1 weight for 2 assets
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_factor_loadings_rows_mismatch() {
        let mut input = simple_1f_input();
        input.factor_loadings = vec![vec![dec!(1.0)]]; // Only 1 row for 2 assets
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_factor_loadings_cols_mismatch() {
        let mut input = simple_1f_input();
        input.factor_loadings = vec![
            vec![dec!(1.0), dec!(0.5)], // 2 factors, but only 1 declared
            vec![dec!(0.3), dec!(0.1)],
        ];
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_factor_covariance_wrong_size() {
        let mut input = simple_1f_input();
        input.factor_covariance = vec![vec![dec!(0.04), dec!(0.01)], vec![dec!(0.01), dec!(0.02)]]; // 2x2 but only 1 factor
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_factor_covariance_asymmetric() {
        let mut input = two_factor_input();
        input.factor_covariance[0][1] = dec!(0.1);
        input.factor_covariance[1][0] = dec!(-0.1);
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_specific_variances_length_mismatch() {
        let mut input = simple_1f_input();
        input.specific_variances = vec![dec!(0.01)]; // Only 1 for 2 assets
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_negative_specific_variance_error() {
        let mut input = simple_1f_input();
        input.specific_variances[0] = dec!(-0.01);
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    #[test]
    fn test_risk_budgets_wrong_count() {
        let mut input = two_factor_input();
        input.risk_budgets = Some(vec![dec!(1.0)]); // 1 budget for 2 factors
        assert!(analyze_factor_risk_budget(&input).is_err());
    }

    // -- Warning tests --

    #[test]
    fn test_weight_sum_warning() {
        let mut input = simple_1f_input();
        input.weights = vec![dec!(0.7), dec!(0.5)]; // Sum = 1.2
        let result = analyze_factor_risk_budget(&input).unwrap();
        assert!(result.warnings.iter().any(|w| w.contains("weights sum")));
    }

    #[test]
    fn test_budget_sum_warning() {
        let mut input = two_factor_input();
        input.risk_budgets = Some(vec![dec!(0.6), dec!(0.6)]); // Sum = 1.2
        let result = analyze_factor_risk_budget(&input).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("risk budgets sum")),
            "Expected warning about risk budgets sum, got: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_low_r_squared_warning() {
        // High idiosyncratic risk, low factor loadings => low R^2
        let input = FactorRiskBudgetInput {
            asset_names: vec!["A".into()],
            weights: vec![dec!(1.0)],
            factor_names: vec!["F1".into()],
            factor_loadings: vec![vec![dec!(0.01)]],
            factor_covariance: vec![vec![dec!(0.0001)]],
            specific_variances: vec![dec!(0.10)], // Very high
            risk_budgets: None,
            rebalance: false,
        };
        let result = analyze_factor_risk_budget(&input).unwrap();
        assert!(
            result.warnings.iter().any(|w| w.contains("R-squared")),
            "Expected low R-squared warning, got: {:?}",
            result.warnings
        );
    }

    // -- Edge cases --

    #[test]
    fn test_single_asset_single_factor() {
        let input = FactorRiskBudgetInput {
            asset_names: vec!["Only".into()],
            weights: vec![dec!(1.0)],
            factor_names: vec!["Market".into()],
            factor_loadings: vec![vec![dec!(1.0)]],
            factor_covariance: vec![vec![dec!(0.04)]],
            specific_variances: vec![dec!(0.01)],
            risk_budgets: None,
            rebalance: false,
        };
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.active_risk_budgets.len(), 1);
        assert_eq!(out.asset_factor_exposures.len(), 1);
        // With single factor and weight=1: portfolio beta = 1.0
        let tolerance = dec!(0.001);
        assert!((out.factor_risk_decomposition[0].factor_exposure - dec!(1.0)).abs() < tolerance);
    }

    #[test]
    fn test_zero_factor_loading_asset() {
        // One asset has zero factor loading
        let input = FactorRiskBudgetInput {
            asset_names: vec!["Active".into(), "Cash".into()],
            weights: vec![dec!(0.8), dec!(0.2)],
            factor_names: vec!["Market".into()],
            factor_loadings: vec![
                vec![dec!(1.0)],
                vec![dec!(0.0)], // Cash has no market exposure
            ],
            factor_covariance: vec![vec![dec!(0.04)]],
            specific_variances: vec![dec!(0.01), dec!(0.0001)],
            risk_budgets: None,
            rebalance: false,
        };
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        // Portfolio beta should reflect only Active's contribution
        let tolerance = dec!(0.001);
        let expected_beta = dec!(0.8); // 0.8 * 1.0 + 0.2 * 0.0
        assert!(
            (out.factor_risk_decomposition[0].factor_exposure - expected_beta).abs() < tolerance
        );
    }

    #[test]
    fn test_zero_specific_variances() {
        let mut input = simple_1f_input();
        input.specific_variances = vec![Decimal::ZERO, Decimal::ZERO];
        let result = analyze_factor_risk_budget(&input).unwrap();
        let out = &result.result;

        // All risk should be systematic
        assert_eq!(out.specific_risk, Decimal::ZERO);
        let tolerance = dec!(0.001);
        assert!(
            (out.r_squared - Decimal::ONE).abs() < tolerance,
            "With zero specific variance, R^2 should be ~1, got {}",
            out.r_squared
        );
    }

    #[test]
    fn test_metadata_populated() {
        let input = simple_1f_input();
        let result = analyze_factor_risk_budget(&input).unwrap();

        assert_eq!(result.methodology, "Factor Risk Budgeting");
        assert!(!result.metadata.version.is_empty());
    }

    // -- Matrix helper tests --

    #[test]
    fn test_sqrt_decimal_perfect_squares() {
        let tolerance = dec!(0.0000001);
        assert!((sqrt_decimal(dec!(4)) - dec!(2)).abs() < tolerance);
        assert!((sqrt_decimal(dec!(9)) - dec!(3)).abs() < tolerance);
        assert!((sqrt_decimal(dec!(16)) - dec!(4)).abs() < tolerance);
    }

    #[test]
    fn test_sqrt_decimal_zero_negative() {
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
        assert_eq!(sqrt_decimal(dec!(-1)), Decimal::ZERO);
    }

    #[test]
    fn test_matrix_multiply_identity() {
        let a = vec![vec![dec!(1), dec!(2)], vec![dec!(3), dec!(4)]];
        let identity = vec![
            vec![Decimal::ONE, Decimal::ZERO],
            vec![Decimal::ZERO, Decimal::ONE],
        ];
        let result = matrix_multiply(&a, &identity);
        assert_eq!(result, a);
    }

    #[test]
    fn test_matrix_transpose_simple() {
        let a = vec![
            vec![dec!(1), dec!(2), dec!(3)],
            vec![dec!(4), dec!(5), dec!(6)],
        ];
        let t = matrix_transpose(&a);
        assert_eq!(t.len(), 3);
        assert_eq!(t[0].len(), 2);
        assert_eq!(t[0][0], dec!(1));
        assert_eq!(t[0][1], dec!(4));
        assert_eq!(t[2][1], dec!(6));
    }

    #[test]
    fn test_matrix_vector_multiply_simple() {
        let mat = vec![vec![dec!(1), dec!(2)], vec![dec!(3), dec!(4)]];
        let v = vec![dec!(5), dec!(6)];
        let result = matrix_vector_multiply(&mat, &v);
        assert_eq!(result[0], dec!(17)); // 1*5 + 2*6
        assert_eq!(result[1], dec!(39)); // 3*5 + 4*6
    }

    #[test]
    fn test_portfolio_variance_simple() {
        let w = vec![dec!(0.5), dec!(0.5)];
        let cov = vec![vec![dec!(0.04), dec!(0.01)], vec![dec!(0.01), dec!(0.09)]];
        let var = portfolio_variance(&w, &cov);
        // 0.25*0.04 + 2*0.25*0.01 + 0.25*0.09 = 0.01 + 0.005 + 0.0225 = 0.0375
        let tolerance = dec!(0.0001);
        assert!(
            (var - dec!(0.0375)).abs() < tolerance,
            "Expected 0.0375, got {}",
            var
        );
    }
}
