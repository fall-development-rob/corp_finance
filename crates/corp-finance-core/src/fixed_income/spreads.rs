use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum Newton-Raphson iterations for YTM and Z-spread solvers.
const MAX_ITERATIONS: u32 = 50;

/// Convergence tolerance (1e-7).
const EPSILON: Decimal = dec!(0.0000001);

/// Basis point shift for spread duration calculation.
const ONE_BP: Decimal = dec!(0.0001);

/// Investment-grade ceiling in decimal (200 bps).
const IG_CEILING: Decimal = dec!(0.0200);

/// High-yield ceiling in decimal (1000 bps).
const HY_CEILING: Decimal = dec!(0.1000);

/// Default recovery rate for CDS calculation.
const DEFAULT_RECOVERY: Decimal = dec!(0.40);

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// A single point on the risk-free benchmark spot curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkPoint {
    /// Time to maturity in years.
    pub maturity: Decimal,
    /// Spot rate (annualised, as a decimal, e.g. 0.05 = 5%).
    pub rate: Rate,
}

/// Input for credit-spread calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditSpreadInput {
    /// Par / face value of the bond.
    pub face_value: Money,
    /// Annual coupon rate as a decimal (0.05 = 5%).
    pub coupon_rate: Rate,
    /// Number of coupon payments per year (1, 2, 4, or 12).
    pub coupon_frequency: u8,
    /// Dirty market price of the bond.
    pub market_price: Money,
    /// Years remaining until maturity.
    pub years_to_maturity: Decimal,
    /// Risk-free benchmark spot-rate curve (at least 2 points, sorted ascending
    /// by maturity).
    pub benchmark_curve: Vec<BenchmarkPoint>,
    /// Recovery rate for CDS spread estimate (default 0.40 if omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_rate: Option<Rate>,
    /// Annual default probability for CDS spread estimate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_probability: Option<Rate>,
}

/// Output of credit-spread calculations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditSpreadOutput {
    /// Interpolated spread: YTM minus linearly-interpolated benchmark yield at
    /// the bond's maturity.
    pub i_spread: Rate,
    /// Government spread (identical to i_spread when benchmark is a sovereign
    /// curve).
    pub g_spread: Rate,
    /// Zero-volatility spread: constant spread added to each spot rate on the
    /// benchmark curve that reprices the bond to its market price.
    pub z_spread: Rate,
    /// Simplified OAS estimate. Currently `None` (callable-bond logic not
    /// implemented).
    pub oas_estimate: Option<Rate>,
    /// Yield to maturity solved from the market price.
    pub ytm: Rate,
    /// Benchmark yield interpolated at the bond's maturity.
    pub benchmark_yield: Rate,
    /// Spread duration: percentage price sensitivity to a 1 bp parallel shift
    /// in the spread.
    pub spread_duration: Decimal,
    /// CDS spread estimate = (1 - recovery) * default_probability. Present
    /// only when `default_probability` is supplied.
    pub cds_spread: Option<Rate>,
    /// Qualitative credit-quality bucket derived from the z-spread.
    pub credit_quality_indicator: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Calculate a comprehensive suite of credit spreads for a fixed-rate bond.
///
/// Returns I-spread, G-spread, Z-spread, spread duration, an optional CDS
/// spread estimate, and a qualitative credit-quality indicator.
pub fn calculate_credit_spreads(
    input: &CreditSpreadInput,
) -> CorpFinanceResult<ComputationOutput<CreditSpreadOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // -- Validation ----------------------------------------------------------
    validate_input(input)?;

    // -- Derived quantities --------------------------------------------------
    let coupon_per_period =
        input.face_value * input.coupon_rate / Decimal::from(input.coupon_frequency);
    let total_periods = input.years_to_maturity * Decimal::from(input.coupon_frequency);

    // Build the cash-flow schedule: (time_in_years, cash_flow_amount)
    let cashflows = build_cashflow_schedule(input, coupon_per_period, total_periods);

    // -- YTM (Newton-Raphson) ------------------------------------------------
    let ytm = solve_ytm(input, &cashflows)?;

    // -- Benchmark yield at maturity (linear interpolation) ------------------
    let benchmark_yield = interpolate_rate(&input.benchmark_curve, input.years_to_maturity)?;

    // -- I-spread / G-spread -------------------------------------------------
    let i_spread = ytm - benchmark_yield;
    let g_spread = i_spread; // identical when benchmark is government curve

    // -- Z-spread (Newton-Raphson) -------------------------------------------
    let z_spread = solve_z_spread(input, &cashflows)?;

    // -- Spread duration -----------------------------------------------------
    let spread_duration = compute_spread_duration(input, &cashflows, z_spread)?;

    // -- CDS spread estimate -------------------------------------------------
    let cds_spread = input.default_probability.map(|pd| {
        let recovery = input.recovery_rate.unwrap_or(DEFAULT_RECOVERY);
        (Decimal::ONE - recovery) * pd
    });

    // -- Credit quality indicator --------------------------------------------
    let credit_quality_indicator = classify_credit_quality(z_spread);

    let output = CreditSpreadOutput {
        i_spread,
        g_spread,
        z_spread,
        oas_estimate: None, // callable-bond OAS not implemented
        ytm,
        benchmark_yield,
        spread_duration,
        cds_spread,
        credit_quality_indicator,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Credit Spreads (I-spread, G-spread, Z-spread, CDS)",
        &serde_json::json!({
            "ytm_method": "Newton-Raphson (50 iter, eps 1e-7)",
            "z_spread_method": "Newton-Raphson (50 iter, eps 1e-7)",
            "spread_duration_bump": "1 bp",
            "cds_model": "simplified annual premium: (1-R)*PD",
            "benchmark_interpolation": "linear",
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &CreditSpreadInput) -> CorpFinanceResult<()> {
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
            reason: "Face value must be positive".into(),
        });
    }
    if input.market_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_price".into(),
            reason: "Market price must be positive".into(),
        });
    }
    if !matches!(input.coupon_frequency, 1 | 2 | 4 | 12) {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_frequency".into(),
            reason: "Coupon frequency must be 1, 2, 4, or 12".into(),
        });
    }
    if input.years_to_maturity <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "years_to_maturity".into(),
            reason: "Years to maturity must be positive".into(),
        });
    }
    if input.benchmark_curve.len() < 2 {
        return Err(CorpFinanceError::InsufficientData(
            "Benchmark curve must contain at least 2 points".into(),
        ));
    }

    // Verify benchmark curve is sorted by maturity
    for w in input.benchmark_curve.windows(2) {
        if w[1].maturity <= w[0].maturity {
            return Err(CorpFinanceError::InvalidInput {
                field: "benchmark_curve".into(),
                reason: "Benchmark curve must be sorted ascending by maturity".into(),
            });
        }
    }

    if let Some(rr) = input.recovery_rate {
        if rr < Decimal::ZERO || rr > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "recovery_rate".into(),
                reason: "Recovery rate must be between 0 and 1".into(),
            });
        }
    }
    if let Some(pd) = input.default_probability {
        if pd < Decimal::ZERO || pd > Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "default_probability".into(),
                reason: "Default probability must be between 0 and 1".into(),
            });
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Cash-flow schedule builder
// ---------------------------------------------------------------------------

/// Produce a vector of (time_in_years, amount) pairs representing each coupon
/// payment and the final principal redemption.
fn build_cashflow_schedule(
    input: &CreditSpreadInput,
    coupon_per_period: Decimal,
    total_periods: Decimal,
) -> Vec<(Decimal, Decimal)> {
    let freq = Decimal::from(input.coupon_frequency);
    // Round up to the nearest integer number of periods to handle fractional
    // years gracefully.
    let n_periods = total_periods_as_u32(total_periods);
    let mut cfs = Vec::with_capacity(n_periods as usize);

    for i in 1..=n_periods {
        let t = Decimal::from(i) / freq;
        let mut cf = coupon_per_period;
        if i == n_periods {
            cf += input.face_value; // principal redemption at maturity
        }
        cfs.push((t, cf));
    }

    cfs
}

/// Convert a Decimal number of periods to a u32, rounding to nearest integer
/// but at least 1.
fn total_periods_as_u32(total: Decimal) -> u32 {
    let rounded = total.round().to_string().parse::<u32>().unwrap_or(1);
    if rounded == 0 {
        1
    } else {
        rounded
    }
}

// ---------------------------------------------------------------------------
// YTM solver (Newton-Raphson)
// ---------------------------------------------------------------------------

/// Price a bond given a periodic yield `y` (annualised, compounded at coupon
/// frequency). Uses iterative discount-factor multiplication to avoid `powd`
/// precision drift.
fn bond_price_from_yield(cashflows: &[(Decimal, Decimal)], y_annual: Decimal, freq: u8) -> Decimal {
    let freq_d = Decimal::from(freq);
    let y_per_period = y_annual / freq_d;
    let one_plus_y = Decimal::ONE + y_per_period;

    let mut price = Decimal::ZERO;

    for &(t, cf) in cashflows {
        // Number of periods to this cash flow.
        let periods = (t * freq_d).round();
        let n = periods.to_string().parse::<u32>().unwrap_or(1);

        // Iterative discount factor to avoid powd precision drift.
        let mut discount = Decimal::ONE;
        for _ in 0..n {
            discount *= one_plus_y;
        }

        if !discount.is_zero() {
            price += cf / discount;
        }
    }

    price
}

/// Derivative of the bond-price function w.r.t. annual yield. Computed
/// analytically as: dP/dy = -1/freq * sum[ t_i * CF_i / (1+y/freq)^(n_i+1) ]
/// where n_i is the number of compounding periods.
fn bond_price_derivative(cashflows: &[(Decimal, Decimal)], y_annual: Decimal, freq: u8) -> Decimal {
    let freq_d = Decimal::from(freq);
    let y_per_period = y_annual / freq_d;
    let one_plus_y = Decimal::ONE + y_per_period;

    let mut deriv = Decimal::ZERO;

    for &(t, cf) in cashflows {
        let periods = (t * freq_d).round();
        let n = periods.to_string().parse::<u32>().unwrap_or(1);

        // (1+y/freq)^(n+1) via iterative multiplication.
        let mut discount = Decimal::ONE;
        for _ in 0..=n {
            discount *= one_plus_y;
        }

        if !discount.is_zero() {
            // dP/dy = sum[ -n/freq * cf / (1+y/freq)^(n+1) ]
            deriv -= Decimal::from(n) / freq_d * cf / discount;
        }
    }

    deriv
}

/// Solve for YTM given the dirty market price using Newton-Raphson.
fn solve_ytm(
    input: &CreditSpreadInput,
    cashflows: &[(Decimal, Decimal)],
) -> CorpFinanceResult<Rate> {
    // Initial guess: coupon yield adjusted for premium/discount.
    let mut y = if input.face_value.is_zero() {
        dec!(0.05)
    } else {
        input.coupon_rate
            + (input.face_value - input.market_price) / (input.face_value * input.years_to_maturity)
    };

    // Clamp initial guess into a reasonable range.
    if y < dec!(-0.50) {
        y = dec!(-0.50);
    } else if y > dec!(2.0) {
        y = dec!(2.0);
    }

    let freq = input.coupon_frequency;

    for iter in 0..MAX_ITERATIONS {
        let price = bond_price_from_yield(cashflows, y, freq);
        let residual = price - input.market_price;

        if residual.abs() < EPSILON {
            return Ok(y);
        }

        let dpdy = bond_price_derivative(cashflows, y, freq);
        if dpdy.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "YTM solver".into(),
                iterations: iter,
                last_delta: residual,
            });
        }

        y -= residual / dpdy;

        // Guard bounds.
        if y < dec!(-0.99) {
            y = dec!(-0.99);
        } else if y > dec!(5.0) {
            y = dec!(5.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "YTM solver".into(),
        iterations: MAX_ITERATIONS,
        last_delta: bond_price_from_yield(cashflows, y, freq) - input.market_price,
    })
}

// ---------------------------------------------------------------------------
// Benchmark interpolation
// ---------------------------------------------------------------------------

/// Linearly interpolate a rate from the benchmark curve for a given maturity.
///
/// If `t` is below the first point or above the last, we extrapolate from the
/// nearest two points.
fn interpolate_rate(curve: &[BenchmarkPoint], t: Decimal) -> CorpFinanceResult<Rate> {
    if curve.len() < 2 {
        return Err(CorpFinanceError::InsufficientData(
            "Need at least 2 benchmark points for interpolation".into(),
        ));
    }

    // Find the two bracketing points.
    let idx = curve.partition_point(|p| p.maturity < t);

    let (left, right) = if idx == 0 {
        // t is at or before first point -- extrapolate from first two.
        (&curve[0], &curve[1])
    } else if idx >= curve.len() {
        // t is beyond last point -- extrapolate from last two.
        (&curve[curve.len() - 2], &curve[curve.len() - 1])
    } else {
        (&curve[idx - 1], &curve[idx])
    };

    let span = right.maturity - left.maturity;
    if span.is_zero() {
        return Ok(left.rate);
    }

    let weight = (t - left.maturity) / span;
    Ok(left.rate + weight * (right.rate - left.rate))
}

// ---------------------------------------------------------------------------
// Z-spread solver (Newton-Raphson)
// ---------------------------------------------------------------------------

/// Price a bond using spot rates from the benchmark curve plus a constant
/// spread `z`. Each cash flow is discounted at its own interpolated spot rate
/// plus `z`, using iterative multiplication to avoid `powd` precision drift.
///
/// Formula: P = sum[ CF_i / (1 + s_i + z)^t_i ]
fn price_with_z_spread(
    cashflows: &[(Decimal, Decimal)],
    curve: &[BenchmarkPoint],
    z: Decimal,
) -> CorpFinanceResult<Decimal> {
    let mut price = Decimal::ZERO;

    for &(t, cf) in cashflows {
        let spot = interpolate_rate(curve, t)?;
        let annual_rate = spot + z;

        // Discount using iterative multiplication. We split t into whole years
        // and a fractional part. For the whole-year portion we multiply
        // iteratively; for the fractional part we use a single-step linear
        // interpolation to avoid powd.
        let discount = iterative_discount(annual_rate, t);

        if !discount.is_zero() {
            price += cf / discount;
        }
    }

    Ok(price)
}

/// Compute (1 + r)^t via iterative multiplication for the integer part and
/// a single linear-interpolation step for any fractional remainder.
///
/// This avoids `Decimal::powd` precision drift while remaining accurate for
/// typical fixed-income tenors.
fn iterative_discount(annual_rate: Decimal, t: Decimal) -> Decimal {
    let one_plus_r = Decimal::ONE + annual_rate;

    // Whole years.
    let whole = t.floor();
    let n = whole.to_string().parse::<u32>().unwrap_or(0);

    let mut factor = Decimal::ONE;
    for _ in 0..n {
        factor *= one_plus_r;
    }

    // Fractional year: linear interpolation between factor and factor*(1+r).
    let frac = t - whole;
    if frac > Decimal::ZERO {
        factor *= Decimal::ONE + frac * annual_rate;
    }

    factor
}

/// Derivative of price w.r.t. z-spread.
///
/// dP/dz = sum[ -t_i * CF_i / (1 + s_i + z)^(t_i + 1) ]
///
/// Approximated using the same iterative discount approach.
fn z_spread_price_derivative(
    cashflows: &[(Decimal, Decimal)],
    curve: &[BenchmarkPoint],
    z: Decimal,
) -> CorpFinanceResult<Decimal> {
    let mut deriv = Decimal::ZERO;

    for &(t, cf) in cashflows {
        let spot = interpolate_rate(curve, t)?;
        let annual_rate = spot + z;
        let one_plus_r = Decimal::ONE + annual_rate;

        // (1 + s + z)^(t+1) via iterative multiplication.
        let discount_t_plus_1 = iterative_discount(annual_rate, t) * one_plus_r;

        if !discount_t_plus_1.is_zero() {
            deriv -= t * cf / discount_t_plus_1;
        }
    }

    Ok(deriv)
}

/// Solve for the Z-spread using Newton-Raphson.
fn solve_z_spread(
    input: &CreditSpreadInput,
    cashflows: &[(Decimal, Decimal)],
) -> CorpFinanceResult<Rate> {
    // Initial guess: I-spread is a reasonable starting point.
    let benchmark_yield = interpolate_rate(&input.benchmark_curve, input.years_to_maturity)?;
    let ytm_guess = input.coupon_rate
        + (input.face_value - input.market_price) / (input.face_value * input.years_to_maturity);
    let mut z = ytm_guess - benchmark_yield;

    // Clamp.
    if z < dec!(-0.50) {
        z = dec!(-0.50);
    } else if z > dec!(2.0) {
        z = dec!(2.0);
    }

    for iter in 0..MAX_ITERATIONS {
        let price = price_with_z_spread(cashflows, &input.benchmark_curve, z)?;
        let residual = price - input.market_price;

        if residual.abs() < EPSILON {
            return Ok(z);
        }

        let dprice_dz = z_spread_price_derivative(cashflows, &input.benchmark_curve, z)?;
        if dprice_dz.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "Z-spread solver".into(),
                iterations: iter,
                last_delta: residual,
            });
        }

        z -= residual / dprice_dz;

        if z < dec!(-0.99) {
            z = dec!(-0.99);
        } else if z > dec!(5.0) {
            z = dec!(5.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "Z-spread solver".into(),
        iterations: MAX_ITERATIONS,
        last_delta: price_with_z_spread(cashflows, &input.benchmark_curve, z)? - input.market_price,
    })
}

// ---------------------------------------------------------------------------
// Spread duration
// ---------------------------------------------------------------------------

/// Spread duration: sensitivity of the bond price to a 1 bp parallel shift
/// in the z-spread.
///
/// SD = (P(z - delta) - P(z + delta)) / (2 * P * delta)
fn compute_spread_duration(
    input: &CreditSpreadInput,
    cashflows: &[(Decimal, Decimal)],
    z: Decimal,
) -> CorpFinanceResult<Decimal> {
    let p_down = price_with_z_spread(cashflows, &input.benchmark_curve, z - ONE_BP)?;
    let p_up = price_with_z_spread(cashflows, &input.benchmark_curve, z + ONE_BP)?;
    let p_base = input.market_price;

    if p_base.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "spread duration denominator (market_price)".into(),
        });
    }

    let denom = dec!(2) * p_base * ONE_BP;
    if denom.is_zero() {
        return Err(CorpFinanceError::DivisionByZero {
            context: "spread duration denominator".into(),
        });
    }

    Ok((p_down - p_up) / denom)
}

// ---------------------------------------------------------------------------
// Credit quality classification
// ---------------------------------------------------------------------------

/// Classify credit quality based on z-spread:
///   < 200 bps -> "investment_grade"
///   200 - 1000 bps -> "high_yield"
///   > 1000 bps -> "distressed"
fn classify_credit_quality(z_spread: Rate) -> String {
    if z_spread < IG_CEILING {
        "investment_grade".to_string()
    } else if z_spread < HY_CEILING {
        "high_yield".to_string()
    } else {
        "distressed".to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Build a flat benchmark curve at a given rate.
    fn flat_curve(rate: Rate) -> Vec<BenchmarkPoint> {
        vec![
            BenchmarkPoint {
                maturity: dec!(1),
                rate,
            },
            BenchmarkPoint {
                maturity: dec!(5),
                rate,
            },
            BenchmarkPoint {
                maturity: dec!(10),
                rate,
            },
            BenchmarkPoint {
                maturity: dec!(30),
                rate,
            },
        ]
    }

    /// Build a sample upward-sloping benchmark curve.
    fn sample_curve() -> Vec<BenchmarkPoint> {
        vec![
            BenchmarkPoint {
                maturity: dec!(1),
                rate: dec!(0.03),
            },
            BenchmarkPoint {
                maturity: dec!(2),
                rate: dec!(0.035),
            },
            BenchmarkPoint {
                maturity: dec!(5),
                rate: dec!(0.04),
            },
            BenchmarkPoint {
                maturity: dec!(10),
                rate: dec!(0.045),
            },
            BenchmarkPoint {
                maturity: dec!(30),
                rate: dec!(0.05),
            },
        ]
    }

    /// A par bond on a flat curve should have near-zero spread.
    fn par_bond_input(curve_rate: Rate) -> CreditSpreadInput {
        // A 5-year semi-annual bond where coupon = curve rate and price = par.
        CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: curve_rate,
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(curve_rate),
            recovery_rate: None,
            default_probability: None,
        }
    }

    // -----------------------------------------------------------------------
    // 1. Par bond -> near-zero spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_par_bond_zero_spread() {
        let input = par_bond_input(dec!(0.05));
        let result = calculate_credit_spreads(&input).unwrap();
        let out = &result.result;

        // YTM should equal coupon rate for a par bond.
        assert!(
            (out.ytm - dec!(0.05)).abs() < dec!(0.001),
            "YTM {}, expected ~0.05",
            out.ytm
        );

        // I-spread should be near zero.
        assert!(
            out.i_spread.abs() < dec!(0.001),
            "I-spread {}, expected ~0",
            out.i_spread
        );

        // Z-spread should be near zero.
        assert!(
            out.z_spread.abs() < dec!(0.002),
            "Z-spread {}, expected ~0",
            out.z_spread
        );
    }

    // -----------------------------------------------------------------------
    // 2. Discount bond -> positive I-spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_i_spread_positive() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.04),
            coupon_frequency: 2,
            market_price: dec!(950),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.03)),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        assert!(
            result.result.i_spread > Decimal::ZERO,
            "I-spread should be positive for a discount bond, got {}",
            result.result.i_spread
        );
    }

    // -----------------------------------------------------------------------
    // 3. Z-spread positive for discount bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_z_spread_positive() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.04),
            coupon_frequency: 2,
            market_price: dec!(950),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.03)),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        assert!(
            result.result.z_spread > Decimal::ZERO,
            "Z-spread should be positive, got {}",
            result.result.z_spread
        );
    }

    // -----------------------------------------------------------------------
    // 4. Z-spread vs I-spread: generally close but not identical
    // -----------------------------------------------------------------------
    #[test]
    fn test_z_spread_vs_i_spread() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(960),
            years_to_maturity: dec!(7),
            benchmark_curve: sample_curve(),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        let out = &result.result;

        // Both should be positive.
        assert!(out.i_spread > Decimal::ZERO);
        assert!(out.z_spread > Decimal::ZERO);

        // They should be within 100 bps of each other for a normal bond.
        assert!(
            (out.z_spread - out.i_spread).abs() < dec!(0.01),
            "Z-spread {} and I-spread {} should be relatively close",
            out.z_spread,
            out.i_spread
        );
    }

    // -----------------------------------------------------------------------
    // 5. Flat curve: Z-spread should equal I-spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_z_spread_flat_curve_equals_i_spread() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.06),
            coupon_frequency: 2,
            market_price: dec!(970),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.04)),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        let out = &result.result;

        // For a flat curve, Z-spread and I-spread should be very close.
        let diff = (out.z_spread - out.i_spread).abs();
        assert!(
            diff < dec!(0.002),
            "On a flat curve, Z-spread ({}) and I-spread ({}) should be nearly equal (diff: {})",
            out.z_spread,
            out.i_spread,
            diff
        );
    }

    // -----------------------------------------------------------------------
    // 6. Spread duration is positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_spread_duration_positive() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(980),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.04)),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        assert!(
            result.result.spread_duration > Decimal::ZERO,
            "Spread duration should be positive, got {}",
            result.result.spread_duration
        );

        // Spread duration should be reasonable (roughly in the years-to-maturity
        // ballpark for a coupon bond).
        assert!(
            result.result.spread_duration < dec!(10),
            "Spread duration {} seems unreasonably high for a 5y bond",
            result.result.spread_duration
        );
    }

    // -----------------------------------------------------------------------
    // 7. CDS spread calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_cds_spread_calculation() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.05)),
            recovery_rate: Some(dec!(0.40)),
            default_probability: Some(dec!(0.02)),
        };
        let result = calculate_credit_spreads(&input).unwrap();
        let cds = result.result.cds_spread.unwrap();

        // CDS = (1 - 0.40) * 0.02 = 0.012 (120 bps)
        assert_eq!(cds, dec!(0.012));
    }

    // -----------------------------------------------------------------------
    // 8. Investment-grade indicator (low spread)
    // -----------------------------------------------------------------------
    #[test]
    fn test_investment_grade_indicator() {
        // Bond priced near par on the curve -> low spread
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.045),
            coupon_frequency: 2,
            market_price: dec!(995),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.04)),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        assert_eq!(
            result.result.credit_quality_indicator, "investment_grade",
            "Z-spread {} should classify as investment_grade",
            result.result.z_spread
        );
    }

    // -----------------------------------------------------------------------
    // 9. High-yield indicator (medium spread)
    // -----------------------------------------------------------------------
    #[test]
    fn test_high_yield_indicator() {
        // Bond priced at a steep discount to generate a ~300-500 bp spread.
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(850),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.03)),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        let z = result.result.z_spread;
        assert!(
            z >= IG_CEILING && z < HY_CEILING,
            "Z-spread {} should be in high_yield range [{}, {})",
            z,
            IG_CEILING,
            HY_CEILING
        );
        assert_eq!(result.result.credit_quality_indicator, "high_yield");
    }

    // -----------------------------------------------------------------------
    // 10. Distressed indicator (high spread)
    // -----------------------------------------------------------------------
    #[test]
    fn test_distressed_indicator() {
        // Bond priced at a very steep discount.
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(600),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.03)),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        assert_eq!(
            result.result.credit_quality_indicator, "distressed",
            "Z-spread {} should classify as distressed",
            result.result.z_spread
        );
    }

    // -----------------------------------------------------------------------
    // 11. Benchmark interpolation accuracy
    // -----------------------------------------------------------------------
    #[test]
    fn test_benchmark_interpolation() {
        let curve = sample_curve();

        // Interpolate at t=3, which is between (2, 0.035) and (5, 0.04).
        let rate = interpolate_rate(&curve, dec!(3)).unwrap();
        // Linear: 0.035 + (3-2)/(5-2) * (0.04 - 0.035) = 0.035 + 1/3 * 0.005
        //       = 0.035 + 0.001666... ~= 0.036667
        let expected = dec!(0.035) + (dec!(1) / dec!(3)) * dec!(0.005);
        assert!(
            (rate - expected).abs() < dec!(0.0001),
            "Interpolated rate {} expected ~{}",
            rate,
            expected
        );

        // Interpolate at exact point t=5 -> 0.04.
        let rate_exact = interpolate_rate(&curve, dec!(5)).unwrap();
        assert!(
            (rate_exact - dec!(0.04)).abs() < dec!(0.0001),
            "At exact point, rate {} expected 0.04",
            rate_exact
        );
    }

    // -----------------------------------------------------------------------
    // 12. YTM solve accuracy
    // -----------------------------------------------------------------------
    #[test]
    fn test_ytm_solve_accuracy() {
        // A known bond: 5% semi-annual coupon, 5y, priced at par -> YTM = 5%.
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.05)),
            recovery_rate: None,
            default_probability: None,
        };
        let result = calculate_credit_spreads(&input).unwrap();
        assert!(
            (result.result.ytm - dec!(0.05)).abs() < dec!(0.001),
            "YTM {} should be ~0.05 for a par bond",
            result.result.ytm
        );

        // A bond priced at 950 with 6% coupon, 2y annual -> YTM should be
        // higher than coupon.
        let input2 = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.06),
            coupon_frequency: 1,
            market_price: dec!(950),
            years_to_maturity: dec!(2),
            benchmark_curve: flat_curve(dec!(0.04)),
            recovery_rate: None,
            default_probability: None,
        };
        let result2 = calculate_credit_spreads(&input2).unwrap();
        assert!(
            result2.result.ytm > dec!(0.06),
            "YTM {} should exceed coupon rate 0.06 for a discount bond",
            result2.result.ytm
        );
    }

    // -----------------------------------------------------------------------
    // 13. Invalid face value -> error
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_face_value_error() {
        let input = CreditSpreadInput {
            face_value: dec!(-100),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.05)),
            recovery_rate: None,
            default_probability: None,
        };
        let err = calculate_credit_spreads(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "face_value");
            }
            other => panic!("Expected InvalidInput for face_value, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 14. Insufficient benchmark points -> error
    // -----------------------------------------------------------------------
    #[test]
    fn test_insufficient_benchmark_points_error() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: vec![BenchmarkPoint {
                maturity: dec!(5),
                rate: dec!(0.04),
            }],
            recovery_rate: None,
            default_probability: None,
        };
        let err = calculate_credit_spreads(&input).unwrap_err();
        match err {
            CorpFinanceError::InsufficientData(_) => {}
            other => panic!("Expected InsufficientData, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 15. Recovery rate out of bounds -> error
    // -----------------------------------------------------------------------
    #[test]
    fn test_recovery_rate_bounds_error() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.05)),
            recovery_rate: Some(dec!(1.5)),
            default_probability: Some(dec!(0.01)),
        };
        let err = calculate_credit_spreads(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "recovery_rate");
            }
            other => panic!("Expected InvalidInput for recovery_rate, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Metadata populated correctly
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = par_bond_input(dec!(0.05));
        let result = calculate_credit_spreads(&input).unwrap();

        assert_eq!(
            result.methodology,
            "Credit Spreads (I-spread, G-spread, Z-spread, CDS)"
        );
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 17. CDS spread with default recovery
    // -----------------------------------------------------------------------
    #[test]
    fn test_cds_spread_default_recovery() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.05)),
            recovery_rate: None, // should default to 0.40
            default_probability: Some(dec!(0.03)),
        };
        let result = calculate_credit_spreads(&input).unwrap();
        let cds = result.result.cds_spread.unwrap();

        // CDS = (1 - 0.40) * 0.03 = 0.018 (180 bps)
        assert_eq!(cds, dec!(0.018));
    }

    // -----------------------------------------------------------------------
    // 18. No default probability -> CDS spread is None
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_default_probability_no_cds() {
        let input = par_bond_input(dec!(0.05));
        let result = calculate_credit_spreads(&input).unwrap();
        assert!(result.result.cds_spread.is_none());
    }

    // -----------------------------------------------------------------------
    // 19. Invalid coupon frequency -> error
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_coupon_frequency_error() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 3, // invalid
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.05)),
            recovery_rate: None,
            default_probability: None,
        };
        let err = calculate_credit_spreads(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "coupon_frequency");
            }
            other => panic!("Expected InvalidInput for coupon_frequency, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // 20. Default probability out of bounds -> error
    // -----------------------------------------------------------------------
    #[test]
    fn test_default_probability_bounds_error() {
        let input = CreditSpreadInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            market_price: dec!(1000),
            years_to_maturity: dec!(5),
            benchmark_curve: flat_curve(dec!(0.05)),
            recovery_rate: Some(dec!(0.40)),
            default_probability: Some(dec!(1.5)),
        };
        let err = calculate_credit_spreads(&input).unwrap_err();
        match err {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "default_probability");
            }
            other => panic!("Expected InvalidInput for default_probability, got {other:?}"),
        }
    }
}
