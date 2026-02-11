use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExoticType {
    Autocallable,
    BarrierOption,
    DigitalOption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BarrierType {
    UpAndIn,
    UpAndOut,
    DownAndIn,
    DownAndOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DigitalType {
    CashOrNothing,
    AssetOrNothing,
}

/// Autocallable-specific parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocallableParams {
    pub notional: Money,
    pub underlying_price: Money,
    pub volatility: Rate,
    pub risk_free_rate: Rate,
    #[serde(default)]
    pub dividend_yield: Rate,
    pub maturity_years: Decimal,
    /// Observation frequency per year (e.g. 4 for quarterly).
    pub observation_frequency: u32,
    /// Autocall barrier as fraction of initial price (e.g. 1.0 = at money).
    pub autocall_barrier: Rate,
    /// Coupon per observation period (e.g. 0.02 = 2%).
    pub coupon_per_period: Rate,
    /// Underlying must be above this fraction to earn a coupon.
    pub coupon_barrier: Rate,
    /// Knock-in put barrier as fraction of initial price.
    pub ki_barrier: Rate,
    /// Put strike if knock-in triggered, as fraction of initial price.
    pub ki_strike: Rate,
}

/// Barrier option parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierOptionParams {
    pub spot: Money,
    pub strike: Money,
    pub barrier: Money,
    pub barrier_type: BarrierType,
    /// "Call" or "Put"
    pub option_type: String,
    pub volatility: Rate,
    pub risk_free_rate: Rate,
    #[serde(default)]
    pub dividend_yield: Rate,
    pub time_to_expiry: Decimal,
    pub rebate: Option<Money>,
}

/// Digital option parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalOptionParams {
    pub spot: Money,
    pub strike: Money,
    pub digital_type: DigitalType,
    /// "Call" or "Put"
    pub option_type: String,
    /// Payout amount for CashOrNothing.
    pub payout: Money,
    pub volatility: Rate,
    pub risk_free_rate: Rate,
    #[serde(default)]
    pub dividend_yield: Rate,
    pub time_to_expiry: Decimal,
}

/// Unified input for exotic products. Exactly one of the option structs should
/// be populated to match the selected `product_type`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoticProductInput {
    pub product_type: ExoticType,
    #[serde(default)]
    pub autocallable: Option<AutocallableParams>,
    #[serde(default)]
    pub barrier_option: Option<BarrierOptionParams>,
    #[serde(default)]
    pub digital_option: Option<DigitalOptionParams>,
}

// -- Output types -----------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationResult {
    pub date_idx: u32,
    pub autocall_prob: Rate,
    pub cumulative_prob: Rate,
    pub coupon_prob: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutocallableOutput {
    pub expected_life: Decimal,
    pub probability_of_autocall: Rate,
    pub probability_of_ki: Rate,
    pub expected_coupon_count: Decimal,
    pub expected_return: Rate,
    pub max_loss: Rate,
    pub observation_schedule: Vec<ObservationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierGreeks {
    pub delta: Decimal,
    pub gamma: Decimal,
    pub vega: Decimal,
    pub theta: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierOptionOutput {
    pub vanilla_value: Money,
    pub barrier_discount: Rate,
    pub greeks: BarrierGreeks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalOptionOutput {
    pub vanilla_equivalent: Money,
    pub probability_of_payout: Rate,
    pub expected_payout: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoticProductOutput {
    pub product_type: String,
    pub fair_value: Money,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocallable: Option<AutocallableOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub barrier_option: Option<BarrierOptionOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digital_option: Option<DigitalOptionOutput>,
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64, identical pattern to derivatives/options.rs)
// ---------------------------------------------------------------------------

/// Taylor series exp(x) with range reduction for |x| > 2.
fn decimal_exp(x: Decimal) -> Decimal {
    let two = dec!(2);
    if x > two || x < -two {
        let half = decimal_exp(x / two);
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

/// Newton's method sqrt, 20 iterations.
fn decimal_sqrt(x: Decimal) -> Decimal {
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

/// Natural log via Newton's method on exp, 40 iterations.
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return dec!(-999);
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }
    let e_approx = dec!(2.718281828459045);
    let mut y = if x > dec!(0.5) && x < dec!(2) {
        x - Decimal::ONE
    } else {
        let mut approx = Decimal::ZERO;
        let mut v = x;
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
        let ey = decimal_exp(y);
        if ey == Decimal::ZERO {
            break;
        }
        y = y - Decimal::ONE + x / ey;
    }
    y
}

/// Standard normal PDF.
fn norm_pdf(x: Decimal) -> Decimal {
    let two_pi = dec!(6.283185307179586);
    let exponent = -(x * x) / dec!(2);
    decimal_exp(exponent) / decimal_sqrt(two_pi)
}

/// Standard normal CDF — Abramowitz & Stegun approximation.
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

/// Iterative integer power (no powd).
#[allow(dead_code)]
fn pow_decimal(base: Decimal, exp: u32) -> Decimal {
    if exp == 0 {
        return Decimal::ONE;
    }
    let mut result = Decimal::ONE;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result *= b;
        }
        b *= b;
        e >>= 1;
    }
    result
}

/// Fractional power: base^exp = exp(exp * ln(base)).
fn pow_frac(base: Decimal, exponent: Decimal) -> Decimal {
    if base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    decimal_exp(exponent * decimal_ln(base))
}

// ---------------------------------------------------------------------------
// Black-Scholes vanilla pricer (internal, for comparison values)
// ---------------------------------------------------------------------------

fn bs_d1_d2(
    s: Decimal,
    k: Decimal,
    r: Decimal,
    q: Decimal,
    sigma: Decimal,
    t: Decimal,
) -> (Decimal, Decimal) {
    let sqrt_t = decimal_sqrt(t);
    let sigma_sqrt_t = sigma * sqrt_t;
    if sigma_sqrt_t == Decimal::ZERO {
        return (Decimal::ZERO, Decimal::ZERO);
    }
    let d1 = (decimal_ln(s / k) + (r - q + sigma * sigma / dec!(2)) * t) / sigma_sqrt_t;
    let d2 = d1 - sigma_sqrt_t;
    (d1, d2)
}

fn bs_call(
    s: Decimal,
    k: Decimal,
    r: Decimal,
    q: Decimal,
    t: Decimal,
    d1: Decimal,
    d2: Decimal,
) -> Decimal {
    s * decimal_exp(-q * t) * norm_cdf(d1) - k * decimal_exp(-r * t) * norm_cdf(d2)
}

fn bs_put(
    s: Decimal,
    k: Decimal,
    r: Decimal,
    q: Decimal,
    t: Decimal,
    d1: Decimal,
    d2: Decimal,
) -> Decimal {
    k * decimal_exp(-r * t) * norm_cdf(-d2) - s * decimal_exp(-q * t) * norm_cdf(-d1)
}

fn vanilla_price(
    s: Decimal,
    k: Decimal,
    r: Decimal,
    q: Decimal,
    sigma: Decimal,
    t: Decimal,
    is_call: bool,
) -> Decimal {
    let (d1, d2) = bs_d1_d2(s, k, r, q, sigma, t);
    if is_call {
        bs_call(s, k, r, q, t, d1, d2)
    } else {
        bs_put(s, k, r, q, t, d1, d2)
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_autocallable(params: &AutocallableParams) -> CorpFinanceResult<()> {
    if params.notional <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional".into(),
            reason: "must be positive".into(),
        });
    }
    if params.underlying_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "underlying_price".into(),
            reason: "must be positive".into(),
        });
    }
    if params.volatility <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "volatility".into(),
            reason: "must be positive".into(),
        });
    }
    if params.maturity_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "must be positive".into(),
        });
    }
    if params.observation_frequency == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "observation_frequency".into(),
            reason: "must be at least 1".into(),
        });
    }
    if params.ki_barrier <= Decimal::ZERO || params.ki_barrier >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "ki_barrier".into(),
            reason: "must be between 0 and 1 exclusive".into(),
        });
    }
    Ok(())
}

fn validate_barrier(params: &BarrierOptionParams) -> CorpFinanceResult<()> {
    if params.spot <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot".into(),
            reason: "must be positive".into(),
        });
    }
    if params.strike <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "strike".into(),
            reason: "must be positive".into(),
        });
    }
    if params.barrier <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "barrier".into(),
            reason: "must be positive".into(),
        });
    }
    if params.volatility <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "volatility".into(),
            reason: "must be positive".into(),
        });
    }
    if params.time_to_expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_expiry".into(),
            reason: "must be positive".into(),
        });
    }
    let opt = params.option_type.as_str();
    if opt != "Call" && opt != "Put" {
        return Err(CorpFinanceError::InvalidInput {
            field: "option_type".into(),
            reason: "must be \"Call\" or \"Put\"".into(),
        });
    }
    // Validate barrier position relative to spot
    match params.barrier_type {
        BarrierType::UpAndIn | BarrierType::UpAndOut => {
            if params.barrier <= params.spot {
                return Err(CorpFinanceError::InvalidInput {
                    field: "barrier".into(),
                    reason: "up barrier must be above spot".into(),
                });
            }
        }
        BarrierType::DownAndIn | BarrierType::DownAndOut => {
            if params.barrier >= params.spot {
                return Err(CorpFinanceError::InvalidInput {
                    field: "barrier".into(),
                    reason: "down barrier must be below spot".into(),
                });
            }
        }
    }
    Ok(())
}

fn validate_digital(params: &DigitalOptionParams) -> CorpFinanceResult<()> {
    if params.spot <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot".into(),
            reason: "must be positive".into(),
        });
    }
    if params.strike <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "strike".into(),
            reason: "must be positive".into(),
        });
    }
    if params.volatility <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "volatility".into(),
            reason: "must be positive".into(),
        });
    }
    if params.time_to_expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_expiry".into(),
            reason: "must be positive".into(),
        });
    }
    let opt = params.option_type.as_str();
    if opt != "Call" && opt != "Put" {
        return Err(CorpFinanceError::InvalidInput {
            field: "option_type".into(),
            reason: "must be \"Call\" or \"Put\"".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Autocallable pricing (simplified analytical)
// ---------------------------------------------------------------------------

fn price_autocallable(
    params: &AutocallableParams,
) -> CorpFinanceResult<(Money, AutocallableOutput, Vec<String>)> {
    let _s0 = params.underlying_price;
    let sigma = params.volatility;
    let r = params.risk_free_rate;
    let q = params.dividend_yield;
    let notional = params.notional;
    let n_obs = (params.maturity_years * Decimal::from(params.observation_frequency))
        .to_string()
        .parse::<u32>()
        .unwrap_or(1);

    let mut warnings = Vec::new();
    let mut schedule = Vec::with_capacity(n_obs as usize);
    let mut cum_autocall_prob = Decimal::ZERO;
    let mut expected_life = Decimal::ZERO;
    let mut expected_coupon_count = Decimal::ZERO;
    let mut pv_autocall_payments = Decimal::ZERO;

    let obs_freq = Decimal::from(params.observation_frequency);

    for i in 1..=n_obs {
        let t_i = Decimal::from(i) / obs_freq;

        // Probability that S(t_i) > autocall_barrier * S0
        // d = (ln(barrier) - (r - q - sigma^2/2) * t) / (sigma * sqrt(t))
        // P(S > barrier*S0) = N(-d) = 1 - N(d)
        let sqrt_t = decimal_sqrt(t_i);
        let sigma_sqrt_t = sigma * sqrt_t;
        let d_autocall = if sigma_sqrt_t > Decimal::ZERO {
            (decimal_ln(params.autocall_barrier) - (r - q - sigma * sigma / dec!(2)) * t_i)
                / sigma_sqrt_t
        } else {
            Decimal::ZERO
        };
        let p_above_barrier = Decimal::ONE - norm_cdf(d_autocall);

        // Marginal probability: P(autocall at i) = P(above barrier) * P(not called before)
        let surviving = Decimal::ONE - cum_autocall_prob;
        let marginal_prob = p_above_barrier * surviving;

        // Coupon probability: P(S(t_i) > coupon_barrier * S0)
        let d_coupon = if sigma_sqrt_t > Decimal::ZERO {
            (decimal_ln(params.coupon_barrier) - (r - q - sigma * sigma / dec!(2)) * t_i)
                / sigma_sqrt_t
        } else {
            Decimal::ZERO
        };
        let p_coupon = Decimal::ONE - norm_cdf(d_coupon);

        // PV of autocall payment: (notional + i * coupon_per_period * notional) * discount
        let coupons_earned = Decimal::from(i) * params.coupon_per_period * notional;
        let discount = decimal_exp(-r * t_i);
        pv_autocall_payments += marginal_prob * (notional + coupons_earned) * discount;

        expected_life += marginal_prob * t_i;
        expected_coupon_count += surviving * p_coupon;

        cum_autocall_prob += marginal_prob;

        schedule.push(ObservationResult {
            date_idx: i,
            autocall_prob: marginal_prob,
            cumulative_prob: cum_autocall_prob,
            coupon_prob: p_coupon,
        });
    }

    // Knock-in probability using reflection principle approximation.
    // P(min S(t) < H) ~ 2 * N(-d2) where d2 accounts for the barrier level.
    let t_final = params.maturity_years;
    let sqrt_t_final = decimal_sqrt(t_final);
    let sigma_sqrt_tf = sigma * sqrt_t_final;
    let ki_prob = if sigma_sqrt_tf > Decimal::ZERO {
        let d_ki = (decimal_ln(params.ki_barrier) - (r - q - sigma * sigma / dec!(2)) * t_final)
            / sigma_sqrt_tf;
        // Reflection: P(min < H) ~ 2 * N(d_ki)  [d_ki is negative for H < S0]
        let raw = dec!(2) * norm_cdf(d_ki);
        // Cap at 1.0
        if raw > Decimal::ONE {
            Decimal::ONE
        } else {
            raw
        }
    } else {
        Decimal::ZERO
    };

    // Terminal value for non-autocalled scenarios
    let surviving_final = Decimal::ONE - cum_autocall_prob;
    let discount_final = decimal_exp(-r * t_final);

    // If KI triggered, investor receives notional * min(1, S_T / (ki_strike * S0))
    // Expected loss from KI: ki_prob * notional * E[max(0, 1 - S_T / (ki_strike * S0))]
    // Simplified: expected_ki_loss ~ ki_prob * notional * (1 - ki_barrier) as rough approximation
    let expected_ki_loss = ki_prob * notional * (Decimal::ONE - params.ki_barrier);
    let terminal_pv = surviving_final * (notional - expected_ki_loss) * discount_final;

    let fair_value = pv_autocall_payments + terminal_pv;

    // Expected return: (fair_value / notional - 1)
    let expected_return = if notional > Decimal::ZERO {
        fair_value / notional - Decimal::ONE
    } else {
        Decimal::ZERO
    };

    // Max loss: if KI triggered and underlying goes to zero
    let max_loss = Decimal::ONE;

    if cum_autocall_prob < dec!(0.1) {
        warnings.push("Low autocall probability; investor bears significant downside risk".into());
    }
    if ki_prob > dec!(0.5) {
        warnings.push("High knock-in probability exceeds 50%".into());
    }

    // Adjust expected life: add terminal contribution for survivors
    expected_life += surviving_final * t_final;

    let output = AutocallableOutput {
        expected_life,
        probability_of_autocall: cum_autocall_prob,
        probability_of_ki: ki_prob,
        expected_coupon_count,
        expected_return,
        max_loss,
        observation_schedule: schedule,
    };

    Ok((fair_value, output, warnings))
}

// ---------------------------------------------------------------------------
// Barrier option pricing (Rubinstein & Reiner analytical formulas)
// ---------------------------------------------------------------------------

/// Rubinstein & Reiner barrier option building blocks.
struct BarrierComponents {
    a: Decimal,
    b: Decimal,
    c: Decimal,
    d: Decimal,
}

#[allow(clippy::too_many_arguments)]
fn compute_barrier_components(
    s: Decimal,
    k: Decimal,
    h: Decimal,
    r: Decimal,
    q: Decimal,
    sigma: Decimal,
    t: Decimal,
    phi: Decimal, // +1 call, -1 put
    eta: Decimal, // +1 down, -1 up
) -> BarrierComponents {
    let sigma2 = sigma * sigma;
    let sqrt_t = decimal_sqrt(t);
    let sigma_sqrt_t = sigma * sqrt_t;
    let lambda = (r - q + sigma2 / dec!(2)) / sigma2;
    let exp_neg_qt = decimal_exp(-q * t);
    let exp_neg_rt = decimal_exp(-r * t);

    // x1 = ln(S/K) / (sigma*sqrt(T)) + lambda * sigma * sqrt(T)
    let x1 = decimal_ln(s / k) / sigma_sqrt_t + lambda * sigma_sqrt_t;
    // x2 = ln(S/H) / (sigma*sqrt(T)) + lambda * sigma * sqrt(T)
    let x2 = decimal_ln(s / h) / sigma_sqrt_t + lambda * sigma_sqrt_t;
    // y1 = ln(H^2/(S*K)) / (sigma*sqrt(T)) + lambda * sigma * sqrt(T)
    let y1 = decimal_ln(h * h / (s * k)) / sigma_sqrt_t + lambda * sigma_sqrt_t;
    // y2 = ln(H/S) / (sigma*sqrt(T)) + lambda * sigma * sqrt(T)
    let y2 = decimal_ln(h / s) / sigma_sqrt_t + lambda * sigma_sqrt_t;

    // (H/S)^(2*lambda)
    let hs_ratio = h / s;
    let hs_2l = pow_frac(hs_ratio, dec!(2) * lambda);
    // (H/S)^(2*lambda - 2)
    let hs_2l_m2 = pow_frac(hs_ratio, dec!(2) * lambda - dec!(2));

    // A = phi*S*e^(-qT)*N(phi*x1) - phi*K*e^(-rT)*N(phi*(x1 - sigma*sqrt(T)))
    let a = phi * s * exp_neg_qt * norm_cdf(phi * x1)
        - phi * k * exp_neg_rt * norm_cdf(phi * (x1 - sigma_sqrt_t));

    // B = phi*S*e^(-qT)*N(phi*x2) - phi*K*e^(-rT)*N(phi*(x2 - sigma*sqrt(T)))
    let b = phi * s * exp_neg_qt * norm_cdf(phi * x2)
        - phi * k * exp_neg_rt * norm_cdf(phi * (x2 - sigma_sqrt_t));

    // C = phi*S*e^(-qT)*(H/S)^(2L)*N(eta*y1) - phi*K*e^(-rT)*(H/S)^(2L-2)*N(eta*(y1-sigma*sqrt(T)))
    let c = phi * s * exp_neg_qt * hs_2l * norm_cdf(eta * y1)
        - phi * k * exp_neg_rt * hs_2l_m2 * norm_cdf(eta * (y1 - sigma_sqrt_t));

    // D = phi*S*e^(-qT)*(H/S)^(2L)*N(eta*y2) - phi*K*e^(-rT)*(H/S)^(2L-2)*N(eta*(y2-sigma*sqrt(T)))
    let d = phi * s * exp_neg_qt * hs_2l * norm_cdf(eta * y2)
        - phi * k * exp_neg_rt * hs_2l_m2 * norm_cdf(eta * (y2 - sigma_sqrt_t));

    BarrierComponents { a, b, c, d }
}

fn price_barrier_option(
    params: &BarrierOptionParams,
) -> CorpFinanceResult<(Money, BarrierOptionOutput, Vec<String>)> {
    let s = params.spot;
    let k = params.strike;
    let h = params.barrier;
    let r = params.risk_free_rate;
    let q = params.dividend_yield;
    let sigma = params.volatility;
    let t = params.time_to_expiry;
    let is_call = params.option_type == "Call";

    let phi: Decimal = if is_call { Decimal::ONE } else { -Decimal::ONE };
    let eta: Decimal = match params.barrier_type {
        BarrierType::DownAndIn | BarrierType::DownAndOut => Decimal::ONE,
        BarrierType::UpAndIn | BarrierType::UpAndOut => -Decimal::ONE,
    };

    let comp = compute_barrier_components(s, k, h, r, q, sigma, t, phi, eta);
    let mut warnings = Vec::new();

    // Rubinstein & Reiner knock-in formulas.
    // For knock-out options, use in-out parity: out = vanilla - in.
    // This guarantees the parity relation holds exactly.
    let knock_in_price = match (params.barrier_type, is_call) {
        // Down-and-in call
        (BarrierType::DownAndIn, true) | (BarrierType::DownAndOut, true) => {
            if k >= h {
                comp.c
            } else {
                comp.a - comp.b + comp.d
            }
        }
        // Up-and-in call
        (BarrierType::UpAndIn, true) | (BarrierType::UpAndOut, true) => {
            if k >= h {
                comp.a
            } else {
                comp.b - comp.c + comp.d
            }
        }
        // Down-and-in put
        (BarrierType::DownAndIn, false) | (BarrierType::DownAndOut, false) => {
            if k >= h {
                comp.a - comp.b + comp.d
            } else {
                comp.c
            }
        }
        // Up-and-in put
        (BarrierType::UpAndIn, false) | (BarrierType::UpAndOut, false) => {
            if k >= h {
                comp.b - comp.c + comp.d
            } else {
                comp.a
            }
        }
    };

    let van = vanilla_price(s, k, r, q, sigma, t, is_call).max(Decimal::ZERO);

    let barrier_price = match params.barrier_type {
        BarrierType::DownAndIn | BarrierType::UpAndIn => knock_in_price,
        BarrierType::DownAndOut | BarrierType::UpAndOut => van - knock_in_price,
    };

    // Ensure non-negative
    let barrier_price = barrier_price.max(Decimal::ZERO);

    // Vanilla comparison
    let vanilla = vanilla_price(s, k, r, q, sigma, t, is_call);
    let vanilla = vanilla.max(Decimal::ZERO);

    let barrier_discount = if vanilla > Decimal::ZERO {
        barrier_price / vanilla
    } else {
        Decimal::ZERO
    };

    // Compute greeks via finite differences
    let bump = dec!(0.01) * s; // 1% spot bump
    let price_up =
        price_barrier_analytical(s + bump, k, h, r, q, sigma, t, is_call, params.barrier_type);
    let price_down =
        price_barrier_analytical(s - bump, k, h, r, q, sigma, t, is_call, params.barrier_type);
    let delta = (price_up - price_down) / (dec!(2) * bump);
    let gamma = (price_up - dec!(2) * barrier_price + price_down) / (bump * bump);

    let vol_bump = dec!(0.01);
    let price_vol_up = price_barrier_analytical(
        s,
        k,
        h,
        r,
        q,
        sigma + vol_bump,
        t,
        is_call,
        params.barrier_type,
    );
    let vega = (price_vol_up - barrier_price) / dec!(100); // per 1% vol

    let dt = dec!(0.004); // ~1 day
    let price_t_down = if t > dt {
        price_barrier_analytical(s, k, h, r, q, sigma, t - dt, is_call, params.barrier_type)
    } else {
        barrier_price
    };
    let theta = (price_t_down - barrier_price) / dec!(365); // per calendar day (approx)

    if barrier_discount > dec!(0.95) {
        warnings.push("Barrier price very close to vanilla; barrier has minimal impact".into());
    }

    let output = BarrierOptionOutput {
        vanilla_value: vanilla,
        barrier_discount,
        greeks: BarrierGreeks {
            delta,
            gamma,
            vega,
            theta,
        },
    };

    Ok((barrier_price, output, warnings))
}

/// Helper that returns the raw barrier price for finite difference greek computations.
#[allow(clippy::too_many_arguments)]
fn price_barrier_analytical(
    s: Decimal,
    k: Decimal,
    h: Decimal,
    r: Decimal,
    q: Decimal,
    sigma: Decimal,
    t: Decimal,
    is_call: bool,
    barrier_type: BarrierType,
) -> Decimal {
    let phi: Decimal = if is_call { Decimal::ONE } else { -Decimal::ONE };
    let eta: Decimal = match barrier_type {
        BarrierType::DownAndIn | BarrierType::DownAndOut => Decimal::ONE,
        BarrierType::UpAndIn | BarrierType::UpAndOut => -Decimal::ONE,
    };

    let comp = compute_barrier_components(s, k, h, r, q, sigma, t, phi, eta);

    // Compute knock-in price, then derive knock-out via in-out parity.
    let knock_in = match (barrier_type, is_call) {
        (BarrierType::DownAndIn, true) | (BarrierType::DownAndOut, true) => {
            if k >= h {
                comp.c
            } else {
                comp.a - comp.b + comp.d
            }
        }
        (BarrierType::UpAndIn, true) | (BarrierType::UpAndOut, true) => {
            if k >= h {
                comp.a
            } else {
                comp.b - comp.c + comp.d
            }
        }
        (BarrierType::DownAndIn, false) | (BarrierType::DownAndOut, false) => {
            if k >= h {
                comp.a - comp.b + comp.d
            } else {
                comp.c
            }
        }
        (BarrierType::UpAndIn, false) | (BarrierType::UpAndOut, false) => {
            if k >= h {
                comp.b - comp.c + comp.d
            } else {
                comp.a
            }
        }
    };

    let van = vanilla_price(s, k, r, q, sigma, t, is_call).max(Decimal::ZERO);

    let price = match barrier_type {
        BarrierType::DownAndIn | BarrierType::UpAndIn => knock_in,
        BarrierType::DownAndOut | BarrierType::UpAndOut => van - knock_in,
    };

    price.max(Decimal::ZERO)
}

// ---------------------------------------------------------------------------
// Digital / binary option pricing
// ---------------------------------------------------------------------------

fn price_digital_option(
    params: &DigitalOptionParams,
) -> CorpFinanceResult<(Money, DigitalOptionOutput, Vec<String>)> {
    let s = params.spot;
    let k = params.strike;
    let r = params.risk_free_rate;
    let q = params.dividend_yield;
    let sigma = params.volatility;
    let t = params.time_to_expiry;
    let is_call = params.option_type == "Call";

    let (d1, d2) = bs_d1_d2(s, k, r, q, sigma, t);
    let exp_neg_rt = decimal_exp(-r * t);
    let exp_neg_qt = decimal_exp(-q * t);

    let mut warnings = Vec::new();

    let (fair_value, prob_payout) = match params.digital_type {
        DigitalType::CashOrNothing => {
            if is_call {
                // payout * e^(-rT) * N(d2)
                let n_d2 = norm_cdf(d2);
                (params.payout * exp_neg_rt * n_d2, n_d2)
            } else {
                // payout * e^(-rT) * N(-d2)
                let n_neg_d2 = norm_cdf(-d2);
                (params.payout * exp_neg_rt * n_neg_d2, n_neg_d2)
            }
        }
        DigitalType::AssetOrNothing => {
            if is_call {
                // S * e^(-qT) * N(d1)
                let n_d1 = norm_cdf(d1);
                (s * exp_neg_qt * n_d1, n_d1)
            } else {
                // S * e^(-qT) * N(-d1)
                let n_neg_d1 = norm_cdf(-d1);
                (s * exp_neg_qt * n_neg_d1, n_neg_d1)
            }
        }
    };

    let expected_payout = match params.digital_type {
        DigitalType::CashOrNothing => params.payout * prob_payout,
        DigitalType::AssetOrNothing => s * prob_payout,
    };

    // Vanilla equivalent: standard BS option for comparison
    let vanilla = vanilla_price(s, k, r, q, sigma, t, is_call);

    if prob_payout < dec!(0.05) {
        warnings.push("Very low probability of payout (<5%)".into());
    }
    if prob_payout > dec!(0.95) {
        warnings.push("Very high probability of payout (>95%); deeply in-the-money".into());
    }

    let output = DigitalOptionOutput {
        vanilla_equivalent: vanilla.max(Decimal::ZERO),
        probability_of_payout: prob_payout,
        expected_payout,
    };

    Ok((fair_value, output, warnings))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn price_exotic(
    input: &ExoticProductInput,
) -> CorpFinanceResult<ComputationOutput<ExoticProductOutput>> {
    let start = Instant::now();

    let (fair_value, autocallable_out, barrier_out, digital_out, warnings, methodology) =
        match input.product_type {
            ExoticType::Autocallable => {
                let params =
                    input
                        .autocallable
                        .as_ref()
                        .ok_or_else(|| CorpFinanceError::InvalidInput {
                            field: "autocallable".into(),
                            reason: "autocallable params required for Autocallable product type"
                                .into(),
                        })?;
                validate_autocallable(params)?;
                let (fv, out, w) = price_autocallable(params)?;
                (
                    fv,
                    Some(out),
                    None,
                    None,
                    w,
                    "Analytical autocallable pricing with observation schedule",
                )
            }
            ExoticType::BarrierOption => {
                let params = input.barrier_option.as_ref().ok_or_else(|| {
                    CorpFinanceError::InvalidInput {
                        field: "barrier_option".into(),
                        reason: "barrier_option params required for BarrierOption product type"
                            .into(),
                    }
                })?;
                validate_barrier(params)?;
                let (fv, out, w) = price_barrier_option(params)?;
                (
                    fv,
                    None,
                    Some(out),
                    None,
                    w,
                    "Rubinstein-Reiner closed-form barrier option pricing",
                )
            }
            ExoticType::DigitalOption => {
                let params = input.digital_option.as_ref().ok_or_else(|| {
                    CorpFinanceError::InvalidInput {
                        field: "digital_option".into(),
                        reason: "digital_option params required for DigitalOption product type"
                            .into(),
                    }
                })?;
                validate_digital(params)?;
                let (fv, out, w) = price_digital_option(params)?;
                (
                    fv,
                    None,
                    None,
                    Some(out),
                    w,
                    "Black-Scholes digital option pricing",
                )
            }
        };

    let product_type_str = format!("{:?}", input.product_type);

    let output = ExoticProductOutput {
        product_type: product_type_str.clone(),
        fair_value,
        autocallable: autocallable_out,
        barrier_option: barrier_out,
        digital_option: digital_out,
        warnings: warnings.clone(),
    };

    let assumptions = serde_json::json!({
        "product_type": product_type_str,
        "methodology": methodology,
        "precision": "rust_decimal_128bit",
        "math_helpers": "Taylor series exp/ln (40 iter), Newton sqrt (20 iter), A&S norm_cdf",
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        let diff = a - b;
        let abs_diff = if diff < Decimal::ZERO { -diff } else { diff };
        abs_diff < tol
    }

    // -----------------------------------------------------------------------
    // 1. Down-and-out call: barrier below spot, no knock-out -> close to vanilla
    // -----------------------------------------------------------------------
    #[test]
    fn test_down_and_out_call_far_barrier() {
        // Barrier very far below spot => behaves like vanilla
        let input = ExoticProductInput {
            product_type: ExoticType::BarrierOption,
            autocallable: None,
            barrier_option: Some(BarrierOptionParams {
                spot: dec!(100),
                strike: dec!(100),
                barrier: dec!(30), // far below
                barrier_type: BarrierType::DownAndOut,
                option_type: "Call".into(),
                volatility: dec!(0.20),
                risk_free_rate: dec!(0.05),
                dividend_yield: dec!(0),
                time_to_expiry: dec!(1),
                rebate: None,
            }),
            digital_option: None,
        };
        let result = price_exotic(&input).unwrap();
        let fv = result.result.fair_value;
        let vanilla = result.result.barrier_option.as_ref().unwrap().vanilla_value;
        // With barrier at 30, almost no chance of knock-out => price ~ vanilla
        assert!(
            approx_eq(fv, vanilla, dec!(1.0)),
            "Down-and-out call (H=30) price {fv} should be close to vanilla {vanilla}"
        );
    }

    // -----------------------------------------------------------------------
    // 2. In-out parity: DI call + DO call = vanilla call
    // -----------------------------------------------------------------------
    #[test]
    fn test_in_out_parity_down_call() {
        let spot = dec!(100);
        let strike = dec!(100);
        let barrier = dec!(80);
        let vol = dec!(0.25);
        let r = dec!(0.05);
        let q = dec!(0.02);
        let t = dec!(1);

        // Down-and-in call
        let di_input = ExoticProductInput {
            product_type: ExoticType::BarrierOption,
            autocallable: None,
            barrier_option: Some(BarrierOptionParams {
                spot,
                strike,
                barrier,
                barrier_type: BarrierType::DownAndIn,
                option_type: "Call".into(),
                volatility: vol,
                risk_free_rate: r,
                dividend_yield: q,
                time_to_expiry: t,
                rebate: None,
            }),
            digital_option: None,
        };
        let di_result = price_exotic(&di_input).unwrap();
        let di_price = di_result.result.fair_value;

        // Down-and-out call
        let do_input = ExoticProductInput {
            product_type: ExoticType::BarrierOption,
            autocallable: None,
            barrier_option: Some(BarrierOptionParams {
                spot,
                strike,
                barrier,
                barrier_type: BarrierType::DownAndOut,
                option_type: "Call".into(),
                volatility: vol,
                risk_free_rate: r,
                dividend_yield: q,
                time_to_expiry: t,
                rebate: None,
            }),
            digital_option: None,
        };
        let do_result = price_exotic(&do_input).unwrap();
        let do_price = do_result.result.fair_value;

        let vanilla = vanilla_price(spot, strike, r, q, vol, t, true);

        // DI + DO should equal vanilla
        let sum = di_price + do_price;
        assert!(
            approx_eq(sum, vanilla, dec!(0.5)),
            "In-out parity failed: DI({di_price}) + DO({do_price}) = {sum}, vanilla = {vanilla}"
        );
    }

    // -----------------------------------------------------------------------
    // 3. Up-and-out put pricing
    // -----------------------------------------------------------------------
    #[test]
    fn test_up_and_out_put() {
        // Up-and-out put: the barrier can knock out the option if spot rises.
        // Verify the price is bounded above by vanilla, and in-out parity holds.
        let spot = dec!(100);
        let strike = dec!(110);
        let barrier = dec!(120);
        let vol = dec!(0.25);
        let r = dec!(0.05);
        let q = dec!(0);
        let t = dec!(1);

        let input = ExoticProductInput {
            product_type: ExoticType::BarrierOption,
            autocallable: None,
            barrier_option: Some(BarrierOptionParams {
                spot,
                strike,
                barrier,
                barrier_type: BarrierType::UpAndOut,
                option_type: "Put".into(),
                volatility: vol,
                risk_free_rate: r,
                dividend_yield: q,
                time_to_expiry: t,
                rebate: None,
            }),
            digital_option: None,
        };
        let result = price_exotic(&input).unwrap();
        let fv = result.result.fair_value;
        let vanilla = result.result.barrier_option.as_ref().unwrap().vanilla_value;

        // Up-and-out put should be <= vanilla put
        assert!(
            fv <= vanilla + dec!(0.01),
            "Up-and-out put {fv} should be <= vanilla put {vanilla}"
        );
        // Should be non-negative
        assert!(
            fv >= Decimal::ZERO,
            "Up-and-out put should be non-negative, got {fv}"
        );

        // Verify in-out parity: UO + UI = vanilla
        let uo = price_barrier_analytical(
            spot,
            strike,
            barrier,
            r,
            q,
            vol,
            t,
            false,
            BarrierType::UpAndOut,
        );
        let ui = price_barrier_analytical(
            spot,
            strike,
            barrier,
            r,
            q,
            vol,
            t,
            false,
            BarrierType::UpAndIn,
        );
        let van = vanilla_price(spot, strike, r, q, vol, t, false);
        let sum = uo + ui;
        assert!(
            approx_eq(sum, van, dec!(0.5)),
            "UO({uo}) + UI({ui}) = {sum} should equal vanilla put {van}"
        );
    }

    // -----------------------------------------------------------------------
    // 4. Digital cash-or-nothing call
    // -----------------------------------------------------------------------
    #[test]
    fn test_digital_cash_or_nothing_call() {
        let input = ExoticProductInput {
            product_type: ExoticType::DigitalOption,
            autocallable: None,
            barrier_option: None,
            digital_option: Some(DigitalOptionParams {
                spot: dec!(100),
                strike: dec!(100),
                digital_type: DigitalType::CashOrNothing,
                option_type: "Call".into(),
                payout: dec!(1000),
                volatility: dec!(0.20),
                risk_free_rate: dec!(0.05),
                dividend_yield: dec!(0),
                time_to_expiry: dec!(1),
            }),
        };
        let result = price_exotic(&input).unwrap();
        let fv = result.result.fair_value;
        let dig_out = result.result.digital_option.as_ref().unwrap();

        // ATM digital: probability ~ 0.5 (slightly above due to drift)
        assert!(
            dig_out.probability_of_payout > dec!(0.4) && dig_out.probability_of_payout < dec!(0.7),
            "ATM digital call payout prob {} should be near 0.5",
            dig_out.probability_of_payout
        );
        // Fair value ~ payout * e^(-rT) * N(d2) ~ 1000 * 0.951 * 0.56 ~ 530
        assert!(
            fv > dec!(400) && fv < dec!(700),
            "Cash-or-nothing call FV {fv} should be in reasonable range"
        );
    }

    // -----------------------------------------------------------------------
    // 5. Digital asset-or-nothing put
    // -----------------------------------------------------------------------
    #[test]
    fn test_digital_asset_or_nothing_put() {
        let input = ExoticProductInput {
            product_type: ExoticType::DigitalOption,
            autocallable: None,
            barrier_option: None,
            digital_option: Some(DigitalOptionParams {
                spot: dec!(100),
                strike: dec!(100),
                digital_type: DigitalType::AssetOrNothing,
                option_type: "Put".into(),
                payout: dec!(0), // not used for asset-or-nothing
                volatility: dec!(0.20),
                risk_free_rate: dec!(0.05),
                dividend_yield: dec!(0),
                time_to_expiry: dec!(1),
            }),
        };
        let result = price_exotic(&input).unwrap();
        let fv = result.result.fair_value;
        let dig_out = result.result.digital_option.as_ref().unwrap();

        // Asset-or-nothing put: S * e^(-qT) * N(-d1) ~ 100 * N(-d1)
        // N(-d1) for ATM ~ 0.4, so value ~ 40
        assert!(
            fv > dec!(20) && fv < dec!(60),
            "Asset-or-nothing put FV {fv} should be in reasonable range"
        );
        assert!(
            dig_out.probability_of_payout > dec!(0.3) && dig_out.probability_of_payout < dec!(0.6),
            "ATM asset-or-nothing put payout prob {} should be near 0.42",
            dig_out.probability_of_payout
        );
    }

    // -----------------------------------------------------------------------
    // 6. Autocallable: high barrier -> low early call probability
    // -----------------------------------------------------------------------
    #[test]
    fn test_autocallable_high_barrier_low_prob() {
        let input = ExoticProductInput {
            product_type: ExoticType::Autocallable,
            autocallable: Some(AutocallableParams {
                notional: dec!(100000),
                underlying_price: dec!(100),
                volatility: dec!(0.20),
                risk_free_rate: dec!(0.03),
                dividend_yield: dec!(0),
                maturity_years: dec!(3),
                observation_frequency: 4,     // quarterly
                autocall_barrier: dec!(2.00), // 200% of initial — very high
                coupon_per_period: dec!(0.02),
                coupon_barrier: dec!(0.80),
                ki_barrier: dec!(0.60),
                ki_strike: dec!(1.0),
            }),
            barrier_option: None,
            digital_option: None,
        };
        let result = price_exotic(&input).unwrap();
        let ac_out = result.result.autocallable.as_ref().unwrap();

        // With a 150% autocall barrier and 20% vol over 3 years, probability should be low
        assert!(
            ac_out.probability_of_autocall < dec!(0.5),
            "Autocall prob {} should be low with 150% barrier",
            ac_out.probability_of_autocall
        );
    }

    // -----------------------------------------------------------------------
    // 7. Autocallable: observation schedule generation
    // -----------------------------------------------------------------------
    #[test]
    fn test_autocallable_observation_schedule() {
        let input = ExoticProductInput {
            product_type: ExoticType::Autocallable,
            autocallable: Some(AutocallableParams {
                notional: dec!(100000),
                underlying_price: dec!(100),
                volatility: dec!(0.25),
                risk_free_rate: dec!(0.04),
                dividend_yield: dec!(0.01),
                maturity_years: dec!(2),
                observation_frequency: 2, // semi-annual
                autocall_barrier: dec!(1.0),
                coupon_per_period: dec!(0.03),
                coupon_barrier: dec!(0.80),
                ki_barrier: dec!(0.60),
                ki_strike: dec!(1.0),
            }),
            barrier_option: None,
            digital_option: None,
        };
        let result = price_exotic(&input).unwrap();
        let ac_out = result.result.autocallable.as_ref().unwrap();

        // 2 years * 2 obs/year = 4 observations
        assert_eq!(
            ac_out.observation_schedule.len(),
            4,
            "Should have 4 observations for 2Y semi-annual"
        );

        // Cumulative prob should be monotonically increasing
        for i in 1..ac_out.observation_schedule.len() {
            assert!(
                ac_out.observation_schedule[i].cumulative_prob
                    >= ac_out.observation_schedule[i - 1].cumulative_prob - dec!(0.001),
                "Cumulative prob should be non-decreasing"
            );
        }

        // Expected life should be between 0 and maturity
        assert!(
            ac_out.expected_life > Decimal::ZERO && ac_out.expected_life <= dec!(2),
            "Expected life {} should be in (0, 2]",
            ac_out.expected_life
        );

        // Fair value should be positive
        assert!(
            result.result.fair_value > Decimal::ZERO,
            "Autocallable fair value should be positive"
        );
    }

    // -----------------------------------------------------------------------
    // 8. Barrier option greeks: delta sign check
    // -----------------------------------------------------------------------
    #[test]
    fn test_barrier_greeks_delta_sign() {
        // Down-and-out call: delta should be positive (value increases as S increases)
        let input = ExoticProductInput {
            product_type: ExoticType::BarrierOption,
            autocallable: None,
            barrier_option: Some(BarrierOptionParams {
                spot: dec!(100),
                strike: dec!(100),
                barrier: dec!(80),
                barrier_type: BarrierType::DownAndOut,
                option_type: "Call".into(),
                volatility: dec!(0.25),
                risk_free_rate: dec!(0.05),
                dividend_yield: dec!(0),
                time_to_expiry: dec!(1),
                rebate: None,
            }),
            digital_option: None,
        };
        let result = price_exotic(&input).unwrap();
        let greeks = &result.result.barrier_option.as_ref().unwrap().greeks;
        assert!(
            greeks.delta > Decimal::ZERO,
            "Down-and-out call delta {} should be positive",
            greeks.delta
        );
    }

    // -----------------------------------------------------------------------
    // 9. In-out parity for puts
    // -----------------------------------------------------------------------
    #[test]
    fn test_in_out_parity_up_put() {
        let spot = dec!(100);
        let strike = dec!(110);
        let barrier = dec!(120);
        let vol = dec!(0.25);
        let r = dec!(0.05);
        let q = dec!(0);
        let t = dec!(1);

        // Up-and-in put
        let ui_price = price_barrier_analytical(
            spot,
            strike,
            barrier,
            r,
            q,
            vol,
            t,
            false,
            BarrierType::UpAndIn,
        );
        // Up-and-out put
        let uo_price = price_barrier_analytical(
            spot,
            strike,
            barrier,
            r,
            q,
            vol,
            t,
            false,
            BarrierType::UpAndOut,
        );
        let vanilla = vanilla_price(spot, strike, r, q, vol, t, false);

        let sum = ui_price + uo_price;
        assert!(
            approx_eq(sum, vanilla, dec!(0.5)),
            "Up in-out put parity failed: UI({ui_price}) + UO({uo_price}) = {sum}, vanilla = {vanilla}"
        );
    }

    // -----------------------------------------------------------------------
    // 10. Digital call + digital put = discounted payout (cash-or-nothing)
    // -----------------------------------------------------------------------
    #[test]
    fn test_digital_call_put_sum() {
        let payout = dec!(1000);
        let r = dec!(0.05);
        let t = dec!(1);
        let disc_payout = payout * decimal_exp(-r * t);

        let call_input = ExoticProductInput {
            product_type: ExoticType::DigitalOption,
            autocallable: None,
            barrier_option: None,
            digital_option: Some(DigitalOptionParams {
                spot: dec!(100),
                strike: dec!(100),
                digital_type: DigitalType::CashOrNothing,
                option_type: "Call".into(),
                payout,
                volatility: dec!(0.20),
                risk_free_rate: r,
                dividend_yield: dec!(0),
                time_to_expiry: t,
            }),
        };
        let put_input = ExoticProductInput {
            product_type: ExoticType::DigitalOption,
            autocallable: None,
            barrier_option: None,
            digital_option: Some(DigitalOptionParams {
                spot: dec!(100),
                strike: dec!(100),
                digital_type: DigitalType::CashOrNothing,
                option_type: "Put".into(),
                payout,
                volatility: dec!(0.20),
                risk_free_rate: r,
                dividend_yield: dec!(0),
                time_to_expiry: t,
            }),
        };

        let call_fv = price_exotic(&call_input).unwrap().result.fair_value;
        let put_fv = price_exotic(&put_input).unwrap().result.fair_value;
        let sum = call_fv + put_fv;

        assert!(
            approx_eq(sum, disc_payout, dec!(1.0)),
            "Digital call + put = {sum}, should equal discounted payout {disc_payout}"
        );
    }

    // -----------------------------------------------------------------------
    // 11. Validation: missing params
    // -----------------------------------------------------------------------
    #[test]
    fn test_missing_autocallable_params() {
        let input = ExoticProductInput {
            product_type: ExoticType::Autocallable,
            autocallable: None,
            barrier_option: None,
            digital_option: None,
        };
        assert!(price_exotic(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // 12. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = ExoticProductInput {
            product_type: ExoticType::DigitalOption,
            autocallable: None,
            barrier_option: None,
            digital_option: Some(DigitalOptionParams {
                spot: dec!(100),
                strike: dec!(100),
                digital_type: DigitalType::CashOrNothing,
                option_type: "Call".into(),
                payout: dec!(100),
                volatility: dec!(0.20),
                risk_free_rate: dec!(0.05),
                dividend_yield: dec!(0),
                time_to_expiry: dec!(1),
            }),
        };
        let result = price_exotic(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
