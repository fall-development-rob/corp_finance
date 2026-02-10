//! Bond pricing module for institutional-grade fixed income analytics.
//!
//! Supports multiple day count conventions, accrued interest, clean/dirty pricing,
//! callable bond yield-to-call (YTC) via Newton's method, and yield-to-worst (YTW).

use chrono::{Datelike, NaiveDate};
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

const NEWTON_MAX_ITERATIONS: u32 = 50;
const NEWTON_EPSILON: Decimal = dec!(0.0000001);

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Day count convention for computing accrued interest and period fractions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DayCountConvention {
    /// 30/360 US corporate convention
    Thirty360,
    /// ACT/360 money market convention
    Actual360,
    /// ACT/365 fixed (UK gilts)
    Actual365,
    /// ACT/ACT (US Treasury)
    ActualActual,
}

/// A single bond cashflow entry (coupon, principal, or both).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondCashflow {
    pub date: NaiveDate,
    pub amount: Money,
    pub cashflow_type: String,
}

/// Input parameters for bond pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondPricingInput {
    /// Par / face value (typically 1000)
    pub face_value: Money,
    /// Annual coupon rate as a decimal (e.g. 0.05 = 5%)
    pub coupon_rate: Rate,
    /// Coupons per year: 1 = annual, 2 = semi-annual, 4 = quarterly, 12 = monthly
    pub coupon_frequency: u8,
    /// Yield to maturity as a decimal
    pub ytm: Rate,
    /// Settlement (valuation) date
    pub settlement_date: NaiveDate,
    /// Maturity date
    pub maturity_date: NaiveDate,
    /// Day count convention
    pub day_count: DayCountConvention,
    /// Call price (for callable bonds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_price: Option<Money>,
    /// Call date (for callable bonds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_date: Option<NaiveDate>,
}

/// Output of bond pricing computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondPricingOutput {
    /// Clean price (excludes accrued interest)
    pub clean_price: Money,
    /// Dirty price (= clean + accrued interest)
    pub dirty_price: Money,
    /// Accrued interest from last coupon to settlement
    pub accrued_interest: Money,
    /// Current yield = annual coupon / clean price
    pub current_yield: Rate,
    /// Years remaining to maturity
    pub years_to_maturity: Decimal,
    /// Number of remaining coupon payments
    pub num_remaining_coupons: u32,
    /// Coupon payment per period
    pub coupon_amount: Money,
    /// Full schedule of future cashflows (coupons + principal)
    pub total_cashflows: Vec<BondCashflow>,
    /// Yield to call (if callable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ytc: Option<Rate>,
    /// Yield to worst = min(YTM, YTC) if callable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ytw: Option<Rate>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Price a bond and compute clean/dirty prices, accrued interest, current yield,
/// cashflow schedule, and optional yield-to-call / yield-to-worst.
pub fn price_bond(
    input: &BondPricingInput,
) -> CorpFinanceResult<ComputationOutput<BondPricingOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    validate_input(input)?;

    let freq = Decimal::from(input.coupon_frequency);
    let coupon_amount = input.face_value * input.coupon_rate / freq;
    let annual_coupon = input.face_value * input.coupon_rate;

    // --- Build cashflow schedule ---
    let coupon_dates = generate_coupon_dates(
        input.settlement_date,
        input.maturity_date,
        input.coupon_frequency,
    );
    let num_remaining_coupons = coupon_dates.len() as u32;

    let total_cashflows = build_cashflow_schedule(&coupon_dates, coupon_amount, input.face_value);

    // --- Accrued interest ---
    let accrued_interest = compute_accrued_interest(
        input.settlement_date,
        input.maturity_date,
        input.coupon_frequency,
        coupon_amount,
        input.day_count,
    );

    // --- Clean price (PV of future cashflows at YTM) ---
    let clean_price = compute_clean_price(
        input.settlement_date,
        &coupon_dates,
        coupon_amount,
        input.face_value,
        input.ytm,
        input.coupon_frequency,
        input.day_count,
    );

    // --- Dirty price ---
    let dirty_price = clean_price + accrued_interest;

    // --- Current yield ---
    let current_yield = if clean_price > Decimal::ZERO {
        annual_coupon / clean_price
    } else {
        warnings.push("Clean price is zero or negative; current yield undefined".into());
        Decimal::ZERO
    };

    // --- Years to maturity ---
    let days_to_maturity = (input.maturity_date - input.settlement_date).num_days();
    let years_to_maturity = Decimal::from(days_to_maturity) / dec!(365.25);

    // --- YTC / YTW for callable bonds ---
    let (ytc, ytw) =
        if let (Some(call_price), Some(call_date)) = (input.call_price, input.call_date) {
            match compute_ytc(
                dirty_price,
                input.settlement_date,
                call_date,
                coupon_amount,
                call_price,
                input.coupon_frequency,
                input.day_count,
            ) {
                Ok(ytc_val) => {
                    let ytw_val = if ytc_val < input.ytm {
                        ytc_val
                    } else {
                        input.ytm
                    };
                    (Some(ytc_val), Some(ytw_val))
                }
                Err(_) => {
                    warnings.push("YTC Newton's method did not converge".into());
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

    let output = BondPricingOutput {
        clean_price,
        dirty_price,
        accrued_interest,
        current_yield,
        years_to_maturity,
        num_remaining_coupons,
        coupon_amount,
        total_cashflows,
        ytc,
        ytw,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Bond Pricing — PV of cashflows with day count convention",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &BondPricingInput) -> CorpFinanceResult<()> {
    if input.face_value <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "face_value".into(),
            reason: "Face value must be positive".into(),
        });
    }
    if input.coupon_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_rate".into(),
            reason: "Coupon rate cannot be negative".into(),
        });
    }
    if !matches!(input.coupon_frequency, 1 | 2 | 4 | 12) {
        return Err(CorpFinanceError::InvalidInput {
            field: "coupon_frequency".into(),
            reason: "Coupon frequency must be 1, 2, 4, or 12".into(),
        });
    }
    if input.maturity_date <= input.settlement_date {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_date".into(),
            reason: "Maturity date must be after settlement date".into(),
        });
    }
    if let Some(call_date) = input.call_date {
        if call_date <= input.settlement_date {
            return Err(CorpFinanceError::InvalidInput {
                field: "call_date".into(),
                reason: "Call date must be after settlement date".into(),
            });
        }
        if call_date >= input.maturity_date {
            return Err(CorpFinanceError::InvalidInput {
                field: "call_date".into(),
                reason: "Call date must be before maturity date".into(),
            });
        }
    }
    if input.call_price.is_some() != input.call_date.is_some() {
        return Err(CorpFinanceError::InvalidInput {
            field: "call_price/call_date".into(),
            reason: "Both call_price and call_date must be provided together, or neither".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Day count fraction helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Coupon schedule generation
// ---------------------------------------------------------------------------

/// Generate all future coupon dates from settlement to maturity (inclusive of maturity).
/// Dates are generated backwards from maturity at regular intervals.
fn generate_coupon_dates(
    settlement: NaiveDate,
    maturity: NaiveDate,
    frequency: u8,
) -> Vec<NaiveDate> {
    let months_per_period = 12 / frequency as i32;
    let mut dates = Vec::new();

    // Walk backwards from maturity
    let mut date = maturity;
    loop {
        if date > settlement {
            dates.push(date);
        } else {
            break;
        }
        date = subtract_months(date, months_per_period);
    }

    dates.sort();
    dates
}

/// Find the last coupon date on or before the settlement date.
fn last_coupon_date_before(settlement: NaiveDate, maturity: NaiveDate, frequency: u8) -> NaiveDate {
    let months_per_period = 12 / frequency as i32;
    let mut date = maturity;

    // Walk backwards from maturity until we find a date <= settlement
    loop {
        let prev = subtract_months(date, months_per_period);
        if prev <= settlement {
            return prev;
        }
        date = prev;
    }
}

/// Subtract a number of months from a date, clamping the day to the month's max.
fn subtract_months(date: NaiveDate, months: i32) -> NaiveDate {
    let total_months = date.year() * 12 + date.month() as i32 - 1 - months;
    let new_year = total_months.div_euclid(12);
    let new_month = (total_months.rem_euclid(12) + 1) as u32;
    let max_day = days_in_month(new_year, new_month);
    let day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap_or(date)
}

/// Add a number of months to a date, clamping the day to the month's max.
fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    let total_months = date.year() * 12 + date.month() as i32 - 1 + months;
    let new_year = total_months.div_euclid(12);
    let new_month = (total_months.rem_euclid(12) + 1) as u32;
    let max_day = days_in_month(new_year, new_month);
    let day = date.day().min(max_day);
    NaiveDate::from_ymd_opt(new_year, new_month, day).unwrap_or(date)
}

/// Number of days in a given month/year.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

// ---------------------------------------------------------------------------
// Cashflow schedule
// ---------------------------------------------------------------------------

fn build_cashflow_schedule(
    coupon_dates: &[NaiveDate],
    coupon_amount: Money,
    face_value: Money,
) -> Vec<BondCashflow> {
    let mut cashflows = Vec::with_capacity(coupon_dates.len());

    for (i, &date) in coupon_dates.iter().enumerate() {
        let is_last = i == coupon_dates.len() - 1;
        if is_last {
            cashflows.push(BondCashflow {
                date,
                amount: coupon_amount + face_value,
                cashflow_type: "coupon+principal".into(),
            });
        } else {
            cashflows.push(BondCashflow {
                date,
                amount: coupon_amount,
                cashflow_type: "coupon".into(),
            });
        }
    }

    cashflows
}

// ---------------------------------------------------------------------------
// Accrued interest
// ---------------------------------------------------------------------------

fn compute_accrued_interest(
    settlement: NaiveDate,
    maturity: NaiveDate,
    frequency: u8,
    coupon_amount: Money,
    day_count: DayCountConvention,
) -> Money {
    let last_coupon = last_coupon_date_before(settlement, maturity, frequency);
    let months_per_period = 12 / frequency as i32;
    let next_coupon = add_months(last_coupon, months_per_period);

    // Fraction of the current coupon period that has elapsed
    let accrued_fraction = match day_count {
        DayCountConvention::Thirty360 => {
            let num = thirty_360_days(last_coupon, settlement);
            let den = thirty_360_days(last_coupon, next_coupon);
            if den == 0 {
                Decimal::ZERO
            } else {
                Decimal::from(num) / Decimal::from(den)
            }
        }
        DayCountConvention::Actual360 => {
            let actual_days = (settlement - last_coupon).num_days();
            let period_days = dec!(360) / Decimal::from(frequency);
            Decimal::from(actual_days) / period_days
        }
        DayCountConvention::Actual365 => {
            let actual_days = (settlement - last_coupon).num_days();
            let period_days = dec!(365) / Decimal::from(frequency);
            Decimal::from(actual_days) / period_days
        }
        DayCountConvention::ActualActual => {
            let actual_days = (settlement - last_coupon).num_days();
            let period_days = (next_coupon - last_coupon).num_days();
            if period_days == 0 {
                Decimal::ZERO
            } else {
                Decimal::from(actual_days) / Decimal::from(period_days)
            }
        }
    };

    coupon_amount * accrued_fraction
}

/// Compute 30/360 day count (raw days, not fraction).
fn thirty_360_days(start: NaiveDate, end: NaiveDate) -> i32 {
    let mut d1 = start.day() as i32;
    let mut d2 = end.day() as i32;
    let m1 = start.month() as i32;
    let m2 = end.month() as i32;
    let y1 = start.year();
    let y2 = end.year();

    if d1 == 31 {
        d1 = 30;
    }
    if d2 == 31 && d1 >= 30 {
        d2 = 30;
    }

    (y2 - y1) * 360 + (m2 - m1) * 30 + (d2 - d1)
}

// ---------------------------------------------------------------------------
// Clean price (PV of future cashflows)
// ---------------------------------------------------------------------------

/// Compute clean price as PV of all future cashflows discounted at YTM.
///
/// Uses iterative multiplication for discount factors (NOT powd) per project
/// convention. The discount is computed from the next coupon date, adjusted for
/// the fractional period between settlement and the next coupon.
fn compute_clean_price(
    settlement: NaiveDate,
    coupon_dates: &[NaiveDate],
    coupon_amount: Money,
    face_value: Money,
    ytm: Rate,
    frequency: u8,
    day_count: DayCountConvention,
) -> Money {
    if coupon_dates.is_empty() {
        return Decimal::ZERO;
    }

    let freq = Decimal::from(frequency);
    let periodic_yield = ytm / freq;

    // Fractional first period: fraction of the current period remaining
    // (from settlement to the next coupon date)
    let last_coupon =
        last_coupon_date_before(settlement, coupon_dates[coupon_dates.len() - 1], frequency);
    let months_per_period = 12 / frequency as i32;
    let next_coupon_from_last = add_months(last_coupon, months_per_period);

    let fraction_remaining = match day_count {
        DayCountConvention::Thirty360 => {
            let num = thirty_360_days(settlement, next_coupon_from_last);
            let den = thirty_360_days(last_coupon, next_coupon_from_last);
            if den == 0 {
                Decimal::ONE
            } else {
                Decimal::from(num) / Decimal::from(den)
            }
        }
        DayCountConvention::Actual360 => {
            let num = (next_coupon_from_last - settlement).num_days();
            let den_days = dec!(360) / Decimal::from(frequency);
            Decimal::from(num) / den_days
        }
        DayCountConvention::Actual365 => {
            let num = (next_coupon_from_last - settlement).num_days();
            let den_days = dec!(365) / Decimal::from(frequency);
            Decimal::from(num) / den_days
        }
        DayCountConvention::ActualActual => {
            let num = (next_coupon_from_last - settlement).num_days();
            let den = (next_coupon_from_last - last_coupon).num_days();
            if den == 0 {
                Decimal::ONE
            } else {
                Decimal::from(num) / Decimal::from(den)
            }
        }
    };

    // Discount factor for the fractional first period
    // df_0 = 1 / (1 + periodic_yield)^fraction_remaining
    // Use linear interpolation for the fractional part to avoid powd:
    // (1 + periodic_yield * fraction) as linear approx for short fractions,
    // or more accurately: iterative approach.
    // For institutional accuracy, we use the standard: df = 1/(1+y)^f
    // approximated via: exp(f * ln(1+y)). But since we must avoid powd and
    // use only Decimal, we compute via repeated squaring on the fractional part.
    let first_period_factor =
        decimal_pow_fraction(Decimal::ONE + periodic_yield, fraction_remaining);

    let mut dirty_pv = Decimal::ZERO;
    // Iterative discount factor: for coupon i (0-indexed), the total discount
    // periods from settlement = fraction_remaining + i
    // df(i) = first_period_factor * (1+periodic_yield)^i
    let mut cumulative_factor = first_period_factor;

    for (i, &date) in coupon_dates.iter().enumerate() {
        if i > 0 {
            cumulative_factor *= Decimal::ONE + periodic_yield;
        }

        let is_last = i == coupon_dates.len() - 1;
        let cashflow = if is_last {
            coupon_amount + face_value
        } else {
            coupon_amount
        };

        if cumulative_factor.is_zero() {
            continue;
        }
        dirty_pv += cashflow / cumulative_factor;
        let _ = date; // date used for schedule; discounting is period-based
    }

    // Clean price = dirty PV - accrued interest
    // But our dirty_pv here is actually the "full price" (price with accrued
    // interest embedded). The standard formula PV = sum(CF/(1+y)^(f+i)) gives
    // the dirty price directly. We subtract accrued to get clean.
    // However, in our architecture, the caller adds accrued separately, so we
    // return the clean price = dirty_pv - accrued.
    // Actually, the PV formula with fractional first period inherently gives
    // the dirty price. We subtract accrued to return clean.
    let accrued = coupon_amount * (Decimal::ONE - fraction_remaining);
    dirty_pv - accrued
}

/// Compute base^exponent for a Decimal fractional exponent using the
/// binary expansion / repeated-squaring approach. This avoids powd().
///
/// For accuracy with Decimal, we use Newton's method for the fractional root
/// and combine with integer power via iterative multiplication.
///
/// Specifically: base^frac where 0 <= frac <= 1
/// We compute this as: exp(frac * ln(base)) approximated via Newton iterations.
///
/// Simplified approach: use (1 + (base-1))^frac via binomial series for
/// values close to 1 (which periodic yields always are):
/// (1+x)^f ~= 1 + f*x + f*(f-1)/2 * x^2 + f*(f-1)*(f-2)/6 * x^3 + ...
/// We use 10 terms for high precision.
fn decimal_pow_fraction(base: Decimal, frac: Decimal) -> Decimal {
    if frac.is_zero() {
        return Decimal::ONE;
    }
    if frac == Decimal::ONE {
        return base;
    }
    if base == Decimal::ONE {
        return Decimal::ONE;
    }

    let x = base - Decimal::ONE;

    // Binomial series: (1+x)^f = sum_{k=0}^{N} C(f,k) * x^k
    // where C(f,k) = f*(f-1)*...*(f-k+1) / k!
    // This converges quickly for |x| < 1, which is always true for periodic yields.
    let mut result = Decimal::ONE;
    let mut term = Decimal::ONE;
    let num_terms = 15;

    for k in 1..=num_terms {
        let k_dec = Decimal::from(k);
        term *= (frac - k_dec + Decimal::ONE) * x / k_dec;
        result += term;
        // Early exit if term is negligible
        if term.abs() < dec!(0.00000000001) {
            break;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Yield to Call (Newton's method)
// ---------------------------------------------------------------------------

/// Compute yield-to-call for a callable bond using Newton's method.
///
/// We solve for the yield `y` such that:
///   dirty_price = sum(coupon / (1+y/freq)^i) + call_price / (1+y/freq)^n
/// where the sum runs over coupons up to the call date.
fn compute_ytc(
    dirty_price: Money,
    settlement: NaiveDate,
    call_date: NaiveDate,
    coupon_amount: Money,
    call_price: Money,
    frequency: u8,
    day_count: DayCountConvention,
) -> CorpFinanceResult<Rate> {
    // Count coupons to call date
    let coupon_dates_to_call = generate_coupon_dates(settlement, call_date, frequency);
    let n = coupon_dates_to_call.len();
    if n == 0 {
        return Err(CorpFinanceError::InsufficientData(
            "No coupon periods between settlement and call date".into(),
        ));
    }

    // Fractional first period
    let last_coupon = last_coupon_date_before(settlement, call_date, frequency);
    let months_per_period = 12 / frequency as i32;
    let next_coupon_from_last = add_months(last_coupon, months_per_period);
    let fraction_remaining = compute_fraction_remaining(
        settlement,
        last_coupon,
        next_coupon_from_last,
        frequency,
        day_count,
    );

    let freq = Decimal::from(frequency);
    let mut y = dec!(0.05); // initial guess

    for iteration in 0..NEWTON_MAX_ITERATIONS {
        let periodic_y = y / freq;
        let one_plus_py = Decimal::ONE + periodic_y;

        // Compute price function and its derivative
        let (price_val, dprice_val) = price_and_derivative_to_call(
            &coupon_dates_to_call,
            coupon_amount,
            call_price,
            one_plus_py,
            fraction_remaining,
            freq,
        );

        let f_val = price_val - dirty_price;

        if f_val.abs() < NEWTON_EPSILON {
            return Ok(y);
        }

        if dprice_val.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "YTC".into(),
                iterations: iteration,
                last_delta: f_val,
            });
        }

        y -= f_val / dprice_val;

        // Guard against divergence
        if y < dec!(-0.50) {
            y = dec!(-0.50);
        } else if y > dec!(1.0) {
            y = dec!(1.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "YTC".into(),
        iterations: NEWTON_MAX_ITERATIONS,
        last_delta: Decimal::ZERO,
    })
}

/// Compute the bond price and its derivative w.r.t. periodic yield for YTC Newton solver.
fn price_and_derivative_to_call(
    coupon_dates: &[NaiveDate],
    coupon_amount: Money,
    call_price: Money,
    one_plus_py: Decimal,
    fraction_remaining: Decimal,
    freq: Decimal,
) -> (Decimal, Decimal) {
    let first_factor = decimal_pow_fraction(one_plus_py, fraction_remaining);

    let mut price = Decimal::ZERO;
    let mut dprice = Decimal::ZERO;
    let mut factor = first_factor;

    for (i, _date) in coupon_dates.iter().enumerate() {
        if i > 0 {
            factor *= one_plus_py;
        }

        let is_last = i == coupon_dates.len() - 1;
        let cf = if is_last {
            coupon_amount + call_price
        } else {
            coupon_amount
        };

        if factor.is_zero() {
            continue;
        }

        price += cf / factor;

        // Derivative: d/dy (cf / (1+y/f)^t) = -t/f * cf / (1+y/f)^(t+1)
        let t = fraction_remaining + Decimal::from(i as u32);
        dprice -= t / freq * cf / (factor * one_plus_py);
    }

    (price, dprice)
}

/// Helper: compute fraction of period remaining from settlement to next coupon.
fn compute_fraction_remaining(
    settlement: NaiveDate,
    last_coupon: NaiveDate,
    next_coupon: NaiveDate,
    frequency: u8,
    day_count: DayCountConvention,
) -> Decimal {
    match day_count {
        DayCountConvention::Thirty360 => {
            let num = thirty_360_days(settlement, next_coupon);
            let den = thirty_360_days(last_coupon, next_coupon);
            if den == 0 {
                Decimal::ONE
            } else {
                Decimal::from(num) / Decimal::from(den)
            }
        }
        DayCountConvention::Actual360 => {
            let num = (next_coupon - settlement).num_days();
            let den_days = dec!(360) / Decimal::from(frequency);
            Decimal::from(num) / den_days
        }
        DayCountConvention::Actual365 => {
            let num = (next_coupon - settlement).num_days();
            let den_days = dec!(365) / Decimal::from(frequency);
            Decimal::from(num) / den_days
        }
        DayCountConvention::ActualActual => {
            let num = (next_coupon - settlement).num_days();
            let den = (next_coupon - last_coupon).num_days();
            if den == 0 {
                Decimal::ONE
            } else {
                Decimal::from(num) / Decimal::from(den)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: build a standard semi-annual bond input for testing.
    fn semi_annual_bond(coupon_rate: Rate, ytm: Rate) -> BondPricingInput {
        BondPricingInput {
            face_value: dec!(1000),
            coupon_rate,
            coupon_frequency: 2,
            ytm,
            // Settlement at a coupon date to avoid accrued interest complications
            settlement_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2029, 1, 15).unwrap(),
            day_count: DayCountConvention::Thirty360,
            call_price: None,
            call_date: None,
        }
    }

    // -----------------------------------------------------------------------
    // 1. Par bond: coupon == YTM => price ~ face value
    // -----------------------------------------------------------------------
    #[test]
    fn test_par_bond_price_at_par() {
        let input = semi_annual_bond(dec!(0.05), dec!(0.05));
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // When coupon rate == YTM, clean price should be approximately par
        let diff = (out.clean_price - dec!(1000)).abs();
        assert!(
            diff < dec!(1.0),
            "Par bond clean price should be ~1000, got {}",
            out.clean_price
        );
    }

    // -----------------------------------------------------------------------
    // 2. Premium bond: coupon > YTM => price > face value
    // -----------------------------------------------------------------------
    #[test]
    fn test_premium_bond() {
        let input = semi_annual_bond(dec!(0.05), dec!(0.03));
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        assert!(
            out.clean_price > dec!(1000),
            "Premium bond (5% coupon, 3% YTM) should price above par, got {}",
            out.clean_price
        );
    }

    // -----------------------------------------------------------------------
    // 3. Discount bond: coupon < YTM => price < face value
    // -----------------------------------------------------------------------
    #[test]
    fn test_discount_bond() {
        let input = semi_annual_bond(dec!(0.05), dec!(0.07));
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        assert!(
            out.clean_price < dec!(1000),
            "Discount bond (5% coupon, 7% YTM) should price below par, got {}",
            out.clean_price
        );
    }

    // -----------------------------------------------------------------------
    // 4. Zero coupon bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_coupon_bond() {
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0),
            coupon_frequency: 2,
            ytm: dec!(0.06),
            settlement_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2029, 1, 15).unwrap(),
            day_count: DayCountConvention::Thirty360,
            call_price: None,
            call_date: None,
        };
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // Zero coupon: price = face / (1 + y/2)^(n*2) = 1000 / (1.03)^10
        // = 1000 / 1.34392 ~= 744.09
        // Using iterative: 1.03^10 = 1.03 * 1.03 * ... (10 times)
        let mut factor = Decimal::ONE;
        for _ in 0..10 {
            factor *= dec!(1.03);
        }
        let expected = dec!(1000) / factor;

        let diff = (out.clean_price - expected).abs();
        assert!(
            diff < dec!(2.0),
            "Zero coupon bond: expected ~{}, got {}",
            expected,
            out.clean_price
        );
    }

    // -----------------------------------------------------------------------
    // 5. Accrued interest — 30/360
    // -----------------------------------------------------------------------
    #[test]
    fn test_accrued_interest_30_360() {
        // Mid-period settlement to get meaningful accrued interest
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.06),
            coupon_frequency: 2,
            ytm: dec!(0.06),
            // Settle 3 months into a 6-month coupon period
            settlement_date: NaiveDate::from_ymd_opt(2024, 4, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2029, 1, 15).unwrap(),
            day_count: DayCountConvention::Thirty360,
            call_price: None,
            call_date: None,
        };

        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // Semi-annual coupon = 1000 * 0.06 / 2 = 30
        assert_eq!(out.coupon_amount, dec!(30));

        // Accrued: 3 months out of 6 = 50% of coupon = 15
        // 30/360: Jan 15 to Apr 15 = 90 days / 180 days = 0.5
        let expected_accrued = dec!(15);
        let diff = (out.accrued_interest - expected_accrued).abs();
        assert!(
            diff < dec!(0.50),
            "Accrued interest (30/360) expected ~{}, got {}",
            expected_accrued,
            out.accrued_interest
        );
    }

    // -----------------------------------------------------------------------
    // 6. Accrued interest — ACT/ACT
    // -----------------------------------------------------------------------
    #[test]
    fn test_accrued_interest_actual_actual() {
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.06),
            coupon_frequency: 2,
            ytm: dec!(0.06),
            settlement_date: NaiveDate::from_ymd_opt(2024, 4, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2029, 1, 15).unwrap(),
            day_count: DayCountConvention::ActualActual,
            call_price: None,
            call_date: None,
        };

        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // ACT/ACT: actual days Jan 15 -> Apr 15 = 91 days (2024 is leap year)
        // Full period Jan 15 -> Jul 15 = 182 days
        // Fraction = 91/182 = 0.5
        // Accrued = 30 * 0.5 = 15
        let diff = (out.accrued_interest - dec!(15)).abs();
        assert!(
            diff < dec!(1.0),
            "Accrued interest (ACT/ACT) expected ~15, got {}",
            out.accrued_interest
        );
    }

    // -----------------------------------------------------------------------
    // 7. Dirty = Clean + Accrued
    // -----------------------------------------------------------------------
    #[test]
    fn test_dirty_vs_clean_relationship() {
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            ytm: dec!(0.04),
            settlement_date: NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2029, 1, 15).unwrap(),
            day_count: DayCountConvention::Thirty360,
            call_price: None,
            call_date: None,
        };

        let result = price_bond(&input).unwrap();
        let out = &result.result;

        let reconstructed_dirty = out.clean_price + out.accrued_interest;
        let diff = (out.dirty_price - reconstructed_dirty).abs();
        assert!(
            diff < dec!(0.01),
            "Dirty ({}) should equal clean ({}) + accrued ({}), difference = {}",
            out.dirty_price,
            out.clean_price,
            out.accrued_interest,
            diff
        );
    }

    // -----------------------------------------------------------------------
    // 8. Current yield
    // -----------------------------------------------------------------------
    #[test]
    fn test_current_yield_calculation() {
        let input = semi_annual_bond(dec!(0.06), dec!(0.06));
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // At par, current yield should equal coupon rate
        let diff = (out.current_yield - dec!(0.06)).abs();
        assert!(
            diff < dec!(0.005),
            "Current yield at par should be ~6%, got {}",
            out.current_yield
        );
    }

    // -----------------------------------------------------------------------
    // 9. Callable bond — YTC
    // -----------------------------------------------------------------------
    #[test]
    fn test_callable_bond_ytc() {
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.06),
            coupon_frequency: 2,
            ytm: dec!(0.05),
            settlement_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2034, 1, 15).unwrap(),
            day_count: DayCountConvention::Thirty360,
            call_price: Some(dec!(1020)),
            call_date: Some(NaiveDate::from_ymd_opt(2029, 1, 15).unwrap()),
        };

        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // YTC should be computed and be a reasonable rate
        assert!(
            out.ytc.is_some(),
            "YTC should be computed for callable bond"
        );
        let ytc = out.ytc.unwrap();
        assert!(
            ytc > dec!(0.01) && ytc < dec!(0.20),
            "YTC should be reasonable, got {}",
            ytc
        );
    }

    // -----------------------------------------------------------------------
    // 10. YTW = min(YTM, YTC)
    // -----------------------------------------------------------------------
    #[test]
    fn test_ytw_is_minimum() {
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.06),
            coupon_frequency: 2,
            ytm: dec!(0.05),
            settlement_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2034, 1, 15).unwrap(),
            day_count: DayCountConvention::Thirty360,
            call_price: Some(dec!(1020)),
            call_date: Some(NaiveDate::from_ymd_opt(2029, 1, 15).unwrap()),
        };

        let result = price_bond(&input).unwrap();
        let out = &result.result;

        assert!(out.ytw.is_some(), "YTW should be computed");
        let ytw = out.ytw.unwrap();
        let ytc = out.ytc.unwrap();

        // YTW = min(YTM, YTC)
        let expected_min = if input.ytm < ytc { input.ytm } else { ytc };
        assert_eq!(
            ytw, expected_min,
            "YTW should be min(YTM={}, YTC={}), got {}",
            input.ytm, ytc, ytw
        );
    }

    // -----------------------------------------------------------------------
    // 11. Semi-annual coupon count
    // -----------------------------------------------------------------------
    #[test]
    fn test_semiannual_coupon() {
        let input = semi_annual_bond(dec!(0.05), dec!(0.05));
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // 5 years, semi-annual = 10 coupons
        assert_eq!(
            out.num_remaining_coupons, 10,
            "5-year semi-annual bond should have 10 coupons, got {}",
            out.num_remaining_coupons
        );
        assert_eq!(out.coupon_amount, dec!(25)); // 1000 * 0.05 / 2
    }

    // -----------------------------------------------------------------------
    // 12. Quarterly coupon
    // -----------------------------------------------------------------------
    #[test]
    fn test_quarterly_coupon() {
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.08),
            coupon_frequency: 4,
            ytm: dec!(0.08),
            settlement_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2027, 1, 15).unwrap(),
            day_count: DayCountConvention::Thirty360,
            call_price: None,
            call_date: None,
        };
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // 3 years quarterly = 12 coupons
        assert_eq!(
            out.num_remaining_coupons, 12,
            "3-year quarterly bond should have 12 coupons, got {}",
            out.num_remaining_coupons
        );
        assert_eq!(out.coupon_amount, dec!(20)); // 1000 * 0.08 / 4

        // At par (coupon == YTM), price should be near par
        let diff = (out.clean_price - dec!(1000)).abs();
        assert!(
            diff < dec!(2.0),
            "Quarterly par bond should price near 1000, got {}",
            out.clean_price
        );
    }

    // -----------------------------------------------------------------------
    // 13. Cashflow schedule
    // -----------------------------------------------------------------------
    #[test]
    fn test_cashflow_schedule() {
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.06),
            coupon_frequency: 2,
            ytm: dec!(0.06),
            settlement_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2026, 1, 15).unwrap(),
            day_count: DayCountConvention::Thirty360,
            call_price: None,
            call_date: None,
        };
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // 2 years semi-annual = 4 cashflows
        assert_eq!(out.total_cashflows.len(), 4);

        // First 3 are coupon-only
        for cf in &out.total_cashflows[..3] {
            assert_eq!(cf.cashflow_type, "coupon");
            assert_eq!(cf.amount, dec!(30));
        }

        // Last is coupon + principal
        let last = &out.total_cashflows[3];
        assert_eq!(last.cashflow_type, "coupon+principal");
        assert_eq!(last.amount, dec!(1030));

        // Dates should be in chronological order
        for window in out.total_cashflows.windows(2) {
            assert!(window[0].date < window[1].date);
        }

        // Verify specific dates
        assert_eq!(
            out.total_cashflows[0].date,
            NaiveDate::from_ymd_opt(2024, 7, 15).unwrap()
        );
        assert_eq!(
            out.total_cashflows[3].date,
            NaiveDate::from_ymd_opt(2026, 1, 15).unwrap()
        );
    }

    // -----------------------------------------------------------------------
    // 14. Invalid face value
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_face_value_error() {
        let mut input = semi_annual_bond(dec!(0.05), dec!(0.05));
        input.face_value = dec!(-100);

        let result = price_bond(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "face_value");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 15. Maturity before settlement
    // -----------------------------------------------------------------------
    #[test]
    fn test_maturity_before_settlement_error() {
        let mut input = semi_annual_bond(dec!(0.05), dec!(0.05));
        input.maturity_date = NaiveDate::from_ymd_opt(2023, 1, 15).unwrap();

        let result = price_bond(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "maturity_date");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = semi_annual_bond(dec!(0.05), dec!(0.05));
        let result = price_bond(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Bond Pricing"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 17. Invalid coupon frequency
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_coupon_frequency() {
        let mut input = semi_annual_bond(dec!(0.05), dec!(0.05));
        input.coupon_frequency = 3; // not allowed

        let result = price_bond(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "coupon_frequency");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 18. Call date validation: call must be between settlement and maturity
    // -----------------------------------------------------------------------
    #[test]
    fn test_call_date_after_maturity_error() {
        let mut input = semi_annual_bond(dec!(0.05), dec!(0.05));
        input.call_price = Some(dec!(1050));
        input.call_date = Some(NaiveDate::from_ymd_opt(2030, 1, 15).unwrap()); // after 2029 maturity

        let result = price_bond(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "call_date");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 19. Annual coupon bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_annual_coupon_bond() {
        let input = BondPricingInput {
            face_value: dec!(1000),
            coupon_rate: dec!(0.04),
            coupon_frequency: 1,
            ytm: dec!(0.04),
            settlement_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
            maturity_date: NaiveDate::from_ymd_opt(2027, 1, 15).unwrap(),
            day_count: DayCountConvention::Actual365,
            call_price: None,
            call_date: None,
        };
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // 3 years annual = 3 coupons
        assert_eq!(out.num_remaining_coupons, 3);
        assert_eq!(out.coupon_amount, dec!(40)); // 1000 * 0.04 / 1

        // At par, clean price should be near par
        let diff = (out.clean_price - dec!(1000)).abs();
        assert!(
            diff < dec!(2.0),
            "Annual par bond should price near 1000, got {}",
            out.clean_price
        );
    }

    // -----------------------------------------------------------------------
    // 20. Years to maturity calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_years_to_maturity() {
        let input = semi_annual_bond(dec!(0.05), dec!(0.05));
        let result = price_bond(&input).unwrap();
        let out = &result.result;

        // 5 years from 2024-01-15 to 2029-01-15
        let diff = (out.years_to_maturity - dec!(5.0)).abs();
        assert!(
            diff < dec!(0.02),
            "Years to maturity should be ~5.0, got {}",
            out.years_to_maturity
        );
    }
}
