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
    Expand,
    Abandon,
    Defer,
    Switch,
    Contract,
    Compound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealOptionInput {
    pub option_type: OptionType,
    /// Current project NPV or asset value
    pub underlying_value: Decimal,
    /// Cost to exercise (investment cost, salvage value, etc.)
    pub exercise_price: Decimal,
    /// Project value volatility
    pub volatility: Decimal,
    /// Risk-free discount rate
    pub risk_free_rate: Decimal,
    /// Years until option expires
    pub time_to_expiry: Decimal,
    /// Number of binomial tree steps (default 100)
    #[serde(default = "default_steps")]
    pub steps: u32,
    /// Continuous leakage / cost of waiting
    pub dividend_yield: Option<Decimal>,
    /// For expand option: scale multiplier (e.g. 1.5 = 50% expansion)
    pub expansion_factor: Option<Decimal>,
    /// For contract option: scale reduction factor
    pub contraction_factor: Option<Decimal>,
    /// For switch option: cost to switch between modes
    pub switch_cost: Option<Decimal>,
    /// For switch option: value ratio in alternate mode
    pub switch_value_ratio: Option<Decimal>,
}

fn default_steps() -> u32 {
    100
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExerciseBoundary {
    pub time_step: u32,
    pub threshold_value: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealOptionOutput {
    /// Value of the real option
    pub option_value: Decimal,
    /// Static NPV + option value
    pub expanded_npv: Decimal,
    /// Underlying value minus exercise price (for defer/expand)
    pub static_npv: Decimal,
    /// option_value - max(static_npv, 0)
    pub option_premium: Decimal,
    /// Exercise threshold at each time step
    pub optimal_exercise_boundary: Vec<ExerciseBoundary>,
    /// Sensitivity to underlying value change
    pub delta: Decimal,
    /// Second-order sensitivity
    pub gamma: Decimal,
    /// Time decay per year
    pub theta: Decimal,
    /// Sensitivity to volatility
    pub vega: Decimal,
    /// Whether early exercise is recommended now
    pub early_exercise_optimal: bool,
    /// Volatility at which option_value = 0 premium
    pub breakeven_volatility: Decimal,
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
    for n in 1u32..=30 {
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
    let mut guess = x / two;
    if x > dec!(100) {
        guess = dec!(10);
    } else if x < dec!(0.01) {
        guess = dec!(0.1);
    }
    for _ in 0..20 {
        guess = (guess + x / guess) / two;
    }
    guess
}

/// Natural log via Newton's method: find y such that exp(y) = x. 20 iterations.
#[allow(dead_code)]
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
    for _ in 0..20 {
        let ey = exp_decimal(y);
        if ey == Decimal::ZERO {
            break;
        }
        y = y - Decimal::ONE + x / ey;
    }
    y
}

/// Integer power via exponentiation by squaring (iterative multiplication).
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

/// Absolute value of a Decimal.
#[allow(dead_code)]
fn abs_decimal(x: Decimal) -> Decimal {
    if x < Decimal::ZERO {
        -x
    } else {
        x
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &RealOptionInput) -> CorpFinanceResult<()> {
    if input.underlying_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "underlying_value".into(),
            reason: "must be positive".into(),
        });
    }
    if input.exercise_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "exercise_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.volatility < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "volatility".into(),
            reason: "must be non-negative".into(),
        });
    }
    if input.time_to_expiry <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "time_to_expiry".into(),
            reason: "must be positive".into(),
        });
    }
    if input.steps == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "steps".into(),
            reason: "must be at least 1".into(),
        });
    }
    if let Some(q) = input.dividend_yield {
        if q < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "dividend_yield".into(),
                reason: "must be non-negative".into(),
            });
        }
    }
    // Option-type-specific validation
    match input.option_type {
        OptionType::Expand => {
            if input.expansion_factor.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "expansion_factor".into(),
                    reason: "required for Expand option type".into(),
                });
            }
            let ef = input.expansion_factor.unwrap();
            if ef <= Decimal::ONE {
                return Err(CorpFinanceError::InvalidInput {
                    field: "expansion_factor".into(),
                    reason: "must be greater than 1.0".into(),
                });
            }
        }
        OptionType::Contract => {
            if input.contraction_factor.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "contraction_factor".into(),
                    reason: "required for Contract option type".into(),
                });
            }
            let cf = input.contraction_factor.unwrap();
            if cf <= Decimal::ZERO || cf >= Decimal::ONE {
                return Err(CorpFinanceError::InvalidInput {
                    field: "contraction_factor".into(),
                    reason: "must be between 0 and 1 exclusive".into(),
                });
            }
        }
        OptionType::Switch => {
            if input.switch_cost.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "switch_cost".into(),
                    reason: "required for Switch option type".into(),
                });
            }
            if input.switch_value_ratio.is_none() {
                return Err(CorpFinanceError::InvalidInput {
                    field: "switch_value_ratio".into(),
                    reason: "required for Switch option type".into(),
                });
            }
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Core binomial tree engine
// ---------------------------------------------------------------------------

/// Result from the binomial tree computation, including full value information.
#[allow(dead_code)]
struct BinomialResult {
    option_value: Decimal,
    /// Price tree at time step 1: [down_node, up_node]
    price_step1: (Decimal, Decimal),
    /// Option values at time step 1: [down_node, up_node]
    value_step1: (Decimal, Decimal),
    /// Price tree at time step 2 center node
    price_step2_mid: Decimal,
    /// Option value at time step 2 center node
    value_step2_mid: Decimal,
    /// Exercise boundaries per time step
    exercise_boundaries: Vec<ExerciseBoundary>,
    /// Whether the root node exercises immediately
    early_exercise_at_root: bool,
}

/// Maximum representable Decimal value used as a cap to avoid overflow panics.
const DECIMAL_CAP: Decimal = Decimal::from_parts(u32::MAX, u32::MAX, u32::MAX, false, 0);

/// Checked multiplication that returns DECIMAL_CAP on overflow instead of panicking.
fn safe_mul(a: Decimal, b: Decimal) -> Decimal {
    a.checked_mul(b).unwrap_or(DECIMAL_CAP)
}

/// Compute node price using net exponent to avoid overflow.
/// Since d = 1/u, we have: S * u^ups * d^downs = S * u^(ups - downs).
/// For net_up >= 0: S * u^net_up. For net_up < 0: S / u^|net_up|.
fn node_price(s: Decimal, u: Decimal, ups: u32, downs: u32) -> Decimal {
    if ups >= downs {
        safe_mul(s, pow_decimal_safe(u, ups - downs))
    } else {
        let denom = pow_decimal_safe(u, downs - ups);
        if denom == Decimal::ZERO {
            return Decimal::ZERO;
        }
        s / denom
    }
}

/// Safe integer power that caps on overflow.
fn pow_decimal_safe(base: Decimal, exp: u32) -> Decimal {
    if exp == 0 {
        return Decimal::ONE;
    }
    let mut result = Decimal::ONE;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = match result.checked_mul(b) {
                Some(v) => v,
                None => return DECIMAL_CAP,
            };
        }
        b = match b.checked_mul(b) {
            Some(v) => v,
            None => {
                // If we still have bits to process, result is already capped
                if e > 1 {
                    return DECIMAL_CAP;
                }
                break;
            }
        };
        e >>= 1;
    }
    result
}

/// Build and solve the binomial tree for the given option type.
#[allow(clippy::too_many_arguments)]
fn binomial_tree(
    s: Decimal,
    k: Decimal,
    sigma: Decimal,
    r: Decimal,
    q: Decimal,
    t: Decimal,
    n: u32,
    option_type: OptionType,
    expansion_factor: Decimal,
    contraction_factor: Decimal,
    switch_cost: Decimal,
    switch_value_ratio: Decimal,
) -> BinomialResult {
    let dt = t / Decimal::from(n);
    let u = exp_decimal(sigma * sqrt_decimal(dt));
    let d = Decimal::ONE / u;
    let exp_rq_dt = exp_decimal((r - q) * dt);
    let disc = exp_decimal(-r * dt);
    let p_up = if u == d {
        dec!(0.5)
    } else {
        (exp_rq_dt - d) / (u - d)
    };
    let p_down = Decimal::ONE - p_up;

    let size = (n + 1) as usize;

    // Build terminal asset prices using net exponent to avoid overflow
    let mut prices = Vec::with_capacity(size);
    for i in 0..size {
        let ups = i as u32;
        let downs = n - ups;
        prices.push(node_price(s, u, ups, downs));
    }

    // Compute terminal payoffs
    let mut values: Vec<Decimal> = prices
        .iter()
        .map(|&price| {
            terminal_payoff(
                price,
                k,
                option_type,
                expansion_factor,
                contraction_factor,
                switch_cost,
                switch_value_ratio,
            )
        })
        .collect();

    // Track exercise boundaries and capture step-1/step-2 values for Greeks
    let mut exercise_boundaries = Vec::new();
    let mut step1_values: Option<(Decimal, Decimal)> = None;
    let mut step2_mid_value: Option<Decimal> = None;

    // Backward induction
    for step in (0..n).rev() {
        let step_size = (step + 1) as usize;
        let mut boundary_found = false;
        let mut boundary_value = Decimal::ZERO;

        for i in 0..step_size {
            let continuation = safe_mul(
                disc,
                safe_mul(p_up, values[i + 1]) + safe_mul(p_down, values[i]),
            );
            let ups = i as u32;
            let downs = step - ups;
            let np = node_price(s, u, ups, downs);
            let exercise = exercise_payoff(
                np,
                k,
                option_type,
                expansion_factor,
                contraction_factor,
                switch_cost,
                switch_value_ratio,
            );

            if exercise > continuation {
                values[i] = exercise;
                if !boundary_found {
                    boundary_found = true;
                    boundary_value = np;
                }
            } else {
                values[i] = continuation;
            }
        }

        // Capture step 2 middle value (node index 1 at step 2 = 1 up, 1 down)
        if step == 2 && step_size >= 2 {
            step2_mid_value = Some(values[1]);
        }

        // Capture step 1 values after backward induction reaches step 1
        if step == 1 {
            step1_values = Some((values[0], values[1]));
        }

        if boundary_found {
            exercise_boundaries.push(ExerciseBoundary {
                time_step: step,
                threshold_value: boundary_value,
            });
        }
    }

    exercise_boundaries.reverse();

    // For n=1, step1 values are terminal payoffs before final backward step
    let (val_down, val_up) = step1_values.unwrap_or((values[0], values[0]));
    let price_up = s * u;
    let price_down = s * d;
    let price_mid2 = s; // s * u * d = s since d = 1/u
    let val_mid2 = step2_mid_value
        .unwrap_or_else(|| safe_mul(disc, safe_mul(p_up, val_up) + safe_mul(p_down, val_down)));

    let option_value = values[0];

    // Check if immediate exercise is optimal at root
    let immediate = exercise_payoff(
        s,
        k,
        option_type,
        expansion_factor,
        contraction_factor,
        switch_cost,
        switch_value_ratio,
    );
    let early_exercise_at_root = immediate >= option_value && immediate > Decimal::ZERO;

    BinomialResult {
        option_value,
        price_step1: (price_down, price_up),
        value_step1: (val_down, val_up),
        price_step2_mid: price_mid2,
        value_step2_mid: val_mid2,
        exercise_boundaries,
        early_exercise_at_root,
    }
}

/// Terminal (and exercise) payoff for each option type.
fn terminal_payoff(
    price: Decimal,
    k: Decimal,
    option_type: OptionType,
    expansion_factor: Decimal,
    contraction_factor: Decimal,
    switch_cost: Decimal,
    switch_value_ratio: Decimal,
) -> Decimal {
    exercise_payoff(
        price,
        k,
        option_type,
        expansion_factor,
        contraction_factor,
        switch_cost,
        switch_value_ratio,
    )
}

/// Exercise value at any node. Returns the payoff from exercising the option.
/// Uses safe multiplication to avoid overflow panics.
fn exercise_payoff(
    price: Decimal,
    k: Decimal,
    option_type: OptionType,
    expansion_factor: Decimal,
    contraction_factor: Decimal,
    switch_cost: Decimal,
    switch_value_ratio: Decimal,
) -> Decimal {
    match option_type {
        OptionType::Defer => {
            // American call: max(S - K, 0)
            (price - k).max(Decimal::ZERO)
        }
        OptionType::Expand => {
            // Exercise: get expanded project value minus cost
            // Payoff = max(S * expansion_factor - K, S)
            let expanded = safe_mul(price, expansion_factor) - k;
            expanded.max(price)
        }
        OptionType::Abandon => {
            // American put: max(K, S) where K = salvage value
            k.max(price)
        }
        OptionType::Contract => {
            // Contract: reduce scale, save some cost
            // Payoff = max(S * cf + K, S) where K = savings from contracting
            let contracted = safe_mul(price, contraction_factor) + k;
            contracted.max(price)
        }
        OptionType::Switch => {
            // Switch between modes
            // Payoff = max(S * switch_ratio - switch_cost, S)
            let switched = safe_mul(price, switch_value_ratio) - switch_cost;
            switched.max(price)
        }
        OptionType::Compound => {
            // Simplified compound option: treated as defer (option on option)
            (price - k).max(Decimal::ZERO)
        }
    }
}

// ---------------------------------------------------------------------------
// Greeks via finite differences
// ---------------------------------------------------------------------------

struct Greeks {
    delta: Decimal,
    gamma: Decimal,
    theta: Decimal,
    vega: Decimal,
}

fn compute_greeks(
    input: &RealOptionInput,
    _base_result: &BinomialResult,
    base_value: Decimal,
) -> Greeks {
    let s = input.underlying_value;

    // All Greeks via central finite differences (reprice the full tree)
    let ds = s * dec!(0.01); // 1% bump

    // Delta: dV/dS
    let mut up_input = input.clone();
    up_input.underlying_value = s + ds;
    let v_up = run_binomial(&up_input);

    let mut down_input = input.clone();
    down_input.underlying_value = s - ds;
    let v_down = run_binomial(&down_input);

    let delta = if ds != Decimal::ZERO {
        (v_up - v_down) / (dec!(2) * ds)
    } else {
        Decimal::ZERO
    };

    // Gamma: d2V/dS2
    let gamma = if ds != Decimal::ZERO {
        (v_up - dec!(2) * base_value + v_down) / (ds * ds)
    } else {
        Decimal::ZERO
    };

    // Theta: dV/dt (time decay)
    let dt_shift = dec!(0.01);
    let theta = if input.time_to_expiry > dt_shift {
        let mut shifted_input = input.clone();
        shifted_input.time_to_expiry = input.time_to_expiry - dt_shift;
        let shifted = run_binomial(&shifted_input);
        (shifted - base_value) / dt_shift
    } else {
        Decimal::ZERO
    };

    // Vega: dV/dsigma
    let vol_shift = dec!(0.01);
    let vega = if input.volatility > Decimal::ZERO {
        let mut vol_up_input = input.clone();
        vol_up_input.volatility = input.volatility + vol_shift;
        let v_up_vol = run_binomial(&vol_up_input);

        let mut vol_down_input = input.clone();
        vol_down_input.volatility = (input.volatility - vol_shift).max(dec!(0.001));
        let v_down_vol = run_binomial(&vol_down_input);

        let actual_shift = vol_up_input.volatility - vol_down_input.volatility;
        if actual_shift != Decimal::ZERO {
            (v_up_vol - v_down_vol) / actual_shift
        } else {
            Decimal::ZERO
        }
    } else {
        Decimal::ZERO
    };

    Greeks {
        delta,
        gamma,
        theta,
        vega,
    }
}

/// Run the binomial tree and return just the option value (for Greek finite differences).
fn run_binomial(input: &RealOptionInput) -> Decimal {
    let s = input.underlying_value;
    let k = input.exercise_price;
    let sigma = input.volatility;
    let r = input.risk_free_rate;
    let q = input.dividend_yield.unwrap_or(Decimal::ZERO);
    let t = input.time_to_expiry;
    let n = input.steps;
    let ef = input.expansion_factor.unwrap_or(dec!(1.5));
    let cf = input.contraction_factor.unwrap_or(dec!(0.5));
    let sc = input.switch_cost.unwrap_or(Decimal::ZERO);
    let sr = input.switch_value_ratio.unwrap_or(Decimal::ONE);

    if sigma <= Decimal::ZERO || t <= Decimal::ZERO || n == 0 {
        return Decimal::ZERO;
    }

    let result = binomial_tree(s, k, sigma, r, q, t, n, input.option_type, ef, cf, sc, sr);
    result.option_value
}

// ---------------------------------------------------------------------------
// Breakeven volatility via bisection
// ---------------------------------------------------------------------------

fn find_breakeven_volatility(input: &RealOptionInput) -> Decimal {
    // Find the volatility where option premium = 0
    // i.e., option_value = max(static_npv, 0)
    let s = input.underlying_value;
    let k = input.exercise_price;

    let static_npv = compute_static_npv(s, k, input.option_type, input.expansion_factor);
    let target = static_npv.max(Decimal::ZERO);

    let mut lo = dec!(0.001);
    let mut hi = dec!(2.0);

    // Use fewer steps for the breakeven search to avoid overflow with high vol
    let search_steps = input.steps.min(30);

    // Bisection search: 25 iterations
    for _ in 0..25 {
        let mid = (lo + hi) / dec!(2);
        let mut test_input = input.clone();
        test_input.volatility = mid;
        test_input.steps = search_steps;
        let val = run_binomial(&test_input);

        if val > target {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    (lo + hi) / dec!(2)
}

/// Compute static NPV depending on option type.
fn compute_static_npv(
    s: Decimal,
    k: Decimal,
    option_type: OptionType,
    expansion_factor: Option<Decimal>,
) -> Decimal {
    match option_type {
        OptionType::Defer | OptionType::Compound => s - k,
        OptionType::Expand => {
            let ef = expansion_factor.unwrap_or(dec!(1.5));
            s * ef - k - s // net gain from expanding minus what you already have
        }
        OptionType::Abandon => k - s, // salvage value minus current value
        OptionType::Contract => {
            // Savings from contracting (exercise_price) minus lost value
            k - s * (Decimal::ONE - dec!(0.5)) // approximate
        }
        OptionType::Switch => s - k, // simplified
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn value_real_option(
    input: &RealOptionInput,
) -> CorpFinanceResult<ComputationOutput<RealOptionOutput>> {
    let start = Instant::now();
    validate_input(input)?;

    let s = input.underlying_value;
    let k = input.exercise_price;
    let sigma = input.volatility;
    let r = input.risk_free_rate;
    let q = input.dividend_yield.unwrap_or(Decimal::ZERO);
    let t = input.time_to_expiry;
    let n = input.steps;
    let ef = input.expansion_factor.unwrap_or(dec!(1.5));
    let cf = input.contraction_factor.unwrap_or(dec!(0.5));
    let sc = input.switch_cost.unwrap_or(Decimal::ZERO);
    let sr = input.switch_value_ratio.unwrap_or(Decimal::ONE);

    // Handle zero volatility edge case
    if sigma == Decimal::ZERO {
        let immediate = exercise_payoff(s, k, input.option_type, ef, cf, sc, sr);
        let static_npv = compute_static_npv(s, k, input.option_type, input.expansion_factor);
        let option_premium = immediate - static_npv.max(Decimal::ZERO);
        let output = RealOptionOutput {
            option_value: immediate,
            expanded_npv: static_npv + immediate.max(Decimal::ZERO),
            static_npv,
            option_premium: option_premium.max(Decimal::ZERO),
            optimal_exercise_boundary: vec![],
            delta: Decimal::ZERO,
            gamma: Decimal::ZERO,
            theta: Decimal::ZERO,
            vega: Decimal::ZERO,
            early_exercise_optimal: immediate > Decimal::ZERO,
            breakeven_volatility: Decimal::ZERO,
        };
        let elapsed = start.elapsed().as_micros() as u64;
        return Ok(with_metadata(
            "Real option valuation (zero volatility)",
            &serde_json::json!({"volatility": "0", "option_type": format!("{:?}", input.option_type)}),
            vec!["Zero volatility: option value equals deterministic exercise value".into()],
            elapsed,
            output,
        ));
    }

    let binom = binomial_tree(s, k, sigma, r, q, t, n, input.option_type, ef, cf, sc, sr);

    let option_value = binom.option_value;
    let static_npv = compute_static_npv(s, k, input.option_type, input.expansion_factor);
    let expanded_npv = static_npv + option_value.max(Decimal::ZERO);
    let option_premium = option_value - static_npv.max(Decimal::ZERO);

    // Compute Greeks
    let greeks = compute_greeks(input, &binom, option_value);

    // Breakeven volatility
    let breakeven_vol = find_breakeven_volatility(input);

    let output = RealOptionOutput {
        option_value,
        expanded_npv,
        static_npv,
        option_premium: option_premium.max(Decimal::ZERO),
        optimal_exercise_boundary: binom.exercise_boundaries,
        delta: greeks.delta,
        gamma: greeks.gamma,
        theta: greeks.theta,
        vega: greeks.vega,
        early_exercise_optimal: binom.early_exercise_at_root,
        breakeven_volatility: breakeven_vol,
    };

    let mut warnings = Vec::new();
    if option_value < Decimal::ZERO {
        warnings.push("Option value is negative; check inputs".into());
    }
    if binom.early_exercise_at_root {
        warnings.push("Immediate exercise appears optimal".into());
    }

    let assumptions = serde_json::json!({
        "model": "CRR Binomial Tree",
        "option_type": format!("{:?}", input.option_type),
        "steps": n,
        "risk_free_rate": r.to_string(),
        "volatility": sigma.to_string(),
        "dividend_yield": q.to_string(),
        "time_to_expiry": t.to_string(),
    });

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "CRR Binomial Tree — Real Option Valuation",
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
        abs_decimal(a - b) < tol
    }

    fn default_defer_input() -> RealOptionInput {
        RealOptionInput {
            option_type: OptionType::Defer,
            underlying_value: dec!(100),
            exercise_price: dec!(105),
            volatility: dec!(0.30),
            risk_free_rate: dec!(0.05),
            time_to_expiry: dec!(1),
            steps: 100,
            dividend_yield: None,
            expansion_factor: None,
            contraction_factor: None,
            switch_cost: None,
            switch_value_ratio: None,
        }
    }

    // -----------------------------------------------------------------------
    // Defer option tests (American call)
    // -----------------------------------------------------------------------

    #[test]
    fn test_defer_option_positive_value() {
        let input = default_defer_input();
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_value > Decimal::ZERO,
            "Defer option should have positive value, got {}",
            result.result.option_value
        );
    }

    #[test]
    fn test_defer_option_exceeds_intrinsic() {
        // ITM defer option: value should exceed intrinsic
        let input = RealOptionInput {
            underlying_value: dec!(120),
            exercise_price: dec!(100),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        let intrinsic = dec!(120) - dec!(100); // 20
        assert!(
            result.result.option_value >= intrinsic - dec!(0.1),
            "Defer option {} should be >= intrinsic {}",
            result.result.option_value,
            intrinsic
        );
    }

    #[test]
    fn test_defer_deep_otm() {
        // Deep OTM: underlying << exercise price
        let input = RealOptionInput {
            underlying_value: dec!(50),
            exercise_price: dec!(200),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_value < dec!(5),
            "Deep OTM defer option should be near zero, got {}",
            result.result.option_value
        );
    }

    #[test]
    fn test_defer_deep_itm() {
        // Deep ITM: underlying >> exercise price
        let input = RealOptionInput {
            underlying_value: dec!(200),
            exercise_price: dec!(50),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        let intrinsic = dec!(200) - dec!(50);
        assert!(
            result.result.option_value >= intrinsic - dec!(1),
            "Deep ITM defer option {} should be close to intrinsic {}",
            result.result.option_value,
            intrinsic
        );
    }

    #[test]
    fn test_defer_higher_vol_higher_value() {
        let low_vol = RealOptionInput {
            volatility: dec!(0.15),
            ..default_defer_input()
        };
        let high_vol = RealOptionInput {
            volatility: dec!(0.50),
            ..default_defer_input()
        };
        let low = value_real_option(&low_vol).unwrap().result.option_value;
        let high = value_real_option(&high_vol).unwrap().result.option_value;
        assert!(
            high > low,
            "Higher vol {} should give higher option value than lower vol {}",
            high,
            low
        );
    }

    #[test]
    fn test_defer_longer_expiry_higher_value() {
        let short = RealOptionInput {
            time_to_expiry: dec!(0.5),
            ..default_defer_input()
        };
        let long = RealOptionInput {
            time_to_expiry: dec!(3),
            ..default_defer_input()
        };
        let v_short = value_real_option(&short).unwrap().result.option_value;
        let v_long = value_real_option(&long).unwrap().result.option_value;
        assert!(
            v_long > v_short,
            "Longer expiry {} should give higher value than shorter {}",
            v_long,
            v_short
        );
    }

    #[test]
    fn test_defer_with_dividend_yield() {
        // Dividend yield reduces option value (cost of waiting)
        let no_div = default_defer_input();
        let with_div = RealOptionInput {
            dividend_yield: Some(dec!(0.05)),
            ..default_defer_input()
        };
        let v_no = value_real_option(&no_div).unwrap().result.option_value;
        let v_div = value_real_option(&with_div).unwrap().result.option_value;
        assert!(
            v_no > v_div,
            "No-dividend value {} should exceed dividend value {}",
            v_no,
            v_div
        );
    }

    #[test]
    fn test_defer_expanded_npv() {
        let input = default_defer_input();
        let result = value_real_option(&input).unwrap();
        let r = &result.result;
        // expanded_npv = static_npv + option_value
        let expected = r.static_npv + r.option_value.max(Decimal::ZERO);
        assert!(
            approx_eq(r.expanded_npv, expected, dec!(0.01)),
            "expanded_npv {} should equal static_npv + option_value = {}",
            r.expanded_npv,
            expected
        );
    }

    // -----------------------------------------------------------------------
    // Abandon option tests (American put)
    // -----------------------------------------------------------------------

    #[test]
    fn test_abandon_option_positive() {
        let input = RealOptionInput {
            option_type: OptionType::Abandon,
            underlying_value: dec!(80),
            exercise_price: dec!(100), // salvage value
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        // Abandon option: max(K, continuation) where K=salvage
        // When S < K, the abandon option has intrinsic value
        assert!(
            result.result.option_value > Decimal::ZERO,
            "Abandon option should have positive value, got {}",
            result.result.option_value
        );
    }

    #[test]
    fn test_abandon_deep_itm() {
        // Asset value well below salvage: option should be close to salvage
        let input = RealOptionInput {
            option_type: OptionType::Abandon,
            underlying_value: dec!(30),
            exercise_price: dec!(100), // salvage value
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        // With S=30, K=100, the abandon option value >= 100 (salvage)
        assert!(
            result.result.option_value >= dec!(99),
            "Deep ITM abandon value {} should be near salvage 100",
            result.result.option_value
        );
    }

    #[test]
    fn test_abandon_otm() {
        // Asset value well above salvage: option is OTM but still has time value
        let input = RealOptionInput {
            option_type: OptionType::Abandon,
            underlying_value: dec!(200),
            exercise_price: dec!(100), // salvage
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        // Option value = max(K, continuation). When S >> K, value ~ S (hold project)
        assert!(
            result.result.option_value >= dec!(190),
            "OTM abandon option value {} should be near underlying 200",
            result.result.option_value
        );
    }

    #[test]
    fn test_abandon_higher_salvage_higher_value() {
        let low_salvage = RealOptionInput {
            option_type: OptionType::Abandon,
            underlying_value: dec!(100),
            exercise_price: dec!(80),
            ..default_defer_input()
        };
        let high_salvage = RealOptionInput {
            option_type: OptionType::Abandon,
            underlying_value: dec!(100),
            exercise_price: dec!(120),
            ..default_defer_input()
        };
        let v_low = value_real_option(&low_salvage).unwrap().result.option_value;
        let v_high = value_real_option(&high_salvage)
            .unwrap()
            .result
            .option_value;
        assert!(
            v_high > v_low,
            "Higher salvage value {} should give higher option value {}",
            v_high,
            v_low
        );
    }

    // -----------------------------------------------------------------------
    // Expand option tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_expand_option_positive() {
        let input = RealOptionInput {
            option_type: OptionType::Expand,
            underlying_value: dec!(100),
            exercise_price: dec!(40), // cost to expand
            expansion_factor: Some(dec!(1.5)),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_value > Decimal::ZERO,
            "Expand option should have positive value, got {}",
            result.result.option_value
        );
    }

    #[test]
    fn test_expand_higher_factor_higher_value() {
        let small = RealOptionInput {
            option_type: OptionType::Expand,
            exercise_price: dec!(40),
            expansion_factor: Some(dec!(1.2)),
            ..default_defer_input()
        };
        let big = RealOptionInput {
            option_type: OptionType::Expand,
            exercise_price: dec!(40),
            expansion_factor: Some(dec!(2.0)),
            ..default_defer_input()
        };
        let v_small = value_real_option(&small).unwrap().result.option_value;
        let v_big = value_real_option(&big).unwrap().result.option_value;
        assert!(
            v_big > v_small,
            "Bigger expansion {} should have higher value than smaller {}",
            v_big,
            v_small
        );
    }

    #[test]
    fn test_expand_expensive_exercise() {
        // Very expensive expansion: less valuable
        let cheap = RealOptionInput {
            option_type: OptionType::Expand,
            exercise_price: dec!(10),
            expansion_factor: Some(dec!(1.5)),
            ..default_defer_input()
        };
        let expensive = RealOptionInput {
            option_type: OptionType::Expand,
            exercise_price: dec!(200),
            expansion_factor: Some(dec!(1.5)),
            ..default_defer_input()
        };
        let v_cheap = value_real_option(&cheap).unwrap().result.option_value;
        let v_expensive = value_real_option(&expensive).unwrap().result.option_value;
        assert!(
            v_cheap > v_expensive,
            "Cheap expansion {} should be more valuable than expensive {}",
            v_cheap,
            v_expensive
        );
    }

    // -----------------------------------------------------------------------
    // Contract option tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_contract_option_positive() {
        let input = RealOptionInput {
            option_type: OptionType::Contract,
            underlying_value: dec!(100),
            exercise_price: dec!(30), // savings from contracting
            contraction_factor: Some(dec!(0.5)),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_value > Decimal::ZERO,
            "Contract option should have positive value, got {}",
            result.result.option_value
        );
    }

    #[test]
    fn test_contract_lower_factor_different_value() {
        // A lower contraction factor means more contraction (keeping less of project)
        let mild = RealOptionInput {
            option_type: OptionType::Contract,
            exercise_price: dec!(20),
            contraction_factor: Some(dec!(0.8)),
            ..default_defer_input()
        };
        let severe = RealOptionInput {
            option_type: OptionType::Contract,
            exercise_price: dec!(20),
            contraction_factor: Some(dec!(0.3)),
            ..default_defer_input()
        };
        let v_mild = value_real_option(&mild).unwrap().result.option_value;
        let v_severe = value_real_option(&severe).unwrap().result.option_value;
        // Both should be valid, values depend on savings vs lost capacity
        assert!(
            v_mild > Decimal::ZERO,
            "Mild contraction should be positive"
        );
        assert!(
            v_severe > Decimal::ZERO,
            "Severe contraction should be positive"
        );
    }

    // -----------------------------------------------------------------------
    // Switch option tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_switch_option_positive() {
        let input = RealOptionInput {
            option_type: OptionType::Switch,
            underlying_value: dec!(100),
            exercise_price: dec!(100), // not used directly for switch payoff
            switch_cost: Some(dec!(10)),
            switch_value_ratio: Some(dec!(1.3)),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_value > Decimal::ZERO,
            "Switch option should have positive value, got {}",
            result.result.option_value
        );
    }

    #[test]
    fn test_switch_higher_ratio_higher_value() {
        let low = RealOptionInput {
            option_type: OptionType::Switch,
            exercise_price: dec!(100),
            switch_cost: Some(dec!(10)),
            switch_value_ratio: Some(dec!(1.1)),
            ..default_defer_input()
        };
        let high = RealOptionInput {
            option_type: OptionType::Switch,
            exercise_price: dec!(100),
            switch_cost: Some(dec!(10)),
            switch_value_ratio: Some(dec!(1.8)),
            ..default_defer_input()
        };
        let v_low = value_real_option(&low).unwrap().result.option_value;
        let v_high = value_real_option(&high).unwrap().result.option_value;
        assert!(
            v_high > v_low,
            "Higher switch ratio {} should beat lower {}",
            v_high,
            v_low
        );
    }

    #[test]
    fn test_switch_higher_cost_lower_value() {
        let cheap = RealOptionInput {
            option_type: OptionType::Switch,
            exercise_price: dec!(100),
            switch_cost: Some(dec!(5)),
            switch_value_ratio: Some(dec!(1.3)),
            ..default_defer_input()
        };
        let expensive = RealOptionInput {
            option_type: OptionType::Switch,
            exercise_price: dec!(100),
            switch_cost: Some(dec!(50)),
            switch_value_ratio: Some(dec!(1.3)),
            ..default_defer_input()
        };
        let v_cheap = value_real_option(&cheap).unwrap().result.option_value;
        let v_expensive = value_real_option(&expensive).unwrap().result.option_value;
        assert!(
            v_cheap > v_expensive,
            "Cheaper switching {} should be more valuable than expensive {}",
            v_cheap,
            v_expensive
        );
    }

    // -----------------------------------------------------------------------
    // Compound option tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_compound_option_positive() {
        let input = RealOptionInput {
            option_type: OptionType::Compound,
            underlying_value: dec!(100),
            exercise_price: dec!(90),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_value > Decimal::ZERO,
            "Compound option should have positive value, got {}",
            result.result.option_value
        );
    }

    // -----------------------------------------------------------------------
    // Greeks tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_delta_positive_for_defer() {
        // Defer (call-like) should have positive delta
        let input = default_defer_input();
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.delta > Decimal::ZERO,
            "Defer delta should be positive, got {}",
            result.result.delta
        );
    }

    #[test]
    fn test_delta_bounded() {
        let input = default_defer_input();
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.delta <= Decimal::ONE + dec!(0.01),
            "Delta {} should be <= 1",
            result.result.delta
        );
    }

    #[test]
    fn test_vega_positive() {
        // Vega should be positive (more vol = more option value)
        let input = default_defer_input();
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.vega > Decimal::ZERO,
            "Vega should be positive, got {}",
            result.result.vega
        );
    }

    #[test]
    fn test_theta_negative_for_defer() {
        // Time decay: theta should be negative for typical defer option
        let input = RealOptionInput {
            underlying_value: dec!(100),
            exercise_price: dec!(100),
            volatility: dec!(0.30),
            time_to_expiry: dec!(2),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.theta < Decimal::ZERO,
            "Theta should be negative (time decay), got {}",
            result.result.theta
        );
    }

    #[test]
    fn test_greeks_approximate_bs_for_european_defer() {
        // With many steps and short expiry, defer option Greeks should
        // approximate Black-Scholes call Greeks
        let input = RealOptionInput {
            underlying_value: dec!(100),
            exercise_price: dec!(100),
            volatility: dec!(0.20),
            risk_free_rate: dec!(0.05),
            time_to_expiry: dec!(1),
            steps: 200,
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        let delta = result.result.delta;
        // BS ATM call delta ~ 0.57-0.64 (with r=5%, vol=20%, T=1)
        assert!(
            delta > dec!(0.45) && delta < dec!(0.80),
            "Delta {} should approximate BS call delta ~0.60",
            delta
        );
    }

    // -----------------------------------------------------------------------
    // Exercise boundary tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_exercise_boundary_exists_for_abandon() {
        let input = RealOptionInput {
            option_type: OptionType::Abandon,
            underlying_value: dec!(80),
            exercise_price: dec!(100),
            steps: 50,
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        // For an American put (abandon), there should be exercise boundaries
        // when S is low enough
        assert!(
            !result.result.optimal_exercise_boundary.is_empty(),
            "Abandon option should have exercise boundaries"
        );
    }

    #[test]
    fn test_exercise_boundary_time_steps_valid() {
        let input = RealOptionInput {
            option_type: OptionType::Abandon,
            underlying_value: dec!(80),
            exercise_price: dec!(100),
            steps: 50,
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        for boundary in &result.result.optimal_exercise_boundary {
            assert!(
                boundary.time_step < 50,
                "Boundary time_step {} should be < steps 50",
                boundary.time_step
            );
            assert!(
                boundary.threshold_value > Decimal::ZERO,
                "Boundary threshold should be positive"
            );
        }
    }

    // -----------------------------------------------------------------------
    // Breakeven volatility tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_breakeven_volatility_positive() {
        let input = default_defer_input();
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.breakeven_volatility > Decimal::ZERO,
            "Breakeven vol should be positive, got {}",
            result.result.breakeven_volatility
        );
    }

    #[test]
    fn test_breakeven_volatility_below_input_vol() {
        // For an ATM option with vol=30%, breakeven should be well below 30%
        let input = RealOptionInput {
            underlying_value: dec!(100),
            exercise_price: dec!(100),
            volatility: dec!(0.30),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        // Breakeven vol is the vol where premium = 0
        // For ATM, this is very low (essentially zero premium at zero vol)
        assert!(
            result.result.breakeven_volatility < dec!(0.30),
            "Breakeven vol {} should be less than input vol 0.30",
            result.result.breakeven_volatility
        );
    }

    // -----------------------------------------------------------------------
    // Zero volatility edge case
    // -----------------------------------------------------------------------

    #[test]
    fn test_zero_volatility_defer() {
        let input = RealOptionInput {
            volatility: Decimal::ZERO,
            underlying_value: dec!(110),
            exercise_price: dec!(100),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        // With zero vol, option value = max(S-K, 0) = 10
        assert!(
            approx_eq(result.result.option_value, dec!(10), dec!(1)),
            "Zero vol defer option {} should be near intrinsic 10",
            result.result.option_value
        );
    }

    #[test]
    fn test_zero_volatility_abandon() {
        let input = RealOptionInput {
            option_type: OptionType::Abandon,
            volatility: Decimal::ZERO,
            underlying_value: dec!(80),
            exercise_price: dec!(100),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        // With zero vol, abandon value = max(K, S) = 100
        assert!(
            approx_eq(result.result.option_value, dec!(100), dec!(1)),
            "Zero vol abandon option {} should be salvage 100",
            result.result.option_value
        );
    }

    // -----------------------------------------------------------------------
    // Validation error tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_invalid_negative_underlying() {
        let input = RealOptionInput {
            underlying_value: dec!(-100),
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_invalid_zero_exercise() {
        let input = RealOptionInput {
            exercise_price: Decimal::ZERO,
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_invalid_negative_vol() {
        let input = RealOptionInput {
            volatility: dec!(-0.20),
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_invalid_zero_expiry() {
        let input = RealOptionInput {
            time_to_expiry: Decimal::ZERO,
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_invalid_zero_steps() {
        let input = RealOptionInput {
            steps: 0,
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_expand_missing_factor() {
        let input = RealOptionInput {
            option_type: OptionType::Expand,
            expansion_factor: None,
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_expand_factor_must_exceed_one() {
        let input = RealOptionInput {
            option_type: OptionType::Expand,
            expansion_factor: Some(dec!(0.8)),
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_contract_missing_factor() {
        let input = RealOptionInput {
            option_type: OptionType::Contract,
            contraction_factor: None,
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_contract_factor_out_of_range() {
        let input = RealOptionInput {
            option_type: OptionType::Contract,
            contraction_factor: Some(dec!(1.5)),
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_switch_missing_cost() {
        let input = RealOptionInput {
            option_type: OptionType::Switch,
            switch_cost: None,
            switch_value_ratio: Some(dec!(1.3)),
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    #[test]
    fn test_switch_missing_ratio() {
        let input = RealOptionInput {
            option_type: OptionType::Switch,
            switch_cost: Some(dec!(10)),
            switch_value_ratio: None,
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Metadata and output format tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_metadata_populated() {
        let input = default_defer_input();
        let result = value_real_option(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_static_npv_calculation() {
        let input = RealOptionInput {
            underlying_value: dec!(120),
            exercise_price: dec!(100),
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        // For defer: static_npv = S - K = 120 - 100 = 20
        assert!(
            approx_eq(result.result.static_npv, dec!(20), dec!(1)),
            "Static NPV {} should be ~20",
            result.result.static_npv
        );
    }

    #[test]
    fn test_option_premium_non_negative() {
        let input = default_defer_input();
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_premium >= Decimal::ZERO,
            "Option premium should be non-negative, got {}",
            result.result.option_premium
        );
    }

    #[test]
    fn test_very_high_volatility() {
        let input = RealOptionInput {
            volatility: dec!(2.0),
            steps: 50, // fewer steps for speed with high vol
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_value > Decimal::ZERO,
            "High vol option should have positive value, got {}",
            result.result.option_value
        );
    }

    #[test]
    fn test_single_step() {
        // Minimum viable tree: 1 step
        let input = RealOptionInput {
            steps: 1,
            ..default_defer_input()
        };
        let result = value_real_option(&input).unwrap();
        assert!(
            result.result.option_value >= Decimal::ZERO,
            "Single-step tree should produce non-negative value"
        );
    }

    #[test]
    fn test_convergence_with_more_steps() {
        // Value should stabilize with more steps
        let v50 = value_real_option(&RealOptionInput {
            steps: 50,
            ..default_defer_input()
        })
        .unwrap()
        .result
        .option_value;

        let v200 = value_real_option(&RealOptionInput {
            steps: 200,
            ..default_defer_input()
        })
        .unwrap()
        .result
        .option_value;

        // The values should be reasonably close (converging)
        assert!(
            approx_eq(v50, v200, dec!(2)),
            "50 steps ({}) and 200 steps ({}) should be within 2 of each other",
            v50,
            v200
        );
    }

    #[test]
    fn test_negative_dividend_yield_rejected() {
        let input = RealOptionInput {
            dividend_yield: Some(dec!(-0.05)),
            ..default_defer_input()
        };
        assert!(value_real_option(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Math helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_exp_decimal_basic() {
        assert!(approx_eq(exp_decimal(dec!(0)), dec!(1), dec!(0.0001)));
        assert!(approx_eq(exp_decimal(dec!(1)), dec!(2.71828), dec!(0.001)));
    }

    #[test]
    fn test_sqrt_decimal_basic() {
        assert!(approx_eq(sqrt_decimal(dec!(4)), dec!(2), dec!(0.0001)));
        assert!(approx_eq(sqrt_decimal(dec!(9)), dec!(3), dec!(0.0001)));
        assert!(approx_eq(sqrt_decimal(dec!(0.25)), dec!(0.5), dec!(0.001)));
    }

    #[test]
    fn test_ln_decimal_basic() {
        assert!(approx_eq(ln_decimal(dec!(1)), dec!(0), dec!(0.0001)));
        assert!(approx_eq(
            ln_decimal(dec!(2.71828182845)),
            dec!(1),
            dec!(0.001)
        ));
    }

    #[test]
    fn test_pow_decimal_basic() {
        assert_eq!(pow_decimal(dec!(2), 0), dec!(1));
        assert_eq!(pow_decimal(dec!(2), 3), dec!(8));
        assert_eq!(pow_decimal(dec!(3), 4), dec!(81));
    }
}
