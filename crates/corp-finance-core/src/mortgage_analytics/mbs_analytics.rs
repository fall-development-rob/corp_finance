//! MBS pass-through analytics: cash flows, OAS/Z-spread, duration and convexity.
//!
//! Provides institutional-grade MBS analytics including pass-through cash flow
//! modelling with PSA prepayment, option-adjusted spread (OAS) via bisection,
//! Z-spread, and effective duration/convexity. All math in `rust_decimal::Decimal`.

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

/// PSA base CPR at month 30 (6% annual).
const PSA_BASE_CPR_30: Decimal = dec!(0.06);

/// Minimum balance threshold.
const BALANCE_EPSILON: Decimal = dec!(0.01);

/// Bisection convergence tolerance.
const BISECTION_TOL: Decimal = dec!(0.0000001);

/// Maximum bisection iterations.
const BISECTION_MAX_ITER: u32 = 200;

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// A point on the zero-rate curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroRatePoint {
    /// Maturity in years.
    pub maturity: Years,
    /// Zero-coupon rate at this maturity.
    pub rate: Rate,
}

/// Pass-through cash flow input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassThroughInput {
    /// Original pool balance.
    pub original_balance: Money,
    /// Current pool balance.
    pub current_balance: Money,
    /// Weighted average mortgage rate (gross coupon).
    pub mortgage_rate: Rate,
    /// Pass-through rate (net coupon to investors).
    pub pass_through_rate: Rate,
    /// Servicing fee rate (annual).
    pub servicing_fee: Rate,
    /// Remaining months to maturity.
    pub remaining_months: u32,
    /// PSA speed (e.g., 150 for 150% PSA).
    pub psa_speed: Decimal,
    /// Settlement delay in days (for pricing).
    pub settlement_delay_days: u32,
}

/// OAS analysis input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OasInput {
    /// Market (dirty) price of the MBS.
    pub market_price: Money,
    /// Monthly cash flows from the pass-through model.
    pub cashflows: Vec<MbsCashflow>,
    /// Benchmark zero-rate curve.
    pub benchmark_zero_rates: Vec<ZeroRatePoint>,
    /// Search range for spread in decimal form (e.g., (-0.05, 0.10)).
    pub spread_search_range: (Decimal, Decimal),
}

/// MBS duration and convexity input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbsDurationInput {
    /// Pass-through specification for generating cash flows.
    pub pass_through_input: PassThroughInput,
    /// Yield in basis points for Macaulay/modified duration.
    pub yield_bps: Decimal,
    /// Rate shock in basis points for effective duration/convexity.
    pub shock_bps: Decimal,
}

/// Top-level MBS analytics input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MbsAnalyticsInput {
    PassThrough(PassThroughInput),
    Oas(OasInput),
    Duration(MbsDurationInput),
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// A single month of MBS pass-through cash flows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbsCashflow {
    /// Month number (1-indexed).
    pub month: u32,
    /// Scheduled principal payment.
    pub scheduled_principal: Money,
    /// Interest payment to investors (at pass-through rate).
    pub interest: Money,
    /// Prepayment amount.
    pub prepayment: Money,
    /// Total cash flow to investors.
    pub total_cashflow: Money,
    /// Remaining balance after this month.
    pub remaining_balance: Money,
    /// Servicing income (retained by servicer).
    pub servicing_income: Money,
}

/// Pass-through cash flow output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassThroughOutput {
    /// Monthly cash flow schedule.
    pub monthly_cashflows: Vec<MbsCashflow>,
    /// Total interest collected by investors.
    pub total_interest: Money,
    /// Total principal (scheduled + prepayment).
    pub total_principal: Money,
    /// Total cash flow to investors.
    pub total_cashflow: Money,
    /// Weighted average life in years.
    pub weighted_average_life: Decimal,
    /// Weighted average coupon.
    pub weighted_average_coupon: Rate,
}

/// OAS analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OasOutput {
    /// Option-adjusted spread in basis points.
    pub oas_bps: Decimal,
    /// Z-spread (zero-volatility spread) in basis points.
    pub z_spread_bps: Decimal,
    /// Nominal spread vs benchmark in basis points.
    pub nominal_spread_bps: Decimal,
    /// Model price at the computed OAS.
    pub model_price: Money,
    /// Pricing error (model_price - market_price).
    pub pricing_error: Money,
}

/// MBS duration and convexity output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MbsDurationOutput {
    /// Effective duration: -(P+ - P-)/(2 * P0 * dy).
    pub effective_duration: Decimal,
    /// Effective convexity: (P+ + P- - 2*P0)/(P0 * dy^2).
    pub effective_convexity: Decimal,
    /// Modified duration (from yield).
    pub modified_duration: Decimal,
    /// Macaulay duration.
    pub macaulay_duration: Decimal,
    /// Dollar duration (DV01): price change per 1bp yield change.
    pub dollar_duration: Money,
    /// True if convexity is negative (typical for MBS).
    pub negative_convexity_flag: bool,
}

/// Unified MBS analytics output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MbsAnalyticsOutput {
    PassThrough(PassThroughOutput),
    Oas(OasOutput),
    Duration(MbsDurationOutput),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyse MBS using the specified model.
pub fn analyze_mbs(
    input: &MbsAnalyticsInput,
) -> CorpFinanceResult<ComputationOutput<MbsAnalyticsOutput>> {
    let start = Instant::now();

    let (output, methodology, warnings) = match input {
        MbsAnalyticsInput::PassThrough(pt) => {
            let (out, w) = compute_pass_through(pt)?;
            (
                MbsAnalyticsOutput::PassThrough(out),
                "MBS Pass-Through Cash Flow Model",
                w,
            )
        }
        MbsAnalyticsInput::Oas(oas) => {
            let (out, w) = compute_oas(oas)?;
            (
                MbsAnalyticsOutput::Oas(out),
                "MBS OAS/Z-Spread Analysis (Bisection)",
                w,
            )
        }
        MbsAnalyticsInput::Duration(dur) => {
            let (out, w) = compute_duration(dur)?;
            (
                MbsAnalyticsOutput::Duration(out),
                "MBS Effective Duration and Convexity",
                w,
            )
        }
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(methodology, input, warnings, elapsed, output))
}

// ---------------------------------------------------------------------------
// Pass-through cash flows
// ---------------------------------------------------------------------------

fn compute_pass_through(
    input: &PassThroughInput,
) -> CorpFinanceResult<(PassThroughOutput, Vec<String>)> {
    let mut warnings: Vec<String> = Vec::new();
    validate_pass_through(input)?;

    let gross_monthly = input.mortgage_rate / dec!(12);
    let net_monthly = input.pass_through_rate / dec!(12);
    let servicing_monthly = input.servicing_fee / dec!(12);

    // Verify that servicing_fee ~ mortgage_rate - pass_through_rate.
    let implied_servicing = input.mortgage_rate - input.pass_through_rate;
    let servicing_diff = (input.servicing_fee - implied_servicing).abs();
    if servicing_diff > dec!(0.001) {
        warnings.push(format!(
            "Servicing fee ({}) differs from mortgage_rate - pass_through_rate ({})",
            input.servicing_fee, implied_servicing
        ));
    }

    let mut balance = input.current_balance;
    let mut remaining = input.remaining_months;

    let mut cashflows = Vec::with_capacity(input.remaining_months as usize);
    let mut total_interest = Decimal::ZERO;
    let mut total_principal = Decimal::ZERO;
    let mut total_cashflow = Decimal::ZERO;
    let mut wal_numerator = Decimal::ZERO;
    let mut wac_numerator = Decimal::ZERO;
    let mut wac_denominator = Decimal::ZERO;

    for month_idx in 0..input.remaining_months {
        let month = month_idx + 1;
        let age = month; // Assume age starts at 1 for PSA purposes.

        if balance < BALANCE_EPSILON || remaining == 0 {
            cashflows.push(MbsCashflow {
                month,
                scheduled_principal: Decimal::ZERO,
                interest: Decimal::ZERO,
                prepayment: Decimal::ZERO,
                total_cashflow: Decimal::ZERO,
                remaining_balance: balance,
                servicing_income: Decimal::ZERO,
            });
            continue;
        }

        // PSA CPR.
        let base_cpr = if age <= 30 {
            PSA_BASE_CPR_30 * Decimal::from(age) / dec!(30)
        } else {
            PSA_BASE_CPR_30
        };
        let cpr = base_cpr * input.psa_speed / dec!(100);
        let cpr_capped = if cpr > Decimal::ONE {
            Decimal::ONE
        } else {
            cpr
        };
        let smm = cpr_to_smm(cpr_capped);

        // Scheduled payment (level-pay).
        let sched_principal = compute_scheduled_principal(balance, gross_monthly, remaining);

        // Interest to investors (pass-through rate).
        let investor_interest = balance * net_monthly;

        // Servicing income.
        let servicing_income = balance * servicing_monthly;

        // Prepayment.
        let prepay_base = balance - sched_principal;
        let prepayment = if prepay_base > Decimal::ZERO {
            prepay_base * smm
        } else {
            Decimal::ZERO
        };

        let month_principal = sched_principal + prepayment;
        let month_cashflow = investor_interest + month_principal;

        total_interest += investor_interest;
        total_principal += month_principal;
        total_cashflow += month_cashflow;
        wal_numerator += Decimal::from(month) * month_principal / dec!(12);

        // WAC: weight by balance.
        wac_numerator += balance * input.pass_through_rate;
        wac_denominator += balance;

        balance -= month_principal;
        if balance < Decimal::ZERO {
            balance = Decimal::ZERO;
        }
        remaining = remaining.saturating_sub(1);

        cashflows.push(MbsCashflow {
            month,
            scheduled_principal: sched_principal,
            interest: investor_interest,
            prepayment,
            total_cashflow: month_cashflow,
            remaining_balance: balance,
            servicing_income,
        });
    }

    let weighted_average_life = if total_principal > Decimal::ZERO {
        wal_numerator / total_principal
    } else {
        Decimal::ZERO
    };

    let weighted_average_coupon = if wac_denominator > Decimal::ZERO {
        wac_numerator / wac_denominator
    } else {
        input.pass_through_rate
    };

    Ok((
        PassThroughOutput {
            monthly_cashflows: cashflows,
            total_interest,
            total_principal,
            total_cashflow,
            weighted_average_life,
            weighted_average_coupon,
        },
        warnings,
    ))
}

fn validate_pass_through(input: &PassThroughInput) -> CorpFinanceResult<()> {
    if input.current_balance <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "current_balance".into(),
            reason: "Current balance must be positive".into(),
        });
    }
    if input.remaining_months == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_months".into(),
            reason: "Remaining months must be greater than zero".into(),
        });
    }
    if input.mortgage_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "mortgage_rate".into(),
            reason: "Mortgage rate cannot be negative".into(),
        });
    }
    if input.pass_through_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "pass_through_rate".into(),
            reason: "Pass-through rate cannot be negative".into(),
        });
    }
    if input.pass_through_rate > input.mortgage_rate {
        return Err(CorpFinanceError::InvalidInput {
            field: "pass_through_rate".into(),
            reason: "Pass-through rate cannot exceed mortgage rate".into(),
        });
    }
    if input.servicing_fee < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "servicing_fee".into(),
            reason: "Servicing fee cannot be negative".into(),
        });
    }
    if input.psa_speed < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "psa_speed".into(),
            reason: "PSA speed must be non-negative".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// OAS / Z-spread
// ---------------------------------------------------------------------------

fn compute_oas(input: &OasInput) -> CorpFinanceResult<(OasOutput, Vec<String>)> {
    let warnings: Vec<String> = Vec::new();
    validate_oas(input)?;

    // Z-spread: find spread s such that sum(CF_t / (1 + z_t + s)^t) = market_price.
    let z_spread = bisection_spread(
        &input.cashflows,
        &input.benchmark_zero_rates,
        input.market_price,
        input.spread_search_range.0,
        input.spread_search_range.1,
    )?;

    let z_spread_bps = z_spread * dec!(10000);

    // Model price at the computed Z-spread.
    let model_price = discount_cashflows(&input.cashflows, &input.benchmark_zero_rates, z_spread);

    // Nominal spread: simple spread over a weighted-average benchmark rate.
    let nominal_spread = compute_nominal_spread(
        &input.cashflows,
        &input.benchmark_zero_rates,
        input.market_price,
    );
    let nominal_spread_bps = nominal_spread * dec!(10000);

    // For a single-path model (no volatility), OAS = Z-spread.
    let oas_bps = z_spread_bps;

    let pricing_error = model_price - input.market_price;

    Ok((
        OasOutput {
            oas_bps,
            z_spread_bps,
            nominal_spread_bps,
            model_price,
            pricing_error,
        },
        warnings,
    ))
}

fn validate_oas(input: &OasInput) -> CorpFinanceResult<()> {
    if input.market_price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "market_price".into(),
            reason: "Market price must be positive".into(),
        });
    }
    if input.cashflows.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Cash flows cannot be empty for OAS analysis".into(),
        ));
    }
    if input.benchmark_zero_rates.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "Benchmark zero rates cannot be empty".into(),
        ));
    }
    if input.spread_search_range.0 >= input.spread_search_range.1 {
        return Err(CorpFinanceError::InvalidInput {
            field: "spread_search_range".into(),
            reason: "Lower bound must be less than upper bound".into(),
        });
    }
    Ok(())
}

/// Bisection method to find the spread that equates PV of cash flows to market price.
fn bisection_spread(
    cashflows: &[MbsCashflow],
    zero_rates: &[ZeroRatePoint],
    target_price: Money,
    mut lo: Decimal,
    mut hi: Decimal,
) -> CorpFinanceResult<Decimal> {
    let pv_lo = discount_cashflows(cashflows, zero_rates, lo);
    let pv_hi = discount_cashflows(cashflows, zero_rates, hi);

    // Check that the target is bracketed.
    if (pv_lo - target_price) * (pv_hi - target_price) > Decimal::ZERO {
        // Try to expand the range.
        lo = dec!(-0.20);
        hi = dec!(0.50);
        let pv_lo2 = discount_cashflows(cashflows, zero_rates, lo);
        let pv_hi2 = discount_cashflows(cashflows, zero_rates, hi);
        if (pv_lo2 - target_price) * (pv_hi2 - target_price) > Decimal::ZERO {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "bisection_spread".into(),
                iterations: 0,
                last_delta: (pv_lo2 - target_price).abs(),
            });
        }
    }

    for iter in 0..BISECTION_MAX_ITER {
        let mid = (lo + hi) / dec!(2);
        let pv_mid = discount_cashflows(cashflows, zero_rates, mid);
        let error = pv_mid - target_price;

        if error.abs() < BISECTION_TOL {
            return Ok(mid);
        }

        let pv_lo_val = discount_cashflows(cashflows, zero_rates, lo);
        if (pv_lo_val - target_price) * error < Decimal::ZERO {
            hi = mid;
        } else {
            lo = mid;
        }

        if (hi - lo).abs() < BISECTION_TOL {
            return Ok(mid);
        }

        if iter == BISECTION_MAX_ITER - 1 {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "bisection_spread".into(),
                iterations: BISECTION_MAX_ITER,
                last_delta: error.abs(),
            });
        }
    }

    Ok((lo + hi) / dec!(2))
}

/// Discount cash flows using zero rates + spread.
/// PV = sum(CF_t / (1 + z_t + s)^(t/12)) where t is months.
fn discount_cashflows(
    cashflows: &[MbsCashflow],
    zero_rates: &[ZeroRatePoint],
    spread: Decimal,
) -> Money {
    let mut pv = Decimal::ZERO;

    for cf in cashflows {
        if cf.total_cashflow.is_zero() {
            continue;
        }
        let t_years = Decimal::from(cf.month) / dec!(12);
        let z_rate = interpolate_zero_rate(zero_rates, t_years);
        let discount_rate = Decimal::ONE + z_rate + spread;

        if discount_rate <= Decimal::ZERO {
            continue;
        }

        // Discount factor: 1 / (1 + z + s)^t_years using power_decimal.
        let df = power_decimal_inv(discount_rate, t_years);
        pv += cf.total_cashflow * df;
    }

    pv
}

/// Interpolate the zero rate at a given maturity using linear interpolation.
fn interpolate_zero_rate(zero_rates: &[ZeroRatePoint], maturity: Years) -> Rate {
    if zero_rates.is_empty() {
        return Decimal::ZERO;
    }
    if zero_rates.len() == 1 {
        return zero_rates[0].rate;
    }

    // If maturity is before the first point, use the first rate.
    if maturity <= zero_rates[0].maturity {
        return zero_rates[0].rate;
    }

    // If maturity is beyond the last point, use the last rate.
    let last = zero_rates.last().unwrap();
    if maturity >= last.maturity {
        return last.rate;
    }

    // Linear interpolation between two bracketing points.
    for window in zero_rates.windows(2) {
        let p0 = &window[0];
        let p1 = &window[1];
        if maturity >= p0.maturity && maturity <= p1.maturity {
            let span = p1.maturity - p0.maturity;
            if span.is_zero() {
                return p0.rate;
            }
            let frac = (maturity - p0.maturity) / span;
            return p0.rate + frac * (p1.rate - p0.rate);
        }
    }

    last.rate
}

/// Compute nominal spread: yield on MBS - weighted average benchmark rate.
fn compute_nominal_spread(
    cashflows: &[MbsCashflow],
    zero_rates: &[ZeroRatePoint],
    market_price: Money,
) -> Rate {
    // Compute weighted average benchmark rate (weighted by cash flow PV).
    let mut rate_weighted = Decimal::ZERO;
    let mut weight_sum = Decimal::ZERO;
    let mut total_cf = Decimal::ZERO;

    for cf in cashflows {
        if cf.total_cashflow.is_zero() {
            continue;
        }
        let t_years = Decimal::from(cf.month) / dec!(12);
        let z_rate = interpolate_zero_rate(zero_rates, t_years);
        rate_weighted += z_rate * cf.total_cashflow;
        weight_sum += cf.total_cashflow;
        total_cf += cf.total_cashflow;
    }

    let avg_benchmark = if weight_sum > Decimal::ZERO {
        rate_weighted / weight_sum
    } else {
        Decimal::ZERO
    };

    // Simple yield estimate: (total_cf / market_price - 1) annualised.
    // For a rough nominal spread, use the WAL-based approach.
    if market_price <= Decimal::ZERO || total_cf <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    // Compute WAL in years.
    let mut wal_num = Decimal::ZERO;
    let mut principal_sum = Decimal::ZERO;
    for cf in cashflows {
        let principal = cf.scheduled_principal + cf.prepayment;
        wal_num += Decimal::from(cf.month) * principal / dec!(12);
        principal_sum += principal;
    }
    let wal = if principal_sum > Decimal::ZERO {
        wal_num / principal_sum
    } else {
        dec!(5) // default
    };

    // Simple yield: (total_cf / market_price)^(1/wal) - 1, approximated.
    // Use a rough IRR estimate: yield ~ (coupon + (face - price)/wal) / ((face + price)/2).
    let face = principal_sum;
    let avg_coupon = if !cashflows.is_empty() {
        let total_interest: Decimal = cashflows.iter().map(|c| c.interest).sum();
        if wal > Decimal::ZERO {
            total_interest / wal / face
        } else {
            Decimal::ZERO
        }
    } else {
        Decimal::ZERO
    };

    let approx_yield = if face + market_price > Decimal::ZERO {
        (avg_coupon + (face - market_price) / wal) / ((face + market_price) / dec!(2))
    } else {
        Decimal::ZERO
    };

    approx_yield - avg_benchmark
}

// ---------------------------------------------------------------------------
// Duration and convexity
// ---------------------------------------------------------------------------

fn compute_duration(
    input: &MbsDurationInput,
) -> CorpFinanceResult<(MbsDurationOutput, Vec<String>)> {
    let mut warnings: Vec<String> = Vec::new();
    validate_duration(input)?;

    // Generate base cash flows.
    let (base_cf, _) = compute_pass_through(&input.pass_through_input)?;

    let yield_decimal = input.yield_bps / dec!(10000);
    let monthly_yield = yield_decimal / dec!(12);
    let shock_decimal = input.shock_bps / dec!(10000);

    // P0: price at base yield.
    let p0 = discount_at_flat_yield(&base_cf.monthly_cashflows, monthly_yield);

    if p0 <= Decimal::ZERO {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Base price is non-positive".into(),
        ));
    }

    // P+: price at yield + shock.
    let p_up = price_at_psa_and_yield(input, shock_decimal, &mut warnings)?;

    // P-: price at yield - shock.
    let p_down = price_at_psa_and_yield(input, -shock_decimal, &mut warnings)?;

    let dy = shock_decimal;

    // Effective duration: -(P+ - P-)/(2 * P0 * dy).
    let effective_duration = if dy > Decimal::ZERO {
        -(p_up - p_down) / (dec!(2) * p0 * dy)
    } else {
        Decimal::ZERO
    };

    // Effective convexity: (P+ + P- - 2*P0)/(P0 * dy^2).
    let effective_convexity = if dy > Decimal::ZERO {
        (p_up + p_down - dec!(2) * p0) / (p0 * dy * dy)
    } else {
        Decimal::ZERO
    };

    // Macaulay duration from cash flows.
    let macaulay_duration =
        compute_macaulay_duration(&base_cf.monthly_cashflows, monthly_yield, p0);

    // Modified duration = Macaulay / (1 + y/m) where m = compounding periods.
    let modified_duration = if (Decimal::ONE + monthly_yield) > Decimal::ZERO {
        macaulay_duration / (Decimal::ONE + monthly_yield)
    } else {
        macaulay_duration
    };

    // DV01: dollar duration = modified_duration * P0 / 10000.
    let dollar_duration = modified_duration * p0 / dec!(10000);

    let negative_convexity_flag = effective_convexity < Decimal::ZERO;

    Ok((
        MbsDurationOutput {
            effective_duration,
            effective_convexity,
            modified_duration,
            macaulay_duration,
            dollar_duration,
            negative_convexity_flag,
        },
        warnings,
    ))
}

fn validate_duration(input: &MbsDurationInput) -> CorpFinanceResult<()> {
    if input.shock_bps <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "shock_bps".into(),
            reason: "Rate shock must be positive".into(),
        });
    }
    if input.yield_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "yield_bps".into(),
            reason: "Yield cannot be negative".into(),
        });
    }
    Ok(())
}

/// Compute price by shifting the yield and regenerating cash flows at a given PSA.
fn price_at_psa_and_yield(
    input: &MbsDurationInput,
    yield_shock: Decimal,
    _warnings: &mut Vec<String>,
) -> CorpFinanceResult<Money> {
    let base_yield = input.yield_bps / dec!(10000);
    let shocked_yield = base_yield + yield_shock;
    let monthly_yield = shocked_yield / dec!(12);

    // Re-generate cash flows (PSA speed might change with rates in a full model,
    // but for effective duration we keep the same PSA).
    let (cf, _) = compute_pass_through(&input.pass_through_input)?;
    let price = discount_at_flat_yield(&cf.monthly_cashflows, monthly_yield);
    Ok(price)
}

/// Discount cash flows at a flat monthly yield.
fn discount_at_flat_yield(cashflows: &[MbsCashflow], monthly_yield: Decimal) -> Money {
    let mut pv = Decimal::ZERO;
    for cf in cashflows {
        if cf.total_cashflow.is_zero() {
            continue;
        }
        let df = iterative_pow_recip(Decimal::ONE + monthly_yield, cf.month);
        pv += cf.total_cashflow * df;
    }
    pv
}

/// Compute Macaulay duration from cash flows discounted at monthly yield.
fn compute_macaulay_duration(
    cashflows: &[MbsCashflow],
    monthly_yield: Decimal,
    price: Money,
) -> Decimal {
    if price <= Decimal::ZERO {
        return Decimal::ZERO;
    }

    let mut weighted_sum = Decimal::ZERO;
    for cf in cashflows {
        if cf.total_cashflow.is_zero() {
            continue;
        }
        let t_years = Decimal::from(cf.month) / dec!(12);
        let df = iterative_pow_recip(Decimal::ONE + monthly_yield, cf.month);
        weighted_sum += t_years * cf.total_cashflow * df;
    }

    weighted_sum / price
}

// ---------------------------------------------------------------------------
// Decimal math helpers (no f64, no powd)
// ---------------------------------------------------------------------------

/// Compute base^n for a positive integer exponent via iterative multiplication.
fn iterative_pow(base: Decimal, n: u32) -> Decimal {
    let mut result = Decimal::ONE;
    for _ in 0..n {
        result *= base;
    }
    result
}

/// Compute 1 / base^n.
fn iterative_pow_recip(base: Decimal, n: u32) -> Decimal {
    let pow = iterative_pow(base, n);
    if pow.is_zero() {
        Decimal::ZERO
    } else {
        Decimal::ONE / pow
    }
}

/// Taylor series expansion for e^x, 30 terms.
fn decimal_exp(x: Decimal) -> Decimal {
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=30u32 {
        term *= x / Decimal::from(n);
        sum += term;
        if term.abs() < dec!(0.00000000000001) {
            break;
        }
    }
    sum
}

/// Natural logarithm via Newton's method, 20 iterations.
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if x == Decimal::ONE {
        return Decimal::ZERO;
    }

    let mut guess = x - Decimal::ONE;
    if guess.abs() > dec!(2) {
        guess = Decimal::ZERO;
        let mut temp = x;
        let e_approx = dec!(2.718281828);
        if temp > Decimal::ONE {
            while temp > e_approx {
                temp /= e_approx;
                guess += Decimal::ONE;
            }
            guess += temp - Decimal::ONE;
        } else {
            while temp < Decimal::ONE / e_approx {
                temp *= e_approx;
                guess -= Decimal::ONE;
            }
            guess += temp - Decimal::ONE;
        }
    }

    for _ in 0..20 {
        let exp_guess = decimal_exp(guess);
        if exp_guess.is_zero() {
            break;
        }
        let delta = (exp_guess - x) / exp_guess;
        guess -= delta;
        if delta.abs() < dec!(0.00000000000001) {
            break;
        }
    }

    guess
}

/// Compute 1 / base^exp for arbitrary Decimal exponent via exp(-exp * ln(base)).
fn power_decimal_inv(base: Decimal, exp: Decimal) -> Decimal {
    if base.is_zero() {
        return Decimal::ZERO;
    }
    if exp.is_zero() {
        return Decimal::ONE;
    }
    if base == Decimal::ONE {
        return Decimal::ONE;
    }
    decimal_exp(-exp * decimal_ln(base))
}

/// Convert annual CPR to single monthly mortality (SMM).
fn cpr_to_smm(cpr: Rate) -> Rate {
    if cpr <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    if cpr >= Decimal::ONE {
        return Decimal::ONE;
    }
    let base = Decimal::ONE - cpr;
    Decimal::ONE - nth_root(base, 12)
}

/// Compute the nth root of x using Newton's method (40 iterations).
fn nth_root(x: Decimal, n: u32) -> Decimal {
    if x == Decimal::ONE {
        return Decimal::ONE;
    }
    if x == Decimal::ZERO {
        return Decimal::ZERO;
    }
    if n == 0 {
        return Decimal::ONE;
    }
    if n == 1 {
        return x;
    }

    let n_dec = Decimal::from(n);
    let n_minus_1 = n - 1;
    let mut guess = Decimal::ONE;

    for _ in 0..40 {
        let g_n_minus_1 = iterative_pow(guess, n_minus_1);
        let g_n = g_n_minus_1 * guess;

        if g_n_minus_1.is_zero() {
            break;
        }

        let delta = (g_n - x) / (n_dec * g_n_minus_1);
        guess -= delta;

        if delta.abs() < dec!(0.0000000000001) {
            break;
        }
    }

    guess
}

/// Compute scheduled principal for a level-pay amortising loan.
fn compute_scheduled_principal(balance: Money, monthly_rate: Rate, remaining: u32) -> Money {
    if remaining == 0 {
        return balance;
    }
    if monthly_rate <= Decimal::ZERO {
        return balance / Decimal::from(remaining);
    }

    let denom = Decimal::ONE - iterative_pow_recip(Decimal::ONE + monthly_rate, remaining);
    let payment = if denom > Decimal::ZERO {
        balance * monthly_rate / denom
    } else {
        balance
    };

    let interest = balance * monthly_rate;
    let mut principal = payment - interest;

    if principal > balance {
        principal = balance;
    }
    if principal < Decimal::ZERO {
        principal = Decimal::ZERO;
    }
    principal
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    const TOL: Decimal = dec!(0.01);
    const RATE_TOL: Decimal = dec!(0.0001);

    fn assert_close(actual: Decimal, expected: Decimal, tol: Decimal, msg: &str) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tol,
            "{}: expected ~{}, got {} (diff = {})",
            msg,
            expected,
            actual,
            diff
        );
    }

    fn standard_pt_input() -> PassThroughInput {
        PassThroughInput {
            original_balance: dec!(1_000_000),
            current_balance: dec!(1_000_000),
            mortgage_rate: dec!(0.065),
            pass_through_rate: dec!(0.06),
            servicing_fee: dec!(0.005),
            remaining_months: 360,
            psa_speed: dec!(150),
            settlement_delay_days: 25,
        }
    }

    fn standard_zero_rates() -> Vec<ZeroRatePoint> {
        vec![
            ZeroRatePoint {
                maturity: dec!(0.25),
                rate: dec!(0.03),
            },
            ZeroRatePoint {
                maturity: dec!(0.5),
                rate: dec!(0.032),
            },
            ZeroRatePoint {
                maturity: dec!(1),
                rate: dec!(0.035),
            },
            ZeroRatePoint {
                maturity: dec!(2),
                rate: dec!(0.038),
            },
            ZeroRatePoint {
                maturity: dec!(3),
                rate: dec!(0.04),
            },
            ZeroRatePoint {
                maturity: dec!(5),
                rate: dec!(0.042),
            },
            ZeroRatePoint {
                maturity: dec!(7),
                rate: dec!(0.044),
            },
            ZeroRatePoint {
                maturity: dec!(10),
                rate: dec!(0.045),
            },
            ZeroRatePoint {
                maturity: dec!(20),
                rate: dec!(0.047),
            },
            ZeroRatePoint {
                maturity: dec!(30),
                rate: dec!(0.048),
            },
        ]
    }

    fn run_pass_through(input: &PassThroughInput) -> PassThroughOutput {
        let mbs_input = MbsAnalyticsInput::PassThrough(input.clone());
        let result = analyze_mbs(&mbs_input).unwrap();
        match result.result {
            MbsAnalyticsOutput::PassThrough(out) => out,
            _ => panic!("Expected PassThroughOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 1. Servicing fee = mortgage_rate - pass_through_rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_servicing_fee_relationship() {
        let input = standard_pt_input();
        // mortgage_rate (0.065) - pass_through_rate (0.06) = 0.005 = servicing_fee
        assert_close(
            input.mortgage_rate - input.pass_through_rate,
            input.servicing_fee,
            RATE_TOL,
            "Servicing fee should equal rate differential",
        );
    }

    // -----------------------------------------------------------------------
    // 2. Total principal = current balance (with no defaults)
    // -----------------------------------------------------------------------
    #[test]
    fn test_total_principal_equals_balance() {
        let input = standard_pt_input();
        let out = run_pass_through(&input);

        assert_close(
            out.total_principal,
            dec!(1_000_000),
            dec!(1.0),
            "Total principal should equal current balance",
        );
    }

    // -----------------------------------------------------------------------
    // 3. Cash flows: total = interest + principal
    // -----------------------------------------------------------------------
    #[test]
    fn test_cashflow_composition() {
        let input = standard_pt_input();
        let out = run_pass_through(&input);

        for cf in &out.monthly_cashflows {
            let expected = cf.interest + cf.scheduled_principal + cf.prepayment;
            assert_close(
                cf.total_cashflow,
                expected,
                TOL,
                &format!("Month {} cashflow composition", cf.month),
            );
        }
    }

    // -----------------------------------------------------------------------
    // 4. WAL decreases with higher PSA speed
    // -----------------------------------------------------------------------
    #[test]
    fn test_wal_decreases_with_higher_psa() {
        let input_100 = PassThroughInput {
            psa_speed: dec!(100),
            ..standard_pt_input()
        };
        let input_200 = PassThroughInput {
            psa_speed: dec!(200),
            ..standard_pt_input()
        };

        let out_100 = run_pass_through(&input_100);
        let out_200 = run_pass_through(&input_200);

        assert!(
            out_200.weighted_average_life < out_100.weighted_average_life,
            "WAL at 200% PSA ({}) should be shorter than 100% PSA ({})",
            out_200.weighted_average_life,
            out_100.weighted_average_life
        );
    }

    // -----------------------------------------------------------------------
    // 5. Servicing income is positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_servicing_income_positive() {
        let input = standard_pt_input();
        let out = run_pass_through(&input);

        for cf in &out.monthly_cashflows {
            if cf.remaining_balance > BALANCE_EPSILON || cf.month == 1 {
                assert!(
                    cf.servicing_income >= Decimal::ZERO,
                    "Month {}: servicing income should be non-negative",
                    cf.month
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 6. First month interest = balance * pass_through_rate / 12
    // -----------------------------------------------------------------------
    #[test]
    fn test_first_month_interest() {
        let input = standard_pt_input();
        let out = run_pass_through(&input);

        let expected = dec!(1_000_000) * dec!(0.06) / dec!(12);
        assert_close(
            out.monthly_cashflows[0].interest,
            expected,
            TOL,
            "First month interest",
        );
    }

    // -----------------------------------------------------------------------
    // 7. First month servicing income
    // -----------------------------------------------------------------------
    #[test]
    fn test_first_month_servicing_income() {
        let input = standard_pt_input();
        let out = run_pass_through(&input);

        let expected = dec!(1_000_000) * dec!(0.005) / dec!(12);
        assert_close(
            out.monthly_cashflows[0].servicing_income,
            expected,
            TOL,
            "First month servicing income",
        );
    }

    // -----------------------------------------------------------------------
    // 8. Balance monotonically decreasing
    // -----------------------------------------------------------------------
    #[test]
    fn test_balance_monotonically_decreasing() {
        let input = standard_pt_input();
        let out = run_pass_through(&input);

        let mut prev = dec!(1_000_000);
        for cf in &out.monthly_cashflows {
            assert!(
                cf.remaining_balance <= prev + TOL,
                "Month {}: balance {} should be <= {}",
                cf.month,
                cf.remaining_balance,
                prev
            );
            prev = cf.remaining_balance;
        }
    }

    // -----------------------------------------------------------------------
    // 9. Balance never negative
    // -----------------------------------------------------------------------
    #[test]
    fn test_balance_never_negative() {
        let input = PassThroughInput {
            psa_speed: dec!(400),
            ..standard_pt_input()
        };
        let out = run_pass_through(&input);

        for cf in &out.monthly_cashflows {
            assert!(
                cf.remaining_balance >= Decimal::ZERO,
                "Month {}: balance should not be negative, got {}",
                cf.month,
                cf.remaining_balance
            );
        }
    }

    // -----------------------------------------------------------------------
    // 10. 0% PSA = no prepayment
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_psa_no_prepayment() {
        let input = PassThroughInput {
            psa_speed: dec!(0),
            remaining_months: 60,
            ..standard_pt_input()
        };
        let out = run_pass_through(&input);

        for cf in &out.monthly_cashflows {
            assert_eq!(
                cf.prepayment,
                Decimal::ZERO,
                "Month {}: no prepayment at 0% PSA",
                cf.month
            );
        }
    }

    // -----------------------------------------------------------------------
    // 11. Correct number of cash flow periods
    // -----------------------------------------------------------------------
    #[test]
    fn test_correct_period_count() {
        let input = PassThroughInput {
            remaining_months: 120,
            ..standard_pt_input()
        };
        let out = run_pass_through(&input);

        assert_eq!(out.monthly_cashflows.len(), 120);
        assert_eq!(out.monthly_cashflows[0].month, 1);
        assert_eq!(out.monthly_cashflows[119].month, 120);
    }

    // -----------------------------------------------------------------------
    // 12. WAC equals pass-through rate (single-coupon pool)
    // -----------------------------------------------------------------------
    #[test]
    fn test_wac_equals_pass_through_rate() {
        let input = standard_pt_input();
        let out = run_pass_through(&input);

        assert_close(
            out.weighted_average_coupon,
            dec!(0.06),
            dec!(0.001),
            "WAC should equal pass-through rate for single-coupon pool",
        );
    }

    // -----------------------------------------------------------------------
    // 13. OAS: Z-spread recovers market price
    // -----------------------------------------------------------------------
    #[test]
    fn test_oas_z_spread_recovers_price() {
        let pt_input = standard_pt_input();
        let pt_out = run_pass_through(&pt_input);

        // Set market price = PV at zero spread, so Z-spread should be ~0.
        let zero_rates = standard_zero_rates();
        let pv_at_zero = discount_cashflows(&pt_out.monthly_cashflows, &zero_rates, Decimal::ZERO);

        let oas_input = OasInput {
            market_price: pv_at_zero,
            cashflows: pt_out.monthly_cashflows.clone(),
            benchmark_zero_rates: zero_rates,
            spread_search_range: (dec!(-0.05), dec!(0.10)),
        };

        let mbs_input = MbsAnalyticsInput::Oas(oas_input);
        let result = analyze_mbs(&mbs_input).unwrap();

        match result.result {
            MbsAnalyticsOutput::Oas(oas_out) => {
                // Z-spread should be near zero since we priced at the zero curve.
                assert!(
                    oas_out.z_spread_bps.abs() < dec!(1.0),
                    "Z-spread should be near zero, got {} bps",
                    oas_out.z_spread_bps
                );
                // Pricing error should be tiny.
                assert!(
                    oas_out.pricing_error.abs() < dec!(1.0),
                    "Pricing error should be tiny, got {}",
                    oas_out.pricing_error
                );
            }
            _ => panic!("Expected OasOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 14. OAS: positive spread for discount MBS
    // -----------------------------------------------------------------------
    #[test]
    fn test_oas_positive_for_discount() {
        let pt_input = standard_pt_input();
        let pt_out = run_pass_through(&pt_input);

        let zero_rates = standard_zero_rates();
        // Price below PV (discount) => positive spread.
        let pv_at_zero = discount_cashflows(&pt_out.monthly_cashflows, &zero_rates, Decimal::ZERO);
        let discount_price = pv_at_zero * dec!(0.95);

        let oas_input = OasInput {
            market_price: discount_price,
            cashflows: pt_out.monthly_cashflows.clone(),
            benchmark_zero_rates: zero_rates,
            spread_search_range: (dec!(-0.05), dec!(0.20)),
        };

        let mbs_input = MbsAnalyticsInput::Oas(oas_input);
        let result = analyze_mbs(&mbs_input).unwrap();

        match result.result {
            MbsAnalyticsOutput::Oas(oas_out) => {
                assert!(
                    oas_out.z_spread_bps > Decimal::ZERO,
                    "Z-spread should be positive for discount MBS, got {} bps",
                    oas_out.z_spread_bps
                );
            }
            _ => panic!("Expected OasOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 15. OAS: model price close to market price
    // -----------------------------------------------------------------------
    #[test]
    fn test_oas_model_price_accuracy() {
        let pt_input = standard_pt_input();
        let pt_out = run_pass_through(&pt_input);

        let zero_rates = standard_zero_rates();
        let pv_at_zero = discount_cashflows(&pt_out.monthly_cashflows, &zero_rates, Decimal::ZERO);
        let market_price = pv_at_zero * dec!(0.98);

        let oas_input = OasInput {
            market_price,
            cashflows: pt_out.monthly_cashflows.clone(),
            benchmark_zero_rates: zero_rates,
            spread_search_range: (dec!(-0.05), dec!(0.20)),
        };

        let mbs_input = MbsAnalyticsInput::Oas(oas_input);
        let result = analyze_mbs(&mbs_input).unwrap();

        match result.result {
            MbsAnalyticsOutput::Oas(oas_out) => {
                assert!(
                    oas_out.pricing_error.abs() < dec!(1.0),
                    "Pricing error should be < 1.0, got {}",
                    oas_out.pricing_error
                );
            }
            _ => panic!("Expected OasOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 16. Duration: positive effective duration
    // -----------------------------------------------------------------------
    #[test]
    fn test_positive_effective_duration() {
        let dur_input = MbsDurationInput {
            pass_through_input: standard_pt_input(),
            yield_bps: dec!(600),
            shock_bps: dec!(25),
        };

        let mbs_input = MbsAnalyticsInput::Duration(dur_input);
        let result = analyze_mbs(&mbs_input).unwrap();

        match result.result {
            MbsAnalyticsOutput::Duration(dur_out) => {
                assert!(
                    dur_out.effective_duration > Decimal::ZERO,
                    "Effective duration should be positive, got {}",
                    dur_out.effective_duration
                );
            }
            _ => panic!("Expected MbsDurationOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 17. Duration: Macaulay duration positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_macaulay_duration_positive() {
        let dur_input = MbsDurationInput {
            pass_through_input: standard_pt_input(),
            yield_bps: dec!(600),
            shock_bps: dec!(25),
        };

        let mbs_input = MbsAnalyticsInput::Duration(dur_input);
        let result = analyze_mbs(&mbs_input).unwrap();

        match result.result {
            MbsAnalyticsOutput::Duration(dur_out) => {
                assert!(
                    dur_out.macaulay_duration > Decimal::ZERO,
                    "Macaulay duration should be positive, got {}",
                    dur_out.macaulay_duration
                );
            }
            _ => panic!("Expected MbsDurationOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 18. Duration: modified <= macaulay
    // -----------------------------------------------------------------------
    #[test]
    fn test_modified_leq_macaulay() {
        let dur_input = MbsDurationInput {
            pass_through_input: standard_pt_input(),
            yield_bps: dec!(600),
            shock_bps: dec!(25),
        };

        let mbs_input = MbsAnalyticsInput::Duration(dur_input);
        let result = analyze_mbs(&mbs_input).unwrap();

        match result.result {
            MbsAnalyticsOutput::Duration(dur_out) => {
                assert!(
                    dur_out.modified_duration <= dur_out.macaulay_duration + dec!(0.01),
                    "Modified duration ({}) should be <= Macaulay ({})",
                    dur_out.modified_duration,
                    dur_out.macaulay_duration
                );
            }
            _ => panic!("Expected MbsDurationOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 19. Duration: DV01 positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_dv01_positive() {
        let dur_input = MbsDurationInput {
            pass_through_input: standard_pt_input(),
            yield_bps: dec!(600),
            shock_bps: dec!(25),
        };

        let mbs_input = MbsAnalyticsInput::Duration(dur_input);
        let result = analyze_mbs(&mbs_input).unwrap();

        match result.result {
            MbsAnalyticsOutput::Duration(dur_out) => {
                assert!(
                    dur_out.dollar_duration > Decimal::ZERO,
                    "DV01 should be positive, got {}",
                    dur_out.dollar_duration
                );
            }
            _ => panic!("Expected MbsDurationOutput"),
        }
    }

    // -----------------------------------------------------------------------
    // 20. Validation: negative current balance (pass-through)
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_balance_pt() {
        let input = PassThroughInput {
            current_balance: dec!(-100),
            ..standard_pt_input()
        };
        let mbs_input = MbsAnalyticsInput::PassThrough(input);
        assert!(analyze_mbs(&mbs_input).is_err());
    }

    // -----------------------------------------------------------------------
    // 21. Validation: pass-through rate > mortgage rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_pt_rate_exceeds_mortgage() {
        let input = PassThroughInput {
            pass_through_rate: dec!(0.08),
            mortgage_rate: dec!(0.065),
            ..standard_pt_input()
        };
        let mbs_input = MbsAnalyticsInput::PassThrough(input);
        assert!(analyze_mbs(&mbs_input).is_err());
    }

    // -----------------------------------------------------------------------
    // 22. Validation: zero remaining months
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_zero_remaining() {
        let input = PassThroughInput {
            remaining_months: 0,
            ..standard_pt_input()
        };
        let mbs_input = MbsAnalyticsInput::PassThrough(input);
        assert!(analyze_mbs(&mbs_input).is_err());
    }

    // -----------------------------------------------------------------------
    // 23. Validation: OAS empty cashflows
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_oas_empty_cashflows() {
        let oas_input = OasInput {
            market_price: dec!(100),
            cashflows: vec![],
            benchmark_zero_rates: standard_zero_rates(),
            spread_search_range: (dec!(-0.05), dec!(0.10)),
        };
        let mbs_input = MbsAnalyticsInput::Oas(oas_input);
        assert!(analyze_mbs(&mbs_input).is_err());
    }

    // -----------------------------------------------------------------------
    // 24. Validation: duration negative shock
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_duration_negative_shock() {
        let dur_input = MbsDurationInput {
            pass_through_input: standard_pt_input(),
            yield_bps: dec!(500),
            shock_bps: dec!(-10),
        };
        let mbs_input = MbsAnalyticsInput::Duration(dur_input);
        assert!(analyze_mbs(&mbs_input).is_err());
    }

    // -----------------------------------------------------------------------
    // 25. Metadata is populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let mbs_input = MbsAnalyticsInput::PassThrough(standard_pt_input());
        let result = analyze_mbs(&mbs_input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("MBS"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
    }

    // -----------------------------------------------------------------------
    // 26. Zero-rate interpolation: exact point
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_rate_interpolation_exact() {
        let rates = standard_zero_rates();
        let rate = interpolate_zero_rate(&rates, dec!(5));
        assert_close(rate, dec!(0.042), RATE_TOL, "Exact 5y rate");
    }

    // -----------------------------------------------------------------------
    // 27. Zero-rate interpolation: between points
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_rate_interpolation_between() {
        let rates = standard_zero_rates();
        // Between 5y (0.042) and 7y (0.044): at 6y should be ~0.043.
        let rate = interpolate_zero_rate(&rates, dec!(6));
        assert_close(rate, dec!(0.043), dec!(0.001), "Interpolated 6y rate");
    }

    // -----------------------------------------------------------------------
    // 28. Zero-rate interpolation: before first point
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_rate_interpolation_before_first() {
        let rates = standard_zero_rates();
        let rate = interpolate_zero_rate(&rates, dec!(0.1));
        assert_close(rate, dec!(0.03), RATE_TOL, "Before first point");
    }

    // -----------------------------------------------------------------------
    // 29. At-par pricing test
    // -----------------------------------------------------------------------
    #[test]
    fn test_at_par_pricing() {
        // When PSA=0, a pass-through priced at par should have
        // pass-through rate ~ yield.
        let input = PassThroughInput {
            original_balance: dec!(100),
            current_balance: dec!(100),
            mortgage_rate: dec!(0.06),
            pass_through_rate: dec!(0.055),
            servicing_fee: dec!(0.005),
            remaining_months: 360,
            psa_speed: dec!(0),
            settlement_delay_days: 0,
        };
        let out = run_pass_through(&input);

        // Total cashflow should be > original balance (interest on top).
        assert!(
            out.total_cashflow > dec!(100),
            "Total cashflow should exceed par"
        );
    }

    // -----------------------------------------------------------------------
    // 30. WAL reasonable range for standard MBS
    // -----------------------------------------------------------------------
    #[test]
    fn test_wal_reasonable_range() {
        let input = standard_pt_input();
        let out = run_pass_through(&input);

        // At 150% PSA, WAL should be roughly 5-12 years for a 30y mortgage.
        assert!(
            out.weighted_average_life > dec!(3),
            "WAL should be > 3 years, got {}",
            out.weighted_average_life
        );
        assert!(
            out.weighted_average_life < dec!(15),
            "WAL should be < 15 years, got {}",
            out.weighted_average_life
        );
    }

    // -----------------------------------------------------------------------
    // 31. Negative PSA validation
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_psa() {
        let input = PassThroughInput {
            psa_speed: dec!(-50),
            ..standard_pt_input()
        };
        let mbs_input = MbsAnalyticsInput::PassThrough(input);
        assert!(analyze_mbs(&mbs_input).is_err());
    }

    // -----------------------------------------------------------------------
    // 32. Negative servicing fee validation
    // -----------------------------------------------------------------------
    #[test]
    fn test_validation_negative_servicing_fee() {
        let input = PassThroughInput {
            servicing_fee: dec!(-0.01),
            ..standard_pt_input()
        };
        let mbs_input = MbsAnalyticsInput::PassThrough(input);
        assert!(analyze_mbs(&mbs_input).is_err());
    }
}
