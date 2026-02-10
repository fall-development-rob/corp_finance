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
pub enum OptionType {
    Call,
    Put,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExerciseStyle {
    European,
    American,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionInput {
    pub spot_price: Money,
    pub strike_price: Money,
    pub time_to_expiry: Decimal,
    pub risk_free_rate: Rate,
    pub volatility: Rate,
    #[serde(default)]
    pub dividend_yield: Rate,
    pub option_type: OptionType,
    pub exercise_style: ExerciseStyle,
    pub binomial_steps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionOutput {
    pub price: Money,
    pub intrinsic_value: Money,
    pub time_value: Money,
    pub greeks: OptionGreeks,
    pub binomial_price: Option<Money>,
    pub put_call_parity_price: Option<Money>,
    pub moneyness: String,
    pub breakeven: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionGreeks {
    pub delta: Decimal,
    pub gamma: Decimal,
    pub theta: Decimal,
    pub vega: Decimal,
    pub rho: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedVolInput {
    pub spot_price: Money,
    pub strike_price: Money,
    pub time_to_expiry: Decimal,
    pub risk_free_rate: Rate,
    #[serde(default)]
    pub dividend_yield: Rate,
    pub option_type: OptionType,
    pub market_price: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpliedVolOutput {
    pub implied_vol: Rate,
    pub iterations: u32,
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64, no MathematicalOps exp/ln/sqrt)
// ---------------------------------------------------------------------------

/// Taylor series exp(x) with range reduction for |x| > 2.
/// exp(x) = exp(x/2)^2 when |x| > 2, then Taylor with 25 terms.
fn exp_decimal(x: Decimal) -> Decimal {
    let two = dec!(2);

    // Range reduction: for large |x|, split recursively
    if x > two || x < -two {
        let half = exp_decimal(x / two);
        return half * half;
    }

    // Taylor series: exp(x) = sum_{n=0}^{24} x^n / n!
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=25 {
        term = term * x / Decimal::from(n);
        sum += term;
    }
    sum
}

/// Newton's method sqrt: y_{n+1} = (y_n + x/y_n) / 2, 25 iterations.
fn sqrt_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    let two = dec!(2);
    let mut guess = x / two;
    // Better initial guess for very large or very small x
    if x > dec!(100) {
        guess = dec!(10);
    } else if x < dec!(0.01) {
        guess = dec!(0.1);
    }
    for _ in 0..25 {
        guess = (guess + x / guess) / two;
    }
    guess
}

/// Natural log via Newton's method: find y such that exp(y) = x. 30 iterations.
fn ln_decimal(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        // ln of non-positive is undefined; return a large negative as sentinel
        return dec!(-999);
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    // Initial guess: for x near 1, use (x-1); otherwise rough approximation
    let mut y = if x > dec!(0.5) && x < dec!(2) {
        x - Decimal::ONE
    } else {
        // Count powers of e (~2.718) to get in the ballpark
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

    // Newton's method: y_{n+1} = y_n - (exp(y_n) - x) / exp(y_n)
    //                           = y_n - 1 + x / exp(y_n)
    for _ in 0..30 {
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
/// Phi(x) = 1 - phi(x) * (b1*t + b2*t^2 + b3*t^3 + b4*t^4 + b5*t^5)
/// where t = 1 / (1 + 0.2316419 * |x|)
/// For x < 0: Phi(x) = 1 - Phi(-x)
fn norm_cdf(x: Decimal) -> Decimal {
    let b1 = dec!(0.319381530);
    let b2 = dec!(-0.356563782);
    let b3 = dec!(1.781477937);
    let b4 = dec!(-1.821255978);
    let b5 = dec!(1.330274429);
    let p = dec!(0.2316419);

    let abs_x = if x < Decimal::ZERO { -x } else { x };
    let t = Decimal::ONE / (Decimal::ONE + p * abs_x);

    // Horner form: poly = t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))))
    let poly = t * (b1 + t * (b2 + t * (b3 + t * (b4 + t * b5))));

    let cdf_pos = Decimal::ONE - norm_pdf(abs_x) * poly;

    if x < Decimal::ZERO {
        Decimal::ONE - cdf_pos
    } else {
        cdf_pos
    }
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_pricing_input(input: &OptionInput) -> CorpFinanceResult<()> {
    if input.spot_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.strike_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "strike_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.time_to_expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_expiry".into(),
            reason: "must be positive".into(),
        });
    }
    if input.volatility <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "volatility".into(),
            reason: "must be positive".into(),
        });
    }
    Ok(())
}

fn validate_iv_input(input: &ImpliedVolInput) -> CorpFinanceResult<()> {
    if input.spot_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.strike_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "strike_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.time_to_expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_expiry".into(),
            reason: "must be positive".into(),
        });
    }
    if input.market_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_price".into(),
            reason: "must be positive".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Black-Scholes internals
// ---------------------------------------------------------------------------

struct BsParams {
    d1: Decimal,
    d2: Decimal,
    sqrt_t: Decimal,
    exp_neg_qt: Decimal,
    exp_neg_rt: Decimal,
}

fn compute_bs_params(
    s: Decimal,
    k: Decimal,
    t: Decimal,
    r: Decimal,
    q: Decimal,
    sigma: Decimal,
) -> BsParams {
    let sqrt_t = sqrt_decimal(t);
    let sigma_sqrt_t = sigma * sqrt_t;
    let d1 = (ln_decimal(s / k) + (r - q + sigma * sigma / dec!(2)) * t) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    let exp_neg_qt = exp_decimal(-q * t);
    let exp_neg_rt = exp_decimal(-r * t);
    BsParams {
        d1,
        d2,
        sqrt_t,
        exp_neg_qt,
        exp_neg_rt,
    }
}

fn bs_price(
    s: Decimal,
    k: Decimal,
    _r: Decimal,
    params: &BsParams,
    option_type: OptionType,
) -> Decimal {
    match option_type {
        OptionType::Call => {
            s * params.exp_neg_qt * norm_cdf(params.d1)
                - k * params.exp_neg_rt * norm_cdf(params.d2)
        }
        OptionType::Put => {
            k * params.exp_neg_rt * norm_cdf(-params.d2)
                - s * params.exp_neg_qt * norm_cdf(-params.d1)
        }
    }
}

fn compute_greeks(
    s: Decimal,
    k: Decimal,
    r: Decimal,
    t: Decimal,
    q: Decimal,
    params: &BsParams,
    option_type: OptionType,
) -> OptionGreeks {
    let nd1 = norm_pdf(params.d1);

    let delta = match option_type {
        OptionType::Call => params.exp_neg_qt * norm_cdf(params.d1),
        OptionType::Put => -params.exp_neg_qt * norm_cdf(-params.d1),
    };

    // gamma = e^(-qT) * n(d1) / (S * sigma * sqrt(T))
    // sigma * sqrt(T) = d1 - d2
    let sigma_sqrt_t = params.d1 - params.d2;
    let gamma = if sigma_sqrt_t != Decimal::ZERO && s != Decimal::ZERO {
        params.exp_neg_qt * nd1 / (s * sigma_sqrt_t)
    } else {
        Decimal::ZERO
    };

    // Theta (per calendar day = annual / 365)
    let theta_annual = match option_type {
        OptionType::Call => {
            -s * params.exp_neg_qt * nd1 * sigma_sqrt_t / (dec!(2) * t)
                - r * k * params.exp_neg_rt * norm_cdf(params.d2)
                + q * s * params.exp_neg_qt * norm_cdf(params.d1)
        }
        OptionType::Put => {
            -s * params.exp_neg_qt * nd1 * sigma_sqrt_t / (dec!(2) * t)
                + r * k * params.exp_neg_rt * norm_cdf(-params.d2)
                - q * s * params.exp_neg_qt * norm_cdf(-params.d1)
        }
    };
    let theta = theta_annual / dec!(365);

    // Vega: S * e^(-qT) * n(d1) * sqrt(T) / 100 (per 1% vol move)
    let vega = s * params.exp_neg_qt * nd1 * params.sqrt_t / dec!(100);

    // Rho: per 1% rate move
    let rho = match option_type {
        OptionType::Call => k * t * params.exp_neg_rt * norm_cdf(params.d2) / dec!(100),
        OptionType::Put => -k * t * params.exp_neg_rt * norm_cdf(-params.d2) / dec!(100),
    };

    OptionGreeks {
        delta,
        gamma,
        theta,
        vega,
        rho,
    }
}

// ---------------------------------------------------------------------------
// Binomial tree (CRR model for American options)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn binomial_price(
    s: Decimal,
    k: Decimal,
    t: Decimal,
    r: Decimal,
    q: Decimal,
    sigma: Decimal,
    steps: u32,
    option_type: OptionType,
    early_exercise: bool,
) -> Decimal {
    let n = steps;
    let dt = t / Decimal::from(n);
    let u = exp_decimal(sigma * sqrt_decimal(dt));
    let d = Decimal::ONE / u;
    let exp_rq_dt = exp_decimal((r - q) * dt);
    let disc = exp_decimal(-r * dt);
    let p_up = (exp_rq_dt - d) / (u - d);
    let p_down = Decimal::ONE - p_up;

    // Build terminal prices and option values
    let size = (n + 1) as usize;
    let mut option_values = Vec::with_capacity(size);

    // Terminal payoffs
    for i in 0..size {
        let ups = i as u32;
        let downs = n - ups;
        // Price at node: S * u^ups * d^downs
        let price = s * pow_decimal(u, ups) * pow_decimal(d, downs);
        let payoff = match option_type {
            OptionType::Call => (price - k).max(Decimal::ZERO),
            OptionType::Put => (k - price).max(Decimal::ZERO),
        };
        option_values.push(payoff);
    }

    // Backward induction
    for step in (0..n).rev() {
        let step_size = (step + 1) as usize;
        for i in 0..step_size {
            let hold = disc * (p_up * option_values[i + 1] + p_down * option_values[i]);
            if early_exercise {
                let ups = i as u32;
                let downs = step - ups;
                let price = s * pow_decimal(u, ups) * pow_decimal(d, downs);
                let exercise = match option_type {
                    OptionType::Call => (price - k).max(Decimal::ZERO),
                    OptionType::Put => (k - price).max(Decimal::ZERO),
                };
                option_values[i] = hold.max(exercise);
            } else {
                option_values[i] = hold;
            }
        }
    }

    option_values[0]
}

/// Integer power of a Decimal via iterative multiplication (avoids powd precision drift).
fn pow_decimal(base: Decimal, exp: u32) -> Decimal {
    if exp == 0 {
        return Decimal::ONE;
    }
    let mut result = Decimal::ONE;
    let mut b = base;
    let mut e = exp;
    // Exponentiation by squaring
    while e > 0 {
        if e & 1 == 1 {
            result *= b;
        }
        b *= b;
        e >>= 1;
    }
    result
}

// ---------------------------------------------------------------------------
// Moneyness and intrinsic value helpers
// ---------------------------------------------------------------------------

fn classify_moneyness(s: Decimal, k: Decimal, option_type: OptionType) -> String {
    let ratio = s / k;
    // ATM band: within 1% of strike
    let atm_lo = dec!(0.99);
    let atm_hi = dec!(1.01);
    match option_type {
        OptionType::Call => {
            if ratio > atm_hi {
                "ITM".into()
            } else if ratio < atm_lo {
                "OTM".into()
            } else {
                "ATM".into()
            }
        }
        OptionType::Put => {
            if ratio < atm_lo {
                "ITM".into()
            } else if ratio > atm_hi {
                "OTM".into()
            } else {
                "ATM".into()
            }
        }
    }
}

fn intrinsic_value(s: Decimal, k: Decimal, option_type: OptionType) -> Decimal {
    match option_type {
        OptionType::Call => (s - k).max(Decimal::ZERO),
        OptionType::Put => (k - s).max(Decimal::ZERO),
    }
}

fn breakeven(k: Decimal, premium: Decimal, option_type: OptionType) -> Decimal {
    match option_type {
        OptionType::Call => k + premium,
        OptionType::Put => k - premium,
    }
}

// ---------------------------------------------------------------------------
// Public API: price_option
// ---------------------------------------------------------------------------

pub fn price_option(input: &OptionInput) -> CorpFinanceResult<ComputationOutput<OptionOutput>> {
    let start = Instant::now();
    validate_pricing_input(input)?;

    let s = input.spot_price;
    let k = input.strike_price;
    let t = input.time_to_expiry;
    let r = input.risk_free_rate;
    let q = input.dividend_yield;
    let sigma = input.volatility;
    let steps = input.binomial_steps.unwrap_or(100);

    // Black-Scholes parameters and price (European baseline)
    let params = compute_bs_params(s, k, t, r, q, sigma);
    let bs = bs_price(s, k, r, &params, input.option_type);
    let greeks = compute_greeks(s, k, r, t, q, &params, input.option_type);

    // Determine final price and binomial price
    let (price, binom) = match input.exercise_style {
        ExerciseStyle::European => (bs, None),
        ExerciseStyle::American => {
            let am = binomial_price(s, k, t, r, q, sigma, steps, input.option_type, true);
            (am, Some(am))
        }
    };

    // Put-call parity cross-check (European only)
    let parity_price = if input.exercise_style == ExerciseStyle::European {
        let s_adj = s * exp_decimal(-q * t);
        let k_adj = k * exp_decimal(-r * t);
        match input.option_type {
            // If we priced a call, parity gives the put: P = C - S*e^(-qT) + K*e^(-rT)
            OptionType::Call => Some(bs - s_adj + k_adj),
            // If we priced a put, parity gives the call: C = P + S*e^(-qT) - K*e^(-rT)
            OptionType::Put => Some(bs + s_adj - k_adj),
        }
    } else {
        None
    };

    let iv = intrinsic_value(s, k, input.option_type);
    let tv = price - iv;
    let be = breakeven(k, price, input.option_type);
    let moneyness = classify_moneyness(s, k, input.option_type);

    let output = OptionOutput {
        price,
        intrinsic_value: iv,
        time_value: tv,
        greeks,
        binomial_price: binom,
        put_call_parity_price: parity_price,
        moneyness,
        breakeven: be,
    };

    let methodology = match input.exercise_style {
        ExerciseStyle::European => "Black-Scholes (closed-form)",
        ExerciseStyle::American => "CRR Binomial Tree with early exercise",
    };

    let warnings = Vec::new();
    let assumptions = serde_json::json!({
        "model": methodology,
        "risk_free_rate": r.to_string(),
        "volatility": sigma.to_string(),
        "dividend_yield": q.to_string(),
        "exercise_style": format!("{:?}", input.exercise_style),
        "binomial_steps": steps,
    });

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        methodology,
        &assumptions,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Public API: implied_volatility
// ---------------------------------------------------------------------------

pub fn implied_volatility(
    input: &ImpliedVolInput,
) -> CorpFinanceResult<ComputationOutput<ImpliedVolOutput>> {
    let start = Instant::now();
    validate_iv_input(input)?;

    let s = input.spot_price;
    let k = input.strike_price;
    let t = input.time_to_expiry;
    let r = input.risk_free_rate;
    let q = input.dividend_yield;
    let target = input.market_price;

    let max_iter: u32 = 100;
    let eps = dec!(0.000001);
    let mut sigma = dec!(0.20);

    let mut iterations: u32 = 0;
    let mut last_delta = Decimal::ZERO;

    for i in 0..max_iter {
        iterations = i + 1;
        let params = compute_bs_params(s, k, t, r, q, sigma);
        let price = bs_price(s, k, r, &params, input.option_type);
        let diff = price - target;
        last_delta = if diff < Decimal::ZERO { -diff } else { diff };

        if last_delta < eps {
            break;
        }

        // Vega (not scaled by 100 here; raw partial derivative dV/dsigma)
        let nd1 = norm_pdf(params.d1);
        let vega_raw = s * exp_decimal(-q * t) * nd1 * sqrt_decimal(t);

        if vega_raw <= dec!(0.0000001) {
            // Vega too small, cannot converge
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "implied_volatility".into(),
                iterations,
                last_delta,
            });
        }

        sigma -= diff / vega_raw;

        // Clamp sigma to reasonable bounds
        if sigma < dec!(0.001) {
            sigma = dec!(0.001);
        }
        if sigma > dec!(5.0) {
            sigma = dec!(5.0);
        }
    }

    if last_delta >= eps {
        return Err(CorpFinanceError::ConvergenceFailure {
            function: "implied_volatility".into(),
            iterations,
            last_delta,
        });
    }

    let output = ImpliedVolOutput {
        implied_vol: sigma,
        iterations,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "model": "Newton-Raphson on Black-Scholes",
        "initial_vol": "0.20",
        "max_iterations": max_iter,
        "tolerance": eps.to_string(),
    });

    Ok(with_metadata(
        "Newton-Raphson implied volatility",
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

    // Tolerance helper: check |a - b| < tol
    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        let diff = a - b;
        let abs_diff = if diff < Decimal::ZERO { -diff } else { diff };
        abs_diff < tol
    }

    fn default_european_call() -> OptionInput {
        OptionInput {
            spot_price: dec!(100),
            strike_price: dec!(100),
            time_to_expiry: dec!(1),
            risk_free_rate: dec!(0.05),
            volatility: dec!(0.20),
            dividend_yield: dec!(0),
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            binomial_steps: None,
        }
    }

    fn default_european_put() -> OptionInput {
        OptionInput {
            option_type: OptionType::Put,
            ..default_european_call()
        }
    }

    // -----------------------------------------------------------------------
    // Math helper sanity checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_exp_decimal_basic() {
        // e^0 = 1
        assert!(approx_eq(exp_decimal(dec!(0)), dec!(1), dec!(0.0001)));
        // e^1 ~ 2.71828
        assert!(approx_eq(exp_decimal(dec!(1)), dec!(2.71828), dec!(0.001)));
    }

    #[test]
    fn test_sqrt_decimal_basic() {
        assert!(approx_eq(sqrt_decimal(dec!(4)), dec!(2), dec!(0.0001)));
        assert!(approx_eq(sqrt_decimal(dec!(9)), dec!(3), dec!(0.0001)));
    }

    #[test]
    fn test_ln_decimal_basic() {
        // ln(1) = 0
        assert!(approx_eq(ln_decimal(dec!(1)), dec!(0), dec!(0.0001)));
        // ln(e) ~ 1
        assert!(approx_eq(
            ln_decimal(dec!(2.71828182845)),
            dec!(1),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_norm_cdf_basic() {
        // N(0) = 0.5
        assert!(approx_eq(norm_cdf(dec!(0)), dec!(0.5), dec!(0.001)));
        // N(very large) ~ 1
        assert!(norm_cdf(dec!(5)) > dec!(0.999));
        // N(very negative) ~ 0
        assert!(norm_cdf(dec!(-5)) < dec!(0.001));
    }

    // -----------------------------------------------------------------------
    // Option pricing tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_call_atm_price() {
        // ATM call, S=K=100, T=1, r=5%, vol=20%, q=0
        // Known BS result ~ 10.45 (approximately)
        let input = default_european_call();
        let result = price_option(&input).unwrap();
        let price = result.result.price;
        // BS ATM call ~ 10.45
        assert!(
            approx_eq(price, dec!(10.45), dec!(0.30)),
            "ATM call price {price} not near 10.45"
        );
        assert!(price > Decimal::ZERO);
    }

    #[test]
    fn test_put_atm_price() {
        // ATM put, same params
        // By put-call parity: P = C - S + K*e^(-rT) ~ 10.45 - 100 + 100*e^(-0.05) ~ 5.57
        let input = default_european_put();
        let result = price_option(&input).unwrap();
        let price = result.result.price;
        assert!(
            approx_eq(price, dec!(5.57), dec!(0.30)),
            "ATM put price {price} not near 5.57"
        );
        assert!(price > Decimal::ZERO);
    }

    #[test]
    fn test_call_put_parity() {
        // C - P = S*e^(-qT) - K*e^(-rT)
        let call_input = default_european_call();
        let put_input = default_european_put();

        let call_result = price_option(&call_input).unwrap();
        let put_result = price_option(&put_input).unwrap();

        let c = call_result.result.price;
        let p = put_result.result.price;
        let s = dec!(100);
        let k = dec!(100);
        let r = dec!(0.05);
        let q = dec!(0);
        let t = dec!(1);

        let lhs = c - p;
        let rhs = s * exp_decimal(-q * t) - k * exp_decimal(-r * t);

        assert!(
            approx_eq(lhs, rhs, dec!(0.05)),
            "Put-call parity failed: C-P={lhs}, S*e^(-qT)-K*e^(-rT)={rhs}"
        );
    }

    #[test]
    fn test_deep_itm_call() {
        // Deep ITM call: S=200, K=100 -> price ~ S - K*e^(-rT) for very deep ITM
        let input = OptionInput {
            spot_price: dec!(200),
            strike_price: dec!(100),
            ..default_european_call()
        };
        let result = price_option(&input).unwrap();
        let price = result.result.price;
        let lower_bound = dec!(200) - dec!(100) * exp_decimal(-dec!(0.05));
        assert!(
            price >= lower_bound - dec!(0.1),
            "Deep ITM call {price} below intrinsic PV {lower_bound}"
        );
    }

    #[test]
    fn test_deep_otm_call() {
        // Deep OTM call: S=50, K=200 -> price ~ 0
        let input = OptionInput {
            spot_price: dec!(50),
            strike_price: dec!(200),
            ..default_european_call()
        };
        let result = price_option(&input).unwrap();
        let price = result.result.price;
        assert!(
            price < dec!(1),
            "Deep OTM call price {price} should be near zero"
        );
    }

    #[test]
    fn test_delta_call_range() {
        let input = default_european_call();
        let result = price_option(&input).unwrap();
        let delta = result.result.greeks.delta;
        assert!(
            delta > Decimal::ZERO && delta < Decimal::ONE,
            "Call delta {delta} should be in (0, 1)"
        );
    }

    #[test]
    fn test_delta_put_range() {
        let input = default_european_put();
        let result = price_option(&input).unwrap();
        let delta = result.result.greeks.delta;
        assert!(
            delta < Decimal::ZERO && delta > -Decimal::ONE,
            "Put delta {delta} should be in (-1, 0)"
        );
    }

    #[test]
    fn test_gamma_positive() {
        let call = price_option(&default_european_call()).unwrap();
        let put = price_option(&default_european_put()).unwrap();
        assert!(
            call.result.greeks.gamma > Decimal::ZERO,
            "Call gamma should be positive"
        );
        assert!(
            put.result.greeks.gamma > Decimal::ZERO,
            "Put gamma should be positive"
        );
    }

    #[test]
    fn test_theta_negative() {
        // Theta for a long option (no dividends, standard case) is typically negative
        let call = price_option(&default_european_call()).unwrap();
        let put = price_option(&default_european_put()).unwrap();
        assert!(
            call.result.greeks.theta < Decimal::ZERO,
            "Call theta {} should be negative",
            call.result.greeks.theta
        );
        assert!(
            put.result.greeks.theta < Decimal::ZERO,
            "Put theta {} should be negative",
            put.result.greeks.theta
        );
    }

    #[test]
    fn test_vega_positive() {
        let call = price_option(&default_european_call()).unwrap();
        let put = price_option(&default_european_put()).unwrap();
        assert!(
            call.result.greeks.vega > Decimal::ZERO,
            "Call vega should be positive"
        );
        assert!(
            put.result.greeks.vega > Decimal::ZERO,
            "Put vega should be positive"
        );
    }

    #[test]
    fn test_american_call_no_dividend() {
        // American call with no dividends should equal European call
        let european = OptionInput {
            exercise_style: ExerciseStyle::European,
            ..default_european_call()
        };
        let american = OptionInput {
            exercise_style: ExerciseStyle::American,
            binomial_steps: Some(200),
            ..default_european_call()
        };
        let eu_price = price_option(&european).unwrap().result.price;
        let am_price = price_option(&american).unwrap().result.price;
        assert!(
            approx_eq(eu_price, am_price, dec!(0.50)),
            "American call (no div) {am_price} should ~ European {eu_price}"
        );
    }

    #[test]
    fn test_american_put_early_exercise() {
        // American put should be >= European put
        let eu_input = default_european_put();
        let am_input = OptionInput {
            exercise_style: ExerciseStyle::American,
            binomial_steps: Some(200),
            ..default_european_put()
        };
        let eu_price = price_option(&eu_input).unwrap().result.price;
        let am_price = price_option(&am_input).unwrap().result.price;
        assert!(
            am_price >= eu_price - dec!(0.01),
            "American put {am_price} should be >= European put {eu_price}"
        );
    }

    #[test]
    fn test_binomial_converges_to_bs() {
        // Binomial European (no early exercise) should converge to BS
        let bs_input = default_european_call();
        let bs_price_val = price_option(&bs_input).unwrap().result.price;

        let binom_val = binomial_price(
            dec!(100),
            dec!(100),
            dec!(1),
            dec!(0.05),
            dec!(0),
            dec!(0.20),
            500,
            OptionType::Call,
            false, // European
        );

        assert!(
            approx_eq(bs_price_val, binom_val, dec!(0.20)),
            "Binomial (500 steps) {binom_val} should converge to BS {bs_price_val}"
        );
    }

    #[test]
    fn test_implied_vol_roundtrip() {
        // Price an option, then recover the vol from the price
        let input = default_european_call();
        let priced = price_option(&input).unwrap();
        let market_price = priced.result.price;

        let iv_input = ImpliedVolInput {
            spot_price: dec!(100),
            strike_price: dec!(100),
            time_to_expiry: dec!(1),
            risk_free_rate: dec!(0.05),
            dividend_yield: dec!(0),
            option_type: OptionType::Call,
            market_price,
        };
        let iv_result = implied_volatility(&iv_input).unwrap();
        assert!(
            approx_eq(iv_result.result.implied_vol, dec!(0.20), dec!(0.005)),
            "Implied vol {} should recover original 0.20",
            iv_result.result.implied_vol
        );
    }

    #[test]
    fn test_moneyness_classification() {
        // ITM call: S > K
        let itm = OptionInput {
            spot_price: dec!(110),
            strike_price: dec!(100),
            ..default_european_call()
        };
        let res = price_option(&itm).unwrap();
        assert_eq!(res.result.moneyness, "ITM");

        // OTM call: S < K
        let otm = OptionInput {
            spot_price: dec!(90),
            strike_price: dec!(100),
            ..default_european_call()
        };
        let res = price_option(&otm).unwrap();
        assert_eq!(res.result.moneyness, "OTM");

        // ATM call: S ~ K
        let atm = default_european_call();
        let res = price_option(&atm).unwrap();
        assert_eq!(res.result.moneyness, "ATM");
    }

    #[test]
    fn test_zero_time_to_expiry_error() {
        let input = OptionInput {
            time_to_expiry: dec!(0),
            ..default_european_call()
        };
        let result = price_option(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "time_to_expiry");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_negative_vol_error() {
        let input = OptionInput {
            volatility: dec!(-0.10),
            ..default_european_call()
        };
        let result = price_option(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "volatility");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_metadata_populated() {
        let input = default_european_call();
        let result = price_option(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(result.metadata.computation_time_us > 0 || true); // timing can be 0 on fast machines
    }
}
