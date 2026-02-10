use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A point on the spot/discount curve: maturity in year fractions, rate as decimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscountPoint {
    pub maturity: Decimal,
    pub rate: Rate,
}

/// A point on the forward rate curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForwardRatePoint {
    pub maturity: Decimal,
    pub rate: Rate,
}

/// Input for interest-rate swap valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrsInput {
    pub notional: Money,
    pub fixed_rate: Rate,
    pub payment_frequency: u8,
    pub remaining_years: Decimal,
    pub discount_curve: Vec<DiscountPoint>,
    pub forward_rates: Option<Vec<ForwardRatePoint>>,
    pub is_pay_fixed: bool,
    pub last_floating_reset: Option<Rate>,
}

/// Single payment in the swap schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapPayment {
    pub period: u32,
    pub payment_date_years: Decimal,
    pub fixed_payment: Money,
    pub floating_payment: Money,
    pub net_payment: Money,
    pub discount_factor: Decimal,
}

/// Output from IRS valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrsOutput {
    pub fixed_leg_pv: Money,
    pub floating_leg_pv: Money,
    pub net_value: Money,
    pub par_swap_rate: Rate,
    pub dv01: Money,
    pub payment_schedule: Vec<SwapPayment>,
    pub annuity_factor: Decimal,
}

/// Input for cross-currency swap valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencySwapInput {
    pub notional_domestic: Money,
    pub notional_foreign: Money,
    pub domestic_fixed_rate: Rate,
    pub foreign_fixed_rate: Rate,
    pub payment_frequency: u8,
    pub remaining_years: Decimal,
    pub domestic_discount_curve: Vec<DiscountPoint>,
    pub foreign_discount_curve: Vec<DiscountPoint>,
    pub spot_fx_rate: Decimal,
    pub is_pay_domestic: bool,
}

/// Output from currency swap valuation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencySwapOutput {
    pub domestic_leg_pv: Money,
    pub foreign_leg_pv: Money,
    pub net_value: Money,
    pub implied_fx_forward: Decimal,
    pub fx_exposure: Money,
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64, no MathematicalOps)
// ---------------------------------------------------------------------------

/// Raise `base` to an integer power using iterative multiplication.
fn pow_int(base: Decimal, n: u32) -> Decimal {
    if n == 0 {
        return Decimal::ONE;
    }
    let mut result = Decimal::ONE;
    let mut b = base;
    let mut exp = n;
    // Square-and-multiply for efficiency
    while exp > 0 {
        if exp & 1 == 1 {
            result *= b;
        }
        b *= b;
        exp >>= 1;
    }
    result
}

/// Taylor-series exponential for Decimal, 25 terms with range reduction for |x| > 2.
/// exp(x) = e^x using: if x is large, split x = k*ln2 + r and compute 2^k * exp(r).
fn exp_decimal(x: Decimal) -> Decimal {
    // ln(2) to high precision
    let ln2 = dec!(0.6931471805599453094172321);
    let two = Decimal::from(2);

    // Range reduction: express x = k * ln2 + r, |r| <= ln2/2
    let mut k: i64 = 0;
    let mut r = x;
    if r.abs() > two {
        // k = round(x / ln2)
        let k_approx = x / ln2;
        k = decimal_to_i64_round(k_approx);
        r = x - Decimal::from(k) * ln2;
    }

    // Taylor series: exp(r) = sum_{n=0}^{24} r^n / n!
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=25 {
        term *= r / Decimal::from(n);
        sum += term;
        // Early exit if term contribution is negligible
        if term.abs() < dec!(0.00000000000000000001) {
            break;
        }
    }

    // Reconstruct: exp(x) = 2^k * exp(r)
    if k >= 0 {
        let factor = pow_int(two, k as u32);
        sum * factor
    } else {
        let factor = pow_int(two, (-k) as u32);
        sum / factor
    }
}

/// Round a Decimal toward nearest integer and return as i64.
fn decimal_to_i64_round(d: Decimal) -> i64 {
    let half = dec!(0.5);
    if d >= Decimal::ZERO {
        let floored = d.floor();
        if d - floored >= half {
            (floored + Decimal::ONE).to_string().parse().unwrap_or(0)
        } else {
            floored.to_string().parse().unwrap_or(0)
        }
    } else {
        let ceiled = d.ceil();
        if ceiled - d >= half {
            (ceiled - Decimal::ONE).to_string().parse().unwrap_or(0)
        } else {
            ceiled.to_string().parse().unwrap_or(0)
        }
    }
}

/// Raise (1 + rate) to a fractional exponent t using exp/ln decomposition.
/// (1+r)^t = exp(t * ln(1+r))
/// ln(1+r) via Taylor series for |r| < 1: ln(1+r) = r - r^2/2 + r^3/3 - ...
fn compound_factor(rate: Decimal, t: Decimal) -> Decimal {
    if t == Decimal::ZERO {
        return Decimal::ONE;
    }
    // For integer t, use iterative multiplication (more precise)
    if t == t.floor() && t > Decimal::ZERO && t <= Decimal::from(100) {
        return pow_int(
            Decimal::ONE + rate,
            t.to_string().parse::<u32>().unwrap_or(0),
        );
    }
    // Fractional: exp(t * ln(1+r))
    let ln_val = ln_one_plus(rate);
    exp_decimal(t * ln_val)
}

/// ln(1+x) via Taylor series for small x; for larger x uses range reduction.
/// ln(1+x) = x - x^2/2 + x^3/3 - x^4/4 + ...  (converges for -1 < x <= 1)
fn ln_one_plus(x: Decimal) -> Decimal {
    // For values where |x| could be large, use a different approach.
    // For spot rates we typically have |x| < 0.5, so direct Taylor works well.
    // But to be safe, handle larger values.
    if x <= dec!(-1) {
        // Not defined for x <= -1
        return Decimal::ZERO;
    }

    // For x > 1 or x in (-1, -0.5), use atanh identity which converges for all x > -1.
    // ln(1+x) = 2*atanh(x/(x+2)), where |x/(x+2)| < 1 for all x > -1.
    if x > Decimal::ONE {
        let y = x / (x + Decimal::from(2));
        return two_atanh(y);
    }
    if x < dec!(-0.5) {
        // ln(1+x) = -ln(1/(1+x)) = -ln(1 + (-x/(1+x)))
        let inner = -x / (Decimal::ONE + x);
        return -ln_one_plus(inner);
    }

    // Direct Taylor for |x| <= 1
    // Use the faster converging form: ln(1+x) = 2*atanh(x/(x+2))
    let y = x / (x + Decimal::from(2));
    two_atanh(y)
}

/// Compute 2*atanh(y) = 2*(y + y^3/3 + y^5/5 + ...) for |y| < 1.
/// This equals ln((1+y)/(1-y)).
fn two_atanh(y: Decimal) -> Decimal {
    let y2 = y * y;
    let mut term = y;
    let mut sum = y;
    for k in 1u32..=40 {
        term *= y2;
        let denom = Decimal::from(2 * k + 1);
        sum += term / denom;
        if (term / denom).abs() < dec!(0.00000000000000000001) {
            break;
        }
    }
    sum * Decimal::from(2)
}

// ---------------------------------------------------------------------------
// Discount-factor and forward-rate utilities
// ---------------------------------------------------------------------------

/// Compute discount factor from a spot rate: DF(t) = 1 / (1 + s)^t
fn discount_factor(spot_rate: Decimal, t: Decimal) -> Decimal {
    let denom = compound_factor(spot_rate, t);
    if denom.is_zero() {
        return Decimal::ZERO;
    }
    Decimal::ONE / denom
}

/// Interpolate a rate for time `t` from a sorted curve. Uses linear interpolation.
fn interpolate_rate(curve: &[DiscountPoint], t: Decimal) -> Decimal {
    if curve.is_empty() {
        return Decimal::ZERO;
    }
    if curve.len() == 1 || t <= curve[0].maturity {
        return curve[0].rate;
    }
    if t >= curve[curve.len() - 1].maturity {
        return curve[curve.len() - 1].rate;
    }
    // Find bracketing points
    for i in 0..curve.len() - 1 {
        if t >= curve[i].maturity && t <= curve[i + 1].maturity {
            let t1 = curve[i].maturity;
            let t2 = curve[i + 1].maturity;
            let r1 = curve[i].rate;
            let r2 = curve[i + 1].rate;
            let span = t2 - t1;
            if span.is_zero() {
                return r1;
            }
            return r1 + (r2 - r1) * (t - t1) / span;
        }
    }
    curve[curve.len() - 1].rate
}

/// Derive a forward rate f(t1, t2) from the spot curve.
/// f(t1,t2) = ((1+s2)^t2 / (1+s1)^t1)^(1/(t2-t1)) - 1
fn implied_forward_rate(curve: &[DiscountPoint], t1: Decimal, t2: Decimal) -> Decimal {
    let s1 = interpolate_rate(curve, t1);
    let s2 = interpolate_rate(curve, t2);
    let num = compound_factor(s2, t2);
    let den = compound_factor(s1, t1);
    if den.is_zero() {
        return Decimal::ZERO;
    }
    let ratio = num / den;
    let dt = t2 - t1;
    if dt.is_zero() {
        return s1;
    }
    // ratio^(1/dt) - 1
    // = exp(ln(ratio) / dt) - 1
    // ln(ratio) = ln(1 + (ratio - 1))
    let ln_ratio = ln_one_plus(ratio - Decimal::ONE);
    exp_decimal(ln_ratio / dt) - Decimal::ONE
}

// ---------------------------------------------------------------------------
// Validation helpers
// ---------------------------------------------------------------------------

fn validate_irs_input(input: &IrsInput) -> CorpFinanceResult<()> {
    if input.notional <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional".into(),
            reason: "Notional must be positive".into(),
        });
    }
    if ![1, 2, 4].contains(&input.payment_frequency) {
        return Err(CorpFinanceError::InvalidInput {
            field: "payment_frequency".into(),
            reason: "Payment frequency must be 1, 2, or 4".into(),
        });
    }
    if input.remaining_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_years".into(),
            reason: "Remaining years must be positive".into(),
        });
    }
    if input.discount_curve.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Discount curve must have at least one point".into(),
        ));
    }
    Ok(())
}

fn validate_currency_swap_input(input: &CurrencySwapInput) -> CorpFinanceResult<()> {
    if input.notional_domestic <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional_domestic".into(),
            reason: "Domestic notional must be positive".into(),
        });
    }
    if input.notional_foreign <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional_foreign".into(),
            reason: "Foreign notional must be positive".into(),
        });
    }
    if ![1, 2, 4].contains(&input.payment_frequency) {
        return Err(CorpFinanceError::InvalidInput {
            field: "payment_frequency".into(),
            reason: "Payment frequency must be 1, 2, or 4".into(),
        });
    }
    if input.remaining_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_years".into(),
            reason: "Remaining years must be positive".into(),
        });
    }
    if input.domestic_discount_curve.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Domestic discount curve must have at least one point".into(),
        ));
    }
    if input.foreign_discount_curve.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Foreign discount curve must have at least one point".into(),
        ));
    }
    if input.spot_fx_rate <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "spot_fx_rate".into(),
            reason: "Spot FX rate must be positive".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Core IRS engine (shared by public function and DV01 bump)
// ---------------------------------------------------------------------------

/// Internal result of swap leg computation (no metadata wrapping).
struct IrsLegsResult {
    fixed_leg_pv: Money,
    floating_leg_pv: Money,
    net_value: Money,
    par_swap_rate: Rate,
    annuity_factor: Decimal,
    schedule: Vec<SwapPayment>,
}

/// Compute swap legs given an explicit discount curve (may be bumped for DV01).
fn compute_irs_legs(input: &IrsInput, curve: &[DiscountPoint]) -> IrsLegsResult {
    let freq = Decimal::from(input.payment_frequency);
    let period_length = Decimal::ONE / freq;
    let num_periods_dec = input.remaining_years * freq;
    // Round to nearest integer, handle fractional gracefully
    let num_periods = num_periods_dec
        .round()
        .to_string()
        .parse::<u32>()
        .unwrap_or(0);

    let mut fixed_leg_pv = Decimal::ZERO;
    let mut floating_leg_pv = Decimal::ZERO;
    let mut annuity_sum = Decimal::ZERO;
    let mut schedule = Vec::with_capacity(num_periods as usize);

    for i in 1..=num_periods {
        let t = Decimal::from(i) * period_length;
        let t_prev = Decimal::from(i - 1) * period_length;

        // Discount factor at payment date
        let spot = interpolate_rate(curve, t);
        let df = discount_factor(spot, t);

        // Fixed payment for this period
        let fixed_pmt = input.notional * input.fixed_rate / freq;

        // Floating rate for this period
        let fwd = if i == 1 {
            if let Some(reset) = input.last_floating_reset {
                reset
            } else if let Some(ref fwds) = input.forward_rates {
                interpolate_rate_from_forwards(fwds, t)
            } else {
                implied_forward_rate(curve, t_prev, t)
            }
        } else if let Some(ref fwds) = input.forward_rates {
            interpolate_rate_from_forwards(fwds, t)
        } else {
            implied_forward_rate(curve, t_prev, t)
        };

        let float_pmt = input.notional * fwd / freq;

        // Net payment from perspective of pay-fixed party
        let net = if input.is_pay_fixed {
            float_pmt - fixed_pmt
        } else {
            fixed_pmt - float_pmt
        };

        fixed_leg_pv += fixed_pmt * df;
        floating_leg_pv += float_pmt * df;
        annuity_sum += df;

        schedule.push(SwapPayment {
            period: i,
            payment_date_years: t,
            fixed_payment: fixed_pmt,
            floating_payment: float_pmt,
            net_payment: net,
            discount_factor: df,
        });
    }

    let annuity_factor = annuity_sum / freq;

    // Par swap rate: R_par = floating_leg_PV / (annuity_sum * notional / freq)
    // R_par = sum(f_i * DF_i) / sum(DF_i)  (already scaled by freq on both sides)
    let par_swap_rate = if annuity_sum.is_zero() {
        Decimal::ZERO
    } else {
        // Sum of forward_i * DF_i over sum of DF_i
        // floating_leg_pv = sum( notional * f_i / freq * DF_i )
        // So sum(f_i * DF_i) = floating_leg_pv * freq / notional
        // par_rate = (floating_leg_pv * freq / notional) / annuity_sum
        if input.notional.is_zero() {
            Decimal::ZERO
        } else {
            floating_leg_pv * freq / (input.notional * annuity_sum)
        }
    };

    let net_value = if input.is_pay_fixed {
        floating_leg_pv - fixed_leg_pv
    } else {
        fixed_leg_pv - floating_leg_pv
    };

    IrsLegsResult {
        fixed_leg_pv,
        floating_leg_pv,
        net_value,
        par_swap_rate,
        annuity_factor,
        schedule,
    }
}

/// Interpolate a forward rate from a supplied forward curve at time `t`.
fn interpolate_rate_from_forwards(fwds: &[ForwardRatePoint], t: Decimal) -> Decimal {
    if fwds.is_empty() {
        return Decimal::ZERO;
    }
    if fwds.len() == 1 || t <= fwds[0].maturity {
        return fwds[0].rate;
    }
    if t >= fwds[fwds.len() - 1].maturity {
        return fwds[fwds.len() - 1].rate;
    }
    for i in 0..fwds.len() - 1 {
        if t >= fwds[i].maturity && t <= fwds[i + 1].maturity {
            let t1 = fwds[i].maturity;
            let t2 = fwds[i + 1].maturity;
            let r1 = fwds[i].rate;
            let r2 = fwds[i + 1].rate;
            let span = t2 - t1;
            if span.is_zero() {
                return r1;
            }
            return r1 + (r2 - r1) * (t - t1) / span;
        }
    }
    fwds[fwds.len() - 1].rate
}

/// Bump all rates in a discount curve by a given amount (in decimal, e.g. 0.0001 for 1bp).
fn bump_curve(curve: &[DiscountPoint], bump: Decimal) -> Vec<DiscountPoint> {
    curve
        .iter()
        .map(|p| DiscountPoint {
            maturity: p.maturity,
            rate: p.rate + bump,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Public API: Interest Rate Swap
// ---------------------------------------------------------------------------

/// Value an interest-rate swap (fixed-for-floating).
///
/// Returns PV of fixed and floating legs, net value from the specified party's
/// perspective, par swap rate, DV01, payment schedule, and annuity factor.
pub fn value_interest_rate_swap(
    input: &IrsInput,
) -> CorpFinanceResult<ComputationOutput<IrsOutput>> {
    let start = Instant::now();
    validate_irs_input(input)?;

    // Base case
    let base = compute_irs_legs(input, &input.discount_curve);

    // DV01: bump all discount rates by +1bp, recompute, take difference
    let one_bp = dec!(0.0001);
    let bumped_curve = bump_curve(&input.discount_curve, one_bp);
    // Also bump forward rates if provided
    let bumped_input = IrsInput {
        discount_curve: bumped_curve.clone(),
        forward_rates: input.forward_rates.as_ref().map(|fwds| {
            fwds.iter()
                .map(|f| ForwardRatePoint {
                    maturity: f.maturity,
                    rate: f.rate + one_bp,
                })
                .collect()
        }),
        last_floating_reset: input.last_floating_reset,
        ..*input
    };
    let bumped = compute_irs_legs(&bumped_input, &bumped_curve);
    let dv01 = (bumped.net_value - base.net_value).abs();

    let output = IrsOutput {
        fixed_leg_pv: base.fixed_leg_pv,
        floating_leg_pv: base.floating_leg_pv,
        net_value: base.net_value,
        par_swap_rate: base.par_swap_rate,
        dv01,
        payment_schedule: base.schedule,
        annuity_factor: base.annuity_factor,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "valuation_method": "discounted_cash_flow",
        "payment_frequency": input.payment_frequency,
        "is_pay_fixed": input.is_pay_fixed,
        "curve_points": input.discount_curve.len(),
        "forward_rates_provided": input.forward_rates.is_some(),
        "last_floating_reset_provided": input.last_floating_reset.is_some(),
    });

    Ok(with_metadata(
        "Interest Rate Swap Valuation (DCF, spot-curve discounting)",
        &assumptions,
        Vec::new(),
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Public API: Currency Swap
// ---------------------------------------------------------------------------

/// Value a cross-currency fixed-for-fixed swap.
///
/// Returns PV of domestic and foreign legs (foreign converted at spot FX),
/// net value, implied FX forward, and FX exposure.
pub fn value_currency_swap(
    input: &CurrencySwapInput,
) -> CorpFinanceResult<ComputationOutput<CurrencySwapOutput>> {
    let start = Instant::now();
    validate_currency_swap_input(input)?;

    let freq = Decimal::from(input.payment_frequency);
    let period_length = Decimal::ONE / freq;
    let num_periods = (input.remaining_years * freq)
        .round()
        .to_string()
        .parse::<u32>()
        .unwrap_or(0);

    // Domestic leg PV: coupon stream + notional at maturity
    let mut dom_leg_pv = Decimal::ZERO;
    for i in 1..=num_periods {
        let t = Decimal::from(i) * period_length;
        let spot = interpolate_rate(&input.domestic_discount_curve, t);
        let df = discount_factor(spot, t);
        let coupon = input.notional_domestic * input.domestic_fixed_rate / freq;
        dom_leg_pv += coupon * df;
        if i == num_periods {
            // Return of notional at maturity
            dom_leg_pv += input.notional_domestic * df;
        }
    }

    // Foreign leg PV (in foreign currency): coupon stream + notional at maturity
    let mut for_leg_pv_foreign = Decimal::ZERO;
    for i in 1..=num_periods {
        let t = Decimal::from(i) * period_length;
        let spot = interpolate_rate(&input.foreign_discount_curve, t);
        let df = discount_factor(spot, t);
        let coupon = input.notional_foreign * input.foreign_fixed_rate / freq;
        for_leg_pv_foreign += coupon * df;
        if i == num_periods {
            for_leg_pv_foreign += input.notional_foreign * df;
        }
    }

    // Convert foreign leg to domestic terms
    let for_leg_pv_domestic = for_leg_pv_foreign * input.spot_fx_rate;

    // Net value from perspective of is_pay_domestic party
    // Pay domestic means: outflow = domestic leg, inflow = foreign leg (converted)
    let net_value = if input.is_pay_domestic {
        for_leg_pv_domestic - dom_leg_pv
    } else {
        dom_leg_pv - for_leg_pv_domestic
    };

    // Implied FX forward: F = S * exp((r_d - r_f) * T)
    // Use the longest maturity rate from each curve as proxy for r_d, r_f
    let r_d = interpolate_rate(&input.domestic_discount_curve, input.remaining_years);
    let r_f = interpolate_rate(&input.foreign_discount_curve, input.remaining_years);
    let implied_fx_forward = input.spot_fx_rate * exp_decimal((r_d - r_f) * input.remaining_years);

    // FX exposure: difference between domestic and converted foreign notional PVs
    let fx_exposure = (for_leg_pv_domestic - dom_leg_pv).abs();

    let output = CurrencySwapOutput {
        domestic_leg_pv: dom_leg_pv,
        foreign_leg_pv: for_leg_pv_domestic,
        net_value,
        implied_fx_forward,
        fx_exposure,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "valuation_method": "discounted_cash_flow_cross_currency",
        "payment_frequency": input.payment_frequency,
        "is_pay_domestic": input.is_pay_domestic,
        "spot_fx_rate": input.spot_fx_rate.to_string(),
        "domestic_curve_points": input.domestic_discount_curve.len(),
        "foreign_curve_points": input.foreign_discount_curve.len(),
    });

    Ok(with_metadata(
        "Cross-Currency Swap Valuation (DCF, dual-curve)",
        &assumptions,
        Vec::new(),
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

    /// Build a flat discount curve at the given rate, with points at each year.
    fn flat_curve(rate: Decimal, max_years: u32) -> Vec<DiscountPoint> {
        (1..=max_years)
            .map(|y| DiscountPoint {
                maturity: Decimal::from(y),
                rate,
            })
            .collect()
    }

    /// Build a flat curve including half-year points for semi-annual / quarterly.
    fn flat_curve_fine(rate: Decimal, max_years: u32, freq: u8) -> Vec<DiscountPoint> {
        let freq_d = Decimal::from(freq);
        let n = max_years * freq as u32;
        (1..=n)
            .map(|i| DiscountPoint {
                maturity: Decimal::from(i) / freq_d,
                rate,
            })
            .collect()
    }

    /// Helper: absolute difference within tolerance.
    fn approx_eq(a: Decimal, b: Decimal, tol: Decimal) -> bool {
        (a - b).abs() <= tol
    }

    // -----------------------------------------------------------------------
    // Test 1: At inception, a swap priced at the par rate has NPV ~ 0
    // -----------------------------------------------------------------------
    #[test]
    fn test_at_inception_value_zero() {
        let curve = flat_curve(dec!(0.05), 5);
        // First compute par rate
        let input_for_par = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.05), // guess; we'll use par_swap_rate
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve.clone(),
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input_for_par).unwrap();
        let par_rate = result.result.par_swap_rate;

        // Now price with the par rate
        let input = IrsInput {
            fixed_rate: par_rate,
            ..input_for_par
        };
        let result2 = value_interest_rate_swap(&input).unwrap();
        assert!(
            approx_eq(result2.result.net_value, Decimal::ZERO, dec!(1.0)),
            "At par rate, NPV should be ~0, got {}",
            result2.result.net_value
        );
    }

    // -----------------------------------------------------------------------
    // Test 2: Pay-fixed party gains when rates rise
    // -----------------------------------------------------------------------
    #[test]
    fn test_pay_fixed_positive_when_rates_rise() {
        // Swap was struck at 3%, but market rates are now 5%
        let curve = flat_curve(dec!(0.05), 5);
        let input = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.03),
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert!(
            result.result.net_value > Decimal::ZERO,
            "Pay-fixed party should gain when rates rise, got {}",
            result.result.net_value
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: Pay-fixed party loses when rates fall
    // -----------------------------------------------------------------------
    #[test]
    fn test_pay_fixed_negative_when_rates_fall() {
        // Swap struck at 5%, market rates dropped to 3%
        let curve = flat_curve(dec!(0.03), 5);
        let input = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.05),
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert!(
            result.result.net_value < Decimal::ZERO,
            "Pay-fixed party should lose when rates fall, got {}",
            result.result.net_value
        );
    }

    // -----------------------------------------------------------------------
    // Test 4: Par swap rate on a flat curve equals the flat rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_par_swap_rate_calculation() {
        let rate = dec!(0.04);
        let curve = flat_curve(rate, 10);
        let input = IrsInput {
            notional: dec!(100_000_000),
            fixed_rate: dec!(0.03), // irrelevant for par rate calc
            payment_frequency: 1,
            remaining_years: dec!(10),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert!(
            approx_eq(result.result.par_swap_rate, rate, dec!(0.0005)),
            "Par swap rate on flat curve should equal the flat rate {}, got {}",
            rate,
            result.result.par_swap_rate
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: Fixed leg PV against manual calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_fixed_leg_pv() {
        let rate = dec!(0.05);
        let curve = flat_curve(rate, 3);
        let notional = dec!(1_000_000);
        let fixed_rate = dec!(0.04);
        let input = IrsInput {
            notional,
            fixed_rate,
            payment_frequency: 1,
            remaining_years: dec!(3),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();

        // Manual: fixed payment = 1M * 0.04 = 40,000 per year
        // DF(1) = 1/1.05, DF(2) = 1/1.05^2, DF(3) = 1/1.05^3
        let df1 = Decimal::ONE / dec!(1.05);
        let df2 = Decimal::ONE / (dec!(1.05) * dec!(1.05));
        let df3 = Decimal::ONE / (dec!(1.05) * dec!(1.05) * dec!(1.05));
        let expected = dec!(40000) * (df1 + df2 + df3);

        assert!(
            approx_eq(result.result.fixed_leg_pv, expected, dec!(1.0)),
            "Fixed leg PV: expected ~{}, got {}",
            expected,
            result.result.fixed_leg_pv
        );
    }

    // -----------------------------------------------------------------------
    // Test 6: Floating leg PV on flat curve
    // -----------------------------------------------------------------------
    #[test]
    fn test_floating_leg_pv() {
        // On a flat curve, forward rates equal the flat rate, so floating leg PV
        // should equal the fixed leg PV priced at that same rate.
        let rate = dec!(0.05);
        let curve = flat_curve(rate, 5);
        let notional = dec!(10_000_000);
        let input = IrsInput {
            notional,
            fixed_rate: rate, // price at par
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert!(
            approx_eq(
                result.result.fixed_leg_pv,
                result.result.floating_leg_pv,
                dec!(100.0)
            ),
            "On flat curve at par, fixed and floating PVs should be close: fixed={}, floating={}",
            result.result.fixed_leg_pv,
            result.result.floating_leg_pv
        );
    }

    // -----------------------------------------------------------------------
    // Test 7: DV01 is positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_dv01_positive() {
        let curve = flat_curve(dec!(0.05), 5);
        let input = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.05),
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert!(
            result.result.dv01 > Decimal::ZERO,
            "DV01 should be positive, got {}",
            result.result.dv01
        );
    }

    // -----------------------------------------------------------------------
    // Test 8: Annuity factor
    // -----------------------------------------------------------------------
    #[test]
    fn test_annuity_factor() {
        let rate = dec!(0.05);
        let curve = flat_curve(rate, 3);
        let input = IrsInput {
            notional: dec!(1_000_000),
            fixed_rate: dec!(0.04),
            payment_frequency: 1,
            remaining_years: dec!(3),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();

        // For annual payments, annuity_factor = sum(DF_i) / 1 = DF(1) + DF(2) + DF(3)
        let df1 = Decimal::ONE / dec!(1.05);
        let df2 = Decimal::ONE / (dec!(1.05) * dec!(1.05));
        let df3 = Decimal::ONE / (dec!(1.05) * dec!(1.05) * dec!(1.05));
        let expected = df1 + df2 + df3;

        assert!(
            approx_eq(result.result.annuity_factor, expected, dec!(0.001)),
            "Annuity factor: expected ~{}, got {}",
            expected,
            result.result.annuity_factor
        );
    }

    // -----------------------------------------------------------------------
    // Test 9: Payment schedule has correct number of entries
    // -----------------------------------------------------------------------
    #[test]
    fn test_payment_schedule_lengths() {
        let curve = flat_curve(dec!(0.05), 5);
        let input = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.04),
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert_eq!(
            result.result.payment_schedule.len(),
            5,
            "Annual 5-year swap should have 5 payments"
        );
    }

    // -----------------------------------------------------------------------
    // Test 10: Net payments sum matches net_value when discounted
    // -----------------------------------------------------------------------
    #[test]
    fn test_net_payments_sum() {
        let curve = flat_curve(dec!(0.04), 5);
        let input = IrsInput {
            notional: dec!(5_000_000),
            fixed_rate: dec!(0.05),
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();

        // PV of net payments should equal net_value
        let pv_net: Decimal = result
            .result
            .payment_schedule
            .iter()
            .map(|p| p.net_payment * p.discount_factor)
            .sum();

        assert!(
            approx_eq(pv_net, result.result.net_value, dec!(1.0)),
            "PV of net payments ({}) should equal net_value ({})",
            pv_net,
            result.result.net_value
        );
    }

    // -----------------------------------------------------------------------
    // Test 11: Semi-annual payments
    // -----------------------------------------------------------------------
    #[test]
    fn test_semiannual_payments() {
        let curve = flat_curve_fine(dec!(0.04), 3, 2);
        let input = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.04),
            payment_frequency: 2,
            remaining_years: dec!(3),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert_eq!(
            result.result.payment_schedule.len(),
            6,
            "Semi-annual 3-year swap should have 6 payments"
        );
        // Each fixed payment should be notional * rate / 2
        let expected_fixed = dec!(10_000_000) * dec!(0.04) / dec!(2);
        assert!(
            approx_eq(
                result.result.payment_schedule[0].fixed_payment,
                expected_fixed,
                dec!(0.01)
            ),
            "Semi-annual fixed payment: expected {}, got {}",
            expected_fixed,
            result.result.payment_schedule[0].fixed_payment
        );
    }

    // -----------------------------------------------------------------------
    // Test 12: Quarterly payments
    // -----------------------------------------------------------------------
    #[test]
    fn test_quarterly_payments() {
        let curve = flat_curve_fine(dec!(0.03), 2, 4);
        let input = IrsInput {
            notional: dec!(5_000_000),
            fixed_rate: dec!(0.03),
            payment_frequency: 4,
            remaining_years: dec!(2),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert_eq!(
            result.result.payment_schedule.len(),
            8,
            "Quarterly 2-year swap should have 8 payments"
        );
        let expected_fixed = dec!(5_000_000) * dec!(0.03) / dec!(4);
        assert!(
            approx_eq(
                result.result.payment_schedule[0].fixed_payment,
                expected_fixed,
                dec!(0.01)
            ),
            "Quarterly fixed payment: expected {}, got {}",
            expected_fixed,
            result.result.payment_schedule[0].fixed_payment
        );
    }

    // -----------------------------------------------------------------------
    // Test 13: Basic currency swap
    // -----------------------------------------------------------------------
    #[test]
    fn test_currency_swap_basic() {
        let dom_curve = flat_curve(dec!(0.04), 5);
        let for_curve = flat_curve(dec!(0.02), 5);
        let input = CurrencySwapInput {
            notional_domestic: dec!(10_000_000),
            notional_foreign: dec!(8_000_000), // EUR
            domestic_fixed_rate: dec!(0.04),
            foreign_fixed_rate: dec!(0.02),
            payment_frequency: 1,
            remaining_years: dec!(5),
            domestic_discount_curve: dom_curve,
            foreign_discount_curve: for_curve,
            spot_fx_rate: dec!(1.25), // 1.25 USD/EUR
            is_pay_domestic: true,
        };
        let result = value_currency_swap(&input).unwrap();
        // Both legs should have positive PV
        assert!(
            result.result.domestic_leg_pv > Decimal::ZERO,
            "Domestic leg PV should be positive"
        );
        assert!(
            result.result.foreign_leg_pv > Decimal::ZERO,
            "Foreign leg PV (in domestic terms) should be positive"
        );
    }

    // -----------------------------------------------------------------------
    // Test 14: Currency swap FX exposure
    // -----------------------------------------------------------------------
    #[test]
    fn test_currency_swap_fx_exposure() {
        let dom_curve = flat_curve(dec!(0.03), 3);
        let for_curve = flat_curve(dec!(0.03), 3);
        let input = CurrencySwapInput {
            notional_domestic: dec!(10_000_000),
            notional_foreign: dec!(10_000_000),
            domestic_fixed_rate: dec!(0.03),
            foreign_fixed_rate: dec!(0.03),
            payment_frequency: 1,
            remaining_years: dec!(3),
            domestic_discount_curve: dom_curve,
            foreign_discount_curve: for_curve,
            spot_fx_rate: dec!(1.0),
            is_pay_domestic: true,
        };
        let result = value_currency_swap(&input).unwrap();
        // With identical curves, rates, notionals, and FX=1, PVs should match
        assert!(
            approx_eq(
                result.result.domestic_leg_pv,
                result.result.foreign_leg_pv,
                dec!(1.0)
            ),
            "Symmetric swap should have matching leg PVs: dom={}, for={}",
            result.result.domestic_leg_pv,
            result.result.foreign_leg_pv
        );
        assert!(
            approx_eq(result.result.net_value, Decimal::ZERO, dec!(1.0)),
            "Symmetric swap net value should be ~0, got {}",
            result.result.net_value
        );
    }

    // -----------------------------------------------------------------------
    // Test 15: Implied FX forward
    // -----------------------------------------------------------------------
    #[test]
    fn test_implied_fx_forward() {
        let dom_curve = flat_curve(dec!(0.05), 5);
        let for_curve = flat_curve(dec!(0.02), 5);
        let input = CurrencySwapInput {
            notional_domestic: dec!(10_000_000),
            notional_foreign: dec!(8_000_000),
            domestic_fixed_rate: dec!(0.05),
            foreign_fixed_rate: dec!(0.02),
            payment_frequency: 1,
            remaining_years: dec!(5),
            domestic_discount_curve: dom_curve,
            foreign_discount_curve: for_curve,
            spot_fx_rate: dec!(1.25),
            is_pay_domestic: true,
        };
        let result = value_currency_swap(&input).unwrap();

        // F = S * exp((r_d - r_f) * T) = 1.25 * exp(0.03 * 5) = 1.25 * exp(0.15)
        let expected = dec!(1.25) * exp_decimal(dec!(0.15));
        assert!(
            approx_eq(result.result.implied_fx_forward, expected, dec!(0.001)),
            "Implied FX forward: expected ~{}, got {}",
            expected,
            result.result.implied_fx_forward
        );
        // Should be > spot since r_d > r_f
        assert!(
            result.result.implied_fx_forward > dec!(1.25),
            "Forward should exceed spot when domestic rate > foreign rate"
        );
    }

    // -----------------------------------------------------------------------
    // Test 16: Invalid notional triggers error
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_notional_error() {
        let curve = flat_curve(dec!(0.05), 5);
        let input = IrsInput {
            notional: dec!(-1_000_000),
            fixed_rate: dec!(0.05),
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input);
        assert!(result.is_err(), "Negative notional should return an error");
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "notional");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 17: Metadata is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let curve = flat_curve(dec!(0.05), 5);
        let input = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.05),
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        assert!(
            !result.methodology.is_empty(),
            "Methodology should be populated"
        );
        assert_eq!(
            result.metadata.precision, "rust_decimal_128bit",
            "Precision should be rust_decimal_128bit"
        );
        assert!(
            !result.metadata.version.is_empty(),
            "Version should be populated"
        );
    }

    // -----------------------------------------------------------------------
    // Test 18: Invalid payment frequency
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_payment_frequency() {
        let curve = flat_curve(dec!(0.05), 5);
        let input = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.05),
            payment_frequency: 3, // invalid
            remaining_years: dec!(5),
            discount_curve: curve,
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "payment_frequency");
            }
            other => panic!("Expected InvalidInput for frequency, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 19: Empty discount curve triggers error
    // -----------------------------------------------------------------------
    #[test]
    fn test_empty_discount_curve() {
        let input = IrsInput {
            notional: dec!(10_000_000),
            fixed_rate: dec!(0.05),
            payment_frequency: 1,
            remaining_years: dec!(5),
            discount_curve: vec![],
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CorpFinanceError::InsufficientData(_)
        ));
    }

    // -----------------------------------------------------------------------
    // Test 20: Currency swap invalid FX rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_currency_swap_invalid_fx() {
        let dom_curve = flat_curve(dec!(0.05), 3);
        let for_curve = flat_curve(dec!(0.03), 3);
        let input = CurrencySwapInput {
            notional_domestic: dec!(10_000_000),
            notional_foreign: dec!(8_000_000),
            domestic_fixed_rate: dec!(0.05),
            foreign_fixed_rate: dec!(0.03),
            payment_frequency: 1,
            remaining_years: dec!(3),
            domestic_discount_curve: dom_curve,
            foreign_discount_curve: for_curve,
            spot_fx_rate: dec!(-1.0),
            is_pay_domestic: true,
        };
        let result = value_currency_swap(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "spot_fx_rate");
            }
            other => panic!("Expected InvalidInput for spot_fx_rate, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Test 21: exp_decimal accuracy
    // -----------------------------------------------------------------------
    #[test]
    fn test_exp_decimal_accuracy() {
        // exp(0) = 1
        assert_eq!(exp_decimal(Decimal::ZERO), Decimal::ONE);

        // exp(1) ~ 2.71828...
        let e = exp_decimal(Decimal::ONE);
        assert!(
            approx_eq(e, dec!(2.71828182845904523536), dec!(0.00001)),
            "exp(1) should be ~2.71828, got {}",
            e
        );

        // exp(-1) ~ 0.36788
        let e_neg = exp_decimal(-Decimal::ONE);
        assert!(
            approx_eq(e_neg, dec!(0.36787944117144232159), dec!(0.00001)),
            "exp(-1) should be ~0.36788, got {}",
            e_neg
        );

        // exp(3) ~ 20.0855
        let e3 = exp_decimal(dec!(3));
        assert!(
            approx_eq(e3, dec!(20.0855369231876677), dec!(0.001)),
            "exp(3) should be ~20.0855, got {}",
            e3
        );
    }

    // -----------------------------------------------------------------------
    // Test 22: Provided forward rates are used
    // -----------------------------------------------------------------------
    #[test]
    fn test_provided_forward_rates() {
        let curve = flat_curve(dec!(0.04), 3);
        let fwds = vec![
            ForwardRatePoint {
                maturity: dec!(1),
                rate: dec!(0.05),
            },
            ForwardRatePoint {
                maturity: dec!(2),
                rate: dec!(0.06),
            },
            ForwardRatePoint {
                maturity: dec!(3),
                rate: dec!(0.07),
            },
        ];
        let input = IrsInput {
            notional: dec!(1_000_000),
            fixed_rate: dec!(0.04),
            payment_frequency: 1,
            remaining_years: dec!(3),
            discount_curve: curve.clone(),
            forward_rates: Some(fwds),
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let result = value_interest_rate_swap(&input).unwrap();
        // Forward rates are above fixed rate, so floating_leg_pv > fixed_leg_pv
        assert!(
            result.result.floating_leg_pv > result.result.fixed_leg_pv,
            "With higher forward rates, floating leg PV should exceed fixed leg PV"
        );
        // Pay-fixed party should have positive value
        assert!(
            result.result.net_value > Decimal::ZERO,
            "Pay-fixed should have positive net value with higher floating rates"
        );
    }

    // -----------------------------------------------------------------------
    // Test 23: Last floating reset used for first period
    // -----------------------------------------------------------------------
    #[test]
    fn test_last_floating_reset() {
        let curve = flat_curve(dec!(0.04), 3);
        let input_no_reset = IrsInput {
            notional: dec!(1_000_000),
            fixed_rate: dec!(0.04),
            payment_frequency: 1,
            remaining_years: dec!(3),
            discount_curve: curve.clone(),
            forward_rates: None,
            is_pay_fixed: true,
            last_floating_reset: None,
        };
        let input_with_reset = IrsInput {
            last_floating_reset: Some(dec!(0.08)), // much higher reset
            ..input_no_reset.clone()
        };

        let r1 = value_interest_rate_swap(&input_no_reset).unwrap();
        let r2 = value_interest_rate_swap(&input_with_reset).unwrap();

        // The version with a high reset rate should have a higher floating leg PV
        assert!(
            r2.result.floating_leg_pv > r1.result.floating_leg_pv,
            "Higher reset rate should increase floating leg PV"
        );
    }
}
