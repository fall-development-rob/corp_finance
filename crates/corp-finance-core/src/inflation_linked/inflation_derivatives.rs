//! Inflation Derivatives Module.
//!
//! Provides institutional-grade analytics for inflation derivative instruments:
//!
//! - **Zero-Coupon Inflation Swap (ZCIS)**: Fair swap rate, leg PVs, NPV,
//!   breakeven inflation implied.
//! - **Year-on-Year Inflation Swap (YYIS)**: Multi-period swap with periodic
//!   exchanges of realized vs fixed inflation.
//! - **Inflation Cap/Floor**: Black-model pricing with caplet/floorlet
//!   decomposition, Greeks (delta, vega), and put-call parity checks.
//!
//! All financial math uses `rust_decimal::Decimal` (never f64). Helpers for
//! `exp`, `ln`, `sqrt`, and `norm_cdf` (Abramowitz & Stegun).

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
// Math helpers (Decimal-only, no f64)
// ---------------------------------------------------------------------------

/// Taylor series exp(x) with range reduction for numerical stability.
///
/// For negative arguments, computes 1/exp(|x|) to avoid catastrophic
/// cancellation from alternating signs in the Taylor series.
/// Uses range reduction: exp(x) = exp(x/2^k)^(2^k) to keep the Taylor
/// argument small (|arg| < 1).
fn decimal_exp(x: Decimal) -> Decimal {
    // Handle negative arguments: exp(-x) = 1/exp(x)
    if x < Decimal::ZERO {
        let pos_exp = decimal_exp(-x);
        if pos_exp == Decimal::ZERO {
            return Decimal::ZERO;
        }
        return Decimal::ONE / pos_exp;
    }

    // Range reduction: find k such that x / 2^k < 1
    let mut k: u32 = 0;
    let mut reduced = x;
    while reduced > Decimal::ONE {
        reduced /= dec!(2);
        k += 1;
    }

    // Taylor series on reduced argument
    let mut sum = Decimal::ONE;
    let mut term = Decimal::ONE;
    for n in 1..=TAYLOR_EXP_TERMS {
        term = term * reduced / Decimal::from(n);
        sum += term;
    }

    // Square back: exp(x) = exp(x/2^k)^(2^k)
    for _ in 0..k {
        sum = sum * sum;
    }
    sum
}

/// Newton's method ln(x) with `NEWTON_ITERATIONS` iterations.
fn decimal_ln(x: Decimal) -> Decimal {
    if x <= Decimal::ZERO {
        return Decimal::ZERO;
    }
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
fn decimal_pow(base: Decimal, exp: Decimal) -> Decimal {
    if base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    decimal_exp(exp * decimal_ln(base))
}

/// Raise a Decimal base to an integer exponent via iterative multiplication.
fn decimal_powi(base: Decimal, n: u32) -> Decimal {
    let mut result = Decimal::ONE;
    for _ in 0..n {
        result *= base;
    }
    result
}

/// Standard normal CDF using Abramowitz & Stegun approximation.
fn norm_cdf(x: Decimal) -> Decimal {
    if x < dec!(-10) {
        return Decimal::ZERO;
    }
    if x > dec!(10) {
        return Decimal::ONE;
    }
    let is_neg = x < Decimal::ZERO;
    let ax = if is_neg { -x } else { x };

    // Abramowitz & Stegun constants
    let p = dec!(0.2316419);
    let b1 = dec!(0.319381530);
    let b2 = dec!(-0.356563782);
    let b3 = dec!(1.781477937);
    let b4 = dec!(-1.821255978);
    let b5 = dec!(1.330274429);

    let t = Decimal::ONE / (Decimal::ONE + p * ax);
    let t2 = t * t;
    let t3 = t2 * t;
    let t4 = t3 * t;
    let t5 = t4 * t;

    // pdf = (1/sqrt(2*pi)) * exp(-x^2/2)
    let inv_sqrt_2pi = dec!(0.3989422804014327);
    let pdf = inv_sqrt_2pi * decimal_exp(-ax * ax / dec!(2));

    let cdf = Decimal::ONE - pdf * (b1 * t + b2 * t2 + b3 * t3 + b4 * t4 + b5 * t5);

    if is_neg {
        Decimal::ONE - cdf
    } else {
        cdf
    }
}

/// Standard normal PDF.
#[allow(dead_code)]
fn norm_pdf(x: Decimal) -> Decimal {
    let inv_sqrt_2pi = dec!(0.3989422804014327);
    inv_sqrt_2pi * decimal_exp(-x * x / dec!(2))
}

// ---------------------------------------------------------------------------
// Input / Output types — Zero-Coupon Inflation Swap (ZCIS)
// ---------------------------------------------------------------------------

/// Input for a zero-coupon inflation swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcisInput {
    /// Notional principal
    pub notional: Money,
    /// Maturity in years
    pub maturity_years: Years,
    /// CPI level at inception
    pub cpi_base: Decimal,
    /// CPI level at valuation
    pub cpi_current: Decimal,
    /// Expected annual inflation rate
    pub expected_inflation: Rate,
    /// Real discount rate (for floating leg)
    pub real_discount_rate: Rate,
    /// Nominal discount rate
    pub nominal_discount_rate: Rate,
}

/// Output of a zero-coupon inflation swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZcisOutput {
    /// Fair fixed swap rate k
    pub fair_swap_rate: Rate,
    /// Present value of the fixed leg
    pub fixed_leg_pv: Money,
    /// Present value of the floating (inflation) leg
    pub floating_leg_pv: Money,
    /// Net present value: PV(floating) - PV(fixed)
    pub swap_npv: Money,
    /// Breakeven inflation implied from the swap
    pub breakeven_inflation_implied: Rate,
}

// ---------------------------------------------------------------------------
// Input / Output types — Year-on-Year Inflation Swap (YYIS)
// ---------------------------------------------------------------------------

/// Input for a year-on-year inflation swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YyisInput {
    /// Notional principal
    pub notional: Money,
    /// Number of payment periods
    pub num_periods: u32,
    /// Payments per year (1, 2, 4)
    pub payment_frequency: u8,
    /// CPI level at inception
    pub cpi_base: Decimal,
    /// Expected inflation rate for each period
    pub expected_inflation_curve: Vec<Rate>,
    /// Real discount rate for each period
    pub real_discount_curve: Vec<Rate>,
    /// Nominal discount rate for each period
    pub nominal_discount_curve: Vec<Rate>,
}

/// A single period cashflow in a YYIS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YyisCashflow {
    /// Period number (1-based)
    pub period: u32,
    /// Year-on-year inflation rate for this period
    pub yoy_inflation: Rate,
    /// Floating leg payment (notional * yoy)
    pub floating_payment: Money,
    /// Fixed leg payment (notional * fair_rate / freq)
    pub fixed_payment: Money,
    /// Discount factor
    pub discount_factor: Decimal,
    /// PV of the net exchange
    pub net_pv: Money,
}

/// Output of a year-on-year inflation swap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YyisOutput {
    /// Fair fixed swap rate
    pub fair_swap_rate: Rate,
    /// Period-by-period cashflows
    pub period_cashflows: Vec<YyisCashflow>,
    /// PV of the fixed leg
    pub fixed_leg_pv: Money,
    /// PV of the floating leg
    pub floating_leg_pv: Money,
    /// Net present value: PV(floating) - PV(fixed)
    pub swap_npv: Money,
}

// ---------------------------------------------------------------------------
// Input / Output types — Inflation Cap / Floor
// ---------------------------------------------------------------------------

/// Cap or Floor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InflationOptionType {
    Cap,
    Floor,
}

/// Input for an inflation cap or floor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflationCapFloorInput {
    /// Notional principal
    pub notional: Money,
    /// Strike inflation rate
    pub strike_rate: Rate,
    /// Cap or Floor
    pub option_type: InflationOptionType,
    /// Number of caplet/floorlet periods
    pub num_periods: u32,
    /// Expected inflation rate per period
    pub expected_inflation_curve: Vec<Rate>,
    /// Inflation volatility (annualized)
    pub inflation_vol: Rate,
    /// Discount rate per period
    pub discount_curve: Vec<Rate>,
}

/// A single caplet or floorlet value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapletFloorletValue {
    /// Period number (1-based)
    pub period: u32,
    /// Premium for this caplet/floorlet
    pub premium: Money,
    /// Intrinsic value
    pub intrinsic: Money,
    /// Time value
    pub time_value: Money,
}

/// Output of inflation cap/floor pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflationCapFloorOutput {
    /// Total option premium
    pub total_premium: Money,
    /// Individual caplet/floorlet values
    pub caplet_floorlet_values: Vec<CapletFloorletValue>,
    /// Breakeven inflation implied
    pub implied_breakeven: Rate,
    /// Delta (sensitivity to expected inflation)
    pub delta: Decimal,
    /// Vega (sensitivity to inflation vol)
    pub vega: Decimal,
}

// ---------------------------------------------------------------------------
// Wrapper types
// ---------------------------------------------------------------------------

/// Model selection for inflation derivatives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InflationDerivativeModel {
    Zcis(ZcisInput),
    Yyis(YyisInput),
    CapFloor(InflationCapFloorInput),
}

/// Top-level input for inflation derivative analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflationDerivativeInput {
    pub model: InflationDerivativeModel,
}

/// Top-level output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InflationDerivativeOutput {
    Zcis(ZcisOutput),
    Yyis(YyisOutput),
    CapFloor(InflationCapFloorOutput),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Analyze inflation derivative instruments.
///
/// Dispatches to the appropriate sub-model:
/// - `Zcis` — zero-coupon inflation swap
/// - `Yyis` — year-on-year inflation swap
/// - `CapFloor` — inflation cap or floor
pub fn analyze_inflation_derivatives(
    input: &InflationDerivativeInput,
) -> CorpFinanceResult<ComputationOutput<InflationDerivativeOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    let result = match &input.model {
        InflationDerivativeModel::Zcis(z) => {
            let out = compute_zcis(z)?;
            InflationDerivativeOutput::Zcis(out)
        }
        InflationDerivativeModel::Yyis(y) => {
            let out = compute_yyis(y)?;
            InflationDerivativeOutput::Yyis(out)
        }
        InflationDerivativeModel::CapFloor(c) => {
            let out = compute_inflation_cap_floor(c)?;
            InflationDerivativeOutput::CapFloor(out)
        }
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Inflation Derivatives Analytics",
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
// ZCIS implementation
// ---------------------------------------------------------------------------

fn compute_zcis(input: &ZcisInput) -> CorpFinanceResult<ZcisOutput> {
    // Validation
    if input.notional <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.maturity_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.cpi_base <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "cpi_base".into(),
            reason: "Must be positive".into(),
        });
    }

    let t = input.maturity_years;

    // Floating leg: pays CPI(T)/CPI(0) - 1 at maturity.
    // Project CPI(T) = CPI_current * (1 + expected_inflation)^T_remaining
    // For simplicity, assume valuation at inception so T_remaining = T,
    // CPI(0) = cpi_base, and CPI(T) projected from cpi_current.
    //
    // The PV of the floating leg:
    //   PV_float = notional * [ E[CPI(T)/CPI(0)] - 1 ] * DF_nominal(T)
    //
    // where E[CPI(T)/CPI(0)] = (cpi_current/cpi_base) * (1 + expected_inflation)^T

    let cpi_ratio = input.cpi_current / input.cpi_base;
    let projected_growth = decimal_powi(Decimal::ONE + input.expected_inflation, t_to_periods(t));
    let expected_cpi_ratio_at_maturity = cpi_ratio * projected_growth;

    // Nominal discount factor: DF = 1 / (1 + r_nom)^T
    let nom_df =
        Decimal::ONE / decimal_powi(Decimal::ONE + input.nominal_discount_rate, t_to_periods(t));

    let floating_leg_pv = input.notional * (expected_cpi_ratio_at_maturity - Decimal::ONE) * nom_df;

    // Fair swap rate k such that:
    //   notional * [ (1+k)^T - 1 ] * DF_nominal(T) = floating_leg_pv
    //   (1+k)^T - 1 = floating_leg_pv / (notional * DF_nominal(T))
    //   (1+k)^T = 1 + floating_leg_pv / (notional * DF_nominal(T))
    //   k = [ 1 + floating_leg_pv / (notional * DF_nominal(T)) ]^(1/T) - 1

    let denom = input.notional * nom_df;
    let fair_swap_rate = if denom > Decimal::ZERO {
        let ratio = Decimal::ONE + floating_leg_pv / denom;
        let inv_t = Decimal::ONE / t;
        decimal_pow(ratio, inv_t) - Decimal::ONE
    } else {
        input.expected_inflation
    };

    // Fixed leg PV at fair rate
    let fixed_leg_factor =
        decimal_powi(Decimal::ONE + fair_swap_rate, t_to_periods(t)) - Decimal::ONE;
    let fixed_leg_pv = input.notional * fixed_leg_factor * nom_df;

    let swap_npv = floating_leg_pv - fixed_leg_pv;

    // Breakeven inflation: the fair swap rate IS the breakeven
    let breakeven_inflation_implied = fair_swap_rate;

    Ok(ZcisOutput {
        fair_swap_rate,
        fixed_leg_pv,
        floating_leg_pv,
        swap_npv,
        breakeven_inflation_implied,
    })
}

/// Convert a decimal year count to integer periods (for annual compounding).
fn t_to_periods(t: Decimal) -> u32 {
    // Round to nearest integer for iterative pow
    let rounded = t.round();
    // Clamp to u32
    if rounded <= Decimal::ZERO {
        return 1;
    }
    let val: u64 = rounded.try_into().unwrap_or(1);
    val as u32
}

// ---------------------------------------------------------------------------
// YYIS implementation
// ---------------------------------------------------------------------------

fn compute_yyis(input: &YyisInput) -> CorpFinanceResult<YyisOutput> {
    // Validation
    if input.notional <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.num_periods == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_periods".into(),
            reason: "Must be at least 1".into(),
        });
    }
    if input.payment_frequency == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "payment_frequency".into(),
            reason: "Must be at least 1".into(),
        });
    }
    if input.expected_inflation_curve.len() < input.num_periods as usize {
        return Err(CorpFinanceError::InsufficientData(
            "expected_inflation_curve must have at least num_periods entries".into(),
        ));
    }
    if input.nominal_discount_curve.len() < input.num_periods as usize {
        return Err(CorpFinanceError::InsufficientData(
            "nominal_discount_curve must have at least num_periods entries".into(),
        ));
    }

    let freq = Decimal::from(input.payment_frequency);

    // Compute floating leg PV: sum over periods of
    //   notional * E[yoy_inflation_t] * DF(t)
    // where DF is built from nominal curve.
    let mut floating_leg_pv = Decimal::ZERO;
    let mut df = Decimal::ONE; // cumulative discount factor
    let mut cashflows: Vec<YyisCashflow> = Vec::with_capacity(input.num_periods as usize);

    for t in 0..input.num_periods {
        let idx = t as usize;
        let yoy_inflation = input.expected_inflation_curve[idx] / freq;
        let nom_rate = input.nominal_discount_curve[idx] / freq;
        df /= Decimal::ONE + nom_rate;

        let floating_pmt = input.notional * yoy_inflation;
        floating_leg_pv += floating_pmt * df;

        cashflows.push(YyisCashflow {
            period: t + 1,
            yoy_inflation,
            floating_payment: floating_pmt,
            fixed_payment: Decimal::ZERO, // filled after fair rate computed
            discount_factor: df,
            net_pv: Decimal::ZERO, // filled after
        });
    }

    // Fair swap rate: set so that PV(fixed) = PV(floating).
    // PV(fixed) = sum_t [ notional * k/freq * DF(t) ]
    //           = notional * k/freq * sum_t[DF(t)]
    // => k/freq = floating_leg_pv / (notional * sum_DF)
    let sum_df: Decimal = cashflows.iter().map(|cf| cf.discount_factor).sum();
    let fair_swap_rate = if sum_df > Decimal::ZERO && input.notional > Decimal::ZERO {
        (floating_leg_pv / (input.notional * sum_df)) * freq
    } else {
        Decimal::ZERO
    };

    // Fill in fixed payments and net PV
    let fixed_per_period = input.notional * fair_swap_rate / freq;
    let mut fixed_leg_pv = Decimal::ZERO;
    for cf in &mut cashflows {
        cf.fixed_payment = fixed_per_period;
        let net = cf.floating_payment - cf.fixed_payment;
        cf.net_pv = net * cf.discount_factor;
        fixed_leg_pv += cf.fixed_payment * cf.discount_factor;
    }

    let swap_npv = floating_leg_pv - fixed_leg_pv;

    Ok(YyisOutput {
        fair_swap_rate,
        period_cashflows: cashflows,
        fixed_leg_pv,
        floating_leg_pv,
        swap_npv,
    })
}

// ---------------------------------------------------------------------------
// Inflation Cap / Floor implementation
// ---------------------------------------------------------------------------

fn compute_inflation_cap_floor(
    input: &InflationCapFloorInput,
) -> CorpFinanceResult<InflationCapFloorOutput> {
    // Validation
    if input.notional <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "notional".into(),
            reason: "Must be positive".into(),
        });
    }
    if input.num_periods == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "num_periods".into(),
            reason: "Must be at least 1".into(),
        });
    }
    if input.inflation_vol < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "inflation_vol".into(),
            reason: "Must be non-negative".into(),
        });
    }
    if input.expected_inflation_curve.len() < input.num_periods as usize {
        return Err(CorpFinanceError::InsufficientData(
            "expected_inflation_curve must have at least num_periods entries".into(),
        ));
    }
    if input.discount_curve.len() < input.num_periods as usize {
        return Err(CorpFinanceError::InsufficientData(
            "discount_curve must have at least num_periods entries".into(),
        ));
    }

    let mut total_premium = Decimal::ZERO;
    let mut caplet_floorlet_values = Vec::with_capacity(input.num_periods as usize);
    let mut df = Decimal::ONE;
    let mut weighted_inflation_sum = Decimal::ZERO;
    let mut df_sum = Decimal::ZERO;

    // Bump for delta/vega computation
    let bump = dec!(0.0001); // 1bp

    let mut total_premium_up = Decimal::ZERO; // for delta
    let mut total_premium_vol_up = Decimal::ZERO; // for vega

    for t in 0..input.num_periods {
        let idx = t as usize;
        let fwd_inflation = input.expected_inflation_curve[idx];
        let disc_rate = input.discount_curve[idx];
        let time_to_expiry = Decimal::from(t + 1);

        // Discount factor (cumulative)
        df /= Decimal::ONE + disc_rate;

        // Black model for caplet/floorlet pricing:
        //   Caplet = DF * [F * N(d1) - K * N(d2)]
        //   Floorlet = DF * [K * N(-d2) - F * N(-d1)]
        // where F = forward inflation, K = strike, vol = inflation_vol * sqrt(T)

        let vol_t = input.inflation_vol * decimal_sqrt(time_to_expiry);

        let premium = if vol_t > Decimal::ZERO {
            let d1 =
                (decimal_ln(fwd_inflation / input.strike_rate) + vol_t * vol_t / dec!(2)) / vol_t;
            let d2 = d1 - vol_t;

            match input.option_type {
                InflationOptionType::Cap => {
                    input.notional
                        * df
                        * (fwd_inflation * norm_cdf(d1) - input.strike_rate * norm_cdf(d2))
                }
                InflationOptionType::Floor => {
                    input.notional
                        * df
                        * (input.strike_rate * norm_cdf(-d2) - fwd_inflation * norm_cdf(-d1))
                }
            }
        } else {
            // Zero vol: intrinsic only
            let intrinsic = match input.option_type {
                InflationOptionType::Cap => {
                    if fwd_inflation > input.strike_rate {
                        fwd_inflation - input.strike_rate
                    } else {
                        Decimal::ZERO
                    }
                }
                InflationOptionType::Floor => {
                    if input.strike_rate > fwd_inflation {
                        input.strike_rate - fwd_inflation
                    } else {
                        Decimal::ZERO
                    }
                }
            };
            input.notional * df * intrinsic
        };

        // Intrinsic value
        let intrinsic = match input.option_type {
            InflationOptionType::Cap => {
                let raw = fwd_inflation - input.strike_rate;
                if raw > Decimal::ZERO {
                    input.notional * df * raw
                } else {
                    Decimal::ZERO
                }
            }
            InflationOptionType::Floor => {
                let raw = input.strike_rate - fwd_inflation;
                if raw > Decimal::ZERO {
                    input.notional * df * raw
                } else {
                    Decimal::ZERO
                }
            }
        };

        let time_value = premium - intrinsic;

        total_premium += premium;

        caplet_floorlet_values.push(CapletFloorletValue {
            period: t + 1,
            premium,
            intrinsic,
            time_value,
        });

        weighted_inflation_sum += fwd_inflation * df;
        df_sum += df;

        // Bumped premium for delta (inflation up by 1bp)
        let fwd_up = fwd_inflation + bump;
        let prem_up = black_capfloor_price(
            input.notional,
            fwd_up,
            input.strike_rate,
            input.inflation_vol,
            time_to_expiry,
            df,
            &input.option_type,
        );
        total_premium_up += prem_up;

        // Bumped premium for vega (vol up by 1bp)
        let prem_vol_up = black_capfloor_price(
            input.notional,
            fwd_inflation,
            input.strike_rate,
            input.inflation_vol + bump,
            time_to_expiry,
            df,
            &input.option_type,
        );
        total_premium_vol_up += prem_vol_up;
    }

    // Implied breakeven = weighted average forward inflation
    let implied_breakeven = if df_sum > Decimal::ZERO {
        weighted_inflation_sum / df_sum
    } else {
        Decimal::ZERO
    };

    // Delta: dPremium / dInflation (per 1bp)
    let delta = if bump > Decimal::ZERO {
        (total_premium_up - total_premium) / bump
    } else {
        Decimal::ZERO
    };

    // Vega: dPremium / dVol (per 1bp)
    let vega = if bump > Decimal::ZERO {
        (total_premium_vol_up - total_premium) / bump
    } else {
        Decimal::ZERO
    };

    Ok(InflationCapFloorOutput {
        total_premium,
        caplet_floorlet_values,
        implied_breakeven,
        delta,
        vega,
    })
}

/// Black model caplet/floorlet price helper.
fn black_capfloor_price(
    notional: Money,
    forward: Rate,
    strike: Rate,
    vol: Rate,
    time_to_expiry: Decimal,
    df: Decimal,
    option_type: &InflationOptionType,
) -> Money {
    let vol_t = vol * decimal_sqrt(time_to_expiry);
    if vol_t <= Decimal::ZERO || forward <= Decimal::ZERO || strike <= Decimal::ZERO {
        // Intrinsic only
        let intr = match option_type {
            InflationOptionType::Cap => {
                if forward > strike {
                    forward - strike
                } else {
                    Decimal::ZERO
                }
            }
            InflationOptionType::Floor => {
                if strike > forward {
                    strike - forward
                } else {
                    Decimal::ZERO
                }
            }
        };
        return notional * df * intr;
    }

    let d1 = (decimal_ln(forward / strike) + vol_t * vol_t / dec!(2)) / vol_t;
    let d2 = d1 - vol_t;

    match option_type {
        InflationOptionType::Cap => {
            notional * df * (forward * norm_cdf(d1) - strike * norm_cdf(d2))
        }
        InflationOptionType::Floor => {
            notional * df * (strike * norm_cdf(-d2) - forward * norm_cdf(-d1))
        }
    }
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

    fn default_zcis_input() -> ZcisInput {
        ZcisInput {
            notional: dec!(10_000_000),
            maturity_years: dec!(5),
            cpi_base: dec!(300),
            cpi_current: dec!(300), // at inception
            expected_inflation: dec!(0.025),
            real_discount_rate: dec!(0.01),
            nominal_discount_rate: dec!(0.035),
        }
    }

    fn default_yyis_input() -> YyisInput {
        YyisInput {
            notional: dec!(10_000_000),
            num_periods: 5,
            payment_frequency: 1,
            cpi_base: dec!(300),
            expected_inflation_curve: vec![
                dec!(0.025),
                dec!(0.024),
                dec!(0.023),
                dec!(0.022),
                dec!(0.021),
            ],
            real_discount_curve: vec![
                dec!(0.01),
                dec!(0.011),
                dec!(0.012),
                dec!(0.013),
                dec!(0.014),
            ],
            nominal_discount_curve: vec![
                dec!(0.035),
                dec!(0.036),
                dec!(0.037),
                dec!(0.038),
                dec!(0.039),
            ],
        }
    }

    fn default_cap_input() -> InflationCapFloorInput {
        InflationCapFloorInput {
            notional: dec!(10_000_000),
            strike_rate: dec!(0.03),
            option_type: InflationOptionType::Cap,
            num_periods: 5,
            expected_inflation_curve: vec![
                dec!(0.025),
                dec!(0.026),
                dec!(0.027),
                dec!(0.028),
                dec!(0.03),
            ],
            inflation_vol: dec!(0.01),
            discount_curve: vec![
                dec!(0.035),
                dec!(0.036),
                dec!(0.037),
                dec!(0.038),
                dec!(0.039),
            ],
        }
    }

    fn default_floor_input() -> InflationCapFloorInput {
        let mut input = default_cap_input();
        input.option_type = InflationOptionType::Floor;
        input
    }

    // -----------------------------------------------------------------------
    // ZCIS tests
    // -----------------------------------------------------------------------

    #[test]
    fn zcis_fair_rate_near_expected_inflation() {
        let input = default_zcis_input();
        let out = compute_zcis(&input).unwrap();
        // Fair swap rate should be close to expected inflation
        let diff = (out.fair_swap_rate - input.expected_inflation).abs();
        assert!(
            diff < dec!(0.01),
            "Fair swap rate {} should be near expected inflation {}",
            out.fair_swap_rate,
            input.expected_inflation
        );
    }

    #[test]
    fn zcis_npv_near_zero_at_fair_rate() {
        let input = default_zcis_input();
        let out = compute_zcis(&input).unwrap();
        // At the fair rate, NPV should be approximately zero
        let npv_ratio = if out.floating_leg_pv != Decimal::ZERO {
            (out.swap_npv / out.floating_leg_pv).abs()
        } else {
            out.swap_npv.abs()
        };
        assert!(
            npv_ratio < dec!(0.01),
            "NPV should be near zero at fair rate, got ratio {}",
            npv_ratio
        );
    }

    #[test]
    fn zcis_positive_legs() {
        let input = default_zcis_input();
        let out = compute_zcis(&input).unwrap();
        assert!(
            out.fixed_leg_pv > Decimal::ZERO,
            "Fixed leg PV should be positive"
        );
        assert!(
            out.floating_leg_pv > Decimal::ZERO,
            "Floating leg PV should be positive"
        );
    }

    #[test]
    fn zcis_breakeven_equals_fair_rate() {
        let input = default_zcis_input();
        let out = compute_zcis(&input).unwrap();
        assert_eq!(out.breakeven_inflation_implied, out.fair_swap_rate);
    }

    #[test]
    fn zcis_higher_inflation_higher_fair_rate() {
        let mut input1 = default_zcis_input();
        input1.expected_inflation = dec!(0.02);
        let out1 = compute_zcis(&input1).unwrap();

        let mut input2 = default_zcis_input();
        input2.expected_inflation = dec!(0.04);
        let out2 = compute_zcis(&input2).unwrap();

        assert!(
            out2.fair_swap_rate > out1.fair_swap_rate,
            "Higher expected inflation should produce higher fair rate"
        );
    }

    #[test]
    fn zcis_invalid_notional() {
        let mut input = default_zcis_input();
        input.notional = dec!(-1000);
        assert!(compute_zcis(&input).is_err());
    }

    #[test]
    fn zcis_invalid_maturity() {
        let mut input = default_zcis_input();
        input.maturity_years = Decimal::ZERO;
        assert!(compute_zcis(&input).is_err());
    }

    #[test]
    fn zcis_zero_inflation() {
        let mut input = default_zcis_input();
        input.expected_inflation = Decimal::ZERO;
        let out = compute_zcis(&input).unwrap();
        // With zero expected inflation, fair rate should be near zero
        assert!(
            out.fair_swap_rate.abs() < dec!(0.001),
            "Fair rate should be near zero with zero inflation: {}",
            out.fair_swap_rate
        );
    }

    // -----------------------------------------------------------------------
    // YYIS tests
    // -----------------------------------------------------------------------

    #[test]
    fn yyis_fair_rate_positive() {
        let input = default_yyis_input();
        let out = compute_yyis(&input).unwrap();
        assert!(out.fair_swap_rate > Decimal::ZERO);
    }

    #[test]
    fn yyis_npv_near_zero_at_fair_rate() {
        let input = default_yyis_input();
        let out = compute_yyis(&input).unwrap();
        let npv_ratio = if out.floating_leg_pv != Decimal::ZERO {
            (out.swap_npv / out.floating_leg_pv).abs()
        } else {
            out.swap_npv.abs()
        };
        assert!(
            npv_ratio < dec!(0.01),
            "YYIS NPV should be near zero at fair rate, got ratio {}",
            npv_ratio
        );
    }

    #[test]
    fn yyis_cashflow_count() {
        let input = default_yyis_input();
        let out = compute_yyis(&input).unwrap();
        assert_eq!(out.period_cashflows.len(), input.num_periods as usize);
    }

    #[test]
    fn yyis_fixed_payments_uniform() {
        let input = default_yyis_input();
        let out = compute_yyis(&input).unwrap();
        let first_fixed = out.period_cashflows[0].fixed_payment;
        for cf in &out.period_cashflows {
            assert_eq!(
                cf.fixed_payment, first_fixed,
                "Fixed payments should be uniform"
            );
        }
    }

    #[test]
    fn yyis_consistency_with_zcis_direction() {
        // YYIS fair rate should be in the same ballpark as the average inflation
        let input = default_yyis_input();
        let out = compute_yyis(&input).unwrap();
        let avg_inflation: Decimal = input.expected_inflation_curve.iter().sum::<Decimal>()
            / Decimal::from(input.expected_inflation_curve.len() as u32);
        let diff = (out.fair_swap_rate - avg_inflation).abs();
        assert!(
            diff < dec!(0.01),
            "YYIS fair rate {} should be near average inflation {}",
            out.fair_swap_rate,
            avg_inflation
        );
    }

    #[test]
    fn yyis_invalid_num_periods() {
        let mut input = default_yyis_input();
        input.num_periods = 0;
        assert!(compute_yyis(&input).is_err());
    }

    #[test]
    fn yyis_insufficient_curve_data() {
        let mut input = default_yyis_input();
        input.expected_inflation_curve = vec![dec!(0.025)]; // only 1 vs 5 needed
        assert!(compute_yyis(&input).is_err());
    }

    // -----------------------------------------------------------------------
    // Inflation Cap/Floor tests
    // -----------------------------------------------------------------------

    #[test]
    fn cap_positive_premium() {
        let input = default_cap_input();
        let out = compute_inflation_cap_floor(&input).unwrap();
        assert!(
            out.total_premium > Decimal::ZERO,
            "Cap premium should be positive"
        );
    }

    #[test]
    fn floor_positive_premium() {
        let input = default_floor_input();
        let out = compute_inflation_cap_floor(&input).unwrap();
        assert!(
            out.total_premium > Decimal::ZERO,
            "Floor premium should be positive"
        );
    }

    #[test]
    fn cap_floor_parity_approximation() {
        // Put-call parity: Cap - Floor ~ Swap value
        // At the same strike, Cap_premium - Floor_premium should be approximately
        // equal to the swap NPV (floating - fixed at that strike level).
        let cap_input = default_cap_input();
        let floor_input = default_floor_input();

        let cap_out = compute_inflation_cap_floor(&cap_input).unwrap();
        let floor_out = compute_inflation_cap_floor(&floor_input).unwrap();

        // Cap - Floor
        let cap_minus_floor = cap_out.total_premium - floor_out.total_premium;

        // The swap-equivalent value: sum of (fwd - strike) * DF * notional
        let mut swap_value = Decimal::ZERO;
        let mut df = Decimal::ONE;
        for t in 0..cap_input.num_periods {
            let idx = t as usize;
            df = df / (Decimal::ONE + cap_input.discount_curve[idx]);
            let fwd = cap_input.expected_inflation_curve[idx];
            swap_value += cap_input.notional * (fwd - cap_input.strike_rate) * df;
        }

        // They should be approximately equal
        let diff = (cap_minus_floor - swap_value).abs();
        let tolerance = cap_out.total_premium.abs() * dec!(0.05); // 5% relative tolerance
        assert!(
            diff < tolerance + dec!(1), // add 1 for near-zero values
            "Cap - Floor ({}) should approx equal swap value ({}), diff = {}",
            cap_minus_floor,
            swap_value,
            diff
        );
    }

    #[test]
    fn cap_premium_increases_with_vol() {
        let mut input_low = default_cap_input();
        input_low.inflation_vol = dec!(0.005);
        let out_low = compute_inflation_cap_floor(&input_low).unwrap();

        let mut input_high = default_cap_input();
        input_high.inflation_vol = dec!(0.02);
        let out_high = compute_inflation_cap_floor(&input_high).unwrap();

        assert!(
            out_high.total_premium >= out_low.total_premium,
            "Higher vol ({}) should give higher premium ({}) vs ({})",
            input_high.inflation_vol,
            out_high.total_premium,
            out_low.total_premium
        );
    }

    #[test]
    fn cap_caplet_count() {
        let input = default_cap_input();
        let out = compute_inflation_cap_floor(&input).unwrap();
        assert_eq!(out.caplet_floorlet_values.len(), input.num_periods as usize);
    }

    #[test]
    fn cap_delta_positive() {
        let input = default_cap_input();
        let out = compute_inflation_cap_floor(&input).unwrap();
        assert!(
            out.delta >= Decimal::ZERO,
            "Cap delta should be non-negative"
        );
    }

    #[test]
    fn floor_delta_negative_or_zero() {
        // Floor delta should be non-positive (value increases when inflation decreases)
        let input = default_floor_input();
        let out = compute_inflation_cap_floor(&input).unwrap();
        assert!(
            out.delta <= Decimal::ZERO,
            "Floor delta should be non-positive, got {}",
            out.delta
        );
    }

    #[test]
    fn cap_vega_positive() {
        let input = default_cap_input();
        let out = compute_inflation_cap_floor(&input).unwrap();
        assert!(out.vega >= Decimal::ZERO, "Cap vega should be non-negative");
    }

    #[test]
    fn cap_zero_vol_intrinsic_only() {
        let mut input = default_cap_input();
        input.inflation_vol = Decimal::ZERO;
        // Make some forwards above strike to have non-zero intrinsic
        input.expected_inflation_curve = vec![
            dec!(0.035),
            dec!(0.035),
            dec!(0.035),
            dec!(0.035),
            dec!(0.035),
        ];
        let out = compute_inflation_cap_floor(&input).unwrap();
        // With zero vol, premium = intrinsic, so time_value = 0
        for cv in &out.caplet_floorlet_values {
            assert!(
                cv.time_value.abs() < dec!(0.01),
                "Time value should be ~0 at zero vol, got {}",
                cv.time_value
            );
        }
    }

    #[test]
    fn cap_invalid_notional() {
        let mut input = default_cap_input();
        input.notional = dec!(-1000);
        assert!(compute_inflation_cap_floor(&input).is_err());
    }

    #[test]
    fn cap_invalid_num_periods() {
        let mut input = default_cap_input();
        input.num_periods = 0;
        assert!(compute_inflation_cap_floor(&input).is_err());
    }

    #[test]
    fn cap_negative_vol_error() {
        let mut input = default_cap_input();
        input.inflation_vol = dec!(-0.01);
        assert!(compute_inflation_cap_floor(&input).is_err());
    }

    #[test]
    fn cap_long_maturity() {
        // Test with a 30-period cap
        let mut input = default_cap_input();
        input.num_periods = 30;
        input.expected_inflation_curve = (0..30)
            .map(|i| dec!(0.025) + Decimal::from(i) * dec!(0.0001))
            .collect();
        input.discount_curve = (0..30).map(|_| dec!(0.035)).collect();
        let out = compute_inflation_cap_floor(&input).unwrap();
        assert!(out.total_premium > Decimal::ZERO);
        assert_eq!(out.caplet_floorlet_values.len(), 30);
    }

    // -----------------------------------------------------------------------
    // Wrapper function tests
    // -----------------------------------------------------------------------

    #[test]
    fn wrapper_zcis() {
        let input = InflationDerivativeInput {
            model: InflationDerivativeModel::Zcis(default_zcis_input()),
        };
        let out = analyze_inflation_derivatives(&input).unwrap();
        assert_eq!(out.methodology, "Inflation Derivatives Analytics");
        match out.result {
            InflationDerivativeOutput::Zcis(z) => {
                assert!(z.fair_swap_rate > Decimal::ZERO);
            }
            _ => panic!("Expected ZCIS output"),
        }
    }

    #[test]
    fn wrapper_yyis() {
        let input = InflationDerivativeInput {
            model: InflationDerivativeModel::Yyis(default_yyis_input()),
        };
        let out = analyze_inflation_derivatives(&input).unwrap();
        match out.result {
            InflationDerivativeOutput::Yyis(y) => {
                assert!(y.fair_swap_rate > Decimal::ZERO);
            }
            _ => panic!("Expected YYIS output"),
        }
    }

    #[test]
    fn wrapper_cap_floor() {
        let input = InflationDerivativeInput {
            model: InflationDerivativeModel::CapFloor(default_cap_input()),
        };
        let out = analyze_inflation_derivatives(&input).unwrap();
        match out.result {
            InflationDerivativeOutput::CapFloor(c) => {
                assert!(c.total_premium > Decimal::ZERO);
            }
            _ => panic!("Expected CapFloor output"),
        }
    }

    // -----------------------------------------------------------------------
    // Math helper tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_norm_cdf_symmetry() {
        // N(0) = 0.5
        let n0 = norm_cdf(Decimal::ZERO);
        let diff = (n0 - dec!(0.5)).abs();
        assert!(diff < dec!(0.001), "N(0) should be 0.5, got {}", n0);
    }

    #[test]
    fn test_norm_cdf_tail() {
        // N(3) should be very close to 1
        let n3 = norm_cdf(dec!(3));
        assert!(n3 > dec!(0.998));
        // N(-3) should be very close to 0
        let nm3 = norm_cdf(dec!(-3));
        assert!(nm3 < dec!(0.002));
    }

    #[test]
    fn test_decimal_powi() {
        let result = decimal_powi(dec!(1.05), 10);
        // (1.05)^10 ≈ 1.62889
        let diff = (result - dec!(1.62889)).abs();
        assert!(
            diff < dec!(0.001),
            "(1.05)^10 should be ~1.62889, got {}",
            result
        );
    }
}
