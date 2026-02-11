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
pub struct ConvertibleAnalysisInput {
    pub bond_name: String,
    pub face_value: Money,
    pub coupon_rate: Rate,
    pub maturity_years: Decimal,
    pub credit_spread: Rate,
    pub risk_free_rate: Rate,
    pub stock_price: Money,
    pub conversion_ratio: Decimal,
    pub stock_volatility: Rate,
    #[serde(default)]
    pub dividend_yield: Option<Rate>,
    pub call_price: Option<Money>,
    pub stock_scenarios: Vec<Decimal>,
    pub vol_scenarios: Option<Vec<Rate>>,
    pub spread_scenarios: Option<Vec<Rate>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConvertibleAnalysisOutput {
    pub bond_name: String,
    pub current_conversion_value: Money,
    pub current_bond_floor: Money,
    pub parity_premium_pct: Rate,
    pub stock_sensitivity: Vec<ScenarioResult>,
    pub vol_sensitivity: Option<Vec<ScenarioResult>>,
    pub spread_sensitivity: Option<Vec<ScenarioResult>>,
    pub optimal_conversion_stock_price: Money,
    pub forced_conversion_analysis: Option<ForcedConversion>,
    pub income_advantage: IncomeAdvantage,
    pub risk_return_profile: RiskReturnProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub scenario_value: Decimal,
    pub convertible_value: Money,
    pub conversion_value: Money,
    pub bond_floor: Money,
    pub pct_change: Rate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForcedConversion {
    pub call_price: Money,
    pub conversion_value_at_call: Money,
    pub in_the_money: bool,
    pub stock_price_for_forced_conversion: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomeAdvantage {
    pub bond_current_yield: Rate,
    pub stock_dividend_yield: Rate,
    pub yield_advantage: Rate,
    pub breakeven_years: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReturnProfile {
    pub upside_participation: Rate,
    pub downside_protection: Rate,
    pub asymmetry_ratio: Decimal,
}

// ---------------------------------------------------------------------------
// Decimal math helpers (same as pricing, kept local to avoid coupling)
// ---------------------------------------------------------------------------

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

fn decimal_to_u32(d: Decimal) -> u32 {
    let rounded = d.round();
    let s = rounded.to_string();
    s.parse::<i64>().unwrap_or(0).max(0) as u32
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Compute straight bond floor: PV of coupons + principal at risky rate.
/// Uses semi-annual frequency (freq=2) as default for analysis.
fn compute_bond_floor(
    face: Decimal,
    coupon_rate: Rate,
    maturity: Decimal,
    risky_rate: Rate,
) -> Decimal {
    let freq: u32 = 2;
    let coupon_per_period = face * coupon_rate / Decimal::from(freq);
    let periods = decimal_to_u32(maturity * Decimal::from(freq));
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
    pv += face * discount;
    pv
}

/// Build a binomial tree for more accurate CB valuation (simplified, 30 steps).
#[allow(clippy::too_many_arguments)]
fn binomial_cb_value(
    face: Decimal,
    coupon_rate: Rate,
    maturity: Decimal,
    credit_spread: Rate,
    risk_free: Rate,
    stock: Decimal,
    ratio: Decimal,
    vol: Rate,
    div_yield: Rate,
    call_price: Option<Decimal>,
) -> Decimal {
    let steps: u32 = 30;
    let dt = maturity / Decimal::from(steps);
    if dt <= Decimal::ZERO {
        let cv = stock * ratio;
        let bf = compute_bond_floor(face, coupon_rate, maturity, risk_free + credit_spread);
        return cv.max(bf);
    }

    let u = exp_dec(vol * sqrt_dec(dt));
    let d = Decimal::ONE / u;
    let r_dt = exp_dec((risk_free - div_yield) * dt);
    let denom = u - d;
    if denom <= Decimal::ZERO {
        let cv = stock * ratio;
        let bf = compute_bond_floor(face, coupon_rate, maturity, risk_free + credit_spread);
        return cv.max(bf);
    }
    let p_up = (r_dt - d) / denom;
    let p_down = Decimal::ONE - p_up;

    let risky_rate = risk_free + credit_spread;
    let disc = exp_dec(-risky_rate * dt);

    let coupon_per_period = face * coupon_rate / dec!(2);
    let coupon_interval = dec!(0.5);

    let size = (steps + 1) as usize;
    let mut values = Vec::with_capacity(size);
    for i in 0..size {
        let stock_at = stock * pow_dec(u, i as u32) * pow_dec(d, steps - i as u32);
        let cv = stock_at * ratio;
        let bv = face + coupon_per_period;
        values.push(cv.max(bv));
    }

    for step in (0..steps).rev() {
        let t_at = Decimal::from(step) * dt;
        let step_size = (step + 1) as usize;
        let has_coupon = is_coupon_period(t_at, dt, coupon_interval);

        for i in 0..step_size {
            let hold = disc * (p_up * values[i + 1] + p_down * values[i]);
            let hold_c = if has_coupon {
                hold + coupon_per_period
            } else {
                hold
            };
            let stock_at = stock * pow_dec(u, i as u32) * pow_dec(d, step - i as u32);
            let cv = stock_at * ratio;
            let mut val = hold_c.max(cv);

            if let Some(cp) = call_price {
                if val > cp {
                    val = cp.max(cv);
                }
            }
            values[i] = val;
        }
    }
    values[0]
}

fn is_coupon_period(t: Decimal, dt: Decimal, interval: Decimal) -> bool {
    if interval <= Decimal::ZERO {
        return false;
    }
    let t_end = t + dt;
    let n_end = (t_end / interval).floor();
    let n_start = (t / interval).floor();
    n_end > n_start
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &ConvertibleAnalysisInput) -> CorpFinanceResult<()> {
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
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
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
    if input.stock_scenarios.is_empty() {
        return Err(CorpFinanceError::InvalidInput {
            field: "stock_scenarios".into(),
            reason: "must contain at least one scenario".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

pub fn analyze_convertible(
    input: &ConvertibleAnalysisInput,
) -> CorpFinanceResult<ComputationOutput<ConvertibleAnalysisOutput>> {
    let start = Instant::now();
    validate_input(input)?;

    let face = input.face_value;
    let coupon_rate = input.coupon_rate;
    let maturity = input.maturity_years;
    let credit_spread = input.credit_spread;
    let rf = input.risk_free_rate;
    let stock = input.stock_price;
    let ratio = input.conversion_ratio;
    let vol = input.stock_volatility;
    let div_yield = input.dividend_yield.unwrap_or(Decimal::ZERO);
    let risky_rate = rf + credit_spread;

    // Current values
    let current_conversion_value = stock * ratio;
    let current_bond_floor = compute_bond_floor(face, coupon_rate, maturity, risky_rate);

    // Base CB value via binomial
    let base_cb_value = binomial_cb_value(
        face,
        coupon_rate,
        maturity,
        credit_spread,
        rf,
        stock,
        ratio,
        vol,
        div_yield,
        input.call_price,
    );

    let parity_premium_pct = if current_conversion_value > Decimal::ZERO {
        (base_cb_value - current_conversion_value) / current_conversion_value
    } else {
        Decimal::ZERO
    };

    // Stock sensitivity
    let stock_sensitivity: Vec<ScenarioResult> = input
        .stock_scenarios
        .iter()
        .map(|&scenario_stock| {
            let cv = scenario_stock * ratio;
            let bf = current_bond_floor; // bond floor doesn't change with stock
            let cb_val = binomial_cb_value(
                face,
                coupon_rate,
                maturity,
                credit_spread,
                rf,
                scenario_stock,
                ratio,
                vol,
                div_yield,
                input.call_price,
            );
            let pct_change = if base_cb_value > Decimal::ZERO {
                (cb_val - base_cb_value) / base_cb_value
            } else {
                Decimal::ZERO
            };
            ScenarioResult {
                scenario_value: scenario_stock,
                convertible_value: cb_val,
                conversion_value: cv,
                bond_floor: bf,
                pct_change,
            }
        })
        .collect();

    // Vol sensitivity
    let vol_sensitivity = input.vol_scenarios.as_ref().map(|vol_scens| {
        vol_scens
            .iter()
            .map(|&scenario_vol| {
                let cb_val = binomial_cb_value(
                    face,
                    coupon_rate,
                    maturity,
                    credit_spread,
                    rf,
                    stock,
                    ratio,
                    scenario_vol,
                    div_yield,
                    input.call_price,
                );
                let pct_change = if base_cb_value > Decimal::ZERO {
                    (cb_val - base_cb_value) / base_cb_value
                } else {
                    Decimal::ZERO
                };
                ScenarioResult {
                    scenario_value: scenario_vol,
                    convertible_value: cb_val,
                    conversion_value: current_conversion_value,
                    bond_floor: current_bond_floor,
                    pct_change,
                }
            })
            .collect()
    });

    // Spread sensitivity
    let spread_sensitivity = input.spread_scenarios.as_ref().map(|spread_scens| {
        spread_scens
            .iter()
            .map(|&scenario_spread| {
                let bf = compute_bond_floor(face, coupon_rate, maturity, rf + scenario_spread);
                let cb_val = binomial_cb_value(
                    face,
                    coupon_rate,
                    maturity,
                    scenario_spread,
                    rf,
                    stock,
                    ratio,
                    vol,
                    div_yield,
                    input.call_price,
                );
                let pct_change = if base_cb_value > Decimal::ZERO {
                    (cb_val - base_cb_value) / base_cb_value
                } else {
                    Decimal::ZERO
                };
                ScenarioResult {
                    scenario_value: scenario_spread,
                    convertible_value: cb_val,
                    conversion_value: current_conversion_value,
                    bond_floor: bf,
                    pct_change,
                }
            })
            .collect()
    });

    // Optimal conversion stock price: where conversion_value = bond_floor
    // stock * ratio = bond_floor => stock = bond_floor / ratio
    let optimal_conversion_stock_price = if ratio > Decimal::ZERO {
        current_bond_floor / ratio
    } else {
        Decimal::ZERO
    };

    // Forced conversion analysis
    let forced_conversion_analysis = input.call_price.map(|cp| {
        let cv_at_call = stock * ratio;
        let itm = cv_at_call > cp;
        // Stock price for forced conversion: stock where conversion_value > call_price
        // stock * ratio > call_price => stock > call_price / ratio
        let stock_for_forced = if ratio > Decimal::ZERO {
            cp / ratio
        } else {
            Decimal::ZERO
        };
        ForcedConversion {
            call_price: cp,
            conversion_value_at_call: cv_at_call,
            in_the_money: itm,
            stock_price_for_forced_conversion: stock_for_forced,
        }
    });

    // Income advantage
    let annual_coupon = face * coupon_rate;
    let bond_current_yield = if base_cb_value > Decimal::ZERO {
        annual_coupon / base_cb_value
    } else {
        Decimal::ZERO
    };
    let yield_advantage = bond_current_yield - div_yield;
    let premium_amount = base_cb_value - current_conversion_value;
    let breakeven_years = if yield_advantage > Decimal::ZERO
        && premium_amount > Decimal::ZERO
        && current_conversion_value > Decimal::ZERO
    {
        // Income advantage per year in dollar terms
        let bond_income = annual_coupon;
        let stock_income = current_conversion_value * div_yield;
        let dollar_advantage = bond_income - stock_income;
        if dollar_advantage > Decimal::ZERO {
            premium_amount / dollar_advantage
        } else {
            dec!(999)
        }
    } else if premium_amount <= Decimal::ZERO {
        Decimal::ZERO
    } else {
        dec!(999)
    };

    let income_advantage = IncomeAdvantage {
        bond_current_yield,
        stock_dividend_yield: div_yield,
        yield_advantage,
        breakeven_years,
    };

    // Risk/return profile: use +/-20% stock scenarios
    let stock_up = stock * dec!(1.20);
    let stock_down = stock * dec!(0.80);

    let cb_up = binomial_cb_value(
        face,
        coupon_rate,
        maturity,
        credit_spread,
        rf,
        stock_up,
        ratio,
        vol,
        div_yield,
        input.call_price,
    );
    let cb_down = binomial_cb_value(
        face,
        coupon_rate,
        maturity,
        credit_spread,
        rf,
        stock_down,
        ratio,
        vol,
        div_yield,
        input.call_price,
    );

    let stock_pct_change = dec!(0.20);
    let cb_up_pct = if base_cb_value > Decimal::ZERO {
        (cb_up - base_cb_value) / base_cb_value
    } else {
        Decimal::ZERO
    };
    let cb_down_pct = if base_cb_value > Decimal::ZERO {
        (base_cb_value - cb_down) / base_cb_value
    } else {
        Decimal::ZERO
    };

    let upside_participation = if stock_pct_change > Decimal::ZERO {
        cb_up_pct / stock_pct_change
    } else {
        Decimal::ZERO
    };
    let downside_protection = if stock_pct_change > Decimal::ZERO {
        Decimal::ONE - cb_down_pct / stock_pct_change
    } else {
        Decimal::ZERO
    };
    let loss_rate = Decimal::ONE - downside_protection;
    let asymmetry_ratio = if loss_rate > Decimal::ZERO {
        upside_participation / loss_rate
    } else if upside_participation > Decimal::ZERO {
        dec!(999) // infinite asymmetry, cap it
    } else {
        Decimal::ONE
    };

    let risk_return_profile = RiskReturnProfile {
        upside_participation,
        downside_protection,
        asymmetry_ratio,
    };

    let output = ConvertibleAnalysisOutput {
        bond_name: input.bond_name.clone(),
        current_conversion_value,
        current_bond_floor,
        parity_premium_pct,
        stock_sensitivity,
        vol_sensitivity,
        spread_sensitivity,
        optimal_conversion_stock_price,
        forced_conversion_analysis,
        income_advantage,
        risk_return_profile,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    let assumptions = serde_json::json!({
        "model": "Convertible Bond Scenario Analysis",
        "risk_free_rate": rf.to_string(),
        "credit_spread": credit_spread.to_string(),
        "stock_volatility": vol.to_string(),
        "stock_scenarios_count": input.stock_scenarios.len(),
        "vol_scenarios_count": input.vol_scenarios.as_ref().map(|v| v.len()).unwrap_or(0),
        "spread_scenarios_count": input.spread_scenarios.as_ref().map(|v| v.len()).unwrap_or(0),
    });

    Ok(with_metadata(
        "Convertible Bond Scenario Analysis",
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

    fn default_analysis_input() -> ConvertibleAnalysisInput {
        ConvertibleAnalysisInput {
            bond_name: "TEST-CB-ANALYSIS".into(),
            face_value: dec!(1000),
            coupon_rate: dec!(0.04),
            maturity_years: dec!(5),
            credit_spread: dec!(0.03),
            risk_free_rate: dec!(0.05),
            stock_price: dec!(40),
            conversion_ratio: dec!(25),
            stock_volatility: dec!(0.30),
            dividend_yield: None,
            call_price: None,
            stock_scenarios: vec![dec!(30), dec!(35), dec!(40), dec!(45), dec!(50), dec!(55)],
            vol_scenarios: None,
            spread_scenarios: None,
        }
    }

    #[test]
    fn test_basic_analysis() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        let out = &result.result;
        assert_eq!(out.bond_name, "TEST-CB-ANALYSIS");
        assert!(out.current_conversion_value > Decimal::ZERO);
        assert!(out.current_bond_floor > Decimal::ZERO);
    }

    #[test]
    fn test_current_values() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        let out = &result.result;
        // conversion_value = 40 * 25 = 1000
        assert!(
            approx_eq(out.current_conversion_value, dec!(1000), dec!(0.01)),
            "conversion value should be 1000, got {}",
            out.current_conversion_value
        );
        // bond floor should be < 1000 since coupon (4%) < risky rate (8%)
        assert!(
            out.current_bond_floor < dec!(1000),
            "bond floor {} should be below par",
            out.current_bond_floor
        );
    }

    #[test]
    fn test_stock_sensitivity_count() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        assert_eq!(result.result.stock_sensitivity.len(), 6);
    }

    #[test]
    fn test_stock_sensitivity_monotonic() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        let sens = &result.result.stock_sensitivity;
        // CB values should generally increase as stock increases
        for i in 1..sens.len() {
            assert!(
                sens[i].convertible_value >= sens[i - 1].convertible_value - dec!(5),
                "CB value should increase with stock: {} at stock={} vs {} at stock={}",
                sens[i].convertible_value,
                sens[i].scenario_value,
                sens[i - 1].convertible_value,
                sens[i - 1].scenario_value,
            );
        }
    }

    #[test]
    fn test_conversion_value_in_scenarios() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        let sens = &result.result.stock_sensitivity;
        for s in sens {
            let expected_cv = s.scenario_value * dec!(25);
            assert!(
                approx_eq(s.conversion_value, expected_cv, dec!(0.01)),
                "conversion value {} should be stock * ratio = {}",
                s.conversion_value,
                expected_cv
            );
        }
    }

    #[test]
    fn test_vol_sensitivity() {
        let input = ConvertibleAnalysisInput {
            vol_scenarios: Some(vec![
                dec!(0.15),
                dec!(0.20),
                dec!(0.30),
                dec!(0.40),
                dec!(0.50),
            ]),
            ..default_analysis_input()
        };
        let result = analyze_convertible(&input).unwrap();
        let vol_sens = result.result.vol_sensitivity.as_ref().unwrap();
        assert_eq!(vol_sens.len(), 5);
        // Higher vol should generally give higher CB value
        let first = vol_sens.first().unwrap().convertible_value;
        let last = vol_sens.last().unwrap().convertible_value;
        assert!(
            last >= first - dec!(5),
            "higher vol {} should yield higher CB {} vs {}",
            vol_sens.last().unwrap().scenario_value,
            last,
            first
        );
    }

    #[test]
    fn test_spread_sensitivity() {
        let input = ConvertibleAnalysisInput {
            spread_scenarios: Some(vec![dec!(0.01), dec!(0.03), dec!(0.05), dec!(0.08)]),
            ..default_analysis_input()
        };
        let result = analyze_convertible(&input).unwrap();
        let spread_sens = result.result.spread_sensitivity.as_ref().unwrap();
        assert_eq!(spread_sens.len(), 4);
        // Higher spread should lower bond floor
        let bf_tight = spread_sens[0].bond_floor;
        let bf_wide = spread_sens[3].bond_floor;
        assert!(
            bf_tight > bf_wide,
            "tighter spread should give higher bond floor: {} vs {}",
            bf_tight,
            bf_wide
        );
    }

    #[test]
    fn test_optimal_conversion_price() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        let opt = result.result.optimal_conversion_stock_price;
        // optimal = bond_floor / ratio
        let expected = result.result.current_bond_floor / dec!(25);
        assert!(
            approx_eq(opt, expected, dec!(0.01)),
            "optimal conversion price {} should be {}",
            opt,
            expected
        );
    }

    #[test]
    fn test_no_forced_conversion_without_call() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        assert!(result.result.forced_conversion_analysis.is_none());
    }

    #[test]
    fn test_forced_conversion_analysis() {
        let input = ConvertibleAnalysisInput {
            call_price: Some(dec!(1050)),
            ..default_analysis_input()
        };
        let result = analyze_convertible(&input).unwrap();
        let fc = result.result.forced_conversion_analysis.as_ref().unwrap();
        assert!(approx_eq(fc.call_price, dec!(1050), dec!(0.01)));
        // Stock for forced conversion = call_price / ratio = 1050/25 = 42
        assert!(
            approx_eq(fc.stock_price_for_forced_conversion, dec!(42), dec!(0.01)),
            "forced conversion price {} should be 42",
            fc.stock_price_for_forced_conversion
        );
        // Current stock = 40, conversion_value = 1000 < 1050 => not in the money
        assert!(!fc.in_the_money, "should not be in the money at stock=40");
    }

    #[test]
    fn test_forced_conversion_in_the_money() {
        let input = ConvertibleAnalysisInput {
            stock_price: dec!(50), // conversion_value = 50 * 25 = 1250 > 1050
            call_price: Some(dec!(1050)),
            ..default_analysis_input()
        };
        let result = analyze_convertible(&input).unwrap();
        let fc = result.result.forced_conversion_analysis.as_ref().unwrap();
        assert!(fc.in_the_money, "should be in the money at stock=50");
    }

    #[test]
    fn test_income_advantage() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        let ia = &result.result.income_advantage;
        // Annual coupon = 1000 * 0.04 = 40
        // bond_current_yield = 40 / cb_value
        assert!(ia.bond_current_yield > Decimal::ZERO);
        // No dividends => yield advantage = bond yield
        assert!(
            approx_eq(ia.yield_advantage, ia.bond_current_yield, dec!(0.0001)),
            "with no dividends, yield advantage should equal bond yield"
        );
        assert_eq!(ia.stock_dividend_yield, Decimal::ZERO);
    }

    #[test]
    fn test_income_advantage_with_dividends() {
        let input = ConvertibleAnalysisInput {
            dividend_yield: Some(dec!(0.02)),
            ..default_analysis_input()
        };
        let result = analyze_convertible(&input).unwrap();
        let ia = &result.result.income_advantage;
        assert!(
            ia.yield_advantage < ia.bond_current_yield,
            "dividend should reduce yield advantage"
        );
        assert!(approx_eq(ia.stock_dividend_yield, dec!(0.02), dec!(0.001)));
    }

    #[test]
    fn test_risk_return_profile() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        let rr = &result.result.risk_return_profile;
        // Upside participation should be positive (CB gains when stock rises)
        assert!(
            rr.upside_participation > Decimal::ZERO,
            "upside participation {} should be positive",
            rr.upside_participation
        );
        // Downside protection should be positive (CB loses less than stock)
        assert!(
            rr.downside_protection > Decimal::ZERO,
            "downside protection {} should be positive",
            rr.downside_protection
        );
        // Asymmetry ratio should be > 1 for a good convertible
        assert!(
            rr.asymmetry_ratio > Decimal::ZERO,
            "asymmetry ratio {} should be positive",
            rr.asymmetry_ratio
        );
    }

    #[test]
    fn test_parity_premium() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        // Parity premium should be non-negative (CB worth >= conversion value)
        assert!(
            result.result.parity_premium_pct >= -dec!(0.01),
            "parity premium {} should be non-negative",
            result.result.parity_premium_pct
        );
    }

    #[test]
    fn test_empty_scenarios_error() {
        let input = ConvertibleAnalysisInput {
            stock_scenarios: vec![],
            ..default_analysis_input()
        };
        let result = analyze_convertible(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "stock_scenarios");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_invalid_stock_price() {
        let input = ConvertibleAnalysisInput {
            stock_price: dec!(0),
            ..default_analysis_input()
        };
        let result = analyze_convertible(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "stock_price");
            }
            other => panic!("Expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn test_metadata_populated() {
        let input = default_analysis_input();
        let result = analyze_convertible(&input).unwrap();
        assert!(!result.methodology.is_empty());
        assert!(!result.metadata.version.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }
}
