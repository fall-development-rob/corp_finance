//! Municipal bond pricing module for institutional-grade muni analytics.
//!
//! Supports tax-exempt yield pricing, tax-equivalent yield (TEY) with
//! federal/state adjustments, de minimis tax rule analysis, callable bond
//! yield-to-call / yield-to-worst, and muni vs. taxable spread comparisons.

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

/// Binomial series terms for fractional exponent approximation.
const BINOMIAL_TERMS: u32 = 15;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Municipal bond type classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuniBondType {
    GeneralObligation,
    Revenue,
    Assessment,
    TaxIncrement,
    CertificateOfParticipation,
}

/// A single cashflow in the muni bond schedule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuniCashFlow {
    /// Period number (1-indexed).
    pub period: u32,
    /// Cash flow amount.
    pub amount: Money,
    /// Cashflow type label.
    pub cashflow_type: String,
}

/// De minimis tax rule analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeMinimisResult {
    /// Whether the market discount is within the de minimis threshold.
    pub is_de_minimis: bool,
    /// Original issue discount or market discount amount.
    pub oid_amount: Money,
    /// Tax treatment description.
    pub tax_treatment: String,
    /// De minimis threshold amount.
    pub threshold: Money,
}

/// Input parameters for municipal bond pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuniBondInput {
    /// Bond name/identifier.
    pub bond_name: String,
    /// Face (par) value, typically 5000 for munis.
    pub face_value: Money,
    /// Annual coupon rate as a decimal (e.g. 0.05 = 5%).
    pub coupon_rate: Rate,
    /// Coupons per year: 1, 2, 4, or 12.
    pub coupon_frequency: u32,
    /// Years to maturity.
    pub maturity_years: Decimal,
    /// Tax-exempt yield to maturity.
    pub yield_to_maturity: Rate,
    /// Investor's marginal federal tax rate.
    pub federal_tax_rate: Rate,
    /// Investor's state tax rate.
    pub state_tax_rate: Rate,
    /// True if bond is exempt from state tax (in-state muni).
    pub state_tax_exempt: bool,
    /// Alternative minimum tax rate (for private activity bonds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amt_rate: Option<Rate>,
    /// Whether this is a private activity bond subject to AMT.
    pub is_private_activity: bool,
    /// Purchase price for de minimis analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_price: Option<Money>,
    /// Par call price (usually face value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub par_call_price: Option<Money>,
    /// Years until first call date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_date_years: Option<Decimal>,
    /// Call premium as a decimal (e.g. 0.02 = 102% of par).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_premium: Option<Rate>,
    /// Credit rating (AAA, AA, A, BBB, BB, B).
    pub credit_rating: String,
    /// Municipal bond type.
    pub bond_type: MuniBondType,
    /// Comparable Treasury yield for spread analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparable_treasury_yield: Option<Rate>,
    /// Comparable corporate yield for spread analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparable_corporate_yield: Option<Rate>,
    /// Day count convention string. Standard for munis is "30/360".
    pub day_count: String,
}

/// Output of municipal bond pricing computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MuniBondOutput {
    /// Clean price (excludes accrued interest).
    pub clean_price: Money,
    /// Dirty price (clean + accrued interest).
    pub dirty_price: Money,
    /// Accrued interest.
    pub accrued_interest: Money,
    /// Current yield = annual coupon / clean price.
    pub current_yield: Rate,
    /// Tax-exempt yield to maturity (same as input YTM).
    pub yield_to_maturity: Rate,
    /// Federal tax-equivalent yield.
    pub tax_equivalent_yield: Rate,
    /// State-adjusted tax-equivalent yield.
    pub state_adjusted_tey: Rate,
    /// After-tax yield for comparison with taxable bonds.
    pub after_tax_yield: Rate,
    /// Muni yield / Treasury yield ratio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muni_to_treasury_ratio: Option<Rate>,
    /// TEY - Treasury yield in basis points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub taxable_equivalent_spread: Option<Decimal>,
    /// TEY - corporate yield in basis points (pickup).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corporate_spread_pickup: Option<Decimal>,
    /// De minimis tax rule analysis.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub de_minimis_analysis: Option<DeMinimisResult>,
    /// Yield to call (if callable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_to_call: Option<Rate>,
    /// Yield to worst = min(YTM, YTC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yield_to_worst: Option<Rate>,
    /// Call protection value — price differential if non-callable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub call_protection_value: Option<Money>,
    /// Credit spread vs AAA muni benchmark in basis points.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credit_spread_bps: Option<Decimal>,
    /// Scheduled cashflow entries.
    pub cashflow_schedule: Vec<MuniCashFlow>,
    /// Total return if held to maturity (sum of coupons + principal).
    pub total_return_if_held: Money,
    /// Warning messages.
    pub warnings: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Price a municipal bond and compute tax-equivalent yields, de minimis analysis,
/// call analysis, and relative value metrics.
pub fn price_muni_bond(
    input: &MuniBondInput,
) -> CorpFinanceResult<ComputationOutput<MuniBondOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    validate_input(input)?;

    let freq = Decimal::from(input.coupon_frequency);
    let coupon_per_period = input.face_value * input.coupon_rate / freq;
    let annual_coupon = input.face_value * input.coupon_rate;
    let n_periods = input.maturity_years * freq;
    let n_int = decimal_to_u32(n_periods);

    // --- Build cashflow schedule ---
    let cashflow_schedule = build_cashflow_schedule(n_int, coupon_per_period, input.face_value);

    // --- PV / Clean price ---
    let clean_price = compute_clean_price(
        coupon_per_period,
        input.face_value,
        input.yield_to_maturity,
        freq,
        n_int,
    );

    // --- Accrued interest (30/360, mid-period assumption = 0 for new issue) ---
    // For simplicity, we assume settlement at a coupon date (accrued = 0) unless
    // maturity_years has a fractional component indicating mid-period settlement.
    let accrued_interest =
        compute_accrued_interest_30_360(input.maturity_years, freq, coupon_per_period);

    let dirty_price = clean_price + accrued_interest;

    // --- Current yield ---
    let current_yield = if clean_price > Decimal::ZERO {
        annual_coupon / clean_price
    } else {
        warnings.push("Clean price is zero or negative; current yield undefined".into());
        Decimal::ZERO
    };

    // --- Tax-Equivalent Yield (federal only) ---
    let tey_federal = compute_federal_tey(
        input.yield_to_maturity,
        input.federal_tax_rate,
        input.is_private_activity,
        input.amt_rate,
        &mut warnings,
    );

    // --- State-Adjusted TEY ---
    let state_adjusted_tey = compute_state_adjusted_tey(
        input.yield_to_maturity,
        input.federal_tax_rate,
        input.state_tax_rate,
        input.state_tax_exempt,
        input.is_private_activity,
        input.amt_rate,
        &mut warnings,
    );

    // --- After-tax yield (what a taxable bond holder would need) ---
    // For a tax-exempt muni, the after-tax yield equals the muni yield itself.
    let after_tax_yield = input.yield_to_maturity;

    // --- Muni / Treasury ratio ---
    let muni_to_treasury_ratio = input.comparable_treasury_yield.map(|tsy| {
        if tsy > Decimal::ZERO {
            input.yield_to_maturity / tsy
        } else {
            warnings.push("Treasury yield is zero; ratio undefined".into());
            Decimal::ZERO
        }
    });

    // --- Taxable-equivalent spread (TEY - Treasury, in bps) ---
    let taxable_equivalent_spread = input
        .comparable_treasury_yield
        .map(|tsy| (state_adjusted_tey - tsy) * dec!(10000));

    // --- Corporate spread pickup (TEY - corporate, in bps) ---
    let corporate_spread_pickup = input
        .comparable_corporate_yield
        .map(|corp| (state_adjusted_tey - corp) * dec!(10000));

    // --- De minimis analysis ---
    let de_minimis_analysis = input
        .purchase_price
        .map(|pp| compute_de_minimis(input.face_value, pp, input.maturity_years));

    // --- Yield to Call / Yield to Worst ---
    let (yield_to_call, yield_to_worst, call_protection_value) =
        compute_call_analysis(input, clean_price, coupon_per_period, freq, &mut warnings);

    // --- Credit spread vs AAA benchmark ---
    let credit_spread_bps = compute_credit_spread(&input.credit_rating);

    // --- Total return if held ---
    let total_return_if_held = annual_coupon * input.maturity_years + input.face_value;

    let output = MuniBondOutput {
        clean_price,
        dirty_price,
        accrued_interest,
        current_yield,
        yield_to_maturity: input.yield_to_maturity,
        tax_equivalent_yield: tey_federal,
        state_adjusted_tey,
        after_tax_yield,
        muni_to_treasury_ratio,
        taxable_equivalent_spread,
        corporate_spread_pickup,
        de_minimis_analysis,
        yield_to_call,
        yield_to_worst,
        call_protection_value,
        credit_spread_bps,
        cashflow_schedule,
        total_return_if_held,
        warnings: warnings.clone(),
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Municipal Bond Pricing — tax-exempt PV with TEY, de minimis, and call analysis",
        input,
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &MuniBondInput) -> CorpFinanceResult<()> {
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
    if input.maturity_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "Maturity years must be positive".into(),
        });
    }
    if input.federal_tax_rate < Decimal::ZERO || input.federal_tax_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "federal_tax_rate".into(),
            reason: "Federal tax rate must be in [0, 1)".into(),
        });
    }
    if input.state_tax_rate < Decimal::ZERO || input.state_tax_rate >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "state_tax_rate".into(),
            reason: "State tax rate must be in [0, 1)".into(),
        });
    }
    if let Some(amt) = input.amt_rate {
        if amt < Decimal::ZERO || amt >= Decimal::ONE {
            return Err(CorpFinanceError::InvalidInput {
                field: "amt_rate".into(),
                reason: "AMT rate must be in [0, 1)".into(),
            });
        }
    }
    if let Some(call_years) = input.call_date_years {
        if call_years <= Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "call_date_years".into(),
                reason: "Call date years must be positive".into(),
            });
        }
        if call_years >= input.maturity_years {
            return Err(CorpFinanceError::InvalidInput {
                field: "call_date_years".into(),
                reason: "Call date must be before maturity".into(),
            });
        }
    }
    if input.day_count != "30/360" {
        return Err(CorpFinanceError::InvalidInput {
            field: "day_count".into(),
            reason: "Municipal bonds use 30/360 day count convention".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Clean price computation
// ---------------------------------------------------------------------------

/// Compute clean price as PV of future cashflows discounted at YTM.
///
/// PV = sum_{i=1}^{n} coupon / (1+y/freq)^i  +  face / (1+y/freq)^n
///
/// Uses iterative multiplication for discount factors (never powd).
fn compute_clean_price(
    coupon_per_period: Money,
    face_value: Money,
    ytm: Rate,
    freq: Decimal,
    n_periods: u32,
) -> Money {
    if n_periods == 0 {
        return face_value;
    }

    let periodic_yield = ytm / freq;
    let one_plus_py = Decimal::ONE + periodic_yield;

    let mut pv = Decimal::ZERO;
    let mut discount_factor = Decimal::ONE;

    for i in 0..n_periods {
        discount_factor *= one_plus_py;
        if discount_factor.is_zero() {
            continue;
        }

        let cashflow = if i == n_periods - 1 {
            coupon_per_period + face_value
        } else {
            coupon_per_period
        };

        pv += cashflow / discount_factor;
    }

    pv
}

// ---------------------------------------------------------------------------
// Accrued interest (30/360)
// ---------------------------------------------------------------------------

/// Compute accrued interest using 30/360 convention.
///
/// For a whole-period settlement (maturity_years is an integer multiple of
/// 1/frequency), accrued is zero. For fractional periods, we compute the
/// fraction of the current coupon period that has elapsed.
fn compute_accrued_interest_30_360(
    maturity_years: Decimal,
    freq: Decimal,
    coupon_per_period: Money,
) -> Money {
    let total_periods = maturity_years * freq;
    let total_periods_rounded = total_periods.round();
    let fractional = (total_periods - total_periods_rounded).abs();

    // If total periods is essentially an integer, settlement is at a coupon date
    if fractional < dec!(0.001) {
        return Decimal::ZERO;
    }

    // Fraction of the current period that has elapsed
    // If total_periods = 19.5 and we round to 20, then 0.5 of the period remains
    // and 0.5 has elapsed.
    let periods_remaining_frac = total_periods - total_periods.floor();
    let elapsed_frac = Decimal::ONE - periods_remaining_frac;

    coupon_per_period * elapsed_frac
}

// ---------------------------------------------------------------------------
// Tax-Equivalent Yield
// ---------------------------------------------------------------------------

/// Compute federal-only tax-equivalent yield.
///
/// TEY = muni_yield / (1 - federal_tax_rate)
/// For AMT private activity bonds, uses AMT rate if higher.
fn compute_federal_tey(
    muni_yield: Rate,
    federal_tax_rate: Rate,
    is_private_activity: bool,
    amt_rate: Option<Rate>,
    warnings: &mut Vec<String>,
) -> Rate {
    let effective_rate = if is_private_activity {
        if let Some(amt) = amt_rate {
            if amt > federal_tax_rate {
                warnings.push(format!(
                    "AMT rate ({}) exceeds federal rate ({}); using AMT rate for TEY",
                    amt, federal_tax_rate
                ));
                amt
            } else {
                federal_tax_rate
            }
        } else {
            warnings
                .push("Private activity bond but no AMT rate provided; using federal rate".into());
            federal_tax_rate
        }
    } else {
        federal_tax_rate
    };

    let denominator = Decimal::ONE - effective_rate;
    if denominator <= Decimal::ZERO {
        return muni_yield; // guard against 100% tax rate
    }

    muni_yield / denominator
}

/// Compute state-adjusted tax-equivalent yield.
///
/// If state_tax_exempt (in-state bond):
///   TEY = muni_yield / (1 - fed_rate - state_rate * (1 - fed_rate))
///
/// If NOT state_tax_exempt (out-of-state):
///   TEY = muni_yield / (1 - fed_rate)
///   (state tax applies on the muni income, so no state benefit)
fn compute_state_adjusted_tey(
    muni_yield: Rate,
    federal_tax_rate: Rate,
    state_tax_rate: Rate,
    state_tax_exempt: bool,
    is_private_activity: bool,
    amt_rate: Option<Rate>,
    warnings: &mut Vec<String>,
) -> Rate {
    let effective_fed_rate = if is_private_activity {
        amt_rate
            .filter(|&amt| amt > federal_tax_rate)
            .unwrap_or(federal_tax_rate)
    } else {
        federal_tax_rate
    };

    if state_tax_exempt {
        // Combined benefit: exempt from both federal and state
        let combined = effective_fed_rate + state_tax_rate * (Decimal::ONE - effective_fed_rate);
        let denominator = Decimal::ONE - combined;
        if denominator <= Decimal::ZERO {
            warnings.push("Combined tax rate >= 100%; TEY calculation clamped".into());
            return muni_yield;
        }
        muni_yield / denominator
    } else {
        // Only federal exemption
        let denominator = Decimal::ONE - effective_fed_rate;
        if denominator <= Decimal::ZERO {
            return muni_yield;
        }
        muni_yield / denominator
    }
}

// ---------------------------------------------------------------------------
// De Minimis Tax Rule
// ---------------------------------------------------------------------------

/// Analyze the de minimis tax rule for a muni bond purchased at a discount.
///
/// - Market discount = face - purchase_price
/// - De minimis threshold = face * 0.0025 * years_to_maturity
/// - If discount <= threshold: taxed as capital gain (favorable)
/// - If discount > threshold: entire discount taxed as ordinary income
fn compute_de_minimis(
    face_value: Money,
    purchase_price: Money,
    maturity_years: Decimal,
) -> DeMinimisResult {
    let market_discount = face_value - purchase_price;
    let threshold = face_value * dec!(0.0025) * maturity_years;

    if market_discount <= Decimal::ZERO {
        // Purchased at par or premium
        return DeMinimisResult {
            is_de_minimis: false,
            oid_amount: Decimal::ZERO,
            tax_treatment: "No market discount — purchased at par or premium".into(),
            threshold,
        };
    }

    if market_discount <= threshold {
        DeMinimisResult {
            is_de_minimis: true,
            oid_amount: market_discount,
            tax_treatment: format!(
                "De minimis: discount ({}) <= threshold ({}). \
                 Discount taxed as capital gain at maturity.",
                market_discount, threshold
            ),
            threshold,
        }
    } else {
        DeMinimisResult {
            is_de_minimis: false,
            oid_amount: market_discount,
            tax_treatment: format!(
                "Exceeds de minimis: discount ({}) > threshold ({}). \
                 Entire discount taxed as ordinary income.",
                market_discount, threshold
            ),
            threshold,
        }
    }
}

// ---------------------------------------------------------------------------
// Call Analysis (YTC / YTW)
// ---------------------------------------------------------------------------

/// Compute yield-to-call, yield-to-worst, and call protection value.
fn compute_call_analysis(
    input: &MuniBondInput,
    clean_price: Money,
    coupon_per_period: Money,
    freq: Decimal,
    warnings: &mut Vec<String>,
) -> (Option<Rate>, Option<Rate>, Option<Money>) {
    let call_date_years = match input.call_date_years {
        Some(y) => y,
        None => return (None, None, None),
    };

    let call_premium = input.call_premium.unwrap_or(Decimal::ZERO);
    let par_call = input.par_call_price.unwrap_or(input.face_value);
    let call_price = par_call * (Decimal::ONE + call_premium);

    let n_call = decimal_to_u32(call_date_years * freq);
    if n_call == 0 {
        warnings.push("Call date too near; cannot compute YTC".into());
        return (None, None, None);
    }

    // Solve for YTC using Newton-Raphson:
    // price = sum_{i=1}^{n_call} coupon / (1+y/freq)^i  +  call_price / (1+y/freq)^n_call
    match solve_ytc_newton(clean_price, coupon_per_period, call_price, n_call, freq) {
        Ok(ytc) => {
            let ytw = if ytc < input.yield_to_maturity {
                ytc
            } else {
                input.yield_to_maturity
            };

            // Call protection value: difference in price if bond were non-callable
            // (priced to maturity) vs callable (priced to call)
            let n_mat = decimal_to_u32(input.maturity_years * freq);
            let price_to_maturity =
                compute_clean_price(coupon_per_period, input.face_value, ytw, freq, n_mat);
            let price_to_call =
                compute_clean_price(coupon_per_period, call_price, ytw, freq, n_call);
            let call_prot = if price_to_maturity > price_to_call {
                price_to_maturity - price_to_call
            } else {
                Decimal::ZERO
            };

            (Some(ytc), Some(ytw), Some(call_prot))
        }
        Err(_) => {
            warnings.push("YTC Newton-Raphson did not converge".into());
            (None, None, None)
        }
    }
}

/// Solve yield-to-call via Newton-Raphson.
///
/// We solve: price = sum(coupon/(1+y/f)^i) + call_price/(1+y/f)^n
/// for the annualized yield y.
fn solve_ytc_newton(
    target_price: Money,
    coupon_per_period: Money,
    call_price: Money,
    n_periods: u32,
    freq: Decimal,
) -> CorpFinanceResult<Rate> {
    // Initial guess: approximate current yield
    let total_cf = coupon_per_period * Decimal::from(n_periods) + call_price;
    let mut y = if target_price > Decimal::ZERO && Decimal::from(n_periods) > Decimal::ZERO {
        // Simple approximation: (total_cf/price - 1) / (n/freq)
        let n_years = Decimal::from(n_periods) / freq;
        ((total_cf / target_price) - Decimal::ONE) / n_years
    } else {
        dec!(0.04)
    };

    for iteration in 0..NEWTON_MAX_ITERATIONS {
        let periodic_y = y / freq;
        let one_plus_py = Decimal::ONE + periodic_y;

        if one_plus_py <= Decimal::ZERO {
            y = dec!(0.01);
            continue;
        }

        // Compute price and derivative iteratively
        let mut price = Decimal::ZERO;
        let mut dprice = Decimal::ZERO;
        let mut factor = Decimal::ONE;

        for i in 1..=n_periods {
            factor *= one_plus_py;
            if factor.is_zero() {
                break;
            }

            let cf = if i == n_periods {
                coupon_per_period + call_price
            } else {
                coupon_per_period
            };

            price += cf / factor;

            // Derivative: d/dy [cf / (1+y/f)^i] = -i/f * cf / (1+y/f)^(i+1)
            let i_dec = Decimal::from(i);
            dprice -= i_dec / freq * cf / (factor * one_plus_py);
        }

        let f_val = price - target_price;

        if f_val.abs() < NEWTON_EPSILON {
            return Ok(y);
        }

        if dprice.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "Muni YTC".into(),
                iterations: iteration,
                last_delta: f_val,
            });
        }

        y -= f_val / dprice;

        // Guard against divergence
        if y < dec!(-0.50) {
            y = dec!(-0.50);
        } else if y > dec!(1.0) {
            y = dec!(1.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "Muni YTC".into(),
        iterations: NEWTON_MAX_ITERATIONS,
        last_delta: Decimal::ZERO,
    })
}

// ---------------------------------------------------------------------------
// Credit Spread
// ---------------------------------------------------------------------------

/// Compute approximate credit spread vs AAA muni benchmark in basis points.
///
/// Midpoints of typical muni credit spread ranges:
/// AAA: 0, AA: 30, A: 75, BBB: 150, BB: 300, B: 500
fn compute_credit_spread(credit_rating: &str) -> Option<Decimal> {
    let rating_upper = credit_rating.to_uppercase();
    let bps = match rating_upper.as_str() {
        "AAA" => dec!(0),
        "AA+" | "AA" | "AA-" => dec!(30),
        "A+" | "A" | "A-" => dec!(75),
        "BBB+" | "BBB" | "BBB-" => dec!(150),
        "BB+" | "BB" | "BB-" => dec!(300),
        "B+" | "B" | "B-" => dec!(500),
        _ => return None,
    };
    Some(bps)
}

// ---------------------------------------------------------------------------
// Cashflow schedule
// ---------------------------------------------------------------------------

fn build_cashflow_schedule(
    n_periods: u32,
    coupon_per_period: Money,
    face_value: Money,
) -> Vec<MuniCashFlow> {
    let mut schedule = Vec::with_capacity(n_periods as usize);

    for i in 1..=n_periods {
        let is_last = i == n_periods;
        let (amount, cf_type) = if is_last {
            (
                coupon_per_period + face_value,
                "coupon+principal".to_string(),
            )
        } else {
            (coupon_per_period, "coupon".to_string())
        };

        schedule.push(MuniCashFlow {
            period: i,
            amount,
            cashflow_type: cf_type,
        });
    }

    schedule
}

// ---------------------------------------------------------------------------
// Math helpers
// ---------------------------------------------------------------------------

/// Convert a Decimal to u32 by rounding.
fn decimal_to_u32(d: Decimal) -> u32 {
    let rounded = d.round();
    if rounded < Decimal::ZERO {
        0
    } else {
        rounded.to_string().parse::<u32>().unwrap_or(0)
    }
}

/// Compute base^exponent for Decimal fractional exponent using binomial series.
///
/// (1+x)^f = sum_{k=0}^{N} C(f,k) * x^k, converges for |x| < 1.
/// Used for fractional discount factor computations.
#[allow(dead_code)]
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
    let mut result = Decimal::ONE;
    let mut term = Decimal::ONE;

    for k in 1..=BINOMIAL_TERMS {
        let k_dec = Decimal::from(k);
        term *= (frac - k_dec + Decimal::ONE) * x / k_dec;
        result += term;
        if term.abs() < dec!(0.00000000001) {
            break;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: build a standard muni bond input for testing.
    fn standard_muni() -> MuniBondInput {
        MuniBondInput {
            bond_name: "Test Muni GO".to_string(),
            face_value: dec!(5000),
            coupon_rate: dec!(0.05),
            coupon_frequency: 2,
            maturity_years: dec!(10),
            yield_to_maturity: dec!(0.05),
            federal_tax_rate: dec!(0.37),
            state_tax_rate: dec!(0.05),
            state_tax_exempt: true,
            amt_rate: None,
            is_private_activity: false,
            purchase_price: None,
            par_call_price: None,
            call_date_years: None,
            call_premium: None,
            credit_rating: "AA".to_string(),
            bond_type: MuniBondType::GeneralObligation,
            comparable_treasury_yield: None,
            comparable_corporate_yield: None,
            day_count: "30/360".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // 1. Muni bond pricing: 5% coupon, semi-annual, 10Y at par
    // -----------------------------------------------------------------------
    #[test]
    fn test_muni_par_bond_price() {
        let input = standard_muni();
        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // When coupon rate == YTM, clean price should be approximately face value
        let diff = (out.clean_price - dec!(5000)).abs();
        assert!(
            diff < dec!(1.0),
            "Par muni bond clean price should be ~5000, got {}",
            out.clean_price
        );
        assert_eq!(out.yield_to_maturity, dec!(0.05));
    }

    // -----------------------------------------------------------------------
    // 2. Tax-equivalent yield: 3% muni, 37% fed rate -> TEY ~ 4.76%
    // -----------------------------------------------------------------------
    #[test]
    fn test_tax_equivalent_yield_federal() {
        let mut input = standard_muni();
        input.yield_to_maturity = dec!(0.03);
        input.coupon_rate = dec!(0.03);
        input.federal_tax_rate = dec!(0.37);
        input.state_tax_exempt = false; // isolate federal TEY

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // TEY = 0.03 / (1 - 0.37) = 0.03 / 0.63 = 0.047619...
        let expected_tey = dec!(0.03) / dec!(0.63);
        let diff = (out.tax_equivalent_yield - expected_tey).abs();
        assert!(
            diff < dec!(0.001),
            "Federal TEY should be ~{}, got {}",
            expected_tey,
            out.tax_equivalent_yield
        );
    }

    // -----------------------------------------------------------------------
    // 3. State-adjusted TEY: in-state bond with state exemption
    // -----------------------------------------------------------------------
    #[test]
    fn test_state_adjusted_tey_in_state() {
        let mut input = standard_muni();
        input.yield_to_maturity = dec!(0.03);
        input.coupon_rate = dec!(0.03);
        input.federal_tax_rate = dec!(0.37);
        input.state_tax_rate = dec!(0.05);
        input.state_tax_exempt = true;

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // Combined rate = 0.37 + 0.05 * (1 - 0.37) = 0.37 + 0.0315 = 0.4015
        // State-adjusted TEY = 0.03 / (1 - 0.4015) = 0.03 / 0.5985 = 0.050125...
        let combined = dec!(0.37) + dec!(0.05) * (Decimal::ONE - dec!(0.37));
        let expected = dec!(0.03) / (Decimal::ONE - combined);
        let diff = (out.state_adjusted_tey - expected).abs();
        assert!(
            diff < dec!(0.001),
            "State-adjusted TEY should be ~{}, got {}",
            expected,
            out.state_adjusted_tey
        );

        // State-adjusted TEY should be higher than federal-only TEY
        assert!(
            out.state_adjusted_tey > out.tax_equivalent_yield,
            "State-adjusted TEY ({}) should exceed federal TEY ({})",
            out.state_adjusted_tey,
            out.tax_equivalent_yield
        );
    }

    // -----------------------------------------------------------------------
    // 4. De minimis: discount below threshold -> capital gain treatment
    // -----------------------------------------------------------------------
    #[test]
    fn test_de_minimis_below_threshold() {
        let mut input = standard_muni();
        // Threshold = 5000 * 0.0025 * 10 = 125
        // Purchase at 4900 => discount = 100 < 125
        input.purchase_price = Some(dec!(4900));

        let result = price_muni_bond(&input).unwrap();
        let dm = result.result.de_minimis_analysis.unwrap();

        assert!(
            dm.is_de_minimis,
            "Discount of 100 should be within de minimis threshold of 125"
        );
        assert_eq!(dm.oid_amount, dec!(100));
        assert!(dm.tax_treatment.contains("capital gain"));
        assert_eq!(dm.threshold, dec!(125));
    }

    // -----------------------------------------------------------------------
    // 5. De minimis: discount above threshold -> ordinary income
    // -----------------------------------------------------------------------
    #[test]
    fn test_de_minimis_above_threshold() {
        let mut input = standard_muni();
        // Threshold = 5000 * 0.0025 * 10 = 125
        // Purchase at 4800 => discount = 200 > 125
        input.purchase_price = Some(dec!(4800));

        let result = price_muni_bond(&input).unwrap();
        let dm = result.result.de_minimis_analysis.unwrap();

        assert!(
            !dm.is_de_minimis,
            "Discount of 200 should exceed de minimis threshold of 125"
        );
        assert_eq!(dm.oid_amount, dec!(200));
        assert!(dm.tax_treatment.contains("ordinary income"));
    }

    // -----------------------------------------------------------------------
    // 6. Yield to call: callable muni
    // -----------------------------------------------------------------------
    #[test]
    fn test_yield_to_call() {
        let mut input = standard_muni();
        input.coupon_rate = dec!(0.05);
        input.yield_to_maturity = dec!(0.04); // trading at premium
        input.par_call_price = Some(dec!(5000));
        input.call_date_years = Some(dec!(5));
        input.call_premium = Some(dec!(0.02)); // call at 102

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        assert!(
            out.yield_to_call.is_some(),
            "YTC should be computed for callable bond"
        );
        let ytc = out.yield_to_call.unwrap();
        assert!(
            ytc > dec!(0.01) && ytc < dec!(0.15),
            "YTC should be a reasonable rate, got {}",
            ytc
        );
    }

    // -----------------------------------------------------------------------
    // 7. Yield to worst: min of YTM and YTC
    // -----------------------------------------------------------------------
    #[test]
    fn test_yield_to_worst() {
        let mut input = standard_muni();
        input.coupon_rate = dec!(0.05);
        input.yield_to_maturity = dec!(0.04);
        input.par_call_price = Some(dec!(5000));
        input.call_date_years = Some(dec!(5));
        input.call_premium = Some(dec!(0.02));

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        assert!(out.yield_to_worst.is_some(), "YTW should be computed");
        let ytw = out.yield_to_worst.unwrap();
        let ytc = out.yield_to_call.unwrap();

        let expected_min = if input.yield_to_maturity < ytc {
            input.yield_to_maturity
        } else {
            ytc
        };
        assert_eq!(
            ytw, expected_min,
            "YTW should be min(YTM={}, YTC={}), got {}",
            input.yield_to_maturity, ytc, ytw
        );
    }

    // -----------------------------------------------------------------------
    // 8. Muni/Treasury ratio calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_muni_treasury_ratio() {
        let mut input = standard_muni();
        input.yield_to_maturity = dec!(0.03);
        input.coupon_rate = dec!(0.03);
        input.comparable_treasury_yield = Some(dec!(0.04));

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        let ratio = out.muni_to_treasury_ratio.unwrap();
        // 0.03 / 0.04 = 0.75
        let expected = dec!(0.75);
        let diff = (ratio - expected).abs();
        assert!(
            diff < dec!(0.001),
            "Muni/Treasury ratio should be 0.75, got {}",
            ratio
        );
    }

    // -----------------------------------------------------------------------
    // 9. AMT-adjusted TEY for private activity bond
    // -----------------------------------------------------------------------
    #[test]
    fn test_amt_adjusted_tey() {
        let mut input = standard_muni();
        input.yield_to_maturity = dec!(0.035);
        input.coupon_rate = dec!(0.035);
        input.federal_tax_rate = dec!(0.37);
        input.is_private_activity = true;
        input.amt_rate = Some(dec!(0.28));
        input.state_tax_exempt = false;

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // AMT rate (0.28) < federal (0.37), so federal rate is used
        // TEY = 0.035 / (1 - 0.37) = 0.035 / 0.63
        let expected = dec!(0.035) / dec!(0.63);
        let diff = (out.tax_equivalent_yield - expected).abs();
        assert!(
            diff < dec!(0.001),
            "TEY with AMT < federal should use federal rate, expected ~{}, got {}",
            expected,
            out.tax_equivalent_yield
        );
    }

    // -----------------------------------------------------------------------
    // 10. AMT rate higher than federal rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_amt_rate_higher_than_federal() {
        let mut input = standard_muni();
        input.yield_to_maturity = dec!(0.035);
        input.coupon_rate = dec!(0.035);
        input.federal_tax_rate = dec!(0.24);
        input.is_private_activity = true;
        input.amt_rate = Some(dec!(0.28));
        input.state_tax_exempt = false;

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // AMT rate (0.28) > federal (0.24), so AMT rate is used
        // TEY = 0.035 / (1 - 0.28) = 0.035 / 0.72
        let expected = dec!(0.035) / dec!(0.72);
        let diff = (out.tax_equivalent_yield - expected).abs();
        assert!(
            diff < dec!(0.001),
            "TEY with AMT > federal should use AMT rate, expected ~{}, got {}",
            expected,
            out.tax_equivalent_yield
        );
    }

    // -----------------------------------------------------------------------
    // 11. Premium bond (coupon > YTM) prices above par
    // -----------------------------------------------------------------------
    #[test]
    fn test_premium_muni_bond() {
        let mut input = standard_muni();
        input.coupon_rate = dec!(0.06);
        input.yield_to_maturity = dec!(0.04);

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        assert!(
            out.clean_price > dec!(5000),
            "Premium muni (6% coupon, 4% YTM) should price above par, got {}",
            out.clean_price
        );
    }

    // -----------------------------------------------------------------------
    // 12. Discount bond (coupon < YTM) prices below par
    // -----------------------------------------------------------------------
    #[test]
    fn test_discount_muni_bond() {
        let mut input = standard_muni();
        input.coupon_rate = dec!(0.03);
        input.yield_to_maturity = dec!(0.05);

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        assert!(
            out.clean_price < dec!(5000),
            "Discount muni (3% coupon, 5% YTM) should price below par, got {}",
            out.clean_price
        );
    }

    // -----------------------------------------------------------------------
    // 13. Cashflow schedule correctness
    // -----------------------------------------------------------------------
    #[test]
    fn test_cashflow_schedule() {
        let mut input = standard_muni();
        input.maturity_years = dec!(3);
        input.coupon_frequency = 2;

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // 3 years, semi-annual = 6 periods
        assert_eq!(out.cashflow_schedule.len(), 6);

        // First 5 are coupon-only
        let coupon_per_period = dec!(5000) * dec!(0.05) / dec!(2); // 125
        for cf in &out.cashflow_schedule[..5] {
            assert_eq!(cf.cashflow_type, "coupon");
            assert_eq!(cf.amount, coupon_per_period);
        }

        // Last is coupon + principal
        let last = &out.cashflow_schedule[5];
        assert_eq!(last.cashflow_type, "coupon+principal");
        assert_eq!(last.amount, coupon_per_period + dec!(5000));
    }

    // -----------------------------------------------------------------------
    // 14. Taxable-equivalent spread in basis points
    // -----------------------------------------------------------------------
    #[test]
    fn test_taxable_equivalent_spread() {
        let mut input = standard_muni();
        input.yield_to_maturity = dec!(0.03);
        input.coupon_rate = dec!(0.03);
        input.federal_tax_rate = dec!(0.37);
        input.state_tax_rate = dec!(0.05);
        input.state_tax_exempt = true;
        input.comparable_treasury_yield = Some(dec!(0.04));

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        let spread = out.taxable_equivalent_spread.unwrap();
        // State-adj TEY > Treasury => positive spread
        assert!(
            spread > Decimal::ZERO,
            "TEY spread vs Treasury should be positive, got {}",
            spread
        );
    }

    // -----------------------------------------------------------------------
    // 15. Corporate spread pickup
    // -----------------------------------------------------------------------
    #[test]
    fn test_corporate_spread_pickup() {
        let mut input = standard_muni();
        input.yield_to_maturity = dec!(0.03);
        input.coupon_rate = dec!(0.03);
        input.federal_tax_rate = dec!(0.37);
        input.state_tax_rate = dec!(0.05);
        input.state_tax_exempt = true;
        input.comparable_corporate_yield = Some(dec!(0.045));

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // Corporate spread pickup should be present
        assert!(out.corporate_spread_pickup.is_some());
    }

    // -----------------------------------------------------------------------
    // 16. Credit spread for various ratings
    // -----------------------------------------------------------------------
    #[test]
    fn test_credit_spread_ratings() {
        assert_eq!(compute_credit_spread("AAA"), Some(dec!(0)));
        assert_eq!(compute_credit_spread("AA"), Some(dec!(30)));
        assert_eq!(compute_credit_spread("A"), Some(dec!(75)));
        assert_eq!(compute_credit_spread("BBB"), Some(dec!(150)));
        assert_eq!(compute_credit_spread("BB"), Some(dec!(300)));
        assert_eq!(compute_credit_spread("B"), Some(dec!(500)));
        assert_eq!(compute_credit_spread("CCC"), None);
    }

    // -----------------------------------------------------------------------
    // 17. Total return if held
    // -----------------------------------------------------------------------
    #[test]
    fn test_total_return_if_held() {
        let input = standard_muni();
        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // Total = annual_coupon * years + face
        // = 5000 * 0.05 * 10 + 5000 = 2500 + 5000 = 7500
        assert_eq!(out.total_return_if_held, dec!(7500));
    }

    // -----------------------------------------------------------------------
    // 18. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = standard_muni();
        let result = price_muni_bond(&input).unwrap();

        assert!(result.methodology.contains("Municipal Bond Pricing"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 19. Validation: invalid face value
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_face_value() {
        let mut input = standard_muni();
        input.face_value = dec!(-1000);

        let result = price_muni_bond(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "face_value"),
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 20. Validation: invalid coupon frequency
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_coupon_frequency() {
        let mut input = standard_muni();
        input.coupon_frequency = 3;

        let result = price_muni_bond(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "coupon_frequency"),
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 21. Validation: wrong day count
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_day_count() {
        let mut input = standard_muni();
        input.day_count = "ACT/360".to_string();

        let result = price_muni_bond(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "day_count"),
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 22. Validation: call date after maturity
    // -----------------------------------------------------------------------
    #[test]
    fn test_call_date_after_maturity() {
        let mut input = standard_muni();
        input.call_date_years = Some(dec!(15)); // maturity is 10

        let result = price_muni_bond(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => assert_eq!(field, "call_date_years"),
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 23. Dirty = Clean + Accrued
    // -----------------------------------------------------------------------
    #[test]
    fn test_dirty_equals_clean_plus_accrued() {
        let input = standard_muni();
        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        let reconstructed = out.clean_price + out.accrued_interest;
        let diff = (out.dirty_price - reconstructed).abs();
        assert!(
            diff < dec!(0.01),
            "Dirty ({}) should equal clean ({}) + accrued ({}), diff = {}",
            out.dirty_price,
            out.clean_price,
            out.accrued_interest,
            diff
        );
    }

    // -----------------------------------------------------------------------
    // 24. Out-of-state bond: no state tax benefit
    // -----------------------------------------------------------------------
    #[test]
    fn test_out_of_state_no_state_benefit() {
        let mut input = standard_muni();
        input.yield_to_maturity = dec!(0.03);
        input.coupon_rate = dec!(0.03);
        input.federal_tax_rate = dec!(0.37);
        input.state_tax_rate = dec!(0.05);
        input.state_tax_exempt = false; // out-of-state

        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // For out-of-state, state_adjusted_tey = federal TEY
        let diff = (out.state_adjusted_tey - out.tax_equivalent_yield).abs();
        assert!(
            diff < dec!(0.0001),
            "Out-of-state: state-adjusted TEY ({}) should equal federal TEY ({})",
            out.state_adjusted_tey,
            out.tax_equivalent_yield
        );
    }

    // -----------------------------------------------------------------------
    // 25. De minimis: purchase at par (no discount)
    // -----------------------------------------------------------------------
    #[test]
    fn test_de_minimis_at_par() {
        let mut input = standard_muni();
        input.purchase_price = Some(dec!(5000)); // at par

        let result = price_muni_bond(&input).unwrap();
        let dm = result.result.de_minimis_analysis.unwrap();

        assert!(!dm.is_de_minimis);
        assert_eq!(dm.oid_amount, Decimal::ZERO);
        assert!(dm.tax_treatment.contains("No market discount"));
    }

    // -----------------------------------------------------------------------
    // 26. Current yield calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_current_yield() {
        let input = standard_muni();
        let result = price_muni_bond(&input).unwrap();
        let out = &result.result;

        // At par: current yield = coupon rate
        let diff = (out.current_yield - dec!(0.05)).abs();
        assert!(
            diff < dec!(0.005),
            "Current yield at par should be ~5%, got {}",
            out.current_yield
        );
    }

    // -----------------------------------------------------------------------
    // 27. Revenue bond type accepted
    // -----------------------------------------------------------------------
    #[test]
    fn test_revenue_bond_type() {
        let mut input = standard_muni();
        input.bond_type = MuniBondType::Revenue;

        let result = price_muni_bond(&input);
        assert!(result.is_ok(), "Revenue bond type should be accepted");
    }
}
