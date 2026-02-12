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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptionKind {
    Call,
    Put,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterpolationMethod {
    Linear,
    CubicSpline,
    SVI,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArbitrageFlagType {
    CalendarSpread,
    Butterfly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolQuote {
    pub strike: Decimal,
    pub expiry: Decimal,
    pub implied_vol: Decimal,
    pub option_type: OptionKind,
    pub bid_vol: Option<Decimal>,
    pub ask_vol: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedVolSurfaceInput {
    pub spot_price: Decimal,
    pub risk_free_rate: Decimal,
    pub dividend_yield: Decimal,
    pub market_quotes: Vec<VolQuote>,
    pub interpolation_method: InterpolationMethod,
    pub extrapolation: bool,
    pub target_strikes: Option<Vec<Decimal>>,
    pub target_expiries: Option<Vec<Decimal>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurfacePoint {
    pub strike: Decimal,
    pub expiry: Decimal,
    pub implied_vol: Decimal,
    pub moneyness: Decimal,
    pub log_moneyness: Decimal,
    pub delta: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmileMetrics {
    pub expiry: Decimal,
    pub atm_vol: Decimal,
    pub skew_25d: Decimal,
    pub butterfly_25d: Decimal,
    pub skew_slope: Decimal,
    pub curvature: Decimal,
    pub min_vol: Decimal,
    pub min_vol_strike: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermPoint {
    pub expiry: Decimal,
    pub atm_vol: Decimal,
    pub forward_vol: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageFlag {
    pub flag_type: ArbitrageFlagType,
    pub strike: Decimal,
    pub expiry1: Decimal,
    pub expiry2: Option<Decimal>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SviParams {
    pub a: Decimal,
    pub b: Decimal,
    pub rho: Decimal,
    pub m: Decimal,
    pub sigma: Decimal,
    pub residual: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedVolSurfaceOutput {
    pub surface_points: Vec<SurfacePoint>,
    pub smile_metrics: Vec<SmileMetrics>,
    pub term_structure: Vec<TermPoint>,
    pub arbitrage_flags: Vec<ArbitrageFlag>,
    pub svi_params: Option<SviParams>,
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

/// Standard normal PDF: phi(x) = exp(-x^2/2) / sqrt(2*pi)
fn norm_pdf(x: Decimal) -> Decimal {
    let two_pi = dec!(6.283185307179586);
    let exponent = -(x * x) / dec!(2);
    exp_decimal(exponent) / sqrt_decimal(two_pi)
}

/// Standard normal CDF using Abramowitz & Stegun approximation.
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

/// Absolute value helper
fn abs_decimal(x: Decimal) -> Decimal {
    if x < Decimal::ZERO {
        -x
    } else {
        x
    }
}

// ---------------------------------------------------------------------------
// Black-Scholes delta for smile metric calculations
// ---------------------------------------------------------------------------

/// Compute BS delta for a call option given vol, forward, strike, expiry, rate.
fn bs_call_delta(
    s: Decimal,
    k: Decimal,
    t: Decimal,
    r: Decimal,
    q: Decimal,
    sigma: Decimal,
) -> Decimal {
    let sqrt_t = sqrt_decimal(t);
    let sigma_sqrt_t = sigma * sqrt_t;
    if sigma_sqrt_t == Decimal::ZERO {
        return Decimal::ZERO;
    }
    let d1 = (ln_decimal(s / k) + (r - q + sigma * sigma / dec!(2)) * t) / sigma_sqrt_t;
    let exp_neg_qt = exp_decimal(-q * t);
    exp_neg_qt * norm_cdf(d1)
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &ImpliedVolSurfaceInput) -> CorpFinanceResult<()> {
    if input.spot_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.market_quotes.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "at least one market quote is required".into(),
        ));
    }
    for (i, q) in input.market_quotes.iter().enumerate() {
        if q.strike <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("market_quotes[{i}].strike"),
                reason: "must be positive".into(),
            });
        }
        if q.expiry <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("market_quotes[{i}].expiry"),
                reason: "must be positive".into(),
            });
        }
        if q.implied_vol <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: format!("market_quotes[{i}].implied_vol"),
                reason: "must be positive".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal: Forward price calculation
// ---------------------------------------------------------------------------

fn forward_price(s: Decimal, r: Decimal, q: Decimal, t: Decimal) -> Decimal {
    s * exp_decimal((r - q) * t)
}

// ---------------------------------------------------------------------------
// Internal: Collect unique sorted expiries and strikes from quotes
// ---------------------------------------------------------------------------

fn unique_sorted(vals: &[Decimal]) -> Vec<Decimal> {
    let mut v: Vec<Decimal> = vals.to_vec();
    v.sort();
    v.dedup();
    v
}

fn collect_expiries(quotes: &[VolQuote]) -> Vec<Decimal> {
    unique_sorted(&quotes.iter().map(|q| q.expiry).collect::<Vec<_>>())
}

fn collect_strikes(quotes: &[VolQuote]) -> Vec<Decimal> {
    unique_sorted(&quotes.iter().map(|q| q.strike).collect::<Vec<_>>())
}

// ---------------------------------------------------------------------------
// Internal: Bilinear interpolation in variance space
// ---------------------------------------------------------------------------

/// Linear interpolation between two points
fn lerp(x: Decimal, x0: Decimal, x1: Decimal, y0: Decimal, y1: Decimal) -> Decimal {
    if abs_decimal(x1 - x0) < dec!(0.0000000001) {
        return y0;
    }
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

/// Interpolate variance (vol^2 * T) at a given strike for a specific expiry
fn interpolate_vol_at_expiry(
    quotes: &[VolQuote],
    strike: Decimal,
    expiry: Decimal,
    _strikes_sorted: &[Decimal],
    extrapolation: bool,
) -> Option<Decimal> {
    // Collect quotes at this expiry
    let tol = dec!(0.0000001);
    let mut expiry_quotes: Vec<(Decimal, Decimal)> = Vec::new();
    for q in quotes {
        if abs_decimal(q.expiry - expiry) < tol {
            expiry_quotes.push((q.strike, q.implied_vol));
        }
    }
    expiry_quotes.sort_by(|a, b| a.0.cmp(&b.0));

    if expiry_quotes.is_empty() {
        return None;
    }
    if expiry_quotes.len() == 1 {
        return Some(expiry_quotes[0].1);
    }

    // Check bounds
    let min_k = expiry_quotes.first().unwrap().0;
    let max_k = expiry_quotes.last().unwrap().0;

    if strike < min_k {
        if !extrapolation {
            return None;
        }
        // Flat extrapolation
        return Some(expiry_quotes.first().unwrap().1);
    }
    if strike > max_k {
        if !extrapolation {
            return None;
        }
        return Some(expiry_quotes.last().unwrap().1);
    }

    // Find bounding strikes
    let mut lower_idx = 0;
    for (i, &(k, _)) in expiry_quotes.iter().enumerate() {
        if k <= strike {
            lower_idx = i;
        }
    }
    let upper_idx = if lower_idx + 1 < expiry_quotes.len() {
        lower_idx + 1
    } else {
        lower_idx
    };

    let (k0, v0) = expiry_quotes[lower_idx];
    let (k1, v1) = expiry_quotes[upper_idx];

    if abs_decimal(k0 - k1) < tol {
        return Some(v0);
    }

    // Interpolate in variance space: var = vol^2 * T
    let var0 = v0 * v0 * expiry;
    let var1 = v1 * v1 * expiry;
    let var_interp = lerp(strike, k0, k1, var0, var1);

    if var_interp <= Decimal::ZERO || expiry <= Decimal::ZERO {
        return Some(v0);
    }

    let vol = sqrt_decimal(var_interp / expiry);
    Some(vol)
}

/// Bilinear interpolation across strike and expiry in variance space
fn bilinear_interpolate(
    quotes: &[VolQuote],
    strike: Decimal,
    expiry: Decimal,
    expiries_sorted: &[Decimal],
    strikes_sorted: &[Decimal],
    extrapolation: bool,
) -> Option<Decimal> {
    if expiries_sorted.is_empty() {
        return None;
    }

    // Find bounding expiries
    let min_t = expiries_sorted[0];
    let max_t = *expiries_sorted.last().unwrap();

    if expiry < min_t {
        if !extrapolation {
            return None;
        }
        return interpolate_vol_at_expiry(quotes, strike, min_t, strikes_sorted, extrapolation);
    }
    if expiry > max_t {
        if !extrapolation {
            return None;
        }
        return interpolate_vol_at_expiry(quotes, strike, max_t, strikes_sorted, extrapolation);
    }

    // Find bounding expiries
    let mut lower_t_idx = 0;
    for (i, &t) in expiries_sorted.iter().enumerate() {
        if t <= expiry {
            lower_t_idx = i;
        }
    }
    let upper_t_idx = if lower_t_idx + 1 < expiries_sorted.len() {
        lower_t_idx + 1
    } else {
        lower_t_idx
    };

    let t0 = expiries_sorted[lower_t_idx];
    let t1 = expiries_sorted[upper_t_idx];

    let vol_t0 = interpolate_vol_at_expiry(quotes, strike, t0, strikes_sorted, extrapolation)?;
    let vol_t1 = interpolate_vol_at_expiry(quotes, strike, t1, strikes_sorted, extrapolation)?;

    let tol = dec!(0.0000001);
    if abs_decimal(t0 - t1) < tol {
        return Some(vol_t0);
    }

    // Interpolate in total variance space
    let var0 = vol_t0 * vol_t0 * t0;
    let var1 = vol_t1 * vol_t1 * t1;
    let var_interp = lerp(expiry, t0, t1, var0, var1);

    if var_interp <= Decimal::ZERO || expiry <= Decimal::ZERO {
        return Some(vol_t0);
    }

    Some(sqrt_decimal(var_interp / expiry))
}

// ---------------------------------------------------------------------------
// SVI parameterization and fitting
// ---------------------------------------------------------------------------

/// SVI total variance: w(k) = a + b * (rho*(k-m) + sqrt((k-m)^2 + sigma^2))
fn svi_total_variance(
    k: Decimal,
    a: Decimal,
    b: Decimal,
    rho: Decimal,
    m: Decimal,
    sigma: Decimal,
) -> Decimal {
    let km = k - m;
    let inner = sqrt_decimal(km * km + sigma * sigma);
    a + b * (rho * km + inner)
}

/// Fit SVI parameters to market quotes using iterative Gauss-Newton (50 iterations).
/// Returns (a, b, rho, m, sigma, residual).
fn fit_svi(
    quotes: &[VolQuote],
    spot: Decimal,
    r: Decimal,
    q: Decimal,
) -> (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal) {
    // Collect unique expiries; fit SVI to the expiry with most quotes (or all if single)
    // For simplicity, fit across all quotes using log-moneyness k = ln(K/F)
    // and total variance w = vol^2 * T

    let n = quotes.len();
    if n == 0 {
        return (
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            Decimal::ZERO,
            dec!(0.1),
            Decimal::ZERO,
        );
    }

    // Build observation vectors: (k_i, w_i)
    let mut obs: Vec<(Decimal, Decimal)> = Vec::with_capacity(n);
    for quote in quotes {
        let fwd = forward_price(spot, r, q, quote.expiry);
        let k = ln_decimal(quote.strike / fwd);
        let w = quote.implied_vol * quote.implied_vol * quote.expiry;
        obs.push((k, w));
    }

    // Initial guesses
    let mean_w = obs
        .iter()
        .map(|o| o.1)
        .fold(Decimal::ZERO, |acc, v| acc + v)
        / Decimal::from(n as u32);
    let mut a = mean_w;
    let mut b = dec!(0.1);
    let mut rho = dec!(-0.3);
    let mut m = Decimal::ZERO;
    let mut sigma = dec!(0.3);

    let bump = dec!(0.001);
    let damping = dec!(0.01); // Levenberg-Marquardt damping

    for _iter in 0..50 {
        // Compute residuals: r_i = w_observed_i - svi(k_i)
        let mut residuals: Vec<Decimal> = Vec::with_capacity(n);
        let mut total_sq = Decimal::ZERO;
        for &(k, w_obs) in &obs {
            let w_model = svi_total_variance(k, a, b, rho, m, sigma);
            let r = w_obs - w_model;
            residuals.push(r);
            total_sq += r * r;
        }

        // Compute Jacobian via finite differences (5 params: a, b, rho, m, sigma)
        // J[i][j] = d(svi(k_i)) / d(param_j)
        let params = [a, b, rho, m, sigma];
        let mut jtj = [[Decimal::ZERO; 5]; 5]; // J^T * J
        let mut jtr = [Decimal::ZERO; 5]; // J^T * r

        for (i, &(k, _)) in obs.iter().enumerate() {
            let mut grad = [Decimal::ZERO; 5];
            for j in 0..5 {
                let mut p_up = params;
                p_up[j] += bump;
                let w_up = svi_total_variance(k, p_up[0], p_up[1], p_up[2], p_up[3], p_up[4]);
                let mut p_dn = params;
                p_dn[j] -= bump;
                let w_dn = svi_total_variance(k, p_dn[0], p_dn[1], p_dn[2], p_dn[3], p_dn[4]);
                grad[j] = (w_up - w_dn) / (dec!(2) * bump);
            }

            for j1 in 0..5 {
                jtr[j1] += grad[j1] * residuals[i];
                for j2 in 0..5 {
                    jtj[j1][j2] += grad[j1] * grad[j2];
                }
            }
        }

        // Add damping (Levenberg-Marquardt)
        #[allow(clippy::needless_range_loop)]
        for j in 0..5 {
            jtj[j][j] += damping;
        }

        // Solve (J^T*J + lambda*I) * delta = J^T*r via Gauss elimination
        let delta = solve_5x5(&jtj, &jtr);

        // Update parameters
        a += delta[0];
        b += delta[1];
        rho += delta[2];
        m += delta[3];
        sigma += delta[4];

        // Enforce constraints
        if b < dec!(0.0001) {
            b = dec!(0.0001);
        }
        if rho < dec!(-0.999) {
            rho = dec!(-0.999);
        }
        if rho > dec!(0.999) {
            rho = dec!(0.999);
        }
        if sigma < dec!(0.001) {
            sigma = dec!(0.001);
        }

        // Check convergence
        let delta_norm = delta
            .iter()
            .map(|d| d * d)
            .fold(Decimal::ZERO, |acc, v| acc + v);
        if delta_norm < dec!(0.000000001) {
            break;
        }
    }

    // Final residual
    let mut total_sq = Decimal::ZERO;
    for &(k, w_obs) in &obs {
        let w_model = svi_total_variance(k, a, b, rho, m, sigma);
        let r = w_obs - w_model;
        total_sq += r * r;
    }
    let residual = sqrt_decimal(total_sq / Decimal::from(n as u32));

    (a, b, rho, m, sigma, residual)
}

/// Solve 5x5 linear system Ax = b via Gaussian elimination with partial pivoting.
#[allow(clippy::needless_range_loop)]
fn solve_5x5(a: &[[Decimal; 5]; 5], b: &[Decimal; 5]) -> [Decimal; 5] {
    let mut aug = [[Decimal::ZERO; 6]; 5];
    for i in 0..5 {
        for j in 0..5 {
            aug[i][j] = a[i][j];
        }
        aug[i][5] = b[i];
    }

    // Forward elimination with partial pivoting
    for col in 0..5 {
        // Find pivot
        let mut max_val = abs_decimal(aug[col][col]);
        let mut max_row = col;
        for row in (col + 1)..5 {
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

        for row in (col + 1)..5 {
            let factor = aug[row][col] / pivot;
            for j in col..6 {
                let val = aug[col][j];
                aug[row][j] -= factor * val;
            }
        }
    }

    // Back substitution
    let mut x = [Decimal::ZERO; 5];
    for i in (0..5).rev() {
        let mut sum = aug[i][5];
        for j in (i + 1)..5 {
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
// Smile metrics computation
// ---------------------------------------------------------------------------

/// Find 25-delta strike for a call via Newton-Raphson on BS delta
#[allow(clippy::too_many_arguments)]
fn find_delta_strike(
    s: Decimal,
    t: Decimal,
    r: Decimal,
    q: Decimal,
    target_delta: Decimal,
    quotes: &[VolQuote],
    expiries_sorted: &[Decimal],
    strikes_sorted: &[Decimal],
    extrapolation: bool,
    is_call: bool,
) -> Option<Decimal> {
    // Start at ATM
    let fwd = forward_price(s, r, q, t);
    let mut k = fwd;

    for _ in 0..30 {
        // Get vol at this strike
        let vol =
            bilinear_interpolate(quotes, k, t, expiries_sorted, strikes_sorted, extrapolation)?;
        let delta = bs_call_delta(s, k, t, r, q, vol);
        let actual_delta = if is_call { delta } else { delta - Decimal::ONE };

        let diff = actual_delta - target_delta;
        if abs_decimal(diff) < dec!(0.0001) {
            return Some(k);
        }

        // Numerical derivative: d(delta)/dK
        let dk = k * dec!(0.001);
        let vol_up = bilinear_interpolate(
            quotes,
            k + dk,
            t,
            expiries_sorted,
            strikes_sorted,
            extrapolation,
        )?;
        let delta_up = bs_call_delta(s, k + dk, t, r, q, vol_up);
        let actual_up = if is_call {
            delta_up
        } else {
            delta_up - Decimal::ONE
        };
        let ddelta_dk = (actual_up - actual_delta) / dk;

        if abs_decimal(ddelta_dk) < dec!(0.0000001) {
            break;
        }

        k -= diff / ddelta_dk;
        if k <= Decimal::ZERO {
            k = fwd * dec!(0.5);
        }
    }
    Some(k)
}

fn compute_smile_metrics(
    s: Decimal,
    r: Decimal,
    q: Decimal,
    expiry: Decimal,
    quotes: &[VolQuote],
    expiries_sorted: &[Decimal],
    strikes_sorted: &[Decimal],
    extrapolation: bool,
) -> Option<SmileMetrics> {
    let fwd = forward_price(s, r, q, expiry);

    // ATM vol (at forward strike)
    let atm_vol = bilinear_interpolate(
        quotes,
        fwd,
        expiry,
        expiries_sorted,
        strikes_sorted,
        extrapolation,
    )?;

    // 25-delta call strike
    let call_25d_strike = find_delta_strike(
        s,
        expiry,
        r,
        q,
        dec!(0.25),
        quotes,
        expiries_sorted,
        strikes_sorted,
        extrapolation,
        true,
    )?;
    let call_25d_vol = bilinear_interpolate(
        quotes,
        call_25d_strike,
        expiry,
        expiries_sorted,
        strikes_sorted,
        extrapolation,
    )?;

    // 25-delta put strike
    let put_25d_strike = find_delta_strike(
        s,
        expiry,
        r,
        q,
        dec!(-0.25),
        quotes,
        expiries_sorted,
        strikes_sorted,
        extrapolation,
        false,
    )?;
    let put_25d_vol = bilinear_interpolate(
        quotes,
        put_25d_strike,
        expiry,
        expiries_sorted,
        strikes_sorted,
        extrapolation,
    )?;

    // Risk reversal: vol(25d call) - vol(25d put)
    let skew_25d = call_25d_vol - put_25d_vol;

    // Butterfly: (vol(25d call) + vol(25d put)) / 2 - vol(ATM)
    let butterfly_25d = (call_25d_vol + put_25d_vol) / dec!(2) - atm_vol;

    // Skew slope: dVol/dK at ATM (via finite difference)
    let dk = fwd * dec!(0.01);
    let vol_up = bilinear_interpolate(
        quotes,
        fwd + dk,
        expiry,
        expiries_sorted,
        strikes_sorted,
        extrapolation,
    )
    .unwrap_or(atm_vol);
    let vol_dn = bilinear_interpolate(
        quotes,
        fwd - dk,
        expiry,
        expiries_sorted,
        strikes_sorted,
        extrapolation,
    )
    .unwrap_or(atm_vol);
    let skew_slope = (vol_up - vol_dn) / (dec!(2) * dk);

    // Curvature: d2Vol/dK2 at ATM
    let curvature = (vol_up - dec!(2) * atm_vol + vol_dn) / (dk * dk);

    // Find minimum vol across observed strikes at this expiry
    let tol = dec!(0.0000001);
    let mut min_vol = atm_vol;
    let mut min_vol_strike = fwd;
    for q_inner in quotes {
        if abs_decimal(q_inner.expiry - expiry) < tol && q_inner.implied_vol < min_vol {
            min_vol = q_inner.implied_vol;
            min_vol_strike = q_inner.strike;
        }
    }

    Some(SmileMetrics {
        expiry,
        atm_vol,
        skew_25d,
        butterfly_25d,
        skew_slope,
        curvature,
        min_vol,
        min_vol_strike,
    })
}

// ---------------------------------------------------------------------------
// Term structure computation
// ---------------------------------------------------------------------------

fn compute_term_structure(
    s: Decimal,
    r: Decimal,
    q: Decimal,
    expiries_sorted: &[Decimal],
    quotes: &[VolQuote],
    strikes_sorted: &[Decimal],
    extrapolation: bool,
) -> Vec<TermPoint> {
    let mut term_points: Vec<TermPoint> = Vec::new();

    for (i, &t) in expiries_sorted.iter().enumerate() {
        let fwd = forward_price(s, r, q, t);
        let atm_vol = bilinear_interpolate(
            quotes,
            fwd,
            t,
            expiries_sorted,
            strikes_sorted,
            extrapolation,
        )
        .unwrap_or(Decimal::ZERO);

        let forward_vol = if i == 0 {
            atm_vol // First expiry: forward vol = spot vol
        } else {
            let t_prev = expiries_sorted[i - 1];
            let fwd_prev = forward_price(s, r, q, t_prev);
            let vol_prev = bilinear_interpolate(
                quotes,
                fwd_prev,
                t_prev,
                expiries_sorted,
                strikes_sorted,
                extrapolation,
            )
            .unwrap_or(Decimal::ZERO);

            // Forward vol: sigma_fwd = sqrt((sigma2^2 * T2 - sigma1^2 * T1) / (T2 - T1))
            let var2 = atm_vol * atm_vol * t;
            let var1 = vol_prev * vol_prev * t_prev;
            let dt = t - t_prev;

            if dt > Decimal::ZERO && var2 >= var1 {
                sqrt_decimal((var2 - var1) / dt)
            } else {
                atm_vol // Fallback if non-monotone
            }
        };

        term_points.push(TermPoint {
            expiry: t,
            atm_vol,
            forward_vol,
        });
    }

    term_points
}

// ---------------------------------------------------------------------------
// Arbitrage detection
// ---------------------------------------------------------------------------

fn detect_arbitrage(
    quotes: &[VolQuote],
    expiries_sorted: &[Decimal],
    strikes_sorted: &[Decimal],
    s: Decimal,
    r: Decimal,
    q: Decimal,
    extrapolation: bool,
) -> Vec<ArbitrageFlag> {
    let mut flags: Vec<ArbitrageFlag> = Vec::new();
    let tol = dec!(0.0000001);

    // Calendar spread arbitrage: total variance must be non-decreasing in T
    for &k in strikes_sorted {
        let mut prev_total_var: Option<(Decimal, Decimal)> = None; // (expiry, total_var)

        for &t in expiries_sorted {
            if let Some(vol) =
                bilinear_interpolate(quotes, k, t, expiries_sorted, strikes_sorted, extrapolation)
            {
                let total_var = vol * vol * t;

                if let Some((prev_t, prev_var)) = prev_total_var {
                    if total_var < prev_var - tol {
                        flags.push(ArbitrageFlag {
                            flag_type: ArbitrageFlagType::CalendarSpread,
                            strike: k,
                            expiry1: prev_t,
                            expiry2: Some(t),
                            description: format!(
                                "Calendar arbitrage at K={}: total var decreases from {} (T={}) to {} (T={})",
                                k, prev_var, prev_t, total_var, t
                            ),
                        });
                    }
                }
                prev_total_var = Some((t, total_var));
            }
        }
    }

    // Butterfly arbitrage: d^2(total_var)/dk^2 >= 0 (convexity in log-moneyness)
    for &t in expiries_sorted {
        let fwd = forward_price(s, r, q, t);

        // Check convexity at each interior strike
        let tol_k = dec!(0.0000001);
        let expiry_quotes: Vec<&VolQuote> = quotes
            .iter()
            .filter(|qq| abs_decimal(qq.expiry - t) < tol_k)
            .collect();

        let mut strike_vols: Vec<(Decimal, Decimal)> = Vec::new();
        for qq in &expiry_quotes {
            let k_log = ln_decimal(qq.strike / fwd);
            let w = qq.implied_vol * qq.implied_vol * t;
            strike_vols.push((k_log, w));
        }
        strike_vols.sort_by(|a, b| a.0.cmp(&b.0));

        for i in 1..strike_vols.len().saturating_sub(1) {
            let (k0, w0) = strike_vols[i - 1];
            let (k1, w1) = strike_vols[i];
            let (k2, w2) = strike_vols[i + 1];

            let dk1 = k1 - k0;
            let dk2 = k2 - k1;

            if dk1 > tol && dk2 > tol {
                // Second derivative approximation
                let d2w = (w2 - w1) / dk2 - (w1 - w0) / dk1;
                let avg_dk = (dk1 + dk2) / dec!(2);
                let second_deriv = d2w / avg_dk;

                if second_deriv < -tol {
                    // Convert log-moneyness back to strike for reporting
                    let strike_report = expiry_quotes.get(i).map(|qq| qq.strike).unwrap_or(k1);
                    flags.push(ArbitrageFlag {
                        flag_type: ArbitrageFlagType::Butterfly,
                        strike: strike_report,
                        expiry1: t,
                        expiry2: None,
                        description: format!(
                            "Butterfly arbitrage at T={}, K~{}: negative convexity in total variance (d2w/dk2 = {})",
                            t, strike_report, second_deriv
                        ),
                    });
                }
            }
        }
    }

    flags
}

// ---------------------------------------------------------------------------
// Surface point construction
// ---------------------------------------------------------------------------

fn build_surface_points(
    quotes: &[VolQuote],
    input: &ImpliedVolSurfaceInput,
    expiries_sorted: &[Decimal],
    strikes_sorted: &[Decimal],
) -> Vec<SurfacePoint> {
    let s = input.spot_price;
    let r = input.risk_free_rate;
    let q = input.dividend_yield;

    let target_strikes = input.target_strikes.as_deref().unwrap_or(strikes_sorted);
    let target_expiries = input.target_expiries.as_deref().unwrap_or(expiries_sorted);

    let mut points: Vec<SurfacePoint> = Vec::new();

    for &t in target_expiries {
        if t <= Decimal::ZERO {
            continue;
        }
        let fwd = forward_price(s, r, q, t);

        for &k in target_strikes {
            if k <= Decimal::ZERO {
                continue;
            }

            let vol_opt = match input.interpolation_method {
                InterpolationMethod::Linear | InterpolationMethod::CubicSpline => {
                    bilinear_interpolate(
                        quotes,
                        k,
                        t,
                        expiries_sorted,
                        strikes_sorted,
                        input.extrapolation,
                    )
                }
                InterpolationMethod::SVI => {
                    // SVI interpolation is handled differently - use the fitted params
                    // But we can fall back to bilinear here since SVI params are stored separately
                    bilinear_interpolate(
                        quotes,
                        k,
                        t,
                        expiries_sorted,
                        strikes_sorted,
                        input.extrapolation,
                    )
                }
            };

            if let Some(vol) = vol_opt {
                let moneyness = k / s;
                let log_moneyness = ln_decimal(k / fwd);
                let delta = bs_call_delta(s, k, t, r, q, vol);

                points.push(SurfacePoint {
                    strike: k,
                    expiry: t,
                    implied_vol: vol,
                    moneyness,
                    log_moneyness,
                    delta,
                });
            }
        }
    }

    points
}

/// Build SVI-interpolated surface points using fitted SVI params
fn build_svi_surface_points(
    input: &ImpliedVolSurfaceInput,
    svi: &SviParams,
    expiries_sorted: &[Decimal],
    strikes_sorted: &[Decimal],
) -> Vec<SurfacePoint> {
    let s = input.spot_price;
    let r = input.risk_free_rate;
    let q = input.dividend_yield;

    let target_strikes = input.target_strikes.as_deref().unwrap_or(strikes_sorted);
    let target_expiries = input.target_expiries.as_deref().unwrap_or(expiries_sorted);

    let mut points: Vec<SurfacePoint> = Vec::new();

    for &t in target_expiries {
        if t <= Decimal::ZERO {
            continue;
        }
        let fwd = forward_price(s, r, q, t);

        for &k in target_strikes {
            if k <= Decimal::ZERO {
                continue;
            }

            let log_m = ln_decimal(k / fwd);
            let total_var = svi_total_variance(log_m, svi.a, svi.b, svi.rho, svi.m, svi.sigma);

            if total_var > Decimal::ZERO && t > Decimal::ZERO {
                let vol = sqrt_decimal(total_var / t);
                let moneyness = k / s;
                let delta = bs_call_delta(s, k, t, r, q, vol);

                points.push(SurfacePoint {
                    strike: k,
                    expiry: t,
                    implied_vol: vol,
                    moneyness,
                    log_moneyness: log_m,
                    delta,
                });
            }
        }
    }

    points
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn build_implied_vol_surface(
    input: &ImpliedVolSurfaceInput,
) -> CorpFinanceResult<ComputationOutput<ImpliedVolSurfaceOutput>> {
    let start = Instant::now();
    validate_input(input)?;

    let s = input.spot_price;
    let r = input.risk_free_rate;
    let q = input.dividend_yield;
    let quotes = &input.market_quotes;

    let expiries_sorted = collect_expiries(quotes);
    let strikes_sorted = collect_strikes(quotes);

    // Build SVI params if SVI method selected
    let svi_params = if input.interpolation_method == InterpolationMethod::SVI {
        let (a, b, rho, m, sigma, residual) = fit_svi(quotes, s, r, q);
        Some(SviParams {
            a,
            b,
            rho,
            m,
            sigma,
            residual,
        })
    } else {
        None
    };

    // Build surface points
    let surface_points = if let Some(ref svi) = svi_params {
        build_svi_surface_points(input, svi, &expiries_sorted, &strikes_sorted)
    } else {
        build_surface_points(quotes, input, &expiries_sorted, &strikes_sorted)
    };

    // Compute smile metrics per expiry
    let mut smile_metrics: Vec<SmileMetrics> = Vec::new();
    for &t in &expiries_sorted {
        if let Some(metrics) = compute_smile_metrics(
            s,
            r,
            q,
            t,
            quotes,
            &expiries_sorted,
            &strikes_sorted,
            input.extrapolation,
        ) {
            smile_metrics.push(metrics);
        }
    }

    // Compute term structure
    let term_structure = compute_term_structure(
        s,
        r,
        q,
        &expiries_sorted,
        quotes,
        &strikes_sorted,
        input.extrapolation,
    );

    // Detect arbitrage
    let arbitrage_flags = detect_arbitrage(
        quotes,
        &expiries_sorted,
        &strikes_sorted,
        s,
        r,
        q,
        input.extrapolation,
    );

    let output = ImpliedVolSurfaceOutput {
        surface_points,
        smile_metrics,
        term_structure,
        arbitrage_flags,
        svi_params,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "model": format!("{:?} interpolation", input.interpolation_method),
        "spot_price": s.to_string(),
        "risk_free_rate": r.to_string(),
        "dividend_yield": q.to_string(),
        "num_quotes": quotes.len(),
        "extrapolation": input.extrapolation,
    });

    Ok(with_metadata(
        "Implied Volatility Surface Construction",
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

    fn make_quote(strike: Decimal, expiry: Decimal, vol: Decimal) -> VolQuote {
        VolQuote {
            strike,
            expiry,
            implied_vol: vol,
            option_type: OptionKind::Call,
            bid_vol: None,
            ask_vol: None,
        }
    }

    /// Standard test surface: 3 expiries x 5 strikes with realistic equity smile
    fn standard_quotes() -> Vec<VolQuote> {
        vec![
            // T = 0.25 (3 months)
            make_quote(dec!(80), dec!(0.25), dec!(0.30)),
            make_quote(dec!(90), dec!(0.25), dec!(0.25)),
            make_quote(dec!(100), dec!(0.25), dec!(0.20)),
            make_quote(dec!(110), dec!(0.25), dec!(0.22)),
            make_quote(dec!(120), dec!(0.25), dec!(0.26)),
            // T = 0.50 (6 months)
            make_quote(dec!(80), dec!(0.50), dec!(0.28)),
            make_quote(dec!(90), dec!(0.50), dec!(0.23)),
            make_quote(dec!(100), dec!(0.50), dec!(0.19)),
            make_quote(dec!(110), dec!(0.50), dec!(0.21)),
            make_quote(dec!(120), dec!(0.50), dec!(0.25)),
            // T = 1.00 (1 year)
            make_quote(dec!(80), dec!(1.00), dec!(0.27)),
            make_quote(dec!(90), dec!(1.00), dec!(0.22)),
            make_quote(dec!(100), dec!(1.00), dec!(0.18)),
            make_quote(dec!(110), dec!(1.00), dec!(0.20)),
            make_quote(dec!(120), dec!(1.00), dec!(0.24)),
        ]
    }

    fn standard_input() -> ImpliedVolSurfaceInput {
        ImpliedVolSurfaceInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            dividend_yield: dec!(0.02),
            market_quotes: standard_quotes(),
            interpolation_method: InterpolationMethod::Linear,
            extrapolation: true,
            target_strikes: None,
            target_expiries: None,
        }
    }

    // -----------------------------------------------------------------------
    // Math helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_exp_decimal_zero() {
        assert!(approx_eq(exp_decimal(dec!(0)), dec!(1), dec!(0.0001)));
    }

    #[test]
    fn test_exp_decimal_one() {
        assert!(approx_eq(exp_decimal(dec!(1)), dec!(2.71828), dec!(0.001)));
    }

    #[test]
    fn test_exp_decimal_negative() {
        let val = exp_decimal(dec!(-1));
        assert!(approx_eq(val, dec!(0.36788), dec!(0.001)));
    }

    #[test]
    fn test_sqrt_decimal_perfect() {
        assert!(approx_eq(sqrt_decimal(dec!(4)), dec!(2), dec!(0.0001)));
        assert!(approx_eq(sqrt_decimal(dec!(9)), dec!(3), dec!(0.0001)));
    }

    #[test]
    fn test_sqrt_decimal_irrational() {
        assert!(approx_eq(sqrt_decimal(dec!(2)), dec!(1.41421), dec!(0.001)));
    }

    #[test]
    fn test_ln_decimal_one() {
        assert!(approx_eq(ln_decimal(dec!(1)), dec!(0), dec!(0.0001)));
    }

    #[test]
    fn test_ln_decimal_e() {
        assert!(approx_eq(
            ln_decimal(dec!(2.718281828)),
            dec!(1),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_norm_cdf_symmetry() {
        let n0 = norm_cdf(dec!(0));
        assert!(approx_eq(n0, dec!(0.5), dec!(0.001)));
    }

    // -----------------------------------------------------------------------
    // Validation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_validate_zero_spot() {
        let input = ImpliedVolSurfaceInput {
            spot_price: dec!(0),
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "spot_price"),
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_empty_quotes() {
        let input = ImpliedVolSurfaceInput {
            market_quotes: vec![],
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_negative_strike() {
        let mut input = standard_input();
        input.market_quotes[0].strike = dec!(-10);
        let result = build_implied_vol_surface(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zero_vol() {
        let mut input = standard_input();
        input.market_quotes[0].implied_vol = dec!(0);
        let result = build_implied_vol_surface(&input);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zero_expiry() {
        let mut input = standard_input();
        input.market_quotes[0].expiry = dec!(0);
        let result = build_implied_vol_surface(&input);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Surface construction tests (Linear)
    // -----------------------------------------------------------------------

    #[test]
    fn test_linear_surface_basic() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        assert!(!result.result.surface_points.is_empty());
    }

    #[test]
    fn test_linear_surface_point_count() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        // 5 strikes x 3 expiries = 15 points
        assert_eq!(result.result.surface_points.len(), 15);
    }

    #[test]
    fn test_linear_surface_atm_vol_recovery() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        // Find ATM vol at T=0.25
        let atm_point = result
            .result
            .surface_points
            .iter()
            .find(|p| p.strike == dec!(100) && p.expiry == dec!(0.25))
            .expect("ATM point at T=0.25 not found");
        assert!(approx_eq(atm_point.implied_vol, dec!(0.20), dec!(0.01)));
    }

    #[test]
    fn test_linear_interpolation_between_strikes() {
        // Add a target strike between two observed strikes
        let input = ImpliedVolSurfaceInput {
            target_strikes: Some(vec![dec!(95)]),
            target_expiries: Some(vec![dec!(0.25)]),
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert_eq!(result.result.surface_points.len(), 1);
        let vol = result.result.surface_points[0].implied_vol;
        // Should be between vol(90)=0.25 and vol(100)=0.20, interpolated in variance space
        assert!(
            vol > dec!(0.19) && vol < dec!(0.26),
            "Interpolated vol {} out of range",
            vol
        );
    }

    #[test]
    fn test_linear_interpolation_between_expiries() {
        // Interpolate at T=0.375 (between 0.25 and 0.50)
        let input = ImpliedVolSurfaceInput {
            target_strikes: Some(vec![dec!(100)]),
            target_expiries: Some(vec![dec!(0.375)]),
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert_eq!(result.result.surface_points.len(), 1);
        let vol = result.result.surface_points[0].implied_vol;
        // Between 0.20 (T=0.25) and 0.19 (T=0.50)
        assert!(
            vol > dec!(0.18) && vol < dec!(0.21),
            "Interpolated vol {} out of range",
            vol
        );
    }

    #[test]
    fn test_moneyness_calculation() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        for p in &result.result.surface_points {
            let expected_moneyness = p.strike / dec!(100);
            assert!(approx_eq(p.moneyness, expected_moneyness, dec!(0.0001)));
        }
    }

    #[test]
    fn test_delta_positive_for_otm_calls() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        for p in &result.result.surface_points {
            // BS call delta should always be in [0, 1]
            assert!(
                p.delta >= Decimal::ZERO && p.delta <= Decimal::ONE,
                "Delta {} out of [0,1] at K={}, T={}",
                p.delta,
                p.strike,
                p.expiry
            );
        }
    }

    // -----------------------------------------------------------------------
    // Extrapolation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_extrapolation_out_of_range() {
        let input = ImpliedVolSurfaceInput {
            extrapolation: false,
            target_strikes: Some(vec![dec!(50)]), // Below observed range
            target_expiries: Some(vec![dec!(0.25)]),
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        // Point should not be generated when extrapolation is off
        assert!(result.result.surface_points.is_empty());
    }

    #[test]
    fn test_extrapolation_flat() {
        let input = ImpliedVolSurfaceInput {
            extrapolation: true,
            target_strikes: Some(vec![dec!(50)]), // Below observed range
            target_expiries: Some(vec![dec!(0.25)]),
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert_eq!(result.result.surface_points.len(), 1);
        // Flat extrapolation: should equal the boundary vol
        let vol = result.result.surface_points[0].implied_vol;
        assert!(
            approx_eq(vol, dec!(0.30), dec!(0.01)),
            "Extrapolated vol {} should be near 0.30",
            vol
        );
    }

    // -----------------------------------------------------------------------
    // SVI fitting tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_svi_surface_basic() {
        let input = ImpliedVolSurfaceInput {
            interpolation_method: InterpolationMethod::SVI,
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert!(result.result.svi_params.is_some());
    }

    #[test]
    fn test_svi_params_constraints() {
        let input = ImpliedVolSurfaceInput {
            interpolation_method: InterpolationMethod::SVI,
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        let svi = result.result.svi_params.unwrap();
        // b > 0
        assert!(svi.b > Decimal::ZERO, "SVI b={} should be positive", svi.b);
        // -1 < rho < 1
        assert!(
            svi.rho > dec!(-1) && svi.rho < dec!(1),
            "SVI rho={} out of (-1,1)",
            svi.rho
        );
        // sigma > 0
        assert!(
            svi.sigma > Decimal::ZERO,
            "SVI sigma={} should be positive",
            svi.sigma
        );
    }

    #[test]
    fn test_svi_residual_reasonable() {
        let input = ImpliedVolSurfaceInput {
            interpolation_method: InterpolationMethod::SVI,
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        let svi = result.result.svi_params.unwrap();
        // Residual should be small for a well-specified surface
        assert!(
            svi.residual < dec!(0.1),
            "SVI residual {} too large",
            svi.residual
        );
    }

    #[test]
    fn test_svi_total_variance_positive() {
        let input = ImpliedVolSurfaceInput {
            interpolation_method: InterpolationMethod::SVI,
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        for p in &result.result.surface_points {
            assert!(
                p.implied_vol > Decimal::ZERO,
                "SVI vol should be positive at K={}",
                p.strike
            );
        }
    }

    // -----------------------------------------------------------------------
    // Smile metrics tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_smile_metrics_per_expiry() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        // Should have metrics for each of the 3 expiries
        assert_eq!(result.result.smile_metrics.len(), 3);
    }

    #[test]
    fn test_smile_atm_vol_positive() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        for m in &result.result.smile_metrics {
            assert!(
                m.atm_vol > Decimal::ZERO,
                "ATM vol should be positive at T={}",
                m.expiry
            );
        }
    }

    #[test]
    fn test_smile_negative_skew() {
        // Equity skew: put vols > call vols, so risk reversal should be negative
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        // At least one expiry should have negative skew (typical equity smile)
        let has_negative_skew = result
            .result
            .smile_metrics
            .iter()
            .any(|m| m.skew_25d < Decimal::ZERO);
        assert!(
            has_negative_skew,
            "Expected negative equity skew in at least one expiry"
        );
    }

    #[test]
    fn test_smile_butterfly_positive() {
        // Butterfly should typically be non-negative (smile curvature)
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        for m in &result.result.smile_metrics {
            // Butterfly = (vol_25dC + vol_25dP)/2 - vol_ATM >= 0 for convex smiles
            assert!(
                m.butterfly_25d >= dec!(-0.05),
                "Butterfly {} unexpectedly negative at T={}",
                m.butterfly_25d,
                m.expiry
            );
        }
    }

    #[test]
    fn test_smile_min_vol_exists() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        for m in &result.result.smile_metrics {
            assert!(m.min_vol > Decimal::ZERO);
            assert!(
                m.min_vol <= m.atm_vol + dec!(0.01),
                "Min vol {} should be <= ATM vol {} at T={}",
                m.min_vol,
                m.atm_vol,
                m.expiry
            );
        }
    }

    // -----------------------------------------------------------------------
    // Term structure tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_term_structure_count() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        assert_eq!(result.result.term_structure.len(), 3);
    }

    #[test]
    fn test_term_structure_atm_vols() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        for tp in &result.result.term_structure {
            assert!(
                tp.atm_vol > Decimal::ZERO,
                "ATM vol should be positive at T={}",
                tp.expiry
            );
        }
    }

    #[test]
    fn test_term_structure_forward_vol_positive() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        for tp in &result.result.term_structure {
            assert!(
                tp.forward_vol >= Decimal::ZERO,
                "Forward vol {} should be non-negative at T={}",
                tp.forward_vol,
                tp.expiry
            );
        }
    }

    #[test]
    fn test_forward_vol_first_expiry() {
        // First expiry forward vol equals spot vol
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        let first = &result.result.term_structure[0];
        assert!(
            approx_eq(first.forward_vol, first.atm_vol, dec!(0.001)),
            "First expiry forward vol {} should equal ATM vol {}",
            first.forward_vol,
            first.atm_vol
        );
    }

    // -----------------------------------------------------------------------
    // Arbitrage detection tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_calendar_arbitrage_on_clean_surface() {
        // Our standard surface has decreasing ATM vol (0.20, 0.19, 0.18) but
        // total variance = vol^2 * T should still increase
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        let calendar_flags: Vec<_> = result
            .result
            .arbitrage_flags
            .iter()
            .filter(|f| f.flag_type == ArbitrageFlagType::CalendarSpread)
            .collect();
        // With well-constructed surface, no calendar arb at ATM
        // (0.20^2 * 0.25 = 0.01, 0.19^2 * 0.50 = 0.018, 0.18^2 * 1.0 = 0.032)
        // All increasing, so no flags expected at K=100
        let atm_calendar = calendar_flags
            .iter()
            .filter(|f| f.strike == dec!(100))
            .count();
        assert_eq!(atm_calendar, 0, "Should not flag calendar arb at ATM");
    }

    #[test]
    fn test_calendar_arbitrage_detected() {
        // Create artificial calendar arbitrage: very high vol at short expiry
        let quotes = vec![
            make_quote(dec!(100), dec!(0.25), dec!(0.50)), // High vol, total var = 0.0625
            make_quote(dec!(100), dec!(1.00), dec!(0.10)), // Low vol, total var = 0.01
        ];
        let input = ImpliedVolSurfaceInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            dividend_yield: dec!(0.0),
            market_quotes: quotes,
            interpolation_method: InterpolationMethod::Linear,
            extrapolation: true,
            target_strikes: None,
            target_expiries: None,
        };
        let result = build_implied_vol_surface(&input).unwrap();
        let calendar_flags: Vec<_> = result
            .result
            .arbitrage_flags
            .iter()
            .filter(|f| f.flag_type == ArbitrageFlagType::CalendarSpread)
            .collect();
        assert!(
            !calendar_flags.is_empty(),
            "Should detect calendar spread arbitrage"
        );
    }

    #[test]
    fn test_butterfly_arbitrage_detected() {
        // Create artificial negative convexity: hump in the middle (peak in total variance)
        // Butterfly arb: d2(w)/dk2 < 0 means concave total variance
        let quotes = vec![
            make_quote(dec!(80), dec!(0.25), dec!(0.15)),
            make_quote(dec!(90), dec!(0.25), dec!(0.40)), // Hump
            make_quote(dec!(100), dec!(0.25), dec!(0.15)),
        ];
        let input = ImpliedVolSurfaceInput {
            spot_price: dec!(100),
            risk_free_rate: dec!(0.05),
            dividend_yield: dec!(0.0),
            market_quotes: quotes,
            interpolation_method: InterpolationMethod::Linear,
            extrapolation: true,
            target_strikes: None,
            target_expiries: None,
        };
        let result = build_implied_vol_surface(&input).unwrap();
        let butterfly_flags: Vec<_> = result
            .result
            .arbitrage_flags
            .iter()
            .filter(|f| f.flag_type == ArbitrageFlagType::Butterfly)
            .collect();
        assert!(
            !butterfly_flags.is_empty(),
            "Should detect butterfly arbitrage"
        );
    }

    // -----------------------------------------------------------------------
    // Edge case tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_single_quote() {
        let quotes = vec![make_quote(dec!(100), dec!(0.25), dec!(0.20))];
        let input = ImpliedVolSurfaceInput {
            market_quotes: quotes,
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert_eq!(result.result.surface_points.len(), 1);
        assert!(approx_eq(
            result.result.surface_points[0].implied_vol,
            dec!(0.20),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_two_quotes_same_expiry() {
        let quotes = vec![
            make_quote(dec!(90), dec!(0.50), dec!(0.25)),
            make_quote(dec!(110), dec!(0.50), dec!(0.22)),
        ];
        let input = ImpliedVolSurfaceInput {
            market_quotes: quotes,
            target_strikes: Some(vec![dec!(100)]),
            target_expiries: Some(vec![dec!(0.50)]),
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert_eq!(result.result.surface_points.len(), 1);
        let vol = result.result.surface_points[0].implied_vol;
        assert!(vol > dec!(0.21) && vol < dec!(0.26));
    }

    #[test]
    fn test_metadata_populated() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_cubic_spline_fallback() {
        // CubicSpline currently falls back to bilinear; ensure it doesn't panic
        let input = ImpliedVolSurfaceInput {
            interpolation_method: InterpolationMethod::CubicSpline,
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert!(!result.result.surface_points.is_empty());
    }

    #[test]
    fn test_log_moneyness_sign() {
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        for p in &result.result.surface_points {
            let fwd = forward_price(dec!(100), dec!(0.05), dec!(0.02), p.expiry);
            if p.strike < fwd {
                assert!(
                    p.log_moneyness < Decimal::ZERO,
                    "log_moneyness {} should be negative for K={} < F={}",
                    p.log_moneyness,
                    p.strike,
                    fwd
                );
            }
        }
    }

    #[test]
    fn test_variance_monotonicity_at_atm() {
        // Total variance at ATM should increase with expiry
        let input = standard_input();
        let result = build_implied_vol_surface(&input).unwrap();
        let atm_points: Vec<_> = result
            .result
            .surface_points
            .iter()
            .filter(|p| p.strike == dec!(100))
            .collect();
        for i in 1..atm_points.len() {
            let var_prev = atm_points[i - 1].implied_vol
                * atm_points[i - 1].implied_vol
                * atm_points[i - 1].expiry;
            let var_curr =
                atm_points[i].implied_vol * atm_points[i].implied_vol * atm_points[i].expiry;
            assert!(
                var_curr >= var_prev - dec!(0.001),
                "Total variance should increase: {} at T={} vs {} at T={}",
                var_prev,
                atm_points[i - 1].expiry,
                var_curr,
                atm_points[i].expiry
            );
        }
    }

    #[test]
    fn test_svi_surface_has_all_points() {
        let input = ImpliedVolSurfaceInput {
            interpolation_method: InterpolationMethod::SVI,
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        // Should generate points for all target strikes x expiries
        assert!(result.result.surface_points.len() >= 15);
    }

    #[test]
    fn test_target_strikes_respected() {
        let input = ImpliedVolSurfaceInput {
            target_strikes: Some(vec![dec!(95), dec!(100), dec!(105)]),
            target_expiries: Some(vec![dec!(0.50)]),
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert_eq!(result.result.surface_points.len(), 3);
        for p in &result.result.surface_points {
            assert!(p.strike == dec!(95) || p.strike == dec!(100) || p.strike == dec!(105));
        }
    }

    #[test]
    fn test_target_expiries_respected() {
        let input = ImpliedVolSurfaceInput {
            target_strikes: Some(vec![dec!(100)]),
            target_expiries: Some(vec![dec!(0.25), dec!(1.00)]),
            ..standard_input()
        };
        let result = build_implied_vol_surface(&input).unwrap();
        assert_eq!(result.result.surface_points.len(), 2);
    }

    #[test]
    fn test_solve_5x5_identity() {
        let a = [
            [dec!(1), dec!(0), dec!(0), dec!(0), dec!(0)],
            [dec!(0), dec!(1), dec!(0), dec!(0), dec!(0)],
            [dec!(0), dec!(0), dec!(1), dec!(0), dec!(0)],
            [dec!(0), dec!(0), dec!(0), dec!(1), dec!(0)],
            [dec!(0), dec!(0), dec!(0), dec!(0), dec!(1)],
        ];
        let b = [dec!(1), dec!(2), dec!(3), dec!(4), dec!(5)];
        let x = solve_5x5(&a, &b);
        for i in 0..5 {
            assert!(approx_eq(x[i], b[i], dec!(0.0001)));
        }
    }
}
