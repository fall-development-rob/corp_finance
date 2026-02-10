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

/// Method for computing risk-parity weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskParityMethod {
    /// w_i = (1/sigma_i) / sum(1/sigma_j) -- simplest, ignores correlations
    InverseVolatility,
    /// Iterative Newton-like solver targeting equal risk contributions
    EqualRiskContribution,
    /// Minimum variance: w = Sigma^{-1} * 1 / (1' Sigma^{-1} 1)
    MinVariance,
}

/// Descriptor for a single asset in the portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetInfo {
    pub name: String,
    pub expected_return: Decimal,
    pub volatility: Decimal,
}

/// Input for risk-parity portfolio construction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParityInput {
    /// Asset descriptions (names, returns, volatilities)
    pub assets: Vec<AssetInfo>,
    /// NxN covariance matrix (row-major)
    pub covariance_matrix: Vec<Vec<Decimal>>,
    /// Optimisation method
    pub method: RiskParityMethod,
    /// Optional target portfolio volatility -- weights are scaled post-hoc
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_volatility: Option<Decimal>,
    /// Risk-free rate for Sharpe computation (defaults to 0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_free_rate: Option<Decimal>,
}

/// A single asset weight in the optimal portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAllocation {
    pub name: String,
    pub weight: Decimal,
}

/// Risk contribution breakdown for a single asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskContribution {
    pub name: String,
    /// Marginal risk contribution = (Sigma * w)_i / sigma_p
    pub marginal_risk: Decimal,
    /// Total risk contribution = w_i * marginal_risk
    pub risk_contribution: Decimal,
    /// Percentage of total portfolio risk
    pub risk_pct: Decimal,
}

/// Output of the risk-parity optimiser.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskParityOutput {
    pub weights: Vec<AssetAllocation>,
    pub risk_contributions: Vec<RiskContribution>,
    pub portfolio_volatility: Decimal,
    pub portfolio_expected_return: Decimal,
    pub portfolio_sharpe: Decimal,
    /// Weighted-average vol / portfolio vol
    pub diversification_ratio: Decimal,
    /// Herfindahl-based effective number of assets: 1 / sum(w_i^2)
    pub effective_num_assets: Decimal,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute risk-parity portfolio weights.
pub fn calculate_risk_parity(
    input: &RiskParityInput,
) -> CorpFinanceResult<ComputationOutput<RiskParityOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // -- Validation --
    let n = input.assets.len();
    if n == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "At least one asset required".into(),
        ));
    }

    validate_covariance_matrix(&input.covariance_matrix, n)?;

    for (i, asset) in input.assets.iter().enumerate() {
        if asset.volatility <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("assets[{}].volatility", i),
                reason: "Volatility must be positive".into(),
            });
        }
    }

    // -- Compute raw weights --
    let raw_weights = match input.method {
        RiskParityMethod::InverseVolatility => inverse_vol_weights(&input.assets),
        RiskParityMethod::EqualRiskContribution => {
            erc_weights(&input.assets, &input.covariance_matrix)
        }
        RiskParityMethod::MinVariance => min_variance_weights(&input.covariance_matrix, n)?,
    };

    // -- Scale to target volatility if requested --
    let weights = if let Some(target_vol) = input.target_volatility {
        let port_var = portfolio_variance(&raw_weights, &input.covariance_matrix);
        let port_vol = sqrt_decimal(port_var);
        if port_vol.is_zero() {
            raw_weights
        } else {
            let scale = target_vol / port_vol;
            raw_weights.iter().map(|w| *w * scale).collect()
        }
    } else {
        raw_weights
    };

    // -- Portfolio metrics --
    let port_var = portfolio_variance(&weights, &input.covariance_matrix);
    let port_vol = sqrt_decimal(port_var);

    let port_ret: Decimal = weights
        .iter()
        .zip(input.assets.iter())
        .map(|(w, a)| *w * a.expected_return)
        .sum();

    let rf = input.risk_free_rate.unwrap_or(Decimal::ZERO);
    let port_sharpe = if port_vol.is_zero() {
        Decimal::ZERO
    } else {
        (port_ret - rf) / port_vol
    };

    // Diversification ratio = weighted-avg vol / portfolio vol
    let weighted_avg_vol: Decimal = weights
        .iter()
        .zip(input.assets.iter())
        .map(|(w, a)| *w * a.volatility)
        .sum();
    let diversification_ratio = if port_vol.is_zero() {
        Decimal::ONE
    } else {
        weighted_avg_vol / port_vol
    };

    // Effective number of assets (Herfindahl inverse)
    let hhi: Decimal = weights.iter().map(|w| *w * *w).sum();
    let effective_num_assets = if hhi.is_zero() {
        Decimal::ZERO
    } else {
        Decimal::ONE / hhi
    };

    // -- Risk contributions --
    let sigma_w = mat_vec_multiply(&input.covariance_matrix, &weights);
    let risk_contributions: Vec<RiskContribution> = weights
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let marginal = if port_vol.is_zero() {
                Decimal::ZERO
            } else {
                sigma_w[i] / port_vol
            };
            let rc = *w * marginal;
            let pct = if port_vol.is_zero() {
                Decimal::ZERO
            } else {
                rc / port_vol
            };
            RiskContribution {
                name: input.assets[i].name.clone(),
                marginal_risk: marginal,
                risk_contribution: rc,
                risk_pct: pct,
            }
        })
        .collect();

    // -- Build allocations --
    let allocations: Vec<AssetAllocation> = weights
        .iter()
        .enumerate()
        .map(|(i, w)| AssetAllocation {
            name: input.assets[i].name.clone(),
            weight: *w,
        })
        .collect();

    // -- Warnings --
    for alloc in &allocations {
        if alloc.weight > dec!(0.50) {
            warnings.push(format!(
                "Concentrated position: {} has weight {:.2}%",
                alloc.name,
                alloc.weight * dec!(100)
            ));
        }
    }
    if effective_num_assets < dec!(2) && n > 1 {
        warnings.push(format!(
            "Low diversification: effective number of assets is {:.2}",
            effective_num_assets
        ));
    }

    let output = RiskParityOutput {
        weights: allocations,
        risk_contributions,
        portfolio_volatility: port_vol,
        portfolio_expected_return: port_ret,
        portfolio_sharpe: port_sharpe,
        diversification_ratio,
        effective_num_assets,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        &format!("Risk Parity ({:?})", input.method),
        &serde_json::json!({
            "num_assets": n,
            "method": format!("{:?}", input.method),
            "target_volatility": input.target_volatility.map(|v| v.to_string()),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Weight computation methods
// ---------------------------------------------------------------------------

/// Inverse-volatility weights: w_i = (1/sigma_i) / sum(1/sigma_j).
fn inverse_vol_weights(assets: &[AssetInfo]) -> Vec<Decimal> {
    let inv_vols: Vec<Decimal> = assets.iter().map(|a| Decimal::ONE / a.volatility).collect();
    let total: Decimal = inv_vols.iter().sum();
    inv_vols.iter().map(|iv| *iv / total).collect()
}

/// Equal Risk Contribution via iterative Newton-like adjustment (20 iters).
fn erc_weights(assets: &[AssetInfo], cov: &[Vec<Decimal>]) -> Vec<Decimal> {
    let n = assets.len();
    // Start from inverse-vol weights
    let mut weights = inverse_vol_weights(assets);
    let target_rc = Decimal::ONE / Decimal::from(n as i64);

    for _ in 0..20 {
        let port_var = portfolio_variance(&weights, cov);
        if port_var.is_zero() {
            break;
        }

        let sigma_w = mat_vec_multiply(cov, &weights);

        // Compute risk contributions: RC_i = w_i * (Sigma*w)_i / port_var
        let rcs: Vec<Decimal> = weights
            .iter()
            .enumerate()
            .map(|(i, w)| *w * sigma_w[i] / port_var)
            .collect();

        // Adjust weights: multiply by (target_rc / actual_rc)
        for i in 0..n {
            if rcs[i] > Decimal::ZERO {
                weights[i] *= target_rc / rcs[i];
            }
        }

        // Normalise
        let total: Decimal = weights.iter().sum();
        if !total.is_zero() {
            for w in &mut weights {
                *w /= total;
            }
        }
    }

    weights
}

/// Minimum-variance weights: w = Sigma^{-1} * 1 / (1' Sigma^{-1} 1).
fn min_variance_weights(cov: &[Vec<Decimal>], n: usize) -> CorpFinanceResult<Vec<Decimal>> {
    let cov_inv = mat_inverse(cov, n)?;
    let ones = vec![Decimal::ONE; n];
    let inv_ones = mat_vec_multiply(&cov_inv, &ones);
    let denom: Decimal = inv_ones.iter().sum();
    if denom.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "min_variance_weights denominator (1' Sigma^-1 1)".into(),
        });
    }
    Ok(inv_ones.iter().map(|v| *v / denom).collect())
}

// ---------------------------------------------------------------------------
// Matrix helpers (private)
// ---------------------------------------------------------------------------

/// Matrix-vector multiplication: result_i = sum_j(mat[i][j] * vec[j]).
fn mat_vec_multiply(mat: &[Vec<Decimal>], v: &[Decimal]) -> Vec<Decimal> {
    mat.iter().map(|row| vec_dot(row, v)).collect()
}

/// Dot product of two vectors.
fn vec_dot(a: &[Decimal], b: &[Decimal]) -> Decimal {
    a.iter().zip(b.iter()).map(|(x, y)| *x * *y).sum()
}

/// Matrix inverse via Gauss-Jordan elimination with partial pivoting.
#[allow(clippy::needless_range_loop)]
fn mat_inverse(mat: &[Vec<Decimal>], n: usize) -> CorpFinanceResult<Vec<Vec<Decimal>>> {
    // Build augmented matrix [mat | I]
    let mut aug: Vec<Vec<Decimal>> = Vec::with_capacity(n);
    for i in 0..n {
        let mut row = Vec::with_capacity(2 * n);
        for j in 0..n {
            row.push(mat[i][j]);
        }
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
        if max_val.is_zero() {
            return Err(CorpFinanceError::FinancialImpossibility(
                "Covariance matrix is singular (not invertible)".into(),
            ));
        }
        if max_row != col {
            aug.swap(max_row, col);
        }

        // Scale pivot row
        let pivot = aug[col][col];
        for j in 0..(2 * n) {
            aug[col][j] /= pivot;
        }

        // Eliminate other rows
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = aug[row][col];
            for j in 0..(2 * n) {
                let val = aug[col][j] * factor;
                aug[row][j] -= val;
            }
        }
    }

    // Extract right half
    let inv: Vec<Vec<Decimal>> = (0..n).map(|i| aug[i][n..(2 * n)].to_vec()).collect();
    Ok(inv)
}

/// Portfolio variance: w' * Sigma * w.
fn portfolio_variance(weights: &[Decimal], cov: &[Vec<Decimal>]) -> Decimal {
    let sigma_w = mat_vec_multiply(cov, weights);
    vec_dot(weights, &sigma_w)
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
// Validation
// ---------------------------------------------------------------------------

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
    // Symmetry check
    let tolerance = dec!(0.0000001);
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

    /// Helper: build a 2-asset input with given vols and correlation.
    fn two_asset_input(
        vol1: Decimal,
        vol2: Decimal,
        corr: Decimal,
        method: RiskParityMethod,
    ) -> RiskParityInput {
        let cov12 = corr * vol1 * vol2;
        RiskParityInput {
            assets: vec![
                AssetInfo {
                    name: "A".into(),
                    expected_return: dec!(0.08),
                    volatility: vol1,
                },
                AssetInfo {
                    name: "B".into(),
                    expected_return: dec!(0.06),
                    volatility: vol2,
                },
            ],
            covariance_matrix: vec![vec![vol1 * vol1, cov12], vec![cov12, vol2 * vol2]],
            method,
            target_volatility: None,
            risk_free_rate: None,
        }
    }

    /// Helper: build a 3-asset input.
    fn three_asset_input(method: RiskParityMethod) -> RiskParityInput {
        let v1 = dec!(0.15);
        let v2 = dec!(0.20);
        let v3 = dec!(0.25);
        let corr12 = dec!(0.3);
        let corr13 = dec!(0.1);
        let corr23 = dec!(0.5);
        RiskParityInput {
            assets: vec![
                AssetInfo {
                    name: "Equities".into(),
                    expected_return: dec!(0.10),
                    volatility: v1,
                },
                AssetInfo {
                    name: "Bonds".into(),
                    expected_return: dec!(0.04),
                    volatility: v2,
                },
                AssetInfo {
                    name: "Commodities".into(),
                    expected_return: dec!(0.07),
                    volatility: v3,
                },
            ],
            covariance_matrix: vec![
                vec![v1 * v1, corr12 * v1 * v2, corr13 * v1 * v3],
                vec![corr12 * v1 * v2, v2 * v2, corr23 * v2 * v3],
                vec![corr13 * v1 * v3, corr23 * v2 * v3, v3 * v3],
            ],
            method,
            target_volatility: None,
            risk_free_rate: None,
        }
    }

    // -- Inverse Volatility tests --

    #[test]
    fn test_inverse_vol_two_assets() {
        let input = two_asset_input(
            dec!(0.20),
            dec!(0.10),
            dec!(0.5),
            RiskParityMethod::InverseVolatility,
        );
        let result = calculate_risk_parity(&input).unwrap();
        let w = &result.result.weights;
        // 1/0.20=5, 1/0.10=10, total=15 => w1=5/15=1/3, w2=10/15=2/3
        let tolerance = dec!(0.001);
        let one_third = Decimal::ONE / dec!(3);
        let two_thirds = dec!(2) / dec!(3);
        assert!((w[0].weight - one_third).abs() < tolerance);
        assert!((w[1].weight - two_thirds).abs() < tolerance);
    }

    #[test]
    fn test_inverse_vol_equal_vols_give_equal_weights() {
        let input = two_asset_input(
            dec!(0.15),
            dec!(0.15),
            dec!(0.3),
            RiskParityMethod::InverseVolatility,
        );
        let result = calculate_risk_parity(&input).unwrap();
        let w = &result.result.weights;
        let tolerance = dec!(0.0001);
        assert!((w[0].weight - dec!(0.5)).abs() < tolerance);
        assert!((w[1].weight - dec!(0.5)).abs() < tolerance);
    }

    #[test]
    fn test_inverse_vol_three_assets() {
        let input = three_asset_input(RiskParityMethod::InverseVolatility);
        let result = calculate_risk_parity(&input).unwrap();
        let w = &result.result.weights;
        // Lower vol asset should get higher weight
        assert!(w[0].weight > w[1].weight); // 0.15 vol > 0.20 vol
        assert!(w[1].weight > w[2].weight); // 0.20 vol > 0.25 vol
                                            // Weights sum to 1
        let total: Decimal = w.iter().map(|a| a.weight).sum();
        assert!((total - Decimal::ONE).abs() < dec!(0.0001));
    }

    // -- ERC tests --

    #[test]
    fn test_erc_two_assets() {
        let input = two_asset_input(
            dec!(0.20),
            dec!(0.10),
            dec!(0.3),
            RiskParityMethod::EqualRiskContribution,
        );
        let result = calculate_risk_parity(&input).unwrap();
        let rc = &result.result.risk_contributions;
        // ERC should produce roughly equal risk percentages
        let tolerance = dec!(0.05);
        assert!(
            (rc[0].risk_pct - rc[1].risk_pct).abs() < tolerance,
            "Risk contributions should be approximately equal: {:.4} vs {:.4}",
            rc[0].risk_pct,
            rc[1].risk_pct
        );
    }

    #[test]
    fn test_erc_three_assets() {
        let input = three_asset_input(RiskParityMethod::EqualRiskContribution);
        let result = calculate_risk_parity(&input).unwrap();
        let rc = &result.result.risk_contributions;
        let avg_pct: Decimal = rc.iter().map(|r| r.risk_pct).sum::<Decimal>() / dec!(3);
        let tolerance = dec!(0.08);
        for r in rc {
            assert!(
                (r.risk_pct - avg_pct).abs() < tolerance,
                "Risk contribution for {} ({:.4}) deviates from average ({:.4})",
                r.name,
                r.risk_pct,
                avg_pct
            );
        }
    }

    #[test]
    fn test_erc_equal_vols_give_equal_weights() {
        let vol = dec!(0.20);
        let input = two_asset_input(vol, vol, dec!(0.5), RiskParityMethod::EqualRiskContribution);
        let result = calculate_risk_parity(&input).unwrap();
        let w = &result.result.weights;
        let tolerance = dec!(0.01);
        assert!(
            (w[0].weight - w[1].weight).abs() < tolerance,
            "Equal vol assets should get equal weights: {} vs {}",
            w[0].weight,
            w[1].weight
        );
    }

    // -- Min Variance tests --

    #[test]
    fn test_min_variance_two_assets() {
        let input = two_asset_input(
            dec!(0.20),
            dec!(0.10),
            dec!(0.3),
            RiskParityMethod::MinVariance,
        );
        let result = calculate_risk_parity(&input).unwrap();
        let w = &result.result.weights;
        // Lower vol asset should get more weight
        assert!(w[1].weight > w[0].weight);
        let total: Decimal = w.iter().map(|a| a.weight).sum();
        assert!((total - Decimal::ONE).abs() < dec!(0.0001));
    }

    #[test]
    fn test_min_variance_three_assets() {
        let input = three_asset_input(RiskParityMethod::MinVariance);
        let result = calculate_risk_parity(&input).unwrap();
        let w = &result.result.weights;
        let total: Decimal = w.iter().map(|a| a.weight).sum();
        assert!((total - Decimal::ONE).abs() < dec!(0.001));
        assert!(result.result.portfolio_volatility > Decimal::ZERO);
    }

    // -- Target volatility tests --

    #[test]
    fn test_target_volatility_scaling() {
        let mut input = two_asset_input(
            dec!(0.20),
            dec!(0.10),
            dec!(0.3),
            RiskParityMethod::InverseVolatility,
        );
        let base = calculate_risk_parity(&input).unwrap();
        let base_vol = base.result.portfolio_volatility;

        // Set target vol to half of base vol
        let target = base_vol / dec!(2);
        input.target_volatility = Some(target);
        let scaled = calculate_risk_parity(&input).unwrap();

        let tolerance = dec!(0.005);
        assert!(
            (scaled.result.portfolio_volatility - target).abs() < tolerance,
            "Portfolio vol {:.4} should be near target {:.4}",
            scaled.result.portfolio_volatility,
            target
        );
    }

    // -- Diversification and metrics tests --

    #[test]
    fn test_diversification_ratio_gte_one() {
        let input = three_asset_input(RiskParityMethod::InverseVolatility);
        let result = calculate_risk_parity(&input).unwrap();
        assert!(
            result.result.diversification_ratio >= dec!(0.99),
            "Diversification ratio {} should be >= 1",
            result.result.diversification_ratio
        );
    }

    #[test]
    fn test_effective_num_assets() {
        let input = three_asset_input(RiskParityMethod::InverseVolatility);
        let result = calculate_risk_parity(&input).unwrap();
        assert!(result.result.effective_num_assets > Decimal::ONE);
        assert!(result.result.effective_num_assets <= dec!(3));
    }

    #[test]
    fn test_sharpe_with_risk_free_rate() {
        let mut input = two_asset_input(
            dec!(0.15),
            dec!(0.10),
            dec!(0.3),
            RiskParityMethod::InverseVolatility,
        );
        input.risk_free_rate = Some(dec!(0.02));
        let result = calculate_risk_parity(&input).unwrap();
        assert!(result.result.portfolio_sharpe > Decimal::ZERO);
    }

    #[test]
    fn test_sharpe_zero_rf_default() {
        let input = two_asset_input(
            dec!(0.15),
            dec!(0.10),
            dec!(0.3),
            RiskParityMethod::InverseVolatility,
        );
        let result = calculate_risk_parity(&input).unwrap();
        // With rf=0, sharpe = ret / vol which should be positive
        assert!(result.result.portfolio_sharpe > Decimal::ZERO);
    }

    // -- Validation tests --

    #[test]
    fn test_validation_mismatched_covariance_rows() {
        let input = RiskParityInput {
            assets: vec![AssetInfo {
                name: "A".into(),
                expected_return: dec!(0.08),
                volatility: dec!(0.15),
            }],
            covariance_matrix: vec![
                vec![dec!(0.0225)],
                vec![dec!(0.01)], // Extra row
            ],
            method: RiskParityMethod::InverseVolatility,
            target_volatility: None,
            risk_free_rate: None,
        };
        assert!(calculate_risk_parity(&input).is_err());
    }

    #[test]
    fn test_validation_non_square_covariance() {
        let input = RiskParityInput {
            assets: vec![
                AssetInfo {
                    name: "A".into(),
                    expected_return: dec!(0.08),
                    volatility: dec!(0.15),
                },
                AssetInfo {
                    name: "B".into(),
                    expected_return: dec!(0.06),
                    volatility: dec!(0.10),
                },
            ],
            covariance_matrix: vec![
                vec![dec!(0.0225), dec!(0.005), dec!(0.001)],
                vec![dec!(0.005), dec!(0.01)],
            ],
            method: RiskParityMethod::InverseVolatility,
            target_volatility: None,
            risk_free_rate: None,
        };
        assert!(calculate_risk_parity(&input).is_err());
    }

    #[test]
    fn test_validation_asymmetric_covariance() {
        let input = RiskParityInput {
            assets: vec![
                AssetInfo {
                    name: "A".into(),
                    expected_return: dec!(0.08),
                    volatility: dec!(0.15),
                },
                AssetInfo {
                    name: "B".into(),
                    expected_return: dec!(0.06),
                    volatility: dec!(0.10),
                },
            ],
            covariance_matrix: vec![
                vec![dec!(0.0225), dec!(0.005)],
                vec![dec!(0.010), dec!(0.01)],
            ],
            method: RiskParityMethod::InverseVolatility,
            target_volatility: None,
            risk_free_rate: None,
        };
        assert!(calculate_risk_parity(&input).is_err());
    }

    #[test]
    fn test_validation_zero_volatility() {
        let input = RiskParityInput {
            assets: vec![AssetInfo {
                name: "A".into(),
                expected_return: dec!(0.05),
                volatility: Decimal::ZERO,
            }],
            covariance_matrix: vec![vec![Decimal::ZERO]],
            method: RiskParityMethod::InverseVolatility,
            target_volatility: None,
            risk_free_rate: None,
        };
        assert!(calculate_risk_parity(&input).is_err());
    }

    #[test]
    fn test_validation_empty_assets() {
        let input = RiskParityInput {
            assets: vec![],
            covariance_matrix: vec![],
            method: RiskParityMethod::InverseVolatility,
            target_volatility: None,
            risk_free_rate: None,
        };
        assert!(calculate_risk_parity(&input).is_err());
    }

    // -- Warning tests --

    #[test]
    fn test_warning_concentrated_position() {
        let input = two_asset_input(
            dec!(0.50),
            dec!(0.05),
            dec!(0.1),
            RiskParityMethod::InverseVolatility,
        );
        let result = calculate_risk_parity(&input).unwrap();
        assert!(!result.warnings.is_empty());
        assert!(result.warnings.iter().any(|w| w.contains("Concentrated")));
    }

    // -- Sqrt helper test --

    #[test]
    fn test_sqrt_decimal_helper() {
        let result = sqrt_decimal(dec!(4));
        assert!((result - dec!(2)).abs() < dec!(0.0000001));
        let result = sqrt_decimal(dec!(9));
        assert!((result - dec!(3)).abs() < dec!(0.0000001));
        assert_eq!(sqrt_decimal(Decimal::ZERO), Decimal::ZERO);
        assert_eq!(sqrt_decimal(dec!(-1)), Decimal::ZERO);
    }

    // -- Matrix helper tests --

    #[test]
    fn test_mat_vec_multiply_identity() {
        let identity = vec![
            vec![Decimal::ONE, Decimal::ZERO],
            vec![Decimal::ZERO, Decimal::ONE],
        ];
        let v = vec![dec!(3), dec!(5)];
        let result = mat_vec_multiply(&identity, &v);
        assert_eq!(result, v);
    }

    #[test]
    fn test_mat_inverse_identity() {
        let identity = vec![
            vec![Decimal::ONE, Decimal::ZERO],
            vec![Decimal::ZERO, Decimal::ONE],
        ];
        let inv = mat_inverse(&identity, 2).unwrap();
        let tolerance = dec!(0.0000001);
        assert!((inv[0][0] - Decimal::ONE).abs() < tolerance);
        assert!(inv[0][1].abs() < tolerance);
        assert!(inv[1][0].abs() < tolerance);
        assert!((inv[1][1] - Decimal::ONE).abs() < tolerance);
    }

    #[test]
    fn test_mat_inverse_2x2() {
        // [[2, 1], [5, 3]] => inverse [[3, -1], [-5, 2]]
        let mat = vec![vec![dec!(2), dec!(1)], vec![dec!(5), dec!(3)]];
        let inv = mat_inverse(&mat, 2).unwrap();
        let tolerance = dec!(0.0001);
        assert!((inv[0][0] - dec!(3)).abs() < tolerance);
        assert!((inv[0][1] - dec!(-1)).abs() < tolerance);
        assert!((inv[1][0] - dec!(-5)).abs() < tolerance);
        assert!((inv[1][1] - dec!(2)).abs() < tolerance);
    }
}
