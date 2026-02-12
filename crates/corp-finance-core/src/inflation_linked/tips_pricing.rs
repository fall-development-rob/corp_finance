//! TIPS / Inflation-Linked Bond Pricing Module.
//!
//! Provides institutional-grade analytics for Treasury Inflation-Protected
//! Securities (TIPS) and generic inflation-linked bonds:
//!
//! - **TIPS Pricing**: CPI-adjusted principal, real/nominal clean and dirty
//!   prices, deflation floor, projected cashflow schedule.
//! - **Breakeven Inflation Analysis**: Fisher equation breakeven rate, term
//!   structure of breakeven inflation, forward breakeven rates, and an
//!   inflation risk premium estimate.
//! - **Real Yield Curve Analysis**: Bootstrap real/nominal zero curves, forward
//!   real rates, forward breakeven inflation, and real duration by maturity.
//!
//! All financial math uses `rust_decimal::Decimal` (never f64). Helpers for
//! `exp`, `ln`, and `sqrt` via Taylor series / Newton iterations.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate, Years};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const TAYLOR_EXP_TERMS: u32 = 30;
const NEWTON_ITERATIONS: u32 = 20;

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Taylor series exp(x) with range reduction for numerical stability.
///
/// For negative arguments, computes 1/exp(|x|) to avoid catastrophic
/// cancellation. Uses range reduction: exp(x) = exp(x/2^k)^(2^k).
#[allow(dead_code)]
fn decimal_exp(x: Decimal) -> Decimal {
    if x < Decimal::ZERO {
        let pos_exp = decimal_exp(-x);
        if pos_exp == Decimal::ZERO {
            return Decimal::ZERO;
        }
        return Decimal::ONE / pos_exp;
    }

    let mut k: u32 = 0;
    let mut reduced = x;
    while reduced > Decimal::ONE {
        reduced /= dec!(2);
        k += 1;
    }

    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=TAYLOR_EXP_TERMS {
        term = term * reduced / Decimal::from(n);
        sum += term;
    }

    for _ in 0..k {
        sum = sum * sum;
    }
    sum
}

/// Newton's method ln(x) with `NEWTON_ITERATIONS` iterations.
/// Uses the identity:  y_{n+1} = y_n + 2 * (x - exp(y_n)) / (x + exp(y_n))
#[allow(dead_code)]
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO; // guard
    }
    // Initial guess: use simple ratio approximation
    let mut y = (x - Decimal::ONE) / (x + Decimal::ONE) * dec!(2);
    for _ in 0..NEWTON_ITERATIONS {
        let ey = decimal_exp(y);
        if ey == Decimal::ZERO {
            break;
        }
        y += dec!(2) * (x - ey) / (x + ey);
    }
    y
}

/// Newton's method sqrt(x) with `NEWTON_ITERATIONS` iterations.
#[allow(dead_code)]
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    let mut guess = x / dec!(2);
    if guess == Decimal::ZERO {
        guess = Decimal::ONE;
    }
    for _ in 0..NEWTON_ITERATIONS {
        if guess == Decimal::ZERO {
            break;
        }
        guess = (guess + x / guess) / dec!(2);
    }
    guess
}

/// Raise a Decimal base to a Decimal exponent: base^exp = exp(exp * ln(base)).
#[allow(dead_code)]
fn decimal_pow(base: Decimal, exp: Decimal) -> Decimal {
    if base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    decimal_exp(exp * decimal_ln(base))
}

// ---------------------------------------------------------------------------
// Input / Output types — TIPS Pricing
// ---------------------------------------------------------------------------

/// A single projected TIPS cashflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipsCashflow {
    /// Period number (1-based)
    pub period: u32,
    /// Approximate date label
    pub date_approx: String,
    /// Coupon in real (constant) terms
    pub real_coupon: Money,
    /// Coupon in nominal (CPI-adjusted) terms
    pub nominal_coupon: Money,
    /// Projected CPI level at this period
    pub projected_cpi: Decimal,
    /// CPI-adjusted principal at this period
    pub adjusted_principal: Money,
}

/// Input for TIPS bond pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipsPricingInput {
    /// Face (par) value of the bond
    pub face_value: Money,
    /// Real (stated) coupon rate as a decimal
    pub real_coupon_rate: Rate,
    /// Coupons per year (1, 2, or 4)
    pub coupon_frequency: u8,
    /// Real yield (discount rate in real terms)
    pub real_yield: Rate,
    /// Settlement date (ISO 8601 string)
    pub settlement_date: String,
    /// Maturity date (ISO 8601 string)
    pub maturity_date: String,
    /// CPI index value at bond issuance (base)
    pub cpi_base: Decimal,
    /// CPI index value at settlement (current)
    pub cpi_current: Decimal,
    /// Expected annual CPI inflation rate for projections
    pub cpi_projected_annual_rate: Rate,
    /// Number of remaining coupon periods
    pub remaining_periods: u32,
}

/// Output of TIPS bond pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipsPricingOutput {
    /// Clean price in real terms
    pub real_clean_price: Money,
    /// Dirty price in real terms
    pub real_dirty_price: Money,
    /// Clean price in nominal (CPI-adjusted) terms
    pub nominal_clean_price: Money,
    /// Dirty price in nominal (CPI-adjusted) terms
    pub nominal_dirty_price: Money,
    /// CPI_current / CPI_base
    pub index_ratio: Decimal,
    /// Accrued interest
    pub accrued_interest: Money,
    /// CPI-adjusted principal
    pub adjusted_principal: Money,
    /// Deflation floor: max(adjusted_principal, face_value)
    pub deflation_floor_value: Money,
    /// Projected cashflow schedule
    pub projected_cashflows: Vec<TipsCashflow>,
}

// ---------------------------------------------------------------------------
// Input / Output types — Breakeven Inflation
// ---------------------------------------------------------------------------

/// A point on a yield curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldPoint {
    pub maturity: Years,
    pub rate: Rate,
}

/// Input for breakeven inflation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakevenInput {
    /// Nominal yield for the tenor
    pub nominal_yield: Rate,
    /// Real yield for the tenor (TIPS yield)
    pub real_yield: Rate,
    /// Nominal yield curve points
    pub nominal_yield_curve: Vec<YieldPoint>,
    /// Real yield curve points (TIPS yields)
    pub real_yield_curve: Vec<YieldPoint>,
}

/// A single breakeven data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakevenPoint {
    pub maturity: Years,
    pub breakeven_rate: Rate,
}

/// Output of breakeven inflation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakevenOutput {
    /// Headline breakeven = nominal - real
    pub breakeven_inflation: Rate,
    /// Breakeven at each maturity on the curve
    pub breakeven_curve: Vec<BreakevenPoint>,
    /// Forward (term) breakeven inflation between adjacent maturities
    pub term_structure_breakeven: Vec<BreakevenPoint>,
    /// Rough estimate: breakeven - expected (if we had expected)
    pub inflation_risk_premium_estimate: Rate,
}

// ---------------------------------------------------------------------------
// Input / Output types — Real Yield Curve
// ---------------------------------------------------------------------------

/// A single TIPS security observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipsSecurity {
    pub maturity: Years,
    pub real_yield: Rate,
    pub nominal_yield: Rate,
    pub cpi_ratio: Decimal,
}

/// A single point on a yield/duration curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurvePoint {
    pub maturity: Years,
    pub value: Decimal,
}

/// Output of real yield curve analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealYieldOutput {
    pub real_zero_curve: Vec<CurvePoint>,
    pub nominal_zero_curve: Vec<CurvePoint>,
    pub breakeven_curve: Vec<CurvePoint>,
    pub forward_real_rates: Vec<CurvePoint>,
    pub forward_breakeven: Vec<CurvePoint>,
    pub real_duration_by_maturity: Vec<CurvePoint>,
}

// ---------------------------------------------------------------------------
// Wrapper types
// ---------------------------------------------------------------------------

/// Model selection enum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TipsModel {
    Pricing(TipsPricingInput),
    Breakeven(BreakevenInput),
    RealYield(RealYieldInput),
}

/// Top-level input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TipsAnalyticsInput {
    pub model: TipsModel,
}

/// Top-level output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TipsAnalyticsOutput {
    Pricing(TipsPricingOutput),
    Breakeven(BreakevenOutput),
    RealYield(RealYieldOutput),
}

/// Input wrapper for real yield curve analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealYieldInput {
    pub tips_securities: Vec<TipsSecurity>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze TIPS or inflation-linked bonds.
///
/// Dispatches to the appropriate sub-model based on `TipsModel`:
/// - `Pricing` — full TIPS bond pricing with CPI adjustment
/// - `Breakeven` — breakeven inflation term structure
/// - `RealYield` — real yield curve construction and forwards
pub fn analyze_tips(
    input: &TipsAnalyticsInput,
) -> CorpFinanceResult<ComputationOutput<TipsAnalyticsOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let result = match &input.model {
        TipsModel::Pricing(p) => {
            let out = compute_tips_pricing(p)?;
            TipsAnalyticsOutput::Pricing(out)
        }
        TipsModel::Breakeven(b) => {
            let out = compute_breakeven(b)?;
            TipsAnalyticsOutput::Breakeven(out)
        }
        TipsModel::RealYield(r) => {
            let out = compute_real_yield_curve(r)?;
            TipsAnalyticsOutput::RealYield(out)
        }
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "TIPS / Inflation-Linked Bond Analytics",
        &serde_json::json!({
            "precision": "rust_decimal_128bit",
            "taylor_exp_terms": TAYLOR_EXP_TERMS,
            "newton_iterations": NEWTON_ITERATIONS,
        }),
        warnings,
        elapsed,
        result,
    ))
}

// ---------------------------------------------------------------------------
// TIPS Pricing implementation
// ---------------------------------------------------------------------------

fn compute_tips_pricing(input: &TipsPricingInput) -> CorpFinanceResult<TipsPricingOutput> {
    // Validation
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.cpi_base <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cpi_base".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.cpi_current <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cpi_current".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.remaining_periods == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_periods".into(),
            reason: "Must be at least 1".into(),
        });
    }
    if input.coupon_frequency == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_frequency".into(),
            reason: "Must be at least 1".into(),
        });
    }

    let freq = Decimal::from(input.coupon_frequency);
    let index_ratio = input.cpi_current / input.cpi_base;
    let adjusted_principal = input.face_value * index_ratio;
    let deflation_floor_value = if adjusted_principal < input.face_value {
        input.face_value
    } else {
        adjusted_principal
    };

    // Build projected cashflows
    let mut cashflows: Vec<TipsCashflow> = Vec::with_capacity(input.remaining_periods as usize);
    let period_inflation_rate = input.cpi_projected_annual_rate / freq;

    // Discount factor: iterative multiplication for precision
    let periodic_yield = input.real_yield / freq;
    let mut real_dirty_price = Decimal::ZERO;
    let mut cpi_level = input.cpi_current;
    let mut df = Decimal::ONE; // discount factor accumulator

    for t in 1..=input.remaining_periods {
        // Project CPI forward one period
        cpi_level *= Decimal::ONE + period_inflation_rate;
        let adj_princ = input.face_value * (cpi_level / input.cpi_base);
        let real_coupon = input.face_value * input.real_coupon_rate / freq;
        let nominal_coupon = adj_princ * input.real_coupon_rate / freq;

        // Discount factor for period t (iterative multiply)
        df /= Decimal::ONE + periodic_yield;

        // PV of coupon in real terms
        let mut cf_real = real_coupon;
        if t == input.remaining_periods {
            // Add principal at maturity (in real terms, face_value)
            cf_real += input.face_value;
        }
        real_dirty_price += cf_real * df;

        cashflows.push(TipsCashflow {
            period: t,
            date_approx: format!("T+{}", t),
            real_coupon,
            nominal_coupon,
            projected_cpi: cpi_level,
            adjusted_principal: adj_princ,
        });
    }

    // Accrued interest: assume settlement is at beginning of current period,
    // so accrued = 0 for a simplified model. In practice we would use day count.
    let accrued_interest = Decimal::ZERO;

    let real_clean_price = real_dirty_price - accrued_interest;

    // Nominal prices = real prices * current index ratio
    let nominal_dirty_price = real_dirty_price * index_ratio;
    let nominal_clean_price = real_clean_price * index_ratio;

    Ok(TipsPricingOutput {
        real_clean_price,
        real_dirty_price,
        nominal_clean_price,
        nominal_dirty_price,
        index_ratio,
        accrued_interest,
        adjusted_principal,
        deflation_floor_value,
        projected_cashflows: cashflows,
    })
}

// ---------------------------------------------------------------------------
// Breakeven Inflation implementation
// ---------------------------------------------------------------------------

fn compute_breakeven(input: &BreakevenInput) -> CorpFinanceResult<BreakevenOutput> {
    if input.nominal_yield_curve.is_empty() || input.real_yield_curve.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Both nominal and real yield curves are required".into(),
        ));
    }
    if input.nominal_yield_curve.len() != input.real_yield_curve.len() {
        return Err(CorpFinanceError::InvalidInput {
            field: "yield_curves".into(),
            reason: "Nominal and real yield curves must have the same number of points".into(),
        });
    }

    // Headline breakeven
    let breakeven_inflation = input.nominal_yield - input.real_yield;

    // Breakeven curve: nominal - real at each maturity
    let breakeven_curve: Vec<BreakevenPoint> = input
        .nominal_yield_curve
        .iter()
        .zip(input.real_yield_curve.iter())
        .map(|(nom, real)| BreakevenPoint {
            maturity: nom.maturity,
            breakeven_rate: nom.rate - real.rate,
        })
        .collect();

    // Forward (term structure) breakeven: implied forward inflation between
    // adjacent maturities. Forward rate f(t1,t2) such that:
    //   (1+r2)^t2 = (1+r1)^t1 * (1+f)^(t2-t1)
    // Applied to breakeven: forward_be(t1,t2) = forward_nominal - forward_real
    let term_structure_breakeven =
        compute_forward_breakevens(&input.nominal_yield_curve, &input.real_yield_curve);

    // Inflation risk premium estimate: simple heuristic
    // premium ~ breakeven - some long-run expected inflation estimate
    // Without an explicit expected inflation, use the shortest-maturity breakeven
    // as a proxy for the "expected" component.
    let short_be = breakeven_curve
        .first()
        .map(|p| p.breakeven_rate)
        .unwrap_or(breakeven_inflation);
    let inflation_risk_premium_estimate = breakeven_inflation - short_be;

    Ok(BreakevenOutput {
        breakeven_inflation,
        breakeven_curve,
        term_structure_breakeven,
        inflation_risk_premium_estimate,
    })
}

/// Compute forward breakeven inflation between adjacent maturity points.
fn compute_forward_breakevens(
    nominal_curve: &[YieldPoint],
    real_curve: &[YieldPoint],
) -> Vec<BreakevenPoint> {
    let mut forwards = Vec::new();

    for i in 1..nominal_curve.len() {
        let t1 = nominal_curve[i - 1].maturity;
        let t2 = nominal_curve[i].maturity;
        let dt = t2 - t1;
        if dt <= Decimal::ZERO {
            continue;
        }

        // Forward nominal rate
        let nom_fwd =
            compute_forward_rate(nominal_curve[i - 1].rate, t1, nominal_curve[i].rate, t2);

        // Forward real rate
        let real_fwd = compute_forward_rate(real_curve[i - 1].rate, t1, real_curve[i].rate, t2);

        let fwd_breakeven = nom_fwd - real_fwd;

        forwards.push(BreakevenPoint {
            maturity: t2,
            breakeven_rate: fwd_breakeven,
        });
    }

    forwards
}

/// Continuous forward rate between t1 and t2:
///   f = (r2*t2 - r1*t1) / (t2 - t1)
fn compute_forward_rate(r1: Rate, t1: Years, r2: Rate, t2: Years) -> Rate {
    let dt = t2 - t1;
    if dt == Decimal::ZERO {
        return r2;
    }
    (r2 * t2 - r1 * t1) / dt
}

// ---------------------------------------------------------------------------
// Real Yield Curve implementation
// ---------------------------------------------------------------------------

fn compute_real_yield_curve(input: &RealYieldInput) -> CorpFinanceResult<RealYieldOutput> {
    if input.tips_securities.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one TIPS security is required".into(),
        ));
    }

    // Sort by maturity
    let mut securities = input.tips_securities.clone();
    securities.sort_by(|a, b| {
        a.maturity
            .partial_cmp(&b.maturity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Build zero curves from par yields (simple bootstrap: for coupon-bearing
    // bonds, the par yield approximately equals the zero rate at shorter
    // maturities — a simplified approach).
    let real_zero_curve: Vec<CurvePoint> = securities
        .iter()
        .map(|s| CurvePoint {
            maturity: s.maturity,
            value: s.real_yield,
        })
        .collect();

    let nominal_zero_curve: Vec<CurvePoint> = securities
        .iter()
        .map(|s| CurvePoint {
            maturity: s.maturity,
            value: s.nominal_yield,
        })
        .collect();

    let breakeven_curve: Vec<CurvePoint> = securities
        .iter()
        .map(|s| CurvePoint {
            maturity: s.maturity,
            value: s.nominal_yield - s.real_yield,
        })
        .collect();

    // Forward real rates
    let forward_real_rates = compute_forward_curve(&real_zero_curve);

    // Forward breakeven
    let forward_breakeven = compute_forward_curve_diff(&nominal_zero_curve, &real_zero_curve);

    // Real duration by maturity: approximate modified duration for a zero-coupon
    // bond at each maturity:  D_mod = T / (1 + y)
    let real_duration_by_maturity: Vec<CurvePoint> = securities
        .iter()
        .map(|s| {
            let denom = Decimal::ONE + s.real_yield;
            let dur = if denom != Decimal::ZERO {
                s.maturity / denom
            } else {
                s.maturity
            };
            CurvePoint {
                maturity: s.maturity,
                value: dur,
            }
        })
        .collect();

    Ok(RealYieldOutput {
        real_zero_curve,
        nominal_zero_curve,
        breakeven_curve,
        forward_real_rates,
        forward_breakeven,
        real_duration_by_maturity,
    })
}

/// Forward curve from zero rates: f(t1,t2) = (r2*t2 - r1*t1) / (t2 - t1)
fn compute_forward_curve(zero_curve: &[CurvePoint]) -> Vec<CurvePoint> {
    let mut forwards = Vec::new();
    for i in 1..zero_curve.len() {
        let t1 = zero_curve[i - 1].maturity;
        let t2 = zero_curve[i].maturity;
        let dt = t2 - t1;
        if dt <= Decimal::ZERO {
            continue;
        }
        let fwd = (zero_curve[i].value * t2 - zero_curve[i - 1].value * t1) / dt;
        forwards.push(CurvePoint {
            maturity: t2,
            value: fwd,
        });
    }
    forwards
}

/// Forward breakeven from nominal and real zero curves:
///   forward_be(t1,t2) = forward_nominal(t1,t2) - forward_real(t1,t2)
fn compute_forward_curve_diff(nominal: &[CurvePoint], real: &[CurvePoint]) -> Vec<CurvePoint> {
    let mut forwards = Vec::new();
    let n = nominal.len().min(real.len());
    for i in 1..n {
        let t1 = nominal[i - 1].maturity;
        let t2 = nominal[i].maturity;
        let dt = t2 - t1;
        if dt <= Decimal::ZERO {
            continue;
        }
        let fwd_nom = (nominal[i].value * t2 - nominal[i - 1].value * t1) / dt;
        let fwd_real = (real[i].value * t2 - real[i - 1].value * t1) / dt;
        forwards.push(CurvePoint {
            maturity: t2,
            value: fwd_nom - fwd_real,
        });
    }
    forwards
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // Helper builders
    // -----------------------------------------------------------------------

    fn default_tips_pricing_input() -> TipsPricingInput {
        TipsPricingInput {
            face_value: dec!(1000),
            real_coupon_rate: dec!(0.0125), // 1.25%
            coupon_frequency: 2,
            real_yield: dec!(0.015),
            settlement_date: "2024-06-15".into(),
            maturity_date: "2034-06-15".into(),
            cpi_base: dec!(250),
            cpi_current: dec!(310),
            cpi_projected_annual_rate: dec!(0.025),
            remaining_periods: 20, // 10 years * 2
        }
    }

    fn default_breakeven_input() -> BreakevenInput {
        BreakevenInput {
            nominal_yield: dec!(0.045),
            real_yield: dec!(0.02),
            nominal_yield_curve: vec![
                YieldPoint {
                    maturity: dec!(2),
                    rate: dec!(0.035),
                },
                YieldPoint {
                    maturity: dec!(5),
                    rate: dec!(0.04),
                },
                YieldPoint {
                    maturity: dec!(10),
                    rate: dec!(0.045),
                },
                YieldPoint {
                    maturity: dec!(30),
                    rate: dec!(0.05),
                },
            ],
            real_yield_curve: vec![
                YieldPoint {
                    maturity: dec!(2),
                    rate: dec!(0.01),
                },
                YieldPoint {
                    maturity: dec!(5),
                    rate: dec!(0.015),
                },
                YieldPoint {
                    maturity: dec!(10),
                    rate: dec!(0.02),
                },
                YieldPoint {
                    maturity: dec!(30),
                    rate: dec!(0.025),
                },
            ],
        }
    }

    fn default_real_yield_input() -> RealYieldInput {
        RealYieldInput {
            tips_securities: vec![
                TipsSecurity {
                    maturity: dec!(2),
                    real_yield: dec!(0.01),
                    nominal_yield: dec!(0.035),
                    cpi_ratio: dec!(1.24),
                },
                TipsSecurity {
                    maturity: dec!(5),
                    real_yield: dec!(0.015),
                    nominal_yield: dec!(0.04),
                    cpi_ratio: dec!(1.24),
                },
                TipsSecurity {
                    maturity: dec!(10),
                    real_yield: dec!(0.02),
                    nominal_yield: dec!(0.045),
                    cpi_ratio: dec!(1.24),
                },
                TipsSecurity {
                    maturity: dec!(30),
                    real_yield: dec!(0.025),
                    nominal_yield: dec!(0.05),
                    cpi_ratio: dec!(1.24),
                },
            ],
        }
    }

    // -----------------------------------------------------------------------
    // TIPS Pricing tests
    // -----------------------------------------------------------------------

    #[test]
    fn tips_index_ratio_inflation() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        // CPI went from 250 to 310 => ratio = 1.24
        assert_eq!(out.index_ratio, dec!(1.24));
        assert!(
            out.index_ratio > Decimal::ONE,
            "Index ratio > 1 means inflation occurred"
        );
    }

    #[test]
    fn tips_adjusted_principal() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        let expected = dec!(1000) * dec!(310) / dec!(250); // 1240
        assert_eq!(out.adjusted_principal, expected);
    }

    #[test]
    fn tips_deflation_floor_no_deflation() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        // adjusted_principal = 1240 > face_value = 1000, floor = adjusted
        assert_eq!(out.deflation_floor_value, out.adjusted_principal);
    }

    #[test]
    fn tips_deflation_floor_with_deflation() {
        let mut input = default_tips_pricing_input();
        input.cpi_current = dec!(240); // CPI dropped below base of 250
        let out = compute_tips_pricing(&input).unwrap();
        // adjusted_principal = 1000 * 240/250 = 960, floor = face = 1000
        assert!(out.adjusted_principal < input.face_value);
        assert_eq!(out.deflation_floor_value, input.face_value);
    }

    #[test]
    fn tips_positive_prices() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        assert!(out.real_clean_price > Decimal::ZERO);
        assert!(out.real_dirty_price > Decimal::ZERO);
        assert!(out.nominal_clean_price > Decimal::ZERO);
        assert!(out.nominal_dirty_price > Decimal::ZERO);
    }

    #[test]
    fn tips_nominal_price_greater_than_real() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        // With positive inflation, nominal price > real price
        assert!(out.nominal_clean_price > out.real_clean_price);
    }

    #[test]
    fn tips_cashflow_count_matches_periods() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        assert_eq!(
            out.projected_cashflows.len(),
            input.remaining_periods as usize
        );
    }

    #[test]
    fn tips_cashflows_increasing_cpi() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        // Each projected CPI should be higher than the previous
        for i in 1..out.projected_cashflows.len() {
            assert!(
                out.projected_cashflows[i].projected_cpi
                    > out.projected_cashflows[i - 1].projected_cpi,
                "CPI should increase each period"
            );
        }
    }

    #[test]
    fn tips_real_coupon_constant() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        let expected_real_coupon = dec!(1000) * dec!(0.0125) / dec!(2);
        for cf in &out.projected_cashflows {
            assert_eq!(
                cf.real_coupon, expected_real_coupon,
                "Real coupon should be constant"
            );
        }
    }

    #[test]
    fn tips_nominal_coupon_increasing() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        for i in 1..out.projected_cashflows.len() {
            assert!(
                out.projected_cashflows[i].nominal_coupon
                    > out.projected_cashflows[i - 1].nominal_coupon,
                "Nominal coupon should increase with inflation"
            );
        }
    }

    #[test]
    fn tips_zero_inflation_index_ratio_one() {
        let mut input = default_tips_pricing_input();
        input.cpi_current = input.cpi_base; // CPI ratio = 1
        input.cpi_projected_annual_rate = Decimal::ZERO;
        let out = compute_tips_pricing(&input).unwrap();
        assert_eq!(out.index_ratio, Decimal::ONE);
        assert_eq!(out.adjusted_principal, input.face_value);
    }

    #[test]
    fn tips_zero_inflation_real_equals_nominal() {
        let mut input = default_tips_pricing_input();
        input.cpi_current = input.cpi_base;
        input.cpi_projected_annual_rate = Decimal::ZERO;
        let out = compute_tips_pricing(&input).unwrap();
        // With ratio = 1 and zero projected inflation, nominal = real
        assert_eq!(out.nominal_clean_price, out.real_clean_price);
    }

    #[test]
    fn tips_invalid_face_value() {
        let mut input = default_tips_pricing_input();
        input.face_value = dec!(-100);
        assert!(compute_tips_pricing(&input).is_err());
    }

    #[test]
    fn tips_invalid_cpi_base_zero() {
        let mut input = default_tips_pricing_input();
        input.cpi_base = Decimal::ZERO;
        assert!(compute_tips_pricing(&input).is_err());
    }

    #[test]
    fn tips_invalid_remaining_periods_zero() {
        let mut input = default_tips_pricing_input();
        input.remaining_periods = 0;
        assert!(compute_tips_pricing(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Breakeven Inflation tests
    // -----------------------------------------------------------------------

    #[test]
    fn breakeven_equals_nominal_minus_real() {
        let input = default_breakeven_input();
        let out = compute_breakeven(&input).unwrap();
        assert_eq!(out.breakeven_inflation, dec!(0.045) - dec!(0.02));
    }

    #[test]
    fn breakeven_curve_positive_entries() {
        let input = default_breakeven_input();
        let out = compute_breakeven(&input).unwrap();
        for p in &out.breakeven_curve {
            assert!(
                p.breakeven_rate > Decimal::ZERO,
                "Breakeven should be positive"
            );
        }
    }

    #[test]
    fn breakeven_curve_length() {
        let input = default_breakeven_input();
        let out = compute_breakeven(&input).unwrap();
        assert_eq!(out.breakeven_curve.len(), 4);
    }

    #[test]
    fn breakeven_term_structure_length() {
        let input = default_breakeven_input();
        let out = compute_breakeven(&input).unwrap();
        // Forward breakevens: n-1 intervals for n points
        assert_eq!(out.term_structure_breakeven.len(), 3);
    }

    #[test]
    fn breakeven_curve_matches_spot_differences() {
        let input = default_breakeven_input();
        let out = compute_breakeven(&input).unwrap();
        for (i, bp) in out.breakeven_curve.iter().enumerate() {
            let expected = input.nominal_yield_curve[i].rate - input.real_yield_curve[i].rate;
            assert_eq!(bp.breakeven_rate, expected);
        }
    }

    #[test]
    fn breakeven_empty_curves_error() {
        let input = BreakevenInput {
            nominal_yield: dec!(0.04),
            real_yield: dec!(0.02),
            nominal_yield_curve: vec![],
            real_yield_curve: vec![],
        };
        assert!(compute_breakeven(&input).is_err());
    }

    #[test]
    fn breakeven_mismatched_curves_error() {
        let input = BreakevenInput {
            nominal_yield: dec!(0.04),
            real_yield: dec!(0.02),
            nominal_yield_curve: vec![
                YieldPoint {
                    maturity: dec!(2),
                    rate: dec!(0.035),
                },
                YieldPoint {
                    maturity: dec!(5),
                    rate: dec!(0.04),
                },
            ],
            real_yield_curve: vec![YieldPoint {
                maturity: dec!(2),
                rate: dec!(0.01),
            }],
        };
        assert!(compute_breakeven(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Real Yield Curve tests
    // -----------------------------------------------------------------------

    #[test]
    fn real_yield_curve_size() {
        let input = default_real_yield_input();
        let out = compute_real_yield_curve(&input).unwrap();
        assert_eq!(out.real_zero_curve.len(), 4);
        assert_eq!(out.nominal_zero_curve.len(), 4);
        assert_eq!(out.breakeven_curve.len(), 4);
    }

    #[test]
    fn real_yield_lower_than_nominal() {
        let input = default_real_yield_input();
        let out = compute_real_yield_curve(&input).unwrap();
        for (r, n) in out
            .real_zero_curve
            .iter()
            .zip(out.nominal_zero_curve.iter())
        {
            assert!(
                r.value < n.value,
                "Real yield should be lower than nominal (positive inflation)"
            );
        }
    }

    #[test]
    fn real_yield_forward_rates_computed() {
        let input = default_real_yield_input();
        let out = compute_real_yield_curve(&input).unwrap();
        assert_eq!(out.forward_real_rates.len(), 3); // n-1 forward rates
    }

    #[test]
    fn real_yield_forward_breakeven_computed() {
        let input = default_real_yield_input();
        let out = compute_real_yield_curve(&input).unwrap();
        assert_eq!(out.forward_breakeven.len(), 3);
    }

    #[test]
    fn real_yield_breakeven_matches_spread() {
        let input = default_real_yield_input();
        let out = compute_real_yield_curve(&input).unwrap();
        for (i, bp) in out.breakeven_curve.iter().enumerate() {
            let expected =
                input.tips_securities[i].nominal_yield - input.tips_securities[i].real_yield;
            assert_eq!(bp.value, expected);
        }
    }

    #[test]
    fn real_yield_duration_positive() {
        let input = default_real_yield_input();
        let out = compute_real_yield_curve(&input).unwrap();
        for dp in &out.real_duration_by_maturity {
            assert!(dp.value > Decimal::ZERO, "Duration should be positive");
        }
    }

    #[test]
    fn real_yield_duration_increases_with_maturity() {
        let input = default_real_yield_input();
        let out = compute_real_yield_curve(&input).unwrap();
        for i in 1..out.real_duration_by_maturity.len() {
            assert!(
                out.real_duration_by_maturity[i].value > out.real_duration_by_maturity[i - 1].value,
                "Duration should increase with maturity"
            );
        }
    }

    #[test]
    fn real_yield_empty_securities_error() {
        let input = RealYieldInput {
            tips_securities: vec![],
        };
        assert!(compute_real_yield_curve(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Forward rate consistency
    // -----------------------------------------------------------------------

    #[test]
    fn forward_breakeven_consistency_with_spot() {
        // The forward breakevens should be consistent with spot breakevens:
        // spot_be(t2) * t2 = spot_be(t1) * t1 + fwd_be(t1,t2) * (t2-t1)
        let input = default_real_yield_input();
        let out = compute_real_yield_curve(&input).unwrap();
        for (i, fwd) in out.forward_breakeven.iter().enumerate() {
            let t1 = out.breakeven_curve[i].maturity;
            let t2 = out.breakeven_curve[i + 1].maturity;
            let be1 = out.breakeven_curve[i].value;
            let be2 = out.breakeven_curve[i + 1].value;
            let dt = t2 - t1;
            let reconstructed = (be1 * t1 + fwd.value * dt) / t2;
            let diff = (reconstructed - be2).abs();
            assert!(
                diff < dec!(0.0000001),
                "Forward breakeven inconsistency at maturity {}: diff={}",
                t2,
                diff
            );
        }
    }

    // -----------------------------------------------------------------------
    // CPI projection
    // -----------------------------------------------------------------------

    #[test]
    fn cpi_projection_compounds_correctly() {
        let input = default_tips_pricing_input();
        let out = compute_tips_pricing(&input).unwrap();
        // After 20 semi-annual periods at 2.5% annual (1.25% per period)
        let freq = Decimal::from(input.coupon_frequency);
        let period_rate = input.cpi_projected_annual_rate / freq;
        let mut expected_cpi = input.cpi_current;
        for cf in &out.projected_cashflows {
            expected_cpi = expected_cpi * (Decimal::ONE + period_rate);
            let diff = (cf.projected_cpi - expected_cpi).abs();
            assert!(
                diff < dec!(0.000001),
                "CPI projection mismatch at period {}",
                cf.period
            );
        }
    }

    // -----------------------------------------------------------------------
    // Math helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_decimal_exp_zero() {
        let result = decimal_exp(Decimal::ZERO);
        assert_eq!(result, Decimal::ONE);
    }

    #[test]
    fn test_decimal_exp_one() {
        let result = decimal_exp(Decimal::ONE);
        let diff = (result - dec!(2.718281828)).abs();
        assert!(
            diff < dec!(0.0001),
            "exp(1) should be ~2.71828, got {}",
            result
        );
    }

    #[test]
    fn test_decimal_ln_one() {
        let result = decimal_ln(Decimal::ONE);
        assert!(
            result.abs() < dec!(0.0001),
            "ln(1) should be ~0, got {}",
            result
        );
    }

    #[test]
    fn test_decimal_sqrt_four() {
        let result = decimal_sqrt(dec!(4));
        let diff = (result - dec!(2)).abs();
        assert!(
            diff < dec!(0.0000001),
            "sqrt(4) should be 2, got {}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Wrapper function test
    // -----------------------------------------------------------------------

    #[test]
    fn analyze_tips_pricing_wrapper() {
        let input = TipsAnalyticsInput {
            model: TipsModel::Pricing(default_tips_pricing_input()),
        };
        let out = analyze_tips(&input).unwrap();
        assert_eq!(out.methodology, "TIPS / Inflation-Linked Bond Analytics");
        match out.result {
            TipsAnalyticsOutput::Pricing(p) => {
                assert!(p.real_clean_price > Decimal::ZERO);
            }
            _ => panic!("Expected Pricing output"),
        }
    }

    #[test]
    fn analyze_tips_breakeven_wrapper() {
        let input = TipsAnalyticsInput {
            model: TipsModel::Breakeven(default_breakeven_input()),
        };
        let out = analyze_tips(&input).unwrap();
        match out.result {
            TipsAnalyticsOutput::Breakeven(b) => {
                assert!(b.breakeven_inflation > Decimal::ZERO);
            }
            _ => panic!("Expected Breakeven output"),
        }
    }

    #[test]
    fn analyze_tips_real_yield_wrapper() {
        let input = TipsAnalyticsInput {
            model: TipsModel::RealYield(default_real_yield_input()),
        };
        let out = analyze_tips(&input).unwrap();
        match out.result {
            TipsAnalyticsOutput::RealYield(r) => {
                assert!(!r.real_zero_curve.is_empty());
            }
            _ => panic!("Expected RealYield output"),
        }
    }
}
