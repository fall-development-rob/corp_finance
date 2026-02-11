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
pub struct ConvertibleBondInput {
    pub bond_name: String,
    pub face_value: Money,
    pub coupon_rate: Rate,
    pub coupon_frequency: u32,
    pub maturity_years: Decimal,
    pub credit_spread: Rate,
    pub risk_free_rate: Rate,
    pub stock_price: Money,
    pub conversion_ratio: Decimal,
    pub stock_volatility: Rate,
    #[serde(default)]
    pub dividend_yield: Option<Rate>,
    pub call_price: Option<Money>,
    pub call_protection_years: Option<Decimal>,
    pub put_price: Option<Money>,
    pub put_date_years: Option<Decimal>,
    pub tree_steps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertibleBondOutput {
    pub bond_name: String,
    pub model_price: Money,
    pub bond_floor: Money,
    pub conversion_value: Money,
    pub conversion_premium_pct: Rate,
    pub investment_premium_pct: Rate,
    pub embedded_option_value: Money,
    pub delta: Decimal,
    pub gamma: Decimal,
    pub vega: Decimal,
    pub theta: Decimal,
    pub yield_to_maturity: Rate,
    pub current_yield: Rate,
    pub breakeven_years: Decimal,
    pub risk_profile: String,
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64)
// ---------------------------------------------------------------------------

/// Taylor series exp(x) with range reduction for |x| > 2.
fn exp_dec(x: Decimal) -> Decimal {
    let two = dec!(2);
    if x > two || x < -two {
        let half = exp_dec(x / two);
        return half * half;
    }
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1u32..=25 {
        term = term * x / Decimal::from(n);
        sum += term;
    }
    sum
}

/// Newton's method sqrt, 25 iterations.
fn sqrt_dec(x: Decimal) -> Decimal {
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
    for _ in 0..25 {
        guess = (guess + x / guess) / two;
    }
    guess
}

/// Integer power by squaring (avoids powd precision drift).
fn pow_dec(base: Decimal, exp: u32) -> Decimal {
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

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &ConvertibleBondInput) -> CorpFinanceResult<()> {
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
            reason: "must be positive".into(),
        });
    }
    if input.stock_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "stock_price".into(),
            reason: "must be positive".into(),
        });
    }
    if input.conversion_ratio <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "conversion_ratio".into(),
            reason: "must be positive".into(),
        });
    }
    if input.maturity_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "must be positive".into(),
        });
    }
    if input.stock_volatility <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "stock_volatility".into(),
            reason: "must be positive".into(),
        });
    }
    if input.coupon_frequency != 1 && input.coupon_frequency != 2 && input.coupon_frequency != 4 {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_frequency".into(),
            reason: "must be 1 (annual), 2 (semi-annual), or 4 (quarterly)".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Bond floor: PV of coupons + principal at risky rate
// ---------------------------------------------------------------------------

fn compute_bond_floor(
    face: Decimal,
    coupon_rate: Rate,
    freq: u32,
    maturity: Decimal,
    risky_rate: Rate,
) -> Decimal {
    let coupon_per_period = face * coupon_rate / Decimal::from(freq);
    let periods_dec = maturity * Decimal::from(freq);
    // Round to nearest integer number of periods
    let periods = decimal_to_u32(periods_dec);
    if periods == 0 {
        return face;
    }

    let rate_per_period = risky_rate / Decimal::from(freq);
    let mut pv = Decimal::ZERO;
    let mut discount = Decimal::ONE;
    for _ in 0..periods {
        discount /= Decimal::ONE + rate_per_period;
        pv += coupon_per_period * discount;
    }
    // Add PV of principal
    pv += face * discount;
    pv
}

/// Convert a Decimal to u32 (rounded).
fn decimal_to_u32(d: Decimal) -> u32 {
    let rounded = d.round();
    // Extract the integer part
    let s = rounded.to_string();
    s.parse::<i64>().unwrap_or(0).max(0) as u32
}

// ---------------------------------------------------------------------------
// CRR binomial tree for convertible bond
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn binomial_cb_price(
    face: Decimal,
    coupon_rate: Rate,
    freq: u32,
    maturity: Decimal,
    credit_spread: Rate,
    risk_free: Rate,
    stock: Decimal,
    ratio: Decimal,
    vol: Rate,
    div_yield: Rate,
    call_price: Option<Decimal>,
    call_protection: Decimal,
    put_price: Option<Decimal>,
    put_date: Decimal,
    steps: u32,
) -> Decimal {
    let n = steps;
    let dt = maturity / Decimal::from(n);
    let u = exp_dec(vol * sqrt_dec(dt));
    let d = Decimal::ONE / u;
    let r_dt = exp_dec((risk_free - div_yield) * dt);
    let p_up = (r_dt - d) / (u - d);
    let p_down = Decimal::ONE - p_up;

    // Discount factor per step at risky rate (for hold value discounting)
    let risky_rate = risk_free + credit_spread;
    let disc = exp_dec(-risky_rate * dt);

    // Coupon per period
    let coupon_per_period = face * coupon_rate / Decimal::from(freq);
    // Time between coupon payments in years
    let coupon_interval = Decimal::ONE / Decimal::from(freq);

    let size = (n + 1) as usize;

    // Terminal values: max(conversion_value, face + final_coupon)
    let mut values = Vec::with_capacity(size);
    for i in 0..size {
        let stock_at_node = stock * pow_dec(u, i as u32) * pow_dec(d, n - i as u32);
        let conversion_val = stock_at_node * ratio;
        let bond_val = face + coupon_per_period; // final coupon + par
        values.push(conversion_val.max(bond_val));
    }

    // Backward induction
    for step in (0..n).rev() {
        let t_at_step = Decimal::from(step) * dt;
        let step_size = (step + 1) as usize;

        // Check if a coupon is paid in this period
        // A coupon is paid if there is a coupon date in (t_at_step, t_at_step + dt]
        let coupon_at_step = is_coupon_period(t_at_step, dt, coupon_interval);

        for i in 0..step_size {
            // Hold value: discounted expected value
            let hold = disc * (p_up * values[i + 1] + p_down * values[i]);
            // Add accrued coupon if this period has a payment
            let hold_with_coupon = if coupon_at_step {
                hold + coupon_per_period
            } else {
                hold
            };

            // Conversion value at this node
            let stock_at_node = stock * pow_dec(u, i as u32) * pow_dec(d, step - i as u32);
            let conversion_val = stock_at_node * ratio;

            // CB value = max(conversion, hold)
            let mut cb_val = hold_with_coupon.max(conversion_val);

            // Put provision: holder can put at put_price if past put_date
            if let Some(pp) = put_price {
                if t_at_step >= put_date {
                    cb_val = cb_val.max(pp);
                }
            }

            // Call provision: issuer can call at call_price if past call protection
            if let Some(cp) = call_price {
                if t_at_step >= call_protection {
                    // Issuer calls only if CB > call_price, but holder can still convert
                    if cb_val > cp {
                        cb_val = cp.max(conversion_val);
                    }
                }
            }

            values[i] = cb_val;
        }
    }

    values[0]
}

/// Check if a coupon payment falls within the period (t, t + dt].
fn is_coupon_period(t: Decimal, dt: Decimal, interval: Decimal) -> bool {
    if interval <= Decimal::ZERO {
        return false;
    }
    let t_end = t + dt;
    // Number of coupons paid by t_end
    let n_end = (t_end / interval).floor();
    // Number of coupons paid by t
    let n_start = (t / interval).floor();
    n_end > n_start
}

// ---------------------------------------------------------------------------
// YTM solver via Newton-Raphson
// ---------------------------------------------------------------------------

fn compute_ytm(
    model_price: Decimal,
    face: Decimal,
    coupon_rate: Rate,
    freq: u32,
    maturity: Decimal,
) -> Rate {
    let coupon_per_period = face * coupon_rate / Decimal::from(freq);
    let periods = decimal_to_u32(maturity * Decimal::from(freq));
    if periods == 0 || model_price <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    // Newton-Raphson: find y such that PV(y) = model_price
    let mut y = coupon_rate; // initial guess = coupon rate
    if y <= Decimal::ZERO {
        y = dec!(0.05);
    }

    for _ in 0..50 {
        let y_per = y / Decimal::from(freq);
        let mut pv = Decimal::ZERO;
        let mut dpv = Decimal::ZERO;
        let mut discount = Decimal::ONE;

        for t in 1..=periods {
            discount /= Decimal::ONE + y_per;
            pv += coupon_per_period * discount;
            // derivative: d(discount)/dy = -t/(freq) * discount / (1+y/freq)
            dpv += coupon_per_period * (-Decimal::from(t) / Decimal::from(freq)) * discount
                / (Decimal::ONE + y_per);
        }
        pv += face * discount;
        dpv += face * (-Decimal::from(periods) / Decimal::from(freq)) * discount
            / (Decimal::ONE + y_per);

        let diff = pv - model_price;
        if dpv == Decimal::ZERO {
            break;
        }
        let adjustment = diff / dpv;
        y -= adjustment;

        let abs_diff = if diff < Decimal::ZERO { -diff } else { diff };
        if abs_diff < dec!(0.0000001) {
            break;
        }

        // Clamp
        if y < dec!(-0.5) {
            y = dec!(-0.5);
        }
        if y > dec!(2.0) {
            y = dec!(2.0);
        }
    }
    y
}

// ---------------------------------------------------------------------------
// Risk profile classification
// ---------------------------------------------------------------------------

fn classify_risk_profile(
    conversion_premium_pct: Decimal,
    bond_floor: Decimal,
    model_price: Decimal,
) -> String {
    if model_price <= Decimal::ZERO {
        return "Distressed".into();
    }
    let bond_floor_ratio = bond_floor / model_price;

    if conversion_premium_pct < dec!(0.20) {
        "Equity-like".into()
    } else if conversion_premium_pct < dec!(0.50) {
        "Balanced".into()
    } else if bond_floor_ratio > dec!(0.90) {
        "Bond-like".into()
    } else {
        "Distressed".into()
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn price_convertible(
    input: &ConvertibleBondInput,
) -> CorpFinanceResult<ComputationOutput<ConvertibleBondOutput>> {
    let start = Instant::now();
    validate_input(input)?;

    let face = input.face_value;
    let coupon_rate = input.coupon_rate;
    let freq = input.coupon_frequency;
    let maturity = input.maturity_years;
    let credit_spread = input.credit_spread;
    let rf = input.risk_free_rate;
    let stock = input.stock_price;
    let ratio = input.conversion_ratio;
    let vol = input.stock_volatility;
    let div_yield = input.dividend_yield.unwrap_or(Decimal::ZERO);
    let call_protection = input.call_protection_years.unwrap_or(Decimal::ZERO);
    let put_date = input.put_date_years.unwrap_or(maturity);
    let steps = input.tree_steps.unwrap_or(100);

    let risky_rate = rf + credit_spread;

    // 1) Bond floor
    let bond_floor = compute_bond_floor(face, coupon_rate, freq, maturity, risky_rate);

    // 2) Conversion value (parity)
    let conversion_value = stock * ratio;

    // 3) Model price via binomial tree
    let model_price = binomial_cb_price(
        face,
        coupon_rate,
        freq,
        maturity,
        credit_spread,
        rf,
        stock,
        ratio,
        vol,
        div_yield,
        input.call_price,
        call_protection,
        input.put_price,
        put_date,
        steps,
    );

    // 4) Premiums
    let conversion_premium_pct = if conversion_value > Decimal::ZERO {
        (model_price - conversion_value) / conversion_value
    } else {
        Decimal::ZERO
    };
    let investment_premium_pct = if bond_floor > Decimal::ZERO {
        (model_price - bond_floor) / bond_floor
    } else {
        Decimal::ZERO
    };
    let embedded_option_value = model_price - bond_floor;

    // 5) Greeks via finite differences
    let ds = stock * dec!(0.01); // 1% stock bump
    let v_up = binomial_cb_price(
        face,
        coupon_rate,
        freq,
        maturity,
        credit_spread,
        rf,
        stock + ds,
        ratio,
        vol,
        div_yield,
        input.call_price,
        call_protection,
        input.put_price,
        put_date,
        steps,
    );
    let v_down = binomial_cb_price(
        face,
        coupon_rate,
        freq,
        maturity,
        credit_spread,
        rf,
        stock - ds,
        ratio,
        vol,
        div_yield,
        input.call_price,
        call_protection,
        input.put_price,
        put_date,
        steps,
    );

    let two_ds = dec!(2) * ds;
    let delta = if two_ds > Decimal::ZERO {
        (v_up - v_down) / two_ds
    } else {
        Decimal::ZERO
    };
    let gamma = if ds > Decimal::ZERO {
        (v_up - dec!(2) * model_price + v_down) / (ds * ds)
    } else {
        Decimal::ZERO
    };

    // Vega: bump vol by 1%
    let dvol = dec!(0.01);
    let v_vol_up = binomial_cb_price(
        face,
        coupon_rate,
        freq,
        maturity,
        credit_spread,
        rf,
        stock,
        ratio,
        vol + dvol,
        div_yield,
        input.call_price,
        call_protection,
        input.put_price,
        put_date,
        steps,
    );
    let vega = (v_vol_up - model_price) / dvol;

    // Theta: reduce maturity by 1/365
    let dt_theta = dec!(1) / dec!(365);
    let theta = if maturity > dt_theta {
        let v_short = binomial_cb_price(
            face,
            coupon_rate,
            freq,
            maturity - dt_theta,
            credit_spread,
            rf,
            stock,
            ratio,
            vol,
            div_yield,
            input.call_price,
            call_protection,
            input.put_price,
            put_date,
            steps,
        );
        (v_short - model_price) / dt_theta
    } else {
        Decimal::ZERO
    };

    // 6) Yields
    let ytm = compute_ytm(model_price, face, coupon_rate, freq, maturity);
    let annual_coupon = face * coupon_rate;
    let current_yield = if model_price > Decimal::ZERO {
        annual_coupon / model_price
    } else {
        Decimal::ZERO
    };

    // 7) Breakeven years
    let breakeven_years = if annual_coupon > Decimal::ZERO && conversion_value > Decimal::ZERO {
        let premium_amount = model_price - conversion_value;
        if premium_amount > Decimal::ZERO {
            // Dividend income on equivalent shares
            let stock_income = conversion_value * div_yield;
            let income_advantage = annual_coupon - stock_income;
            if income_advantage > Decimal::ZERO {
                premium_amount / income_advantage
            } else {
                dec!(999)
            }
        } else {
            Decimal::ZERO
        }
    } else {
        dec!(999)
    };

    // 8) Risk profile
    let risk_profile = classify_risk_profile(conversion_premium_pct, bond_floor, model_price);

    let output = ConvertibleBondOutput {
        bond_name: input.bond_name.clone(),
        model_price,
        bond_floor,
        conversion_value,
        conversion_premium_pct,
        investment_premium_pct,
        embedded_option_value,
        delta,
        gamma,
        vega,
        theta,
        yield_to_maturity: ytm,
        current_yield,
        breakeven_years,
        risk_profile,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "model": "CRR Binomial Tree for Convertible Bond",
        "tree_steps": steps,
        "risk_free_rate": rf.to_string(),
        "credit_spread": credit_spread.to_string(),
        "stock_volatility": vol.to_string(),
        "dividend_yield": div_yield.to_string(),
        "callable": input.call_price.is_some(),
        "puttable": input.put_price.is_some(),
    });

    Ok(with_metadata(
        "CRR Binomial Tree for Convertible Bond",
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
        let diff = a - b;
        let abs_diff = if diff < Decimal::ZERO { -diff } else { diff };
        abs_diff < tol
    }

    fn default_cb_input() -> ConvertibleBondInput {
        ConvertibleBondInput {
            bond_name: "TEST-CB".into(),
            face_value: dec!(1000),
            coupon_rate: dec!(0.04),
            coupon_frequency: 2,
            maturity_years: dec!(5),
            credit_spread: dec!(0.03),
            risk_free_rate: dec!(0.05),
            stock_price: dec!(40),
            conversion_ratio: dec!(25),
            stock_volatility: dec!(0.30),
            dividend_yield: None,
            call_price: None,
            call_protection_years: None,
            put_price: None,
            put_date_years: None,
            tree_steps: Some(50),
        }
    }

    #[test]
    fn test_basic_pricing() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        let out = &result.result;
        assert!(
            out.model_price > Decimal::ZERO,
            "model price must be positive"
        );
        assert!(
            out.model_price >= out.bond_floor,
            "model price {} must be >= bond floor {}",
            out.model_price,
            out.bond_floor
        );
        assert!(
            out.model_price >= out.conversion_value,
            "model price {} must be >= conversion value {}",
            out.model_price,
            out.conversion_value
        );
    }

    #[test]
    fn test_bond_floor_value() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        let bond_floor = result.result.bond_floor;
        // Bond floor should be PV of coupons + par at risky rate (8%)
        // Semi-annual coupon = 1000 * 0.04 / 2 = 20
        // 10 periods at 4% per period
        // Should be less than par since coupon < risky rate
        assert!(
            bond_floor < dec!(1000),
            "bond floor {} should be below par for low coupon",
            bond_floor
        );
        assert!(
            bond_floor > dec!(700),
            "bond floor {} should be reasonable",
            bond_floor
        );
    }

    #[test]
    fn test_conversion_value() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        // conversion_value = stock_price * conversion_ratio = 40 * 25 = 1000
        assert!(
            approx_eq(result.result.conversion_value, dec!(1000), dec!(0.01)),
            "conversion value {} should be 1000",
            result.result.conversion_value
        );
    }

    #[test]
    fn test_high_stock_equity_like() {
        // When stock is high, CB is equity-like
        let input = ConvertibleBondInput {
            stock_price: dec!(60),
            ..default_cb_input()
        };
        let result = price_convertible(&input).unwrap();
        let out = &result.result;
        // conversion_value = 60 * 25 = 1500, well above bond floor
        assert_eq!(out.risk_profile, "Equity-like");
        assert!(
            out.conversion_premium_pct < dec!(0.20),
            "premium {} should be < 20%",
            out.conversion_premium_pct
        );
    }

    #[test]
    fn test_low_stock_bond_like_or_distressed() {
        // When stock is low, CB should be bond-like
        let input = ConvertibleBondInput {
            stock_price: dec!(15),
            ..default_cb_input()
        };
        let result = price_convertible(&input).unwrap();
        let out = &result.result;
        // conversion_value = 15 * 25 = 375, well below bond floor
        assert!(
            out.risk_profile == "Bond-like" || out.risk_profile == "Distressed",
            "risk profile should be Bond-like or Distressed, got {}",
            out.risk_profile
        );
    }

    #[test]
    fn test_callable_cb_capped() {
        // Callable CB should be worth less than uncallable
        let uncallable = default_cb_input();
        let callable = ConvertibleBondInput {
            call_price: Some(dec!(1050)),
            call_protection_years: Some(dec!(2)),
            ..default_cb_input()
        };
        let v_uncallable = price_convertible(&uncallable).unwrap().result.model_price;
        let v_callable = price_convertible(&callable).unwrap().result.model_price;
        assert!(
            v_callable <= v_uncallable + dec!(1),
            "callable {} should be <= uncallable {}",
            v_callable,
            v_uncallable
        );
    }

    #[test]
    fn test_puttable_cb_higher() {
        // Puttable CB should be worth at least as much as non-puttable
        let plain = default_cb_input();
        let puttable = ConvertibleBondInput {
            put_price: Some(dec!(950)),
            put_date_years: Some(dec!(3)),
            ..default_cb_input()
        };
        let v_plain = price_convertible(&plain).unwrap().result.model_price;
        let v_puttable = price_convertible(&puttable).unwrap().result.model_price;
        assert!(
            v_puttable >= v_plain - dec!(1),
            "puttable {} should be >= plain {}",
            v_puttable,
            v_plain
        );
    }

    #[test]
    fn test_delta_positive() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        assert!(
            result.result.delta > Decimal::ZERO,
            "delta {} should be positive (CB value increases with stock)",
            result.result.delta
        );
    }

    #[test]
    fn test_delta_bounded() {
        // Delta should be between 0 and conversion_ratio
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        let delta = result.result.delta;
        assert!(
            delta >= Decimal::ZERO && delta <= input.conversion_ratio + dec!(1),
            "delta {} should be in [0, {}]",
            delta,
            input.conversion_ratio
        );
    }

    #[test]
    fn test_vega_positive() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        assert!(
            result.result.vega > Decimal::ZERO,
            "vega {} should be positive (CB benefits from higher vol)",
            result.result.vega
        );
    }

    #[test]
    fn test_embedded_option_value() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        let out = &result.result;
        let expected = out.model_price - out.bond_floor;
        assert!(
            approx_eq(out.embedded_option_value, expected, dec!(0.01)),
            "embedded option value {} should be model - floor = {}",
            out.embedded_option_value,
            expected
        );
        assert!(
            out.embedded_option_value >= Decimal::ZERO,
            "embedded option value should be non-negative"
        );
    }

    #[test]
    fn test_premiums_consistent() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        let out = &result.result;
        // conversion premium = (model - conversion_value) / conversion_value
        if out.conversion_value > Decimal::ZERO {
            let expected = (out.model_price - out.conversion_value) / out.conversion_value;
            assert!(
                approx_eq(out.conversion_premium_pct, expected, dec!(0.0001)),
                "conversion premium inconsistent"
            );
        }
    }

    #[test]
    fn test_current_yield() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        let out = &result.result;
        let expected = dec!(1000) * dec!(0.04) / out.model_price;
        assert!(
            approx_eq(out.current_yield, expected, dec!(0.001)),
            "current yield {} should be {}",
            out.current_yield,
            expected
        );
    }

    #[test]
    fn test_ytm_reasonable() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        let ytm = result.result.yield_to_maturity;
        // YTM should be somewhere between 0 and 50% for a reasonable bond
        assert!(
            ytm > dec!(-0.5) && ytm < dec!(0.5),
            "YTM {} should be in a reasonable range",
            ytm
        );
    }

    #[test]
    fn test_invalid_face_value() {
        let input = ConvertibleBondInput {
            face_value: dec!(0),
            ..default_cb_input()
        };
        let result = price_convertible(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "face_value");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_coupon_frequency() {
        let input = ConvertibleBondInput {
            coupon_frequency: 3,
            ..default_cb_input()
        };
        let result = price_convertible(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "coupon_frequency");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_metadata_populated() {
        let input = default_cb_input();
        let result = price_convertible(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    #[test]
    fn test_higher_vol_higher_price() {
        let low_vol = ConvertibleBondInput {
            stock_volatility: dec!(0.15),
            ..default_cb_input()
        };
        let high_vol = ConvertibleBondInput {
            stock_volatility: dec!(0.45),
            ..default_cb_input()
        };
        let v_low = price_convertible(&low_vol).unwrap().result.model_price;
        let v_high = price_convertible(&high_vol).unwrap().result.model_price;
        assert!(
            v_high >= v_low - dec!(1),
            "higher vol {} should give higher price than low vol {}",
            v_high,
            v_low
        );
    }
}
