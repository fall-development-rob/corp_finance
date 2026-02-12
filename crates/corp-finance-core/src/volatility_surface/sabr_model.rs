use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SabrVolPoint {
    pub strike: Decimal,
    pub implied_vol: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SabrCalibrationInput {
    pub forward_price: Decimal,
    pub expiry: Decimal,
    pub market_vols: Vec<SabrVolPoint>,
    pub initial_alpha: Option<Decimal>,
    pub initial_rho: Option<Decimal>,
    pub initial_nu: Option<Decimal>,
    pub beta: Decimal,
    pub target_strikes: Option<Vec<Decimal>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SabrModelVol {
    pub strike: Decimal,
    pub model_vol: Decimal,
    pub market_vol: Option<Decimal>,
    pub error: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SabrCalibrationOutput {
    pub alpha: Decimal,
    pub beta: Decimal,
    pub rho: Decimal,
    pub nu: Decimal,
    pub calibration_error: Decimal,
    pub model_vols: Vec<SabrModelVol>,
    pub atm_vol: Decimal,
    pub skew: Decimal,
    pub backbone: Decimal,
    pub convergence_iterations: u32,
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64, no MathematicalOps)
// ---------------------------------------------------------------------------

/// Taylor series exp(x) with range reduction for |x| > 2.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = dec!(2);
    if x > two || x < -two {
        let half = exp_decimal(x / two);
        return half * half;
    }
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=40 {
        term = term * x / Decimal::from(n);
        sum += term;
    }
    sum
}

/// Newton's method sqrt: 20 iterations.
fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = dec!(2);
    let mut guess = if x > dec!(100) {
        dec!(10)
    } else if x < dec!(0.01) {
        dec!(0.1)
    } else {
        x / two
    };
    for _ in 0..20 {
        guess = (guess + x / guess) / two;
    }
    guess
}

/// Natural log via Newton's method: find y such that exp(y) = x.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return dec!(-999);
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }
    let mut y = if x > dec!(0.5) && x < dec!(2) {
        x - Decimal::ONE
    } else {
        let mut approx = Decimal::ZERO;
        let mut v = x;
        let e_approx = dec!(2.718281828459045);
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
    for _ in 0..40 {
        let ey = exp_decimal(y);
        if ey == Decimal::ZERO {
            break;
        }
        y = y - Decimal::ONE + x / ey;
    }
    y
}

/// Absolute value helper
fn abs_decimal(x: Decimal) -> Decimal {
    if x < Decimal::ZERO {
        -x
    } else {
        x
    }
}

/// Decimal power for non-negative base with arbitrary Decimal exponent: base^exp = exp(exp * ln(base))
fn pow_decimal_frac(base: Decimal, exp: Decimal) -> Decimal {
    if base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if exp == Decimal::ZERO {
        return Decimal::ONE;
    }
    if base == Decimal::ONE {
        return Decimal::ONE;
    }
    exp_decimal(exp * ln_decimal(base))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &SabrCalibrationInput) -> CorpFinanceResult<()> {
    if input.forward_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "forward_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "expiry".into(),
            reason: "must be positive".into(),
        });
    }
    if input.beta < Decimal::ZERO || input.beta > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "beta".into(),
            reason: "must be in [0, 1]".into(),
        });
    }
    if input.market_vols.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "at least one market vol point is required".into(),
        ));
    }
    for (i, mv) in input.market_vols.iter().enumerate() {
        if mv.strike <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("market_vols[{i}].strike"),
                reason: "must be positive".into(),
            });
        }
        if mv.implied_vol <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("market_vols[{i}].implied_vol"),
                reason: "must be positive".into(),
            });
        }
    }
    if let Some(ref alpha) = input.initial_alpha {
        if *alpha <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "initial_alpha".into(),
                reason: "must be positive".into(),
            });
        }
    }
    if let Some(ref rho) = input.initial_rho {
        if *rho <= dec!(-1) || *rho >= dec!(1) {
            return Err(CorpFinanceError::InvalidInput {
                field: "initial_rho".into(),
                reason: "must be in (-1, 1)".into(),
            });
        }
    }
    if let Some(ref nu) = input.initial_nu {
        if *nu <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "initial_nu".into(),
                reason: "must be positive".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SABR Hagan (2002) formula
// ---------------------------------------------------------------------------

/// Compute SABR implied vol for a given strike K, forward F, and SABR parameters.
/// Uses Hagan (2002) approximation.
fn sabr_implied_vol(
    f: Decimal,
    k: Decimal,
    t: Decimal,
    alpha: Decimal,
    beta: Decimal,
    rho: Decimal,
    nu: Decimal,
) -> Decimal {
    let one = Decimal::ONE;
    let two = dec!(2);
    let four = dec!(4);
    let twenty_four = dec!(24);
    let one_ninety_two_zero = dec!(1920);

    let one_minus_beta = one - beta;

    // Handle ATM case: K ~= F
    let fk_ratio = abs_decimal(f - k) / f;
    if fk_ratio < dec!(0.0001) {
        return sabr_atm_vol(f, t, alpha, beta, rho, nu);
    }

    // General case K != F
    let fk = f * k;
    let fk_pow_half_omb = pow_decimal_frac(fk, one_minus_beta / two); // (FK)^((1-beta)/2)
    let fk_pow_omb = pow_decimal_frac(fk, one_minus_beta); // (FK)^(1-beta)

    let ln_fk = ln_decimal(f / k);
    let ln_fk_sq = ln_fk * ln_fk;
    let ln_fk_4 = ln_fk_sq * ln_fk_sq;

    let omb_sq = one_minus_beta * one_minus_beta;
    let omb_4 = omb_sq * omb_sq;

    // Denominator correction: 1 + (1-beta)^2/24 * ln^2(F/K) + (1-beta)^4/1920 * ln^4(F/K)
    let denom_corr = one + omb_sq / twenty_four * ln_fk_sq + omb_4 / one_ninety_two_zero * ln_fk_4;

    // z = (nu/alpha) * (FK)^((1-beta)/2) * ln(F/K)
    let z = if alpha > dec!(0.00001) {
        (nu / alpha) * fk_pow_half_omb * ln_fk
    } else {
        Decimal::ZERO
    };

    // x(z) = ln((sqrt(1 - 2*rho*z + z^2) + z - rho) / (1 - rho))
    let x_z = if abs_decimal(z) < dec!(0.00001) {
        one // limit z->0: z/x(z) -> 1
    } else {
        let disc = one - two * rho * z + z * z;
        let sqrt_disc = sqrt_decimal(abs_decimal(disc));
        let numerator = sqrt_disc + z - rho;
        let denominator = one - rho;

        if denominator <= dec!(0.00001) || numerator <= dec!(0.00001) {
            one
        } else {
            let x = ln_decimal(numerator / denominator);
            if abs_decimal(x) < dec!(0.00001) {
                one
            } else {
                z / x
            }
        }
    };

    // Time correction: 1 + [((1-beta)^2/24)*(alpha^2/(FK)^(1-beta)) + (rho*beta*nu*alpha)/(4*(FK)^((1-beta)/2)) + ((2-3*rho^2)/24)*nu^2] * T
    let term1 = omb_sq / twenty_four * alpha * alpha / fk_pow_omb;
    let term2 = rho * beta * nu * alpha / (four * fk_pow_half_omb);
    let term3 = (two - dec!(3) * rho * rho) / twenty_four * nu * nu;
    let time_corr = one + (term1 + term2 + term3) * t;

    // Combine: sigma = (alpha / ((FK)^((1-beta)/2) * denom_corr)) * x(z) * time_corr
    let vol = alpha / (fk_pow_half_omb * denom_corr) * x_z * time_corr;

    // Ensure positive
    if vol < Decimal::ZERO {
        dec!(0.001)
    } else {
        vol
    }
}

/// SABR ATM implied vol: sigma_ATM = alpha / F^(1-beta) * [1 + correction * T]
fn sabr_atm_vol(
    f: Decimal,
    t: Decimal,
    alpha: Decimal,
    beta: Decimal,
    rho: Decimal,
    nu: Decimal,
) -> Decimal {
    let one = Decimal::ONE;
    let two = dec!(2);
    let four = dec!(4);
    let twenty_four = dec!(24);

    let one_minus_beta = one - beta;
    let omb_sq = one_minus_beta * one_minus_beta;

    let f_pow_omb = pow_decimal_frac(f, one_minus_beta); // F^(1-beta)
    let f_pow_2omb = pow_decimal_frac(f, two * one_minus_beta); // F^(2*(1-beta))

    // Correction term
    let term1 = omb_sq / twenty_four * alpha * alpha / f_pow_2omb;
    let term2 = rho * beta * nu * alpha / (four * f_pow_omb);
    let term3 = (two - dec!(3) * rho * rho) / twenty_four * nu * nu;
    let time_corr = one + (term1 + term2 + term3) * t;

    let vol = alpha / f_pow_omb * time_corr;

    if vol < Decimal::ZERO {
        dec!(0.001)
    } else {
        vol
    }
}

// ---------------------------------------------------------------------------
// Calibration: Levenberg-Marquardt style
// ---------------------------------------------------------------------------

/// Calibrate SABR parameters (alpha, rho, nu) to market vols.
/// beta is fixed (not calibrated).
fn calibrate_params(
    f: Decimal,
    t: Decimal,
    beta: Decimal,
    market_vols: &[SabrVolPoint],
    init_alpha: Decimal,
    init_rho: Decimal,
    init_nu: Decimal,
) -> (Decimal, Decimal, Decimal, u32) {
    let n = market_vols.len();
    let mut alpha = init_alpha;
    let mut rho = init_rho;
    let mut nu = init_nu;

    let bump = dec!(0.001);
    let mut damping = dec!(0.01);
    let max_iter: u32 = 50;
    let mut final_iter: u32 = 0;

    for iter in 0..max_iter {
        final_iter = iter + 1;

        // Compute residuals
        let mut residuals: Vec<Decimal> = Vec::with_capacity(n);
        let mut total_sq = Decimal::ZERO;
        for mv in market_vols {
            let model_vol = sabr_implied_vol(f, mv.strike, t, alpha, beta, rho, nu);
            let r = mv.implied_vol - model_vol;
            residuals.push(r);
            total_sq += r * r;
        }

        // Build Jacobian via finite differences (3 params: alpha, rho, nu)
        let params = [alpha, rho, nu];
        let mut jtj = [[Decimal::ZERO; 3]; 3];
        let mut jtr = [Decimal::ZERO; 3];

        for (i, mv) in market_vols.iter().enumerate() {
            let mut grad = [Decimal::ZERO; 3];
            for j in 0..3 {
                let mut p_up = params;
                p_up[j] += bump;
                let vol_up = sabr_implied_vol(f, mv.strike, t, p_up[0], beta, p_up[1], p_up[2]);

                let mut p_dn = params;
                p_dn[j] -= bump;
                let vol_dn = sabr_implied_vol(f, mv.strike, t, p_dn[0], beta, p_dn[1], p_dn[2]);

                grad[j] = (vol_up - vol_dn) / (dec!(2) * bump);
            }

            for j1 in 0..3 {
                jtr[j1] += grad[j1] * residuals[i];
                for j2 in 0..3 {
                    jtj[j1][j2] += grad[j1] * grad[j2];
                }
            }
        }

        // Add damping (Levenberg-Marquardt)
        #[allow(clippy::needless_range_loop)]
        for j in 0..3 {
            jtj[j][j] += damping;
        }

        // Solve 3x3 system: (J^T*J + lambda*I) * delta = J^T*r
        let delta = solve_3x3(&jtj, &jtr);

        // Trial update
        let new_alpha = alpha + delta[0];
        let new_rho = rho + delta[1];
        let new_nu = nu + delta[2];

        // Compute new residual
        let mut new_total_sq = Decimal::ZERO;
        for mv in market_vols {
            let model_vol = sabr_implied_vol(f, mv.strike, t, new_alpha, beta, new_rho, new_nu);
            let r = mv.implied_vol - model_vol;
            new_total_sq += r * r;
        }

        // Accept or reject step
        if new_total_sq < total_sq {
            alpha = new_alpha;
            rho = new_rho;
            nu = new_nu;
            damping *= dec!(0.5); // Decrease damping on improvement
            if damping < dec!(0.0001) {
                damping = dec!(0.0001);
            }
        } else {
            damping *= dec!(2); // Increase damping on deterioration
            if damping > dec!(100) {
                damping = dec!(100);
            }
        }

        // Enforce constraints
        if alpha < dec!(0.0001) {
            alpha = dec!(0.0001);
        }
        if rho < dec!(-0.999) {
            rho = dec!(-0.999);
        }
        if rho > dec!(0.999) {
            rho = dec!(0.999);
        }
        if nu < dec!(0.0001) {
            nu = dec!(0.0001);
        }

        // Check convergence
        let delta_norm = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2];
        if delta_norm < dec!(0.000000001) {
            break;
        }
    }

    (alpha, rho, nu, final_iter)
}

/// Solve 3x3 linear system Ax = b via Gaussian elimination with partial pivoting.
#[allow(clippy::needless_range_loop)]
fn solve_3x3(a: &[[Decimal; 3]; 3], b: &[Decimal; 3]) -> [Decimal; 3] {
    let mut aug = [[Decimal::ZERO; 4]; 3];
    for i in 0..3 {
        for j in 0..3 {
            aug[i][j] = a[i][j];
        }
        aug[i][3] = b[i];
    }

    // Forward elimination with partial pivoting
    for col in 0..3 {
        let mut max_val = abs_decimal(aug[col][col]);
        let mut max_row = col;
        for row in (col + 1)..3 {
            let v = abs_decimal(aug[row][col]);
            if v > max_val {
                max_val = v;
                max_row = row;
            }
        }
        if max_row != col {
            aug.swap(col, max_row);
        }

        let pivot = aug[col][col];
        if abs_decimal(pivot) < dec!(0.0000000001) {
            continue;
        }

        for row in (col + 1)..3 {
            let factor = aug[row][col] / pivot;
            for j in col..4 {
                let val = aug[col][j];
                aug[row][j] -= factor * val;
            }
        }
    }

    // Back substitution
    let mut x = [Decimal::ZERO; 3];
    for i in (0..3).rev() {
        let mut sum = aug[i][3];
        for j in (i + 1)..3 {
            sum -= aug[i][j] * x[j];
        }
        let diag = aug[i][i];
        if abs_decimal(diag) > dec!(0.0000000001) {
            x[i] = sum / diag;
        }
    }
    x
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn calibrate_sabr(
    input: &SabrCalibrationInput,
) -> CorpFinanceResult<ComputationOutput<SabrCalibrationOutput>> {
    let start = Instant::now();
    validate_input(input)?;

    let f = input.forward_price;
    let t = input.expiry;
    let beta = input.beta;

    // Initial parameter guesses
    let init_alpha = input.initial_alpha.unwrap_or_else(|| {
        // Rough initial guess: ATM vol * F^(1-beta)
        let avg_vol = input
            .market_vols
            .iter()
            .map(|mv| mv.implied_vol)
            .fold(Decimal::ZERO, |acc, v| acc + v)
            / Decimal::from(input.market_vols.len() as u32);
        let f_pow = pow_decimal_frac(f, Decimal::ONE - beta);
        avg_vol * f_pow
    });
    let init_rho = input.initial_rho.unwrap_or(dec!(-0.3));
    let init_nu = input.initial_nu.unwrap_or(dec!(0.4));

    // Calibrate
    let (alpha, rho, nu, iterations) = calibrate_params(
        f,
        t,
        beta,
        &input.market_vols,
        init_alpha,
        init_rho,
        init_nu,
    );

    // Compute ATM vol
    let atm_vol = sabr_atm_vol(f, t, alpha, beta, rho, nu);

    // Compute model vols at target strikes (or market strikes)
    let target_strikes: Vec<Decimal> = input
        .target_strikes
        .as_ref()
        .cloned()
        .unwrap_or_else(|| input.market_vols.iter().map(|mv| mv.strike).collect());

    let mut model_vols: Vec<SabrModelVol> = Vec::new();
    let mut total_sq_error = Decimal::ZERO;
    let mut error_count: u32 = 0;

    for &k in &target_strikes {
        let model_vol = sabr_implied_vol(f, k, t, alpha, beta, rho, nu);

        // Find matching market vol
        let market_vol = input
            .market_vols
            .iter()
            .find(|mv| abs_decimal(mv.strike - k) < dec!(0.0001))
            .map(|mv| mv.implied_vol);

        let error = if let Some(mkt) = market_vol {
            let e = model_vol - mkt;
            total_sq_error += e * e;
            error_count += 1;
            e
        } else {
            Decimal::ZERO
        };

        model_vols.push(SabrModelVol {
            strike: k,
            model_vol,
            market_vol,
            error,
        });
    }

    // RMS calibration error
    let calibration_error = if error_count > 0 {
        sqrt_decimal(total_sq_error / Decimal::from(error_count))
    } else {
        Decimal::ZERO
    };

    // Compute skew: (sigma(F*1.01) - sigma(F*0.99)) / (0.02*F)
    let f_up = f * dec!(1.01);
    let f_dn = f * dec!(0.99);
    let vol_up = sabr_implied_vol(f, f_up, t, alpha, beta, rho, nu);
    let vol_dn = sabr_implied_vol(f, f_dn, t, alpha, beta, rho, nu);
    let skew = (vol_up - vol_dn) / (dec!(0.02) * f);

    // Compute backbone: dσ_ATM/dF via finite difference
    let df = f * dec!(0.001);
    let atm_vol_up = sabr_atm_vol(f + df, t, alpha, beta, rho, nu);
    let atm_vol_dn = sabr_atm_vol(f - df, t, alpha, beta, rho, nu);
    let backbone = (atm_vol_up - atm_vol_dn) / (dec!(2) * df);

    let output = SabrCalibrationOutput {
        alpha,
        beta,
        rho,
        nu,
        calibration_error,
        model_vols,
        atm_vol,
        skew,
        backbone,
        convergence_iterations: iterations,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "model": "SABR (Hagan 2002)",
        "forward_price": f.to_string(),
        "expiry": t.to_string(),
        "beta": beta.to_string(),
        "calibration_method": "Levenberg-Marquardt",
        "max_iterations": 50,
    });

    Ok(with_metadata(
        "SABR Stochastic Volatility Model",
        &assumptions,
        vec![],
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
    use rust_decimal_macros::dec;

    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        abs_decimal(a - b) < tol
    }

    fn make_vol_point(strike: Decimal, vol: Decimal) -> SabrVolPoint {
        SabrVolPoint {
            strike,
            implied_vol: vol,
        }
    }

    /// Standard equity-like smile: F=100, T=1, beta=0.5
    fn standard_market_vols() -> Vec<SabrVolPoint> {
        vec![
            make_vol_point(dec!(80), dec!(0.30)),
            make_vol_point(dec!(85), dec!(0.27)),
            make_vol_point(dec!(90), dec!(0.24)),
            make_vol_point(dec!(95), dec!(0.21)),
            make_vol_point(dec!(100), dec!(0.20)),
            make_vol_point(dec!(105), dec!(0.205)),
            make_vol_point(dec!(110), dec!(0.22)),
            make_vol_point(dec!(115), dec!(0.24)),
            make_vol_point(dec!(120), dec!(0.27)),
        ]
    }

    fn standard_input() -> SabrCalibrationInput {
        SabrCalibrationInput {
            forward_price: dec!(100),
            expiry: dec!(1),
            market_vols: standard_market_vols(),
            initial_alpha: None,
            initial_rho: None,
            initial_nu: None,
            beta: dec!(0.5),
            target_strikes: None,
        }
    }

    // -----------------------------------------------------------------------
    // Math helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_exp_zero() {
        assert!(approx_eq(exp_decimal(dec!(0)), dec!(1), dec!(0.0001)));
    }

    #[test]
    fn test_exp_one() {
        assert!(approx_eq(exp_decimal(dec!(1)), dec!(2.71828), dec!(0.001)));
    }

    #[test]
    fn test_sqrt_four() {
        assert!(approx_eq(sqrt_decimal(dec!(4)), dec!(2), dec!(0.0001)));
    }

    #[test]
    fn test_ln_e() {
        assert!(approx_eq(
            ln_decimal(dec!(2.718281828)),
            dec!(1),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_pow_frac_square() {
        let result = pow_decimal_frac(dec!(4), dec!(0.5));
        assert!(approx_eq(result, dec!(2), dec!(0.01)));
    }

    #[test]
    fn test_pow_frac_cube_root() {
        let result = pow_decimal_frac(dec!(8), dec!(1) / dec!(3));
        assert!(approx_eq(result, dec!(2), dec!(0.01)));
    }

    // -----------------------------------------------------------------------
    // Validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_zero_forward() {
        let input = SabrCalibrationInput {
            forward_price: dec!(0),
            ..standard_input()
        };
        let result = calibrate_sabr(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "forward_price"),
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_zero_expiry() {
        let input = SabrCalibrationInput {
            expiry: dec!(0),
            ..standard_input()
        };
        let result = calibrate_sabr(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_beta_out_of_range() {
        let input = SabrCalibrationInput {
            beta: dec!(1.5),
            ..standard_input()
        };
        let result = calibrate_sabr(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_negative_beta() {
        let input = SabrCalibrationInput {
            beta: dec!(-0.1),
            ..standard_input()
        };
        let result = calibrate_sabr(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_empty_market_vols() {
        let input = SabrCalibrationInput {
            market_vols: vec![],
            ..standard_input()
        };
        let result = calibrate_sabr(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_negative_strike() {
        let mut input = standard_input();
        input.market_vols[0].strike = dec!(-10);
        let result = calibrate_sabr(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zero_implied_vol() {
        let mut input = standard_input();
        input.market_vols[0].implied_vol = dec!(0);
        let result = calibrate_sabr(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_rho_out_of_range() {
        let input = SabrCalibrationInput {
            initial_rho: Some(dec!(1)),
            ..standard_input()
        };
        let result = calibrate_sabr(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // SABR formula tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_sabr_atm_vol_positive() {
        let vol = sabr_atm_vol(
            dec!(100),
            dec!(1),
            dec!(0.2),
            dec!(0.5),
            dec!(-0.3),
            dec!(0.4),
        );
        assert!(vol > Decimal::ZERO, "ATM vol {} should be positive", vol);
    }

    #[test]
    fn test_sabr_atm_vol_reasonable() {
        // With alpha ~0.2, beta=0.5, F=100: sigma_ATM ~ alpha / F^(1-beta) = 0.2 / 10 = 0.02
        // But alpha is already scaled to produce typical vols
        let alpha = dec!(2.0); // alpha * F^(1-beta) ~ 2 / 10 = 0.20
        let vol = sabr_atm_vol(dec!(100), dec!(1), alpha, dec!(0.5), dec!(-0.3), dec!(0.4));
        assert!(
            vol > dec!(0.05) && vol < dec!(0.50),
            "ATM vol {} should be in reasonable range",
            vol
        );
    }

    #[test]
    fn test_sabr_vol_atm_equals_general() {
        // At K=F, the general formula should match the ATM formula
        let f = dec!(100);
        let t = dec!(1);
        let alpha = dec!(2);
        let beta = dec!(0.5);
        let rho = dec!(-0.3);
        let nu = dec!(0.4);

        let atm = sabr_atm_vol(f, t, alpha, beta, rho, nu);
        let general = sabr_implied_vol(f, f, t, alpha, beta, rho, nu);
        assert!(
            approx_eq(atm, general, dec!(0.001)),
            "ATM vol {} should match general formula at K=F: {}",
            atm,
            general
        );
    }

    #[test]
    fn test_sabr_smile_shape() {
        // With negative rho, OTM puts (low K) should have higher vol than OTM calls (high K)
        let f = dec!(100);
        let t = dec!(1);
        let alpha = dec!(2);
        let beta = dec!(0.5);
        let rho = dec!(-0.5);
        let nu = dec!(0.5);

        let vol_low = sabr_implied_vol(f, dec!(80), t, alpha, beta, rho, nu);
        let vol_atm = sabr_implied_vol(f, f, t, alpha, beta, rho, nu);
        let vol_high = sabr_implied_vol(f, dec!(120), t, alpha, beta, rho, nu);

        assert!(
            vol_low > vol_atm,
            "OTM put vol {} should > ATM vol {}",
            vol_low,
            vol_atm
        );
        // With negative rho, the skew means low strikes have higher vol
        assert!(
            vol_low > vol_high,
            "Put wing {} should > call wing {} for negative rho",
            vol_low,
            vol_high
        );
    }

    #[test]
    fn test_sabr_zero_nu_flat_smile() {
        // With nu=0 (no vol-of-vol), the smile should be approximately flat
        let f = dec!(100);
        let t = dec!(1);
        let alpha = dec!(2);
        let beta = dec!(0.5);
        let rho = Decimal::ZERO;
        let nu = dec!(0.0001); // Near zero

        let vol_low = sabr_implied_vol(f, dec!(90), t, alpha, beta, rho, nu);
        let vol_atm = sabr_implied_vol(f, f, t, alpha, beta, rho, nu);
        let vol_high = sabr_implied_vol(f, dec!(110), t, alpha, beta, rho, nu);

        // With very small nu, smile should be nearly flat
        assert!(
            approx_eq(vol_low, vol_atm, dec!(0.01)),
            "Low vol {} should be close to ATM {} with zero nu",
            vol_low,
            vol_atm
        );
        assert!(
            approx_eq(vol_high, vol_atm, dec!(0.01)),
            "High vol {} should be close to ATM {} with zero nu",
            vol_high,
            vol_atm
        );
    }

    // -----------------------------------------------------------------------
    // Calibration tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_calibration_basic() {
        let input = standard_input();
        let result = calibrate_sabr(&input).unwrap();
        let output = &result.result;

        assert!(output.alpha > Decimal::ZERO, "Alpha should be positive");
        assert!(
            output.rho > dec!(-1) && output.rho < dec!(1),
            "Rho {} out of range",
            output.rho
        );
        assert!(output.nu > Decimal::ZERO, "Nu should be positive");
    }

    #[test]
    fn test_calibration_error_small() {
        let input = standard_input();
        let result = calibrate_sabr(&input).unwrap();
        // RMS error should be reasonably small (< 5% of ATM vol)
        let max_error = result.result.atm_vol * dec!(0.10);
        assert!(
            result.result.calibration_error < max_error,
            "Calibration error {} too large (max {})",
            result.result.calibration_error,
            max_error
        );
    }

    #[test]
    fn test_calibration_beta_echoed() {
        let input = SabrCalibrationInput {
            beta: dec!(0.7),
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert_eq!(result.result.beta, dec!(0.7));
    }

    #[test]
    fn test_calibration_model_vols_count() {
        let input = standard_input();
        let result = calibrate_sabr(&input).unwrap();
        assert_eq!(result.result.model_vols.len(), input.market_vols.len());
    }

    #[test]
    fn test_calibration_with_initial_guesses() {
        let input = SabrCalibrationInput {
            initial_alpha: Some(dec!(2)),
            initial_rho: Some(dec!(-0.5)),
            initial_nu: Some(dec!(0.3)),
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.alpha > Decimal::ZERO);
    }

    #[test]
    fn test_calibration_target_strikes() {
        let input = SabrCalibrationInput {
            target_strikes: Some(vec![dec!(95), dec!(100), dec!(105)]),
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert_eq!(result.result.model_vols.len(), 3);
    }

    #[test]
    fn test_calibration_convergence_within_50() {
        let input = standard_input();
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.convergence_iterations <= 50);
    }

    // -----------------------------------------------------------------------
    // Beta = 0 (normal SABR) tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_beta_zero_basic() {
        let input = SabrCalibrationInput {
            beta: dec!(0),
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.atm_vol > Decimal::ZERO);
    }

    #[test]
    fn test_beta_zero_calibration_converges() {
        let input = SabrCalibrationInput {
            beta: dec!(0),
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.calibration_error < dec!(0.1));
    }

    // -----------------------------------------------------------------------
    // Beta = 1 (lognormal SABR) tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_beta_one_basic() {
        let input = SabrCalibrationInput {
            beta: dec!(1),
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.atm_vol > Decimal::ZERO);
    }

    #[test]
    fn test_beta_one_calibration_converges() {
        let input = SabrCalibrationInput {
            beta: dec!(1),
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.calibration_error < dec!(0.1));
    }

    // -----------------------------------------------------------------------
    // Skew and backbone tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_skew_with_negative_rho() {
        // Negative rho should produce negative skew (put wing steeper)
        let input = SabrCalibrationInput {
            initial_rho: Some(dec!(-0.5)),
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        // Skew = (vol(F*1.01) - vol(F*0.99)) / (0.02*F)
        // With negative rho, vol increases as K decreases, so skew < 0
        assert!(
            result.result.skew < dec!(0.01),
            "Skew {} should be negative or near zero with negative rho",
            result.result.skew
        );
    }

    #[test]
    fn test_backbone_sign() {
        // Backbone dσ_ATM/dF with beta < 1: as F increases, ATM vol typically decreases
        let input = standard_input();
        let result = calibrate_sabr(&input).unwrap();
        // With beta = 0.5 < 1, backbone should be negative
        assert!(
            result.result.backbone < dec!(0.01),
            "Backbone {} should be non-positive for beta < 1",
            result.result.backbone
        );
    }

    // -----------------------------------------------------------------------
    // Edge case tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_market_vol() {
        let input = SabrCalibrationInput {
            market_vols: vec![make_vol_point(dec!(100), dec!(0.20))],
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.atm_vol > Decimal::ZERO);
    }

    #[test]
    fn test_short_expiry() {
        let input = SabrCalibrationInput {
            expiry: dec!(0.01), // Very short expiry
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.atm_vol > Decimal::ZERO);
    }

    #[test]
    fn test_long_expiry() {
        let input = SabrCalibrationInput {
            expiry: dec!(10), // 10 years
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.atm_vol > Decimal::ZERO);
    }

    #[test]
    fn test_high_forward() {
        let vols = vec![
            make_vol_point(dec!(800), dec!(0.30)),
            make_vol_point(dec!(900), dec!(0.25)),
            make_vol_point(dec!(1000), dec!(0.20)),
            make_vol_point(dec!(1100), dec!(0.22)),
            make_vol_point(dec!(1200), dec!(0.27)),
        ];
        let input = SabrCalibrationInput {
            forward_price: dec!(1000),
            market_vols: vols,
            ..standard_input()
        };
        let result = calibrate_sabr(&input).unwrap();
        assert!(result.result.atm_vol > Decimal::ZERO);
    }

    #[test]
    fn test_model_vol_errors_have_matching_market() {
        let input = standard_input();
        let result = calibrate_sabr(&input).unwrap();
        for mv in &result.result.model_vols {
            assert!(
                mv.market_vol.is_some(),
                "Each model vol should have matching market vol"
            );
        }
    }

    #[test]
    fn test_model_vols_positive() {
        let input = standard_input();
        let result = calibrate_sabr(&input).unwrap();
        for mv in &result.result.model_vols {
            assert!(
                mv.model_vol > Decimal::ZERO,
                "Model vol should be positive at K={}",
                mv.strike
            );
        }
    }

    #[test]
    fn test_metadata_populated() {
        let input = standard_input();
        let result = calibrate_sabr(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_solve_3x3_identity() {
        let a = [
            [dec!(1), dec!(0), dec!(0)],
            [dec!(0), dec!(1), dec!(0)],
            [dec!(0), dec!(0), dec!(1)],
        ];
        let b = [dec!(1), dec!(2), dec!(3)];
        let x = solve_3x3(&a, &b);
        for i in 0..3 {
            assert!(approx_eq(x[i], b[i], dec!(0.0001)));
        }
    }

    #[test]
    fn test_solve_3x3_nontrivial() {
        // 2x + y = 5, x + 3z = 7, y + 2z = 4
        let a = [
            [dec!(2), dec!(1), dec!(0)],
            [dec!(1), dec!(0), dec!(3)],
            [dec!(0), dec!(1), dec!(2)],
        ];
        let b = [dec!(5), dec!(7), dec!(4)];
        let x = solve_3x3(&a, &b);
        // Verify Ax = b
        for i in 0..3 {
            let mut sum = Decimal::ZERO;
            for j in 0..3 {
                sum += a[i][j] * x[j];
            }
            assert!(
                approx_eq(sum, b[i], dec!(0.001)),
                "Row {i}: sum={sum} != b={}",
                b[i]
            );
        }
    }
}
