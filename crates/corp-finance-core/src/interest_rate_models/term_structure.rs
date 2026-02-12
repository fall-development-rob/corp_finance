//! Yield curve fitting models for term structure estimation.
//!
//! Implements Nelson-Siegel (4-parameter), Svensson (6-parameter), and
//! bootstrapping methods for constructing zero-coupon yield curves from
//! observed market rates. All math uses `rust_decimal::Decimal`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Rate, Years};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

const TAYLOR_EXP_TERMS: u32 = 30;
const NEWTON_ITERATIONS: u32 = 20;

fn decimal_exp(x: Decimal) -> Decimal {
    if x.is_zero() {
        return Decimal::ONE;
    }
    if x < dec!(-60) {
        return Decimal::ZERO;
    }
    if x > dec!(40) {
        return Decimal::MAX;
    }
    if x < Decimal::ZERO {
        let pos_exp = decimal_exp_positive(Decimal::ZERO - x);
        if pos_exp.is_zero() {
            return Decimal::MAX;
        }
        return Decimal::ONE / pos_exp;
    }
    decimal_exp_positive(x)
}

fn decimal_exp_positive(x: Decimal) -> Decimal {
    if x > dec!(10) {
        let half = decimal_exp_positive(x / dec!(2));
        match half.checked_mul(half) {
            Some(result) => return result,
            None => return Decimal::MAX,
        }
    }

    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for k in 1..=TAYLOR_EXP_TERMS {
        match term.checked_mul(x) {
            Some(product) => {
                term = product / Decimal::from(k);
            }
            None => return sum,
        }
        match sum.checked_add(term) {
            Some(new_sum) => {
                sum = new_sum;
            }
            None => return sum,
        }
        if term < dec!(0.0000000000000000000000000001) {
            break;
        }
    }
    sum
}

fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }
    let mut y = if x > Decimal::ONE {
        x - Decimal::ONE
    } else {
        Decimal::ZERO - (Decimal::ONE / x - Decimal::ONE)
    };
    if y > dec!(20) {
        y = dec!(20);
    } else if y < dec!(-20) {
        y = dec!(-20);
    }
    for _ in 0..NEWTON_ITERATIONS {
        let exp_y = decimal_exp(y);
        if exp_y.is_zero() {
            break;
        }
        y = y - Decimal::ONE + x / exp_y;
        if y > dec!(50) {
            y = dec!(50);
        } else if y < dec!(-50) {
            y = dec!(-50);
        }
    }
    y
}

fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let mut guess = x / dec!(2);
    if guess.is_zero() {
        guess = dec!(0.001);
    }
    for _ in 0..NEWTON_ITERATIONS {
        if guess.is_zero() {
            return Decimal::ZERO;
        }
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

// ---------------------------------------------------------------------------
// Common types
// ---------------------------------------------------------------------------

/// An observed market rate at a given maturity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketRate {
    /// Time to maturity in years
    pub maturity: Years,
    /// Observed rate (continuously compounded)
    pub rate: Rate,
}

/// A point on the zero rate curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroRatePoint {
    pub maturity: Years,
    pub rate: Rate,
}

/// A discount factor at a given maturity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountFactor {
    pub maturity: Years,
    pub factor: Decimal,
}

/// A forward rate between two maturities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRate {
    pub start: Years,
    pub end: Years,
    pub rate: Rate,
}

// ---------------------------------------------------------------------------
// Nelson-Siegel
// ---------------------------------------------------------------------------

/// Parameters for the Nelson-Siegel model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NelsonSiegelParams {
    /// Long-term level (asymptote)
    pub beta0: Decimal,
    /// Short-term component
    pub beta1: Decimal,
    /// Medium-term hump component
    pub beta2: Decimal,
    /// Decay factor
    pub lambda: Decimal,
}

/// Input for Nelson-Siegel curve fitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NelsonSiegelInput {
    /// Observed market rates
    pub market_rates: Vec<MarketRate>,
    /// Optional initial parameter guess
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_params: Option<NelsonSiegelParams>,
}

/// Output of Nelson-Siegel curve fitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NelsonSiegelOutput {
    /// Fitted parameters
    pub params: NelsonSiegelParams,
    /// Model-fitted rates at each market maturity
    pub fitted_rates: Vec<Rate>,
    /// Residuals (market - model) at each maturity
    pub residuals: Vec<Decimal>,
    /// Root mean square error
    pub rmse: Decimal,
    /// R-squared goodness of fit
    pub r_squared: Decimal,
}

/// Evaluate the Nelson-Siegel yield at maturity tau.
/// y(tau) = beta0 + beta1 * (1-exp(-tau/lambda)) / (tau/lambda)
///        + beta2 * ((1-exp(-tau/lambda))/(tau/lambda) - exp(-tau/lambda))
fn nelson_siegel_yield(params: &NelsonSiegelParams, tau: Decimal) -> Decimal {
    if tau <= Decimal::ZERO {
        return params.beta0 + params.beta1;
    }
    if params.lambda <= Decimal::ZERO {
        return params.beta0;
    }

    let x = tau / params.lambda;
    if x.is_zero() {
        return params.beta0 + params.beta1;
    }

    let exp_neg_x = decimal_exp(Decimal::ZERO - x);
    let factor1 = (Decimal::ONE - exp_neg_x) / x;
    let factor2 = factor1 - exp_neg_x;

    params.beta0 + params.beta1 * factor1 + params.beta2 * factor2
}

/// Fit Nelson-Siegel via iterative grid search + local refinement.
fn fit_nelson_siegel(input: &NelsonSiegelInput) -> CorpFinanceResult<NelsonSiegelOutput> {
    if input.market_rates.len() < 3 {
        return Err(CorpFinanceError::InsufficientData(
            "Nelson-Siegel requires at least 3 market rates".into(),
        ));
    }

    // Compute mean market rate for R-squared denominator
    let n = Decimal::from(input.market_rates.len() as u32);
    let mean_rate: Decimal = input.market_rates.iter().map(|r| r.rate).sum::<Decimal>() / n;

    // Start from initial params or use heuristic defaults
    let initial = input.initial_params.clone().unwrap_or_else(|| {
        // Heuristic: beta0 ~ long-term rate, beta1 ~ short-long spread
        let long_rate = input
            .market_rates
            .last()
            .map(|r| r.rate)
            .unwrap_or(dec!(0.04));
        let short_rate = input
            .market_rates
            .first()
            .map(|r| r.rate)
            .unwrap_or(dec!(0.02));
        NelsonSiegelParams {
            beta0: long_rate,
            beta1: short_rate - long_rate,
            beta2: Decimal::ZERO,
            lambda: dec!(1.5),
        }
    });

    let mut best_params = initial;
    let mut best_sse = compute_sse(&input.market_rates, &best_params, nelson_siegel_yield);

    // Grid search over lambda (key parameter) with refinement
    let lambda_grid = [
        dec!(0.3),
        dec!(0.5),
        dec!(0.8),
        dec!(1.0),
        dec!(1.5),
        dec!(2.0),
        dec!(3.0),
        dec!(5.0),
    ];

    for &lam in &lambda_grid {
        let trial = fit_ns_given_lambda(&input.market_rates, lam);
        let sse = compute_sse(&input.market_rates, &trial, nelson_siegel_yield);
        if sse < best_sse {
            best_sse = sse;
            best_params = trial;
        }
    }

    // Local refinement: try lambda +/- small increments around the best
    let base_lambda = best_params.lambda;
    for delta_pct in &[dec!(-0.3), dec!(-0.1), dec!(0.1), dec!(0.3)] {
        let trial_lambda = base_lambda * (Decimal::ONE + *delta_pct);
        if trial_lambda > Decimal::ZERO {
            let trial = fit_ns_given_lambda(&input.market_rates, trial_lambda);
            let sse = compute_sse(&input.market_rates, &trial, nelson_siegel_yield);
            if sse < best_sse {
                best_sse = sse;
                best_params = trial;
            }
        }
    }

    // Compute fitted rates, residuals, RMSE, R-squared
    let mut fitted_rates = Vec::with_capacity(input.market_rates.len());
    let mut residuals = Vec::with_capacity(input.market_rates.len());
    let mut ss_res = Decimal::ZERO;
    let mut ss_tot = Decimal::ZERO;

    for mr in &input.market_rates {
        let fitted = nelson_siegel_yield(&best_params, mr.maturity);
        let resid = mr.rate - fitted;
        fitted_rates.push(fitted);
        residuals.push(resid);
        ss_res += resid * resid;
        ss_tot += (mr.rate - mean_rate) * (mr.rate - mean_rate);
    }

    let rmse = decimal_sqrt(ss_res / n);
    let r_squared = if ss_tot > Decimal::ZERO {
        Decimal::ONE - ss_res / ss_tot
    } else {
        Decimal::ONE
    };

    Ok(NelsonSiegelOutput {
        params: best_params,
        fitted_rates,
        residuals,
        rmse,
        r_squared,
    })
}

/// Fit beta0, beta1, beta2 by OLS given a fixed lambda.
/// With lambda fixed, the NS model is linear in beta0, beta1, beta2.
fn fit_ns_given_lambda(market: &[MarketRate], lambda: Decimal) -> NelsonSiegelParams {
    // Design matrix columns: [1, F1(tau), F2(tau)] for each observation
    // y(tau) = beta0 * 1 + beta1 * F1(tau) + beta2 * F2(tau)
    // where F1(tau) = (1-exp(-tau/lam))/(tau/lam)
    //       F2(tau) = F1(tau) - exp(-tau/lam)
    //
    // Solve normal equations: (X^T X) beta = X^T y  (3x3 system)

    // Accumulators for X^T X (symmetric 3x3) and X^T y (3x1)
    let mut xtx = [[Decimal::ZERO; 3]; 3];
    let mut xty = [Decimal::ZERO; 3];

    for mr in market {
        let tau = mr.maturity;
        let (f1, f2) = if tau <= Decimal::ZERO || lambda <= Decimal::ZERO {
            (Decimal::ONE, Decimal::ZERO)
        } else {
            let x = tau / lambda;
            let exp_neg = decimal_exp(Decimal::ZERO - x);
            let fac1 = (Decimal::ONE - exp_neg) / x;
            let fac2 = fac1 - exp_neg;
            (fac1, fac2)
        };

        let row = [Decimal::ONE, f1, f2];
        let y = mr.rate;

        for i in 0..3 {
            xty[i] += row[i] * y;
            for j in 0..3 {
                xtx[i][j] += row[i] * row[j];
            }
        }
    }

    // Solve 3x3 system via Cramer's rule
    let beta = solve_3x3(xtx, xty);

    NelsonSiegelParams {
        beta0: beta[0],
        beta1: beta[1],
        beta2: beta[2],
        lambda,
    }
}

/// Solve a 3x3 linear system Ax = b via Cramer's rule.
fn solve_3x3(a: [[Decimal; 3]; 3], b: [Decimal; 3]) -> [Decimal; 3] {
    let det_a = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);

    if det_a.is_zero() {
        return [Decimal::ZERO; 3];
    }

    let mut result = [Decimal::ZERO; 3];

    for col in 0..3 {
        let mut a_mod = a;
        for row in 0..3 {
            a_mod[row][col] = b[row];
        }
        let det_mod = a_mod[0][0] * (a_mod[1][1] * a_mod[2][2] - a_mod[1][2] * a_mod[2][1])
            - a_mod[0][1] * (a_mod[1][0] * a_mod[2][2] - a_mod[1][2] * a_mod[2][0])
            + a_mod[0][2] * (a_mod[1][0] * a_mod[2][1] - a_mod[1][1] * a_mod[2][0]);
        result[col] = det_mod / det_a;
    }

    result
}

/// Compute sum of squared errors for a parametric yield model.
fn compute_sse(
    market: &[MarketRate],
    params: &NelsonSiegelParams,
    model_fn: fn(&NelsonSiegelParams, Decimal) -> Decimal,
) -> Decimal {
    let mut sse = Decimal::ZERO;
    for mr in market {
        let fitted = model_fn(params, mr.maturity);
        let resid = mr.rate - fitted;
        sse += resid * resid;
    }
    sse
}

// ---------------------------------------------------------------------------
// Svensson (Extended Nelson-Siegel)
// ---------------------------------------------------------------------------

/// Parameters for the Svensson model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvenssonParams {
    pub beta0: Decimal,
    pub beta1: Decimal,
    pub beta2: Decimal,
    pub beta3: Decimal,
    pub lambda1: Decimal,
    pub lambda2: Decimal,
}

/// Input for Svensson curve fitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvenssonInput {
    pub market_rates: Vec<MarketRate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_params: Option<SvenssonParams>,
}

/// Output of Svensson curve fitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvenssonOutput {
    pub params: SvenssonParams,
    pub fitted_rates: Vec<Rate>,
    pub residuals: Vec<Decimal>,
    pub rmse: Decimal,
    pub r_squared: Decimal,
}

/// Evaluate the Svensson yield at maturity tau.
fn svensson_yield(params: &SvenssonParams, tau: Decimal) -> Decimal {
    if tau <= Decimal::ZERO {
        return params.beta0 + params.beta1;
    }

    let ns_part = {
        let lam = if params.lambda1 <= Decimal::ZERO {
            dec!(1)
        } else {
            params.lambda1
        };
        let x = tau / lam;
        let exp_neg_x = decimal_exp(Decimal::ZERO - x);
        let f1 = (Decimal::ONE - exp_neg_x) / x;
        let f2 = f1 - exp_neg_x;
        params.beta0 + params.beta1 * f1 + params.beta2 * f2
    };

    let sv_extra = {
        let lam2 = if params.lambda2 <= Decimal::ZERO {
            dec!(1)
        } else {
            params.lambda2
        };
        let x2 = tau / lam2;
        let exp_neg_x2 = decimal_exp(Decimal::ZERO - x2);
        let f3 = (Decimal::ONE - exp_neg_x2) / x2 - exp_neg_x2;
        params.beta3 * f3
    };

    ns_part + sv_extra
}

fn fit_svensson(input: &SvenssonInput) -> CorpFinanceResult<SvenssonOutput> {
    if input.market_rates.len() < 4 {
        return Err(CorpFinanceError::InsufficientData(
            "Svensson requires at least 4 market rates".into(),
        ));
    }

    let n = Decimal::from(input.market_rates.len() as u32);
    let mean_rate: Decimal = input.market_rates.iter().map(|r| r.rate).sum::<Decimal>() / n;

    // Grid search over lambda1 and lambda2
    let lambda_grid = [
        dec!(0.5),
        dec!(1.0),
        dec!(1.5),
        dec!(2.0),
        dec!(3.0),
        dec!(5.0),
    ];

    let mut best_params: Option<SvenssonParams> = None;
    let mut best_sse = Decimal::MAX;

    for &l1 in &lambda_grid {
        for &l2 in &lambda_grid {
            if l1 == l2 {
                continue; // lambda1 and lambda2 should differ
            }
            let trial = fit_svensson_given_lambdas(&input.market_rates, l1, l2);
            let sse = compute_sse_svensson(&input.market_rates, &trial);
            if sse < best_sse {
                best_sse = sse;
                best_params = Some(trial);
            }
        }
    }

    let best = best_params.unwrap_or(SvenssonParams {
        beta0: mean_rate,
        beta1: Decimal::ZERO,
        beta2: Decimal::ZERO,
        beta3: Decimal::ZERO,
        lambda1: dec!(1.5),
        lambda2: dec!(3.0),
    });

    // Local refinement around best lambdas
    let mut refined = best.clone();
    let mut refined_sse = best_sse;

    for d1 in &[dec!(-0.2), dec!(0.0), dec!(0.2)] {
        for d2 in &[dec!(-0.2), dec!(0.0), dec!(0.2)] {
            let l1_trial = refined.lambda1 + *d1;
            let l2_trial = refined.lambda2 + *d2;
            if l1_trial > Decimal::ZERO && l2_trial > Decimal::ZERO && l1_trial != l2_trial {
                let trial = fit_svensson_given_lambdas(&input.market_rates, l1_trial, l2_trial);
                let sse = compute_sse_svensson(&input.market_rates, &trial);
                if sse < refined_sse {
                    refined_sse = sse;
                    refined = trial;
                }
            }
        }
    }

    // Compute fitted rates, residuals, RMSE, R-squared
    let mut fitted_rates = Vec::with_capacity(input.market_rates.len());
    let mut residuals = Vec::with_capacity(input.market_rates.len());
    let mut ss_res = Decimal::ZERO;
    let mut ss_tot = Decimal::ZERO;

    for mr in &input.market_rates {
        let fitted = svensson_yield(&refined, mr.maturity);
        let resid = mr.rate - fitted;
        fitted_rates.push(fitted);
        residuals.push(resid);
        ss_res += resid * resid;
        ss_tot += (mr.rate - mean_rate) * (mr.rate - mean_rate);
    }

    let rmse = decimal_sqrt(ss_res / n);
    let r_squared = if ss_tot > Decimal::ZERO {
        Decimal::ONE - ss_res / ss_tot
    } else {
        Decimal::ONE
    };

    Ok(SvenssonOutput {
        params: refined,
        fitted_rates,
        residuals,
        rmse,
        r_squared,
    })
}

/// Fit Svensson betas via OLS given fixed lambda1, lambda2.
/// The model is linear in beta0..beta3 with 4 basis functions.
fn fit_svensson_given_lambdas(
    market: &[MarketRate],
    lambda1: Decimal,
    lambda2: Decimal,
) -> SvenssonParams {
    // 4x4 normal equations
    let mut xtx = [[Decimal::ZERO; 4]; 4];
    let mut xty = [Decimal::ZERO; 4];

    for mr in market {
        let tau = mr.maturity;

        let (f1, f2) = if tau <= Decimal::ZERO || lambda1 <= Decimal::ZERO {
            (Decimal::ONE, Decimal::ZERO)
        } else {
            let x = tau / lambda1;
            let e = decimal_exp(Decimal::ZERO - x);
            let fac1 = (Decimal::ONE - e) / x;
            (fac1, fac1 - e)
        };

        let f3 = if tau <= Decimal::ZERO || lambda2 <= Decimal::ZERO {
            Decimal::ZERO
        } else {
            let x2 = tau / lambda2;
            let e2 = decimal_exp(Decimal::ZERO - x2);
            (Decimal::ONE - e2) / x2 - e2
        };

        let row = [Decimal::ONE, f1, f2, f3];
        let y = mr.rate;

        for i in 0..4 {
            xty[i] += row[i] * y;
            for j in 0..4 {
                xtx[i][j] += row[i] * row[j];
            }
        }
    }

    let beta = solve_4x4(xtx, xty);

    SvenssonParams {
        beta0: beta[0],
        beta1: beta[1],
        beta2: beta[2],
        beta3: beta[3],
        lambda1,
        lambda2,
    }
}

/// Solve 4x4 linear system via Gaussian elimination with partial pivoting.
#[allow(clippy::needless_range_loop)]
fn solve_4x4(a: [[Decimal; 4]; 4], b: [Decimal; 4]) -> [Decimal; 4] {
    let mut aug = [[Decimal::ZERO; 5]; 4];
    for i in 0..4 {
        for j in 0..4 {
            aug[i][j] = a[i][j];
        }
        aug[i][4] = b[i];
    }

    // Forward elimination with partial pivoting
    for col in 0..4 {
        // Find pivot
        let mut max_val = aug[col][col].abs();
        let mut max_row = col;
        for row in (col + 1)..4 {
            let val = aug[row][col].abs();
            if val > max_val {
                max_val = val;
                max_row = row;
            }
        }

        if max_val.is_zero() {
            continue; // singular column
        }

        // Swap rows
        if max_row != col {
            aug.swap(col, max_row);
        }

        // Eliminate below
        let pivot = aug[col][col];
        for row in (col + 1)..4 {
            let factor = aug[row][col] / pivot;
            for j in col..5 {
                let val = aug[col][j];
                aug[row][j] -= factor * val;
            }
        }
    }

    // Back substitution
    let mut x = [Decimal::ZERO; 4];
    for i in (0..4).rev() {
        if aug[i][i].is_zero() {
            continue;
        }
        let mut sum = aug[i][4];
        for j in (i + 1)..4 {
            sum -= aug[i][j] * x[j];
        }
        x[i] = sum / aug[i][i];
    }

    x
}

fn compute_sse_svensson(market: &[MarketRate], params: &SvenssonParams) -> Decimal {
    let mut sse = Decimal::ZERO;
    for mr in market {
        let fitted = svensson_yield(params, mr.maturity);
        let resid = mr.rate - fitted;
        sse += resid * resid;
    }
    sse
}

// ---------------------------------------------------------------------------
// Bootstrapping
// ---------------------------------------------------------------------------

/// Type of instrument used in bootstrapping.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstrumentType {
    ZeroCoupon,
    ParBond,
    Swap,
}

/// A market instrument for bootstrapping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapInstrument {
    /// Time to maturity in years
    pub maturity: Years,
    /// Annual coupon rate (0 for zero-coupon instruments)
    pub coupon_rate: Rate,
    /// Observed market price (per 100 face)
    pub price: Decimal,
    /// Type of instrument
    pub instrument_type: InstrumentType,
}

/// Input for the bootstrapping method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapInput {
    pub instruments: Vec<BootstrapInstrument>,
}

/// Output of the bootstrapping method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapOutput {
    /// Bootstrapped zero-coupon yield curve
    pub zero_curve: Vec<ZeroRatePoint>,
    /// Discount factors at each maturity
    pub discount_factors: Vec<DiscountFactor>,
    /// Forward rates between consecutive maturities
    pub forward_rates: Vec<ForwardRate>,
}

fn validate_bootstrap(input: &BootstrapInput) -> CorpFinanceResult<()> {
    if input.instruments.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Bootstrapping requires at least 1 instrument".into(),
        ));
    }
    for (i, inst) in input.instruments.iter().enumerate() {
        if inst.maturity <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("instruments[{i}].maturity"),
                reason: "Maturity must be positive".into(),
            });
        }
        if inst.price <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("instruments[{i}].price"),
                reason: "Price must be positive".into(),
            });
        }
    }
    // Verify sorted by maturity
    for w in input.instruments.windows(2) {
        if w[1].maturity <= w[0].maturity {
            return Err(CorpFinanceError::InvalidInput {
                field: "instruments".into(),
                reason: "Instruments must be sorted by ascending maturity".into(),
            });
        }
    }
    Ok(())
}

fn run_bootstrap(input: &BootstrapInput) -> CorpFinanceResult<BootstrapOutput> {
    validate_bootstrap(input)?;

    let face = dec!(100);
    let mut zero_curve: Vec<ZeroRatePoint> = Vec::new();
    let mut discount_factors: Vec<DiscountFactor> = Vec::new();

    for inst in &input.instruments {
        match inst.instrument_type {
            InstrumentType::ZeroCoupon => {
                // Price = face * exp(-r * T) => r = -ln(price/face) / T
                let ratio = inst.price / face;
                let rate = if inst.maturity > Decimal::ZERO {
                    Decimal::ZERO - decimal_ln(ratio) / inst.maturity
                } else {
                    Decimal::ZERO
                };
                let df = ratio;

                zero_curve.push(ZeroRatePoint {
                    maturity: inst.maturity,
                    rate,
                });
                discount_factors.push(DiscountFactor {
                    maturity: inst.maturity,
                    factor: df,
                });
            }
            InstrumentType::ParBond | InstrumentType::Swap => {
                // Bootstrap: Price = sum(coupon * DF_i) + face * DF_n
                // where DF_i are known from previously bootstrapped points.
                // Solve for DF_n.
                let annual_coupon = face * inst.coupon_rate;

                // For annual coupon payments at t = 1, 2, ..., T
                // We assume annual payments for simplicity
                let t_mat_int = inst
                    .maturity
                    .round()
                    .to_string()
                    .parse::<u32>()
                    .unwrap_or(1);

                let mut pv_coupons = Decimal::ZERO;
                for t in 1..t_mat_int {
                    let t_dec = Decimal::from(t);
                    // Interpolate discount factor at time t from already-bootstrapped curve
                    let df_t = interpolate_df(&discount_factors, t_dec);
                    pv_coupons += annual_coupon * df_t;
                }

                // DF_n = (Price - PV_coupons) / (coupon + face)
                let remaining = inst.price - pv_coupons;
                let cf_final = annual_coupon + face;

                if cf_final.is_zero() {
                    return Err(CorpFinanceError::DivisionByZero {
                        context: format!(
                            "Bootstrap: final cashflow is zero for instrument at maturity {}",
                            inst.maturity
                        ),
                    });
                }

                let df_n = remaining / cf_final;

                // Zero rate: r = -ln(df) / T
                let rate = if inst.maturity > Decimal::ZERO && df_n > Decimal::ZERO {
                    Decimal::ZERO - decimal_ln(df_n) / inst.maturity
                } else {
                    Decimal::ZERO
                };

                zero_curve.push(ZeroRatePoint {
                    maturity: inst.maturity,
                    rate,
                });
                discount_factors.push(DiscountFactor {
                    maturity: inst.maturity,
                    factor: df_n,
                });
            }
        }
    }

    // Compute forward rates between consecutive maturities
    let mut forward_rates: Vec<ForwardRate> = Vec::new();
    for w in zero_curve.windows(2) {
        let t1 = w[0].maturity;
        let t2 = w[1].maturity;
        let z1 = w[0].rate;
        let z2 = w[1].rate;
        let dt = t2 - t1;

        let fwd = if dt > Decimal::ZERO {
            (z2 * t2 - z1 * t1) / dt
        } else {
            z2
        };

        forward_rates.push(ForwardRate {
            start: t1,
            end: t2,
            rate: fwd,
        });
    }

    Ok(BootstrapOutput {
        zero_curve,
        discount_factors,
        forward_rates,
    })
}

/// Interpolate a discount factor at time t from bootstrapped points.
fn interpolate_df(dfs: &[DiscountFactor], t: Decimal) -> Decimal {
    if dfs.is_empty() {
        return decimal_exp(Decimal::ZERO - dec!(0.03) * t); // fallback
    }
    if t <= dfs[0].maturity {
        // Extrapolate using the first rate
        if dfs[0].maturity > Decimal::ZERO {
            let rate = Decimal::ZERO - decimal_ln(dfs[0].factor) / dfs[0].maturity;
            return decimal_exp(Decimal::ZERO - rate * t);
        }
        return dfs[0].factor;
    }
    if t >= dfs[dfs.len() - 1].maturity {
        return dfs[dfs.len() - 1].factor;
    }

    // Linear interpolation in log-discount-factor space
    for w in dfs.windows(2) {
        if t >= w[0].maturity && t <= w[1].maturity {
            let dt = w[1].maturity - w[0].maturity;
            if dt.is_zero() {
                return w[0].factor;
            }
            let frac = (t - w[0].maturity) / dt;
            let ln_df0 = decimal_ln(w[0].factor);
            let ln_df1 = decimal_ln(w[1].factor);
            let ln_df_t = ln_df0 + frac * (ln_df1 - ln_df0);
            return decimal_exp(ln_df_t);
        }
    }

    dfs[dfs.len() - 1].factor
}

// ---------------------------------------------------------------------------
// Wrapper enum and public API
// ---------------------------------------------------------------------------

/// Selects which term structure model to use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TermStructureModel {
    NelsonSiegel(NelsonSiegelInput),
    Svensson(SvenssonInput),
    Bootstrap(BootstrapInput),
}

/// Top-level input for term structure fitting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermStructureInput {
    pub model: TermStructureModel,
}

/// Top-level output wrapping model-specific results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TermStructureOutput {
    NelsonSiegel(NelsonSiegelOutput),
    Svensson(SvenssonOutput),
    Bootstrap(BootstrapOutput),
}

/// Fit a term structure model to market data.
pub fn fit_term_structure(
    input: &TermStructureInput,
) -> CorpFinanceResult<ComputationOutput<TermStructureOutput>> {
    let start = Instant::now();

    let (output, method_name) = match &input.model {
        TermStructureModel::NelsonSiegel(ns) => {
            let result = fit_nelson_siegel(ns)?;
            (
                TermStructureOutput::NelsonSiegel(result),
                "Nelson-Siegel 4-Parameter Yield Curve",
            )
        }
        TermStructureModel::Svensson(sv) => {
            let result = fit_svensson(sv)?;
            (
                TermStructureOutput::Svensson(result),
                "Svensson 6-Parameter Extended Nelson-Siegel",
            )
        }
        TermStructureModel::Bootstrap(bs) => {
            let result = run_bootstrap(bs)?;
            (
                TermStructureOutput::Bootstrap(result),
                "Piecewise Bootstrap with Linear Interpolation",
            )
        }
    };

    let elapsed = start.elapsed().as_micros() as u64;

    let assumptions = serde_json::json!({
        "math_precision": "rust_decimal_128bit",
        "interpolation": "piecewise_linear",
    });

    Ok(with_metadata(
        method_name,
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

    fn assert_close(actual: Decimal, expected: Decimal, tolerance: Decimal, label: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "{label}: expected ~{expected}, got {actual} (diff {diff} > tolerance {tolerance})"
        );
    }

    /// Standard upward-sloping yield curve data
    fn standard_market_rates() -> Vec<MarketRate> {
        vec![
            MarketRate {
                maturity: dec!(0.25),
                rate: dec!(0.020),
            },
            MarketRate {
                maturity: dec!(0.5),
                rate: dec!(0.022),
            },
            MarketRate {
                maturity: dec!(1),
                rate: dec!(0.025),
            },
            MarketRate {
                maturity: dec!(2),
                rate: dec!(0.030),
            },
            MarketRate {
                maturity: dec!(3),
                rate: dec!(0.033),
            },
            MarketRate {
                maturity: dec!(5),
                rate: dec!(0.037),
            },
            MarketRate {
                maturity: dec!(7),
                rate: dec!(0.040),
            },
            MarketRate {
                maturity: dec!(10),
                rate: dec!(0.042),
            },
            MarketRate {
                maturity: dec!(20),
                rate: dec!(0.045),
            },
            MarketRate {
                maturity: dec!(30),
                rate: dec!(0.046),
            },
        ]
    }

    // -----------------------------------------------------------------------
    // Nelson-Siegel tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_ns_flat_curve() {
        // beta1=beta2=0 => flat curve at beta0
        let flat_rates: Vec<MarketRate> = vec![
            MarketRate {
                maturity: dec!(1),
                rate: dec!(0.04),
            },
            MarketRate {
                maturity: dec!(5),
                rate: dec!(0.04),
            },
            MarketRate {
                maturity: dec!(10),
                rate: dec!(0.04),
            },
            MarketRate {
                maturity: dec!(20),
                rate: dec!(0.04),
            },
        ];
        let input = NelsonSiegelInput {
            market_rates: flat_rates,
            initial_params: None,
        };
        let result = fit_nelson_siegel(&input).unwrap();

        // beta0 should be ~0.04, beta1 and beta2 near zero
        assert_close(
            result.params.beta0,
            dec!(0.04),
            dec!(0.002),
            "NS flat curve beta0",
        );
        assert!(
            result.rmse < dec!(0.001),
            "NS flat curve RMSE should be < 1bp, got {}",
            result.rmse
        );
    }

    #[test]
    fn test_ns_upward_sloping() {
        let input = NelsonSiegelInput {
            market_rates: standard_market_rates(),
            initial_params: None,
        };
        let result = fit_nelson_siegel(&input).unwrap();

        // RMSE should be small (< 5bp)
        assert!(
            result.rmse < dec!(0.0005),
            "NS upward sloping RMSE should be < 5bp, got {}",
            result.rmse
        );
    }

    #[test]
    fn test_ns_inverted_curve() {
        let inverted: Vec<MarketRate> = vec![
            MarketRate {
                maturity: dec!(0.5),
                rate: dec!(0.05),
            },
            MarketRate {
                maturity: dec!(1),
                rate: dec!(0.048),
            },
            MarketRate {
                maturity: dec!(2),
                rate: dec!(0.044),
            },
            MarketRate {
                maturity: dec!(5),
                rate: dec!(0.040),
            },
            MarketRate {
                maturity: dec!(10),
                rate: dec!(0.038),
            },
        ];
        let input = NelsonSiegelInput {
            market_rates: inverted,
            initial_params: None,
        };
        let result = fit_nelson_siegel(&input).unwrap();

        // beta1 should be positive (negative slope means short > long)
        // NS: at tau=0, y = beta0 + beta1; at tau->inf, y = beta0
        // Inverted: y(0) > y(inf) => beta1 > 0
        assert!(
            result.params.beta1 > Decimal::ZERO,
            "Inverted curve should have positive beta1 (short-end excess), got {}",
            result.params.beta1
        );
    }

    #[test]
    fn test_ns_hump_shaped() {
        let humped: Vec<MarketRate> = vec![
            MarketRate {
                maturity: dec!(0.5),
                rate: dec!(0.03),
            },
            MarketRate {
                maturity: dec!(1),
                rate: dec!(0.035),
            },
            MarketRate {
                maturity: dec!(2),
                rate: dec!(0.042),
            },
            MarketRate {
                maturity: dec!(3),
                rate: dec!(0.045),
            },
            MarketRate {
                maturity: dec!(5),
                rate: dec!(0.043),
            },
            MarketRate {
                maturity: dec!(10),
                rate: dec!(0.040),
            },
        ];
        let input = NelsonSiegelInput {
            market_rates: humped,
            initial_params: None,
        };
        let result = fit_nelson_siegel(&input).unwrap();

        // beta2 controls the hump — should be significant
        assert!(
            result.params.beta2.abs() > dec!(0.001),
            "Humped curve should have significant beta2, got {}",
            result.params.beta2
        );
        assert!(
            result.rmse < dec!(0.005),
            "Humped curve RMSE should be reasonable, got {}",
            result.rmse
        );
    }

    #[test]
    fn test_ns_fitted_rates_count() {
        let input = NelsonSiegelInput {
            market_rates: standard_market_rates(),
            initial_params: None,
        };
        let result = fit_nelson_siegel(&input).unwrap();
        assert_eq!(result.fitted_rates.len(), standard_market_rates().len());
    }

    #[test]
    fn test_ns_residuals_count() {
        let input = NelsonSiegelInput {
            market_rates: standard_market_rates(),
            initial_params: None,
        };
        let result = fit_nelson_siegel(&input).unwrap();
        assert_eq!(result.residuals.len(), standard_market_rates().len());
    }

    #[test]
    fn test_ns_r_squared_positive() {
        let input = NelsonSiegelInput {
            market_rates: standard_market_rates(),
            initial_params: None,
        };
        let result = fit_nelson_siegel(&input).unwrap();
        assert!(
            result.r_squared > dec!(0.9),
            "R-squared should be high for well-behaved data, got {}",
            result.r_squared
        );
    }

    #[test]
    fn test_ns_insufficient_data() {
        let input = NelsonSiegelInput {
            market_rates: vec![
                MarketRate {
                    maturity: dec!(1),
                    rate: dec!(0.03),
                },
                MarketRate {
                    maturity: dec!(5),
                    rate: dec!(0.04),
                },
            ],
            initial_params: None,
        };
        let err = fit_nelson_siegel(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }

    #[test]
    fn test_ns_with_initial_params() {
        let input = NelsonSiegelInput {
            market_rates: standard_market_rates(),
            initial_params: Some(NelsonSiegelParams {
                beta0: dec!(0.045),
                beta1: dec!(-0.02),
                beta2: dec!(0.01),
                lambda: dec!(2.0),
            }),
        };
        let result = fit_nelson_siegel(&input).unwrap();
        assert!(
            result.rmse < dec!(0.005),
            "NS with initial params should still fit well"
        );
    }

    #[test]
    fn test_ns_yield_evaluation() {
        let params = NelsonSiegelParams {
            beta0: dec!(0.05),
            beta1: Decimal::ZERO,
            beta2: Decimal::ZERO,
            lambda: dec!(1.5),
        };
        // With beta1=beta2=0, yield should be beta0 everywhere
        let y1 = nelson_siegel_yield(&params, dec!(1));
        let y10 = nelson_siegel_yield(&params, dec!(10));
        assert_close(y1, dec!(0.05), dec!(0.0001), "NS flat yield at 1y");
        assert_close(y10, dec!(0.05), dec!(0.0001), "NS flat yield at 10y");
    }

    // -----------------------------------------------------------------------
    // Svensson tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_svensson_better_fit_than_ns() {
        // Double-hump curve should be better fitted by Svensson
        let double_hump: Vec<MarketRate> = vec![
            MarketRate {
                maturity: dec!(0.25),
                rate: dec!(0.025),
            },
            MarketRate {
                maturity: dec!(0.5),
                rate: dec!(0.030),
            },
            MarketRate {
                maturity: dec!(1),
                rate: dec!(0.038),
            },
            MarketRate {
                maturity: dec!(2),
                rate: dec!(0.035),
            },
            MarketRate {
                maturity: dec!(3),
                rate: dec!(0.032),
            },
            MarketRate {
                maturity: dec!(5),
                rate: dec!(0.036),
            },
            MarketRate {
                maturity: dec!(7),
                rate: dec!(0.040),
            },
            MarketRate {
                maturity: dec!(10),
                rate: dec!(0.042),
            },
        ];

        let ns_input = NelsonSiegelInput {
            market_rates: double_hump.clone(),
            initial_params: None,
        };
        let ns_result = fit_nelson_siegel(&ns_input).unwrap();

        let sv_input = SvenssonInput {
            market_rates: double_hump,
            initial_params: None,
        };
        let sv_result = fit_svensson(&sv_input).unwrap();

        assert!(
            sv_result.rmse <= ns_result.rmse + dec!(0.001),
            "Svensson RMSE ({}) should be <= NS RMSE ({}) for double-hump",
            sv_result.rmse,
            ns_result.rmse
        );
    }

    #[test]
    fn test_svensson_fitted_rates_count() {
        let input = SvenssonInput {
            market_rates: standard_market_rates(),
            initial_params: None,
        };
        let result = fit_svensson(&input).unwrap();
        assert_eq!(result.fitted_rates.len(), standard_market_rates().len());
    }

    #[test]
    fn test_svensson_r_squared_high() {
        let input = SvenssonInput {
            market_rates: standard_market_rates(),
            initial_params: None,
        };
        let result = fit_svensson(&input).unwrap();
        assert!(
            result.r_squared > dec!(0.9),
            "Svensson R-squared should be high, got {}",
            result.r_squared
        );
    }

    #[test]
    fn test_svensson_insufficient_data() {
        let input = SvenssonInput {
            market_rates: vec![
                MarketRate {
                    maturity: dec!(1),
                    rate: dec!(0.03),
                },
                MarketRate {
                    maturity: dec!(5),
                    rate: dec!(0.04),
                },
                MarketRate {
                    maturity: dec!(10),
                    rate: dec!(0.045),
                },
            ],
            initial_params: None,
        };
        let err = fit_svensson(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }

    #[test]
    fn test_svensson_rmse_small() {
        let input = SvenssonInput {
            market_rates: standard_market_rates(),
            initial_params: None,
        };
        let result = fit_svensson(&input).unwrap();
        assert!(
            result.rmse < dec!(0.005),
            "Svensson RMSE should be < 50bp for standard curve, got {}",
            result.rmse
        );
    }

    // -----------------------------------------------------------------------
    // Bootstrap tests
    // -----------------------------------------------------------------------

    fn standard_bootstrap_instruments() -> Vec<BootstrapInstrument> {
        vec![
            BootstrapInstrument {
                maturity: dec!(1),
                coupon_rate: Decimal::ZERO,
                price: dec!(97.0),
                instrument_type: InstrumentType::ZeroCoupon,
            },
            BootstrapInstrument {
                maturity: dec!(2),
                coupon_rate: dec!(0.035),
                price: dec!(100),
                instrument_type: InstrumentType::ParBond,
            },
            BootstrapInstrument {
                maturity: dec!(3),
                coupon_rate: dec!(0.04),
                price: dec!(100),
                instrument_type: InstrumentType::ParBond,
            },
            BootstrapInstrument {
                maturity: dec!(5),
                coupon_rate: dec!(0.045),
                price: dec!(100),
                instrument_type: InstrumentType::ParBond,
            },
        ]
    }

    #[test]
    fn test_bootstrap_zero_coupon_rate() {
        // Price = 97, face = 100, T = 1
        // r = -ln(97/100) / 1 = -ln(0.97)
        let input = BootstrapInput {
            instruments: vec![BootstrapInstrument {
                maturity: dec!(1),
                coupon_rate: Decimal::ZERO,
                price: dec!(97),
                instrument_type: InstrumentType::ZeroCoupon,
            }],
        };
        let result = run_bootstrap(&input).unwrap();

        let expected_rate = Decimal::ZERO - decimal_ln(dec!(0.97));
        assert_close(
            result.zero_curve[0].rate,
            expected_rate,
            dec!(0.001),
            "Bootstrap zero coupon rate",
        );
    }

    #[test]
    fn test_bootstrap_discount_factors_decreasing() {
        let input = BootstrapInput {
            instruments: standard_bootstrap_instruments(),
        };
        let result = run_bootstrap(&input).unwrap();

        // Discount factors should be decreasing with maturity
        for w in result.discount_factors.windows(2) {
            assert!(
                w[1].factor < w[0].factor,
                "DF at {} ({}) should be < DF at {} ({})",
                w[1].maturity,
                w[1].factor,
                w[0].maturity,
                w[0].factor
            );
        }
    }

    #[test]
    fn test_bootstrap_forward_rate_consistency() {
        // f(t1,t2) = (Z(t2)*t2 - Z(t1)*t1) / (t2 - t1)
        let input = BootstrapInput {
            instruments: standard_bootstrap_instruments(),
        };
        let result = run_bootstrap(&input).unwrap();

        for (i, fwd) in result.forward_rates.iter().enumerate() {
            let z1 = result.zero_curve[i].rate;
            let t1 = result.zero_curve[i].maturity;
            let z2 = result.zero_curve[i + 1].rate;
            let t2 = result.zero_curve[i + 1].maturity;
            let dt = t2 - t1;

            let expected_fwd = (z2 * t2 - z1 * t1) / dt;
            assert_close(
                fwd.rate,
                expected_fwd,
                dec!(0.001),
                &format!("Forward rate consistency at {t1}-{t2}"),
            );
        }
    }

    #[test]
    fn test_bootstrap_zero_curve_length() {
        let instruments = standard_bootstrap_instruments();
        let n = instruments.len();
        let input = BootstrapInput { instruments };
        let result = run_bootstrap(&input).unwrap();
        assert_eq!(result.zero_curve.len(), n);
    }

    #[test]
    fn test_bootstrap_forward_rates_length() {
        let instruments = standard_bootstrap_instruments();
        let n = instruments.len();
        let input = BootstrapInput { instruments };
        let result = run_bootstrap(&input).unwrap();
        // Forward rates between n points = n-1
        assert_eq!(result.forward_rates.len(), n - 1);
    }

    #[test]
    fn test_bootstrap_par_bond_at_par() {
        // A par bond priced at 100 should give a zero rate close to the coupon rate
        let input = BootstrapInput {
            instruments: vec![BootstrapInstrument {
                maturity: dec!(1),
                coupon_rate: dec!(0.05),
                price: dec!(100),
                instrument_type: InstrumentType::ParBond,
            }],
        };
        let result = run_bootstrap(&input).unwrap();

        // For 1-year par bond: 100 = 105 * DF(1) => DF = 100/105 => r ~ 0.04879
        // This is close to the coupon rate but not exactly equal (continuously compounded)
        assert_close(
            result.zero_curve[0].rate,
            dec!(0.04879),
            dec!(0.002),
            "1y par bond zero rate",
        );
    }

    #[test]
    fn test_bootstrap_empty_instruments() {
        let input = BootstrapInput {
            instruments: vec![],
        };
        let err = run_bootstrap(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }

    #[test]
    fn test_bootstrap_unsorted_rejected() {
        let input = BootstrapInput {
            instruments: vec![
                BootstrapInstrument {
                    maturity: dec!(5),
                    coupon_rate: dec!(0.04),
                    price: dec!(100),
                    instrument_type: InstrumentType::ParBond,
                },
                BootstrapInstrument {
                    maturity: dec!(1),
                    coupon_rate: Decimal::ZERO,
                    price: dec!(97),
                    instrument_type: InstrumentType::ZeroCoupon,
                },
            ],
        };
        let err = run_bootstrap(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "instruments");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_bootstrap_negative_price_rejected() {
        let input = BootstrapInput {
            instruments: vec![BootstrapInstrument {
                maturity: dec!(1),
                coupon_rate: Decimal::ZERO,
                price: dec!(-5),
                instrument_type: InstrumentType::ZeroCoupon,
            }],
        };
        let err = run_bootstrap(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert!(field.contains("price"));
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_bootstrap_swap_instrument() {
        // Swap treated like a par bond for bootstrapping purposes
        let input = BootstrapInput {
            instruments: vec![
                BootstrapInstrument {
                    maturity: dec!(1),
                    coupon_rate: Decimal::ZERO,
                    price: dec!(97),
                    instrument_type: InstrumentType::ZeroCoupon,
                },
                BootstrapInstrument {
                    maturity: dec!(2),
                    coupon_rate: dec!(0.04),
                    price: dec!(100),
                    instrument_type: InstrumentType::Swap,
                },
            ],
        };
        let result = run_bootstrap(&input).unwrap();
        assert_eq!(result.zero_curve.len(), 2);
        assert!(result.zero_curve[1].rate > Decimal::ZERO);
    }

    // -----------------------------------------------------------------------
    // Round-trip test
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_bootstrap_then_ns() {
        // Bootstrap a curve, then fit NS to the bootstrapped zero rates
        let bootstrap_input = BootstrapInput {
            instruments: standard_bootstrap_instruments(),
        };
        let bs_result = run_bootstrap(&bootstrap_input).unwrap();

        // Convert bootstrapped zero curve to MarketRate for NS fitting
        let market_rates: Vec<MarketRate> = bs_result
            .zero_curve
            .iter()
            .map(|zr| MarketRate {
                maturity: zr.maturity,
                rate: zr.rate,
            })
            .collect();

        let ns_input = NelsonSiegelInput {
            market_rates,
            initial_params: None,
        };
        let ns_result = fit_nelson_siegel(&ns_input).unwrap();

        // NS should fit the bootstrapped curve well
        assert!(
            ns_result.rmse < dec!(0.005),
            "Round-trip RMSE should be < 50bp, got {}",
            ns_result.rmse
        );
    }

    // -----------------------------------------------------------------------
    // Wrapper function tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_fit_term_structure_ns() {
        let input = TermStructureInput {
            model: TermStructureModel::NelsonSiegel(NelsonSiegelInput {
                market_rates: standard_market_rates(),
                initial_params: None,
            }),
        };
        let result = fit_term_structure(&input).unwrap();
        assert_eq!(result.methodology, "Nelson-Siegel 4-Parameter Yield Curve");
        match result.result {
            TermStructureOutput::NelsonSiegel(ns) => {
                assert!(ns.rmse < dec!(0.005));
            }
            _ => panic!("Expected NelsonSiegel output"),
        }
    }

    #[test]
    fn test_fit_term_structure_svensson() {
        let input = TermStructureInput {
            model: TermStructureModel::Svensson(SvenssonInput {
                market_rates: standard_market_rates(),
                initial_params: None,
            }),
        };
        let result = fit_term_structure(&input).unwrap();
        assert_eq!(
            result.methodology,
            "Svensson 6-Parameter Extended Nelson-Siegel"
        );
    }

    #[test]
    fn test_fit_term_structure_bootstrap() {
        let input = TermStructureInput {
            model: TermStructureModel::Bootstrap(BootstrapInput {
                instruments: standard_bootstrap_instruments(),
            }),
        };
        let result = fit_term_structure(&input).unwrap();
        assert_eq!(
            result.methodology,
            "Piecewise Bootstrap with Linear Interpolation"
        );
        match result.result {
            TermStructureOutput::Bootstrap(bs) => {
                assert!(!bs.zero_curve.is_empty());
            }
            _ => panic!("Expected Bootstrap output"),
        }
    }

    #[test]
    fn test_fit_term_structure_metadata() {
        let input = TermStructureInput {
            model: TermStructureModel::NelsonSiegel(NelsonSiegelInput {
                market_rates: standard_market_rates(),
                initial_params: None,
            }),
        };
        let result = fit_term_structure(&input).unwrap();
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
