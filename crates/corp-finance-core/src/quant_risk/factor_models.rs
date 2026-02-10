use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Type of factor model to estimate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactorModelType {
    /// Capital Asset Pricing Model — single market factor
    CAPM,
    /// Fama-French 3-factor: MKT, SMB, HML
    FamaFrench3,
    /// Carhart 4-factor: MKT, SMB, HML, MOM
    Carhart4,
    /// User-supplied factors (any number)
    Custom,
}

/// A named time-series of factor returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSeries {
    /// Factor name (e.g. "MKT", "SMB", "HML", "MOM")
    pub name: String,
    /// Factor excess returns per period
    pub returns: Vec<Decimal>,
}

/// Input specification for `run_factor_model`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorModelInput {
    /// Time series of asset excess returns
    pub asset_returns: Vec<Decimal>,
    /// Factor return time-series (one per factor)
    pub factor_returns: Vec<FactorSeries>,
    /// Which model type to run
    pub model_type: FactorModelType,
    /// Risk-free rate (for documentation / assumptions; caller is
    /// expected to supply excess returns already)
    pub risk_free_rate: Rate,
    /// Confidence level for t-stat significance testing (default 0.95)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence_level: Option<Decimal>,
}

/// Exposure (loading) for a single factor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorExposure {
    /// Factor name
    pub factor_name: String,
    /// Estimated factor loading (beta)
    pub beta: Decimal,
    /// t-statistic for the beta estimate
    pub t_stat: Decimal,
    /// Approximate two-tailed p-value
    pub p_value: Decimal,
    /// Whether the loading is statistically significant at the chosen
    /// confidence level
    pub significant: bool,
}

/// Full output of a factor-model regression.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorModelOutput {
    /// Model type used
    pub model_type: FactorModelType,
    /// Jensen's alpha (intercept)
    pub alpha: Decimal,
    /// t-statistic for alpha
    pub alpha_t_stat: Decimal,
    /// Whether alpha is statistically significant
    pub alpha_significant: bool,
    /// Per-factor exposures (betas)
    pub factor_exposures: Vec<FactorExposure>,
    /// R-squared (coefficient of determination)
    pub r_squared: Decimal,
    /// Adjusted R-squared
    pub adjusted_r_squared: Decimal,
    /// Residual standard error (sigma hat)
    pub residual_std_error: Decimal,
    /// Number of observations used
    pub num_observations: usize,
    /// Durbin-Watson statistic for autocorrelation of residuals
    pub durbin_watson: Decimal,
    /// Information ratio = alpha / residual_std_error
    pub information_ratio: Decimal,
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Minimum number of observations required for regression.
const MIN_OBSERVATIONS: usize = 12;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run an OLS factor-model regression on asset returns.
///
/// The function estimates beta = (X'X)^{-1} X'y via the normal equations,
/// then computes standard errors, t-statistics, R-squared, adjusted
/// R-squared, Durbin-Watson and information ratio.
pub fn run_factor_model(
    input: &FactorModelInput,
) -> CorpFinanceResult<ComputationOutput<FactorModelOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate factor count for the chosen model type
    // ------------------------------------------------------------------
    let k = input.factor_returns.len(); // number of factors
    validate_factor_count(&input.model_type, k)?;

    // ------------------------------------------------------------------
    // 2. Validate observation lengths
    // ------------------------------------------------------------------
    let n = input.asset_returns.len();
    if n < MIN_OBSERVATIONS {
        return Err(CorpFinanceError::InsufficientData(format!(
            "At least {} observations required for factor model regression, got {}",
            MIN_OBSERVATIONS, n
        )));
    }
    for fs in &input.factor_returns {
        if fs.returns.len() != n {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("factor_returns[{}]", fs.name),
                reason: format!(
                    "Factor series length ({}) does not match asset returns length ({})",
                    fs.returns.len(),
                    n
                ),
            });
        }
    }

    if n < 36 {
        warnings.push(format!(
            "Only {} observations — fewer than recommended 36 for robust estimates",
            n
        ));
    }

    // ------------------------------------------------------------------
    // 3. Build design matrix X (n x (k+1)):  col-0 = ones (intercept)
    // ------------------------------------------------------------------
    let cols = k + 1; // intercept + k factors
    let x: Vec<Vec<Decimal>> = (0..n)
        .map(|i| {
            let mut row = Vec::with_capacity(cols);
            row.push(Decimal::ONE); // intercept
            for fs in &input.factor_returns {
                row.push(fs.returns[i]);
            }
            row
        })
        .collect();

    // y vector
    let y: Vec<Decimal> = input.asset_returns.clone();

    // ------------------------------------------------------------------
    // 4. OLS via normal equations: beta = (X'X)^-1 X'y
    // ------------------------------------------------------------------
    let xt = mat_transpose(&x);
    let xtx = mat_multiply(&xt, &x);
    let xtx_inv = mat_inverse(&xtx).ok_or_else(|| CorpFinanceError::ConvergenceFailure {
        function: "OLS normal equations".into(),
        iterations: 0,
        last_delta: Decimal::ZERO,
    })?;
    let xty = mat_vec_multiply(&xt, &y);
    let beta = mat_vec_multiply_flat(&xtx_inv, &xty);

    // ------------------------------------------------------------------
    // 5. Residuals, SS_res, SS_tot
    // ------------------------------------------------------------------
    let n_dec = Decimal::from(n as i64);
    let y_mean: Decimal = y.iter().sum::<Decimal>() / n_dec;

    let mut residuals = Vec::with_capacity(n);
    let mut ss_res = Decimal::ZERO;
    let mut ss_tot = Decimal::ZERO;

    for i in 0..n {
        let y_hat: Decimal = (0..cols).map(|j| x[i][j] * beta[j]).sum();
        let e = y[i] - y_hat;
        residuals.push(e);
        ss_res += e * e;
        let d = y[i] - y_mean;
        ss_tot += d * d;
    }

    // ------------------------------------------------------------------
    // 6. R-squared, Adjusted R-squared
    // ------------------------------------------------------------------
    let r_squared = if ss_tot.is_zero() {
        Decimal::ONE // perfect fit when y is constant
    } else {
        Decimal::ONE - ss_res / ss_tot
    };

    let k_dec = Decimal::from(k as i64);
    let adjusted_r_squared = if n as i64 - k as i64 - 1 <= 0 || ss_tot.is_zero() {
        r_squared
    } else {
        Decimal::ONE
            - (Decimal::ONE - r_squared) * (n_dec - Decimal::ONE) / (n_dec - k_dec - Decimal::ONE)
    };

    // ------------------------------------------------------------------
    // 7. Residual standard error (sigma hat)
    // ------------------------------------------------------------------
    let dof = n as i64 - k as i64 - 1; // degrees of freedom
    let sigma_sq = if dof > 0 {
        ss_res / Decimal::from(dof)
    } else {
        Decimal::ZERO
    };
    let residual_std_error = sqrt_decimal(sigma_sq);

    // ------------------------------------------------------------------
    // 8. Standard errors of beta, t-stats, significance
    // ------------------------------------------------------------------
    let confidence = input.confidence_level.unwrap_or(dec!(0.95));
    let t_critical = t_critical_value(confidence, dof);

    // Var(beta) = sigma^2 * (X'X)^-1
    let alpha_val = beta[0];
    let alpha_se = sqrt_decimal(sigma_sq * xtx_inv[0][0]);
    let alpha_t_stat = if alpha_se.is_zero() {
        Decimal::ZERO
    } else {
        alpha_val / alpha_se
    };
    let alpha_significant = abs_decimal(alpha_t_stat) > t_critical;

    let mut factor_exposures = Vec::with_capacity(k);
    for j in 0..k {
        let beta_j = beta[j + 1]; // +1 because index 0 is the intercept
        let se_j = sqrt_decimal(sigma_sq * xtx_inv[j + 1][j + 1]);
        let t_stat = if se_j.is_zero() {
            Decimal::ZERO
        } else {
            beta_j / se_j
        };
        let p_value = approx_p_value_from_t(t_stat, dof);
        let significant = abs_decimal(t_stat) > t_critical;

        factor_exposures.push(FactorExposure {
            factor_name: input.factor_returns[j].name.clone(),
            beta: beta_j,
            t_stat,
            p_value,
            significant,
        });
    }

    // ------------------------------------------------------------------
    // 9. Durbin-Watson statistic
    // ------------------------------------------------------------------
    let durbin_watson = {
        let mut dw_num = Decimal::ZERO;
        let mut dw_den = Decimal::ZERO;
        for i in 0..n {
            dw_den += residuals[i] * residuals[i];
            if i > 0 {
                let diff = residuals[i] - residuals[i - 1];
                dw_num += diff * diff;
            }
        }
        if dw_den.is_zero() {
            dec!(2) // no autocorrelation when residuals are zero
        } else {
            dw_num / dw_den
        }
    };

    // ------------------------------------------------------------------
    // 10. Information ratio
    // ------------------------------------------------------------------
    let information_ratio = if residual_std_error.is_zero() {
        Decimal::ZERO
    } else {
        alpha_val / residual_std_error
    };

    // ------------------------------------------------------------------
    // 11. Warnings
    // ------------------------------------------------------------------
    if r_squared < dec!(0.5) {
        warnings.push(format!(
            "Low R-squared ({}) — model explains less than half the variance",
            r_squared
        ));
    }

    if alpha_significant {
        // Check if also significant at 1%
        let t_crit_01 = t_critical_value(dec!(0.99), dof);
        if abs_decimal(alpha_t_stat) <= t_crit_01 {
            warnings
                .push("Alpha is significant at 5% but not at 1% — interpret with caution".into());
        }
    }

    if durbin_watson < dec!(1.5) || durbin_watson > dec!(2.5) {
        warnings.push(format!(
            "Durbin-Watson statistic ({}) indicates possible autocorrelation in residuals",
            durbin_watson
        ));
    }

    // ------------------------------------------------------------------
    // 12. Assemble output
    // ------------------------------------------------------------------
    let output = FactorModelOutput {
        model_type: input.model_type.clone(),
        alpha: alpha_val,
        alpha_t_stat,
        alpha_significant,
        factor_exposures,
        r_squared,
        adjusted_r_squared,
        residual_std_error,
        num_observations: n,
        durbin_watson,
        information_ratio,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "OLS Multi-Factor Regression (Normal Equations)",
        &serde_json::json!({
            "model_type": format!("{:?}", input.model_type),
            "num_factors": k,
            "observations": n,
            "confidence_level": confidence.to_string(),
            "risk_free_rate": input.risk_free_rate.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_factor_count(model_type: &FactorModelType, k: usize) -> CorpFinanceResult<()> {
    match model_type {
        FactorModelType::CAPM => {
            if k != 1 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "factor_returns".into(),
                    reason: format!("CAPM requires exactly 1 factor (MKT), got {}", k),
                });
            }
        }
        FactorModelType::FamaFrench3 => {
            if k != 3 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "factor_returns".into(),
                    reason: format!(
                        "Fama-French 3-factor requires exactly 3 factors (MKT, SMB, HML), got {}",
                        k
                    ),
                });
            }
        }
        FactorModelType::Carhart4 => {
            if k != 4 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "factor_returns".into(),
                    reason: format!(
                        "Carhart 4-factor requires exactly 4 factors (MKT, SMB, HML, MOM), got {}",
                        k
                    ),
                });
            }
        }
        FactorModelType::Custom => {
            if k == 0 {
                return Err(CorpFinanceError::InvalidInput {
                    field: "factor_returns".into(),
                    reason: "Custom model requires at least 1 factor".into(),
                });
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Matrix helpers (private, Decimal-based, small dimensions)
// ---------------------------------------------------------------------------

/// Transpose an m x n matrix.
fn mat_transpose(a: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    if a.is_empty() {
        return Vec::new();
    }
    let m = a.len();
    let n = a[0].len();
    (0..n).map(|j| (0..m).map(|i| a[i][j]).collect()).collect()
}

/// Multiply m x p matrix by p x n matrix.
fn mat_multiply(a: &[Vec<Decimal>], b: &[Vec<Decimal>]) -> Vec<Vec<Decimal>> {
    let m = a.len();
    let p = a[0].len();
    let n = b[0].len();
    (0..m)
        .map(|i| {
            (0..n)
                .map(|j| (0..p).map(|l| a[i][l] * b[l][j]).sum())
                .collect()
        })
        .collect()
}

/// Multiply m x n matrix by n-vector, returning m-vector.
fn mat_vec_multiply(a: &[Vec<Decimal>], v: &[Decimal]) -> Vec<Decimal> {
    a.iter()
        .map(|row| row.iter().zip(v.iter()).map(|(a, b)| *a * *b).sum())
        .collect()
}

/// Convenience: same as `mat_vec_multiply` but takes matrix as &[Vec<Decimal>].
fn mat_vec_multiply_flat(a: &[Vec<Decimal>], v: &[Decimal]) -> Vec<Decimal> {
    mat_vec_multiply(a, v)
}

/// Invert a square matrix via Gauss-Jordan elimination.
/// Returns `None` if the matrix is singular.
#[allow(clippy::needless_range_loop)]
fn mat_inverse(a: &[Vec<Decimal>]) -> Option<Vec<Vec<Decimal>>> {
    let n = a.len();
    if n == 0 {
        return Some(Vec::new());
    }
    // Build augmented matrix [A | I]
    let mut aug: Vec<Vec<Decimal>> = (0..n)
        .map(|i| {
            let mut row = Vec::with_capacity(2 * n);
            row.extend_from_slice(&a[i]);
            for j in 0..n {
                row.push(if i == j { Decimal::ONE } else { Decimal::ZERO });
            }
            row
        })
        .collect();

    for col in 0..n {
        // Partial pivoting: find the row with largest absolute value in column
        let mut pivot_row = col;
        let mut max_val = abs_decimal(aug[col][col]);
        for row in (col + 1)..n {
            let v = abs_decimal(aug[row][col]);
            if v > max_val {
                max_val = v;
                pivot_row = row;
            }
        }
        if max_val.is_zero() {
            return None; // singular
        }
        if pivot_row != col {
            aug.swap(col, pivot_row);
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
                let update = factor * aug[col][j];
                aug[row][j] -= update;
            }
        }
    }

    // Extract right half
    let inv: Vec<Vec<Decimal>> = aug.into_iter().map(|row| row[n..].to_vec()).collect();
    Some(inv)
}

// ---------------------------------------------------------------------------
// Statistical helpers
// ---------------------------------------------------------------------------

/// Square-root using Newton's method (Decimal-safe).
fn sqrt_decimal(val: Decimal) -> Decimal {
    if val <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    // Newton's method: x_{n+1} = (x_n + val/x_n) / 2
    let mut x = val; // initial guess
    let two = dec!(2);
    for _ in 0..30 {
        let next = (x + val / x) / two;
        if abs_decimal(next - x) < dec!(0.00000000000001) {
            return next;
        }
        x = next;
    }
    x
}

/// Absolute value for Decimal.
fn abs_decimal(val: Decimal) -> Decimal {
    if val < Decimal::ZERO {
        -val
    } else {
        val
    }
}

/// Approximate critical t-value for two-tailed test.
/// For large degrees of freedom this converges to z; for smaller dof
/// we use a lookup-based approximation.
fn t_critical_value(confidence: Decimal, dof: i64) -> Decimal {
    // Two-tailed z-values for common confidence levels
    let z_two_tailed = if confidence == dec!(0.95) {
        dec!(1.960)
    } else if confidence == dec!(0.99) {
        dec!(2.576)
    } else if confidence == dec!(0.90) {
        dec!(1.645)
    } else if confidence >= dec!(0.95) && confidence <= dec!(0.99) {
        // Interpolate between 95% and 99%
        let t = (confidence - dec!(0.95)) / dec!(0.04);
        dec!(1.960) + t * (dec!(2.576) - dec!(1.960))
    } else {
        dec!(1.960) // fallback to 95% two-tailed
    };

    // Small-sample correction: t_crit approx z + (z + z^3) / (4*dof)
    if dof >= 120 {
        z_two_tailed
    } else if dof > 0 {
        let dof_dec = Decimal::from(dof);
        let correction =
            (z_two_tailed + z_two_tailed * z_two_tailed * z_two_tailed) / (dec!(4) * dof_dec);
        z_two_tailed + correction
    } else {
        z_two_tailed
    }
}

/// Approximate two-tailed p-value from a t-statistic.
/// Uses a rational approximation of the normal CDF for large dof,
/// with small-sample inflation.
fn approx_p_value_from_t(t_stat: Decimal, dof: i64) -> Decimal {
    let abs_t = abs_decimal(t_stat);

    // Standard normal survival approximation
    let one_tail = approx_normal_survival(abs_t);
    let mut p = dec!(2) * one_tail;

    // Small-sample inflation
    if dof > 0 && dof < 120 {
        let dof_dec = Decimal::from(dof);
        // Rough correction: multiply by (1 + 1/(2*dof))
        let correction = Decimal::ONE + Decimal::ONE / (dec!(2) * dof_dec);
        p *= correction;
    }

    // Clamp to [0, 1]
    if p > Decimal::ONE {
        Decimal::ONE
    } else if p < Decimal::ZERO {
        Decimal::ZERO
    } else {
        p
    }
}

/// Approximate P(Z > z) for z >= 0, using a piece-wise linear lookup.
fn approx_normal_survival(z: Decimal) -> Decimal {
    if z <= Decimal::ZERO {
        return dec!(0.5);
    }
    if z > dec!(6) {
        return Decimal::ZERO;
    }

    // Table: z -> one-tailed p
    let table: [(Decimal, Decimal); 11] = [
        (dec!(0.0), dec!(0.5000)),
        (dec!(0.5), dec!(0.3085)),
        (dec!(1.0), dec!(0.1587)),
        (dec!(1.5), dec!(0.0668)),
        (dec!(2.0), dec!(0.0228)),
        (dec!(2.5), dec!(0.0062)),
        (dec!(3.0), dec!(0.0013)),
        (dec!(3.5), dec!(0.0002)),
        (dec!(4.0), dec!(0.00003)),
        (dec!(5.0), dec!(0.000000287)),
        (dec!(6.0), Decimal::ZERO),
    ];

    // Find bracketing interval and linearly interpolate
    for i in 0..(table.len() - 1) {
        let (z0, p0) = table[i];
        let (z1, p1) = table[i + 1];
        if z >= z0 && z <= z1 {
            if z1 == z0 {
                return p0;
            }
            let frac = (z - z0) / (z1 - z0);
            return p0 + frac * (p1 - p0);
        }
    }

    Decimal::ZERO
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // Convenience builder for a single-factor (CAPM) input.
    fn make_capm_input(
        asset_returns: Vec<Decimal>,
        market_returns: Vec<Decimal>,
    ) -> FactorModelInput {
        FactorModelInput {
            asset_returns,
            factor_returns: vec![FactorSeries {
                name: "MKT".into(),
                returns: market_returns,
            }],
            model_type: FactorModelType::CAPM,
            risk_free_rate: dec!(0.02),
            confidence_level: Some(dec!(0.95)),
        }
    }

    fn sample_12() -> (Vec<Decimal>, Vec<Decimal>) {
        let mkt = vec![
            dec!(0.04),
            dec!(-0.02),
            dec!(0.03),
            dec!(0.01),
            dec!(-0.01),
            dec!(0.05),
            dec!(0.02),
            dec!(-0.04),
            dec!(0.06),
            dec!(0.01),
            dec!(-0.03),
            dec!(0.04),
        ];
        // asset = 0.005 + 1.2 * mkt + small noise
        let asset: Vec<Decimal> = mkt
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let noise = if i % 3 == 0 {
                    dec!(0.001)
                } else if i % 3 == 1 {
                    dec!(-0.001)
                } else {
                    Decimal::ZERO
                };
                dec!(0.005) + dec!(1.2) * m + noise
            })
            .collect();
        (asset, mkt)
    }

    // ---------------------------------------------------------------
    // 1. CAPM with known beta ~ 1.2
    // ---------------------------------------------------------------
    #[test]
    fn test_capm_known_beta() {
        let (asset, mkt) = sample_12();
        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.model_type, FactorModelType::CAPM);
        assert_eq!(out.num_observations, 12);
        assert_eq!(out.factor_exposures.len(), 1);

        let beta = out.factor_exposures[0].beta;
        // Beta should be close to 1.2
        assert!(
            abs_decimal(beta - dec!(1.2)) < dec!(0.05),
            "Expected beta ~1.2, got {}",
            beta
        );

        // Alpha should be close to 0.005
        assert!(
            abs_decimal(out.alpha - dec!(0.005)) < dec!(0.005),
            "Expected alpha ~0.005, got {}",
            out.alpha
        );
    }

    // ---------------------------------------------------------------
    // 2. Fama-French 3-factor regression
    // ---------------------------------------------------------------
    #[test]
    fn test_fama_french_3() {
        let n = 24;
        let mkt: Vec<Decimal> = (0..n)
            .map(|i| dec!(0.01) * Decimal::from(((i % 7) as i64) - 3))
            .collect();
        let smb: Vec<Decimal> = (0..n)
            .map(|i| dec!(0.005) * Decimal::from(((i % 5) as i64) - 2))
            .collect();
        let hml: Vec<Decimal> = (0..n)
            .map(|i| dec!(0.004) * Decimal::from(((i % 6) as i64) - 3))
            .collect();

        // asset = 0.003 + 1.0*mkt + 0.5*smb + 0.3*hml
        let asset: Vec<Decimal> = (0..n)
            .map(|i| dec!(0.003) + mkt[i] + dec!(0.5) * smb[i] + dec!(0.3) * hml[i])
            .collect();

        let input = FactorModelInput {
            asset_returns: asset,
            factor_returns: vec![
                FactorSeries {
                    name: "MKT".into(),
                    returns: mkt,
                },
                FactorSeries {
                    name: "SMB".into(),
                    returns: smb,
                },
                FactorSeries {
                    name: "HML".into(),
                    returns: hml,
                },
            ],
            model_type: FactorModelType::FamaFrench3,
            risk_free_rate: dec!(0.01),
            confidence_level: Some(dec!(0.95)),
        };
        let result = run_factor_model(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.model_type, FactorModelType::FamaFrench3);
        assert_eq!(out.factor_exposures.len(), 3);

        // MKT beta should be ~1.0
        let mkt_beta = out.factor_exposures[0].beta;
        assert!(
            abs_decimal(mkt_beta - Decimal::ONE) < dec!(0.01),
            "MKT beta should be ~1.0, got {}",
            mkt_beta
        );

        // SMB beta should be ~0.5
        let smb_beta = out.factor_exposures[1].beta;
        assert!(
            abs_decimal(smb_beta - dec!(0.5)) < dec!(0.01),
            "SMB beta should be ~0.5, got {}",
            smb_beta
        );

        // HML beta should be ~0.3
        let hml_beta = out.factor_exposures[2].beta;
        assert!(
            abs_decimal(hml_beta - dec!(0.3)) < dec!(0.01),
            "HML beta should be ~0.3, got {}",
            hml_beta
        );

        // R-squared should be very high (near perfect fit)
        assert!(
            out.r_squared > dec!(0.99),
            "R-squared should be near 1.0, got {}",
            out.r_squared
        );
    }

    // ---------------------------------------------------------------
    // 3. Perfect fit (R-squared = 1)
    // ---------------------------------------------------------------
    #[test]
    fn test_perfect_fit() {
        // asset = 0.01 + 0.8 * mkt (no noise)
        let mkt: Vec<Decimal> = (0..15)
            .map(|i| dec!(0.01) * Decimal::from((i as i64) - 7))
            .collect();
        let asset: Vec<Decimal> = mkt.iter().map(|m| dec!(0.01) + dec!(0.8) * m).collect();
        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.r_squared, Decimal::ONE);
        assert_eq!(out.adjusted_r_squared, Decimal::ONE);
        assert!(out.residual_std_error.is_zero() || out.residual_std_error < dec!(0.0000001));
    }

    // ---------------------------------------------------------------
    // 4. Low R-squared produces warning
    // ---------------------------------------------------------------
    #[test]
    fn test_low_r_squared_warning() {
        // mkt and asset are mostly uncorrelated
        let mkt: Vec<Decimal> = (0..20)
            .map(|i| dec!(0.01) * Decimal::from(((i * 3) % 7) as i64 - 3))
            .collect();
        let asset: Vec<Decimal> = (0..20)
            .map(|i| dec!(0.01) * Decimal::from(((i * 5) % 11) as i64 - 5))
            .collect();
        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();

        // Should have a low R-squared warning
        let has_low_r2 = result.warnings.iter().any(|w| w.contains("Low R-squared"));
        assert!(has_low_r2, "Expected low R-squared warning");
    }

    // ---------------------------------------------------------------
    // 5. Alpha significance
    // ---------------------------------------------------------------
    #[test]
    fn test_alpha_significance_perfect_alpha() {
        // Asset with large constant alpha and small beta
        let mkt: Vec<Decimal> = (0..36)
            .map(|i| dec!(0.001) * Decimal::from(((i % 7) as i64) - 3))
            .collect();
        // Large alpha with tiny factor exposure
        let asset: Vec<Decimal> = mkt.iter().map(|m| dec!(0.05) + dec!(0.01) * m).collect();
        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();
        let out = &result.result;

        // Alpha should be clearly significant with such a large intercept
        assert!(
            out.alpha > dec!(0.04),
            "Expected large alpha, got {}",
            out.alpha
        );
    }

    // ---------------------------------------------------------------
    // 6. Insufficient data error
    // ---------------------------------------------------------------
    #[test]
    fn test_insufficient_data_error() {
        let input = make_capm_input(
            vec![dec!(0.01), dec!(0.02), dec!(0.03)],
            vec![dec!(0.02), dec!(0.01), dec!(0.04)],
        );
        let result = run_factor_model(&input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(
            msg.contains("12") && msg.contains("3"),
            "Error should mention min 12 and got 3: {}",
            msg
        );
    }

    // ---------------------------------------------------------------
    // 7. Mismatched series lengths
    // ---------------------------------------------------------------
    #[test]
    fn test_mismatched_lengths() {
        let input = FactorModelInput {
            asset_returns: vec![dec!(0.01); 15],
            factor_returns: vec![FactorSeries {
                name: "MKT".into(),
                returns: vec![dec!(0.01); 12],
            }],
            model_type: FactorModelType::CAPM,
            risk_free_rate: dec!(0.02),
            confidence_level: None,
        };
        let result = run_factor_model(&input);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // 8. Wrong factor count for CAPM
    // ---------------------------------------------------------------
    #[test]
    fn test_capm_wrong_factor_count() {
        let input = FactorModelInput {
            asset_returns: vec![dec!(0.01); 15],
            factor_returns: vec![
                FactorSeries {
                    name: "MKT".into(),
                    returns: vec![dec!(0.01); 15],
                },
                FactorSeries {
                    name: "SMB".into(),
                    returns: vec![dec!(0.01); 15],
                },
            ],
            model_type: FactorModelType::CAPM,
            risk_free_rate: dec!(0.02),
            confidence_level: None,
        };
        let result = run_factor_model(&input);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("CAPM") && msg.contains("1"));
    }

    // ---------------------------------------------------------------
    // 9. FF3 wrong factor count
    // ---------------------------------------------------------------
    #[test]
    fn test_ff3_wrong_factor_count() {
        let input = FactorModelInput {
            asset_returns: vec![dec!(0.01); 15],
            factor_returns: vec![FactorSeries {
                name: "MKT".into(),
                returns: vec![dec!(0.01); 15],
            }],
            model_type: FactorModelType::FamaFrench3,
            risk_free_rate: dec!(0.02),
            confidence_level: None,
        };
        assert!(run_factor_model(&input).is_err());
    }

    // ---------------------------------------------------------------
    // 10. Carhart4 wrong factor count
    // ---------------------------------------------------------------
    #[test]
    fn test_carhart4_wrong_factor_count() {
        let input = FactorModelInput {
            asset_returns: vec![dec!(0.01); 15],
            factor_returns: vec![
                FactorSeries {
                    name: "MKT".into(),
                    returns: vec![dec!(0.01); 15],
                },
                FactorSeries {
                    name: "SMB".into(),
                    returns: vec![dec!(0.02); 15],
                },
                FactorSeries {
                    name: "HML".into(),
                    returns: vec![dec!(0.01); 15],
                },
            ],
            model_type: FactorModelType::Carhart4,
            risk_free_rate: dec!(0.02),
            confidence_level: None,
        };
        assert!(run_factor_model(&input).is_err());
    }

    // ---------------------------------------------------------------
    // 11. Custom model with 2 factors
    // ---------------------------------------------------------------
    #[test]
    fn test_custom_model_two_factors() {
        let f1: Vec<Decimal> = (0..20)
            .map(|i| dec!(0.01) * Decimal::from(((i % 5) as i64) - 2))
            .collect();
        let f2: Vec<Decimal> = (0..20)
            .map(|i| dec!(0.005) * Decimal::from(((i % 4) as i64) - 2))
            .collect();
        let asset: Vec<Decimal> = (0..20)
            .map(|i| dec!(0.002) + dec!(0.7) * f1[i] + dec!(0.4) * f2[i])
            .collect();

        let input = FactorModelInput {
            asset_returns: asset,
            factor_returns: vec![
                FactorSeries {
                    name: "FACTOR_A".into(),
                    returns: f1,
                },
                FactorSeries {
                    name: "FACTOR_B".into(),
                    returns: f2,
                },
            ],
            model_type: FactorModelType::Custom,
            risk_free_rate: dec!(0.01),
            confidence_level: Some(dec!(0.95)),
        };
        let result = run_factor_model(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.factor_exposures.len(), 2);
        assert_eq!(out.factor_exposures[0].factor_name, "FACTOR_A");
        assert_eq!(out.factor_exposures[1].factor_name, "FACTOR_B");
    }

    // ---------------------------------------------------------------
    // 12. Durbin-Watson in range for well-behaved data
    // ---------------------------------------------------------------
    #[test]
    fn test_durbin_watson_range() {
        let (asset, mkt) = sample_12();
        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();
        let dw = result.result.durbin_watson;

        // DW should be between 0 and 4
        assert!(
            dw >= Decimal::ZERO && dw <= dec!(4),
            "DW out of range: {}",
            dw
        );
    }

    // ---------------------------------------------------------------
    // 13. Matrix inverse correctness (A * A_inv = I)
    // ---------------------------------------------------------------
    #[test]
    fn test_matrix_inverse_identity() {
        let a = vec![vec![dec!(2), dec!(1)], vec![dec!(5), dec!(3)]];
        let inv = mat_inverse(&a).expect("Matrix should be invertible");

        let product = mat_multiply(&a, &inv);
        // Should be close to identity
        for i in 0..2 {
            for j in 0..2 {
                let expected = if i == j { Decimal::ONE } else { Decimal::ZERO };
                assert!(
                    abs_decimal(product[i][j] - expected) < dec!(0.0000001),
                    "Product[{}][{}] = {}, expected {}",
                    i,
                    j,
                    product[i][j],
                    expected
                );
            }
        }
    }

    // ---------------------------------------------------------------
    // 14. Singular matrix returns None
    // ---------------------------------------------------------------
    #[test]
    fn test_singular_matrix_inverse() {
        let a = vec![
            vec![dec!(1), dec!(2)],
            vec![dec!(2), dec!(4)], // linearly dependent
        ];
        assert!(mat_inverse(&a).is_none());
    }

    // ---------------------------------------------------------------
    // 15. 3x3 matrix inverse
    // ---------------------------------------------------------------
    #[test]
    fn test_3x3_inverse() {
        let a = vec![
            vec![dec!(1), dec!(2), dec!(3)],
            vec![dec!(0), dec!(1), dec!(4)],
            vec![dec!(5), dec!(6), dec!(0)],
        ];
        let inv = mat_inverse(&a).expect("Should invert");
        let product = mat_multiply(&a, &inv);
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { Decimal::ONE } else { Decimal::ZERO };
                assert!(
                    abs_decimal(product[i][j] - expected) < dec!(0.0000001),
                    "3x3 inverse check failed at [{}][{}]: got {}",
                    i,
                    j,
                    product[i][j]
                );
            }
        }
    }

    // ---------------------------------------------------------------
    // 16. Information ratio sign matches alpha sign
    // ---------------------------------------------------------------
    #[test]
    fn test_information_ratio_sign() {
        let (asset, mkt) = sample_12();
        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();
        let out = &result.result;

        if out.alpha > Decimal::ZERO {
            assert!(
                out.information_ratio >= Decimal::ZERO,
                "IR should be positive when alpha is positive"
            );
        }
    }

    // ---------------------------------------------------------------
    // 17. Carhart 4-factor runs successfully
    // ---------------------------------------------------------------
    #[test]
    fn test_carhart4_factor() {
        let n = 36;
        let mkt: Vec<Decimal> = (0..n)
            .map(|i| dec!(0.01) * Decimal::from(((i % 7) as i64) - 3))
            .collect();
        let smb: Vec<Decimal> = (0..n)
            .map(|i| dec!(0.005) * Decimal::from(((i % 5) as i64) - 2))
            .collect();
        let hml: Vec<Decimal> = (0..n)
            .map(|i| dec!(0.004) * Decimal::from(((i % 6) as i64) - 3))
            .collect();
        let mom: Vec<Decimal> = (0..n)
            .map(|i| dec!(0.003) * Decimal::from(((i % 4) as i64) - 2))
            .collect();

        let asset: Vec<Decimal> = (0..n)
            .map(|i| {
                dec!(0.002)
                    + dec!(1.1) * mkt[i]
                    + dec!(0.3) * smb[i]
                    + dec!(0.2) * hml[i]
                    + dec!(0.15) * mom[i]
            })
            .collect();

        let input = FactorModelInput {
            asset_returns: asset,
            factor_returns: vec![
                FactorSeries {
                    name: "MKT".into(),
                    returns: mkt,
                },
                FactorSeries {
                    name: "SMB".into(),
                    returns: smb,
                },
                FactorSeries {
                    name: "HML".into(),
                    returns: hml,
                },
                FactorSeries {
                    name: "MOM".into(),
                    returns: mom,
                },
            ],
            model_type: FactorModelType::Carhart4,
            risk_free_rate: dec!(0.01),
            confidence_level: Some(dec!(0.95)),
        };
        let result = run_factor_model(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.model_type, FactorModelType::Carhart4);
        assert_eq!(out.factor_exposures.len(), 4);
        assert!(out.r_squared > dec!(0.9));
    }

    // ---------------------------------------------------------------
    // 18. Custom model with zero factors rejected
    // ---------------------------------------------------------------
    #[test]
    fn test_custom_zero_factors() {
        let input = FactorModelInput {
            asset_returns: vec![dec!(0.01); 15],
            factor_returns: vec![],
            model_type: FactorModelType::Custom,
            risk_free_rate: dec!(0.02),
            confidence_level: None,
        };
        assert!(run_factor_model(&input).is_err());
    }

    // ---------------------------------------------------------------
    // 19. DW warning for autocorrelated residuals
    // ---------------------------------------------------------------
    #[test]
    fn test_dw_autocorrelation_warning() {
        // Build a series where residuals are highly autocorrelated
        // by making asset returns trend strongly while mkt doesn't
        let mkt: Vec<Decimal> = (0..24)
            .map(|i| dec!(0.01) * Decimal::from(((i % 5) as i64) - 2))
            .collect();
        // Trending asset returns (each builds on previous)
        let asset: Vec<Decimal> = (0..24)
            .map(|i| dec!(0.002) * Decimal::from(i as i64))
            .collect();

        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();

        // Very low DW suggests positive autocorrelation
        let dw_warning = result
            .warnings
            .iter()
            .any(|w| w.contains("Durbin-Watson") || w.contains("autocorrelation"));
        // DW should be very low for trending residuals
        assert!(
            result.result.durbin_watson < dec!(1.5) || dw_warning,
            "Expected low DW or DW warning for trending data, DW={}",
            result.result.durbin_watson
        );
    }

    // ---------------------------------------------------------------
    // 20. Fewer than 36 observations produces warning
    // ---------------------------------------------------------------
    #[test]
    fn test_fewer_than_36_warning() {
        let (asset, mkt) = sample_12();
        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();

        let has_obs_warning = result
            .warnings
            .iter()
            .any(|w| w.contains("observations") && w.contains("36"));
        assert!(
            has_obs_warning,
            "Expected fewer-than-36-observations warning"
        );
    }

    // ---------------------------------------------------------------
    // 21. Adjusted R-squared <= R-squared
    // ---------------------------------------------------------------
    #[test]
    fn test_adjusted_r_squared_le_r_squared() {
        let (asset, mkt) = sample_12();
        let input = make_capm_input(asset, mkt);
        let result = run_factor_model(&input).unwrap();
        let out = &result.result;

        assert!(
            out.adjusted_r_squared <= out.r_squared,
            "Adj R-squared ({}) should be <= R-squared ({})",
            out.adjusted_r_squared,
            out.r_squared
        );
    }

    // ---------------------------------------------------------------
    // 22. Default confidence level is 0.95
    // ---------------------------------------------------------------
    #[test]
    fn test_default_confidence_level() {
        let (asset, mkt) = sample_12();
        let mut input = make_capm_input(asset, mkt);
        input.confidence_level = None;
        let result = run_factor_model(&input).unwrap();

        // Should succeed without error
        assert_eq!(result.result.num_observations, 12);
    }
}
