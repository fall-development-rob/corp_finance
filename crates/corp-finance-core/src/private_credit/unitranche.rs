//! Unitranche pricing, first-out/last-out (FOLO) structuring, and blended
//! yield analysis for private credit.
//!
//! Models the economics of a single unitranche facility that is internally
//! split into a first-out (senior) tranche and a last-out (junior) tranche,
//! each with its own spread. The module computes blended yields, borrower
//! leverage metrics, covenant headroom, and yield-to-call/YTM via
//! Newton-Raphson IRR solves.

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
/// Basis points divisor
const BPS: Decimal = dec!(10000);

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

/// Full input for a unitranche pricing computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitrancheInput {
    /// Deal identifier
    pub deal_name: String,
    /// Total unitranche facility commitment
    pub total_commitment: Money,
    /// Borrower LTM EBITDA
    pub borrower_ebitda: Money,
    /// Borrower LTM revenue
    pub borrower_revenue: Money,
    /// Percentage of unitranche allocated to first-out (e.g. 0.60 = 60%)
    pub first_out_pct: Rate,
    /// Spread over base rate for first-out tranche in basis points
    pub first_out_spread_bps: Decimal,
    /// Spread over base rate for last-out tranche in basis points
    pub last_out_spread_bps: Decimal,
    /// Base rate (SOFR or equivalent), as a decimal (e.g. 0.05 = 5%)
    pub base_rate: Rate,
    /// Original issue discount as a decimal (e.g. 0.02 = 2 points)
    pub oid_pct: Rate,
    /// Upfront fee as a decimal (e.g. 0.01 = 1%)
    pub upfront_fee_pct: Rate,
    /// Commitment fee on undrawn portion in basis points
    pub commitment_fee_bps: Decimal,
    /// Percentage of facility currently drawn (0-1)
    pub drawn_pct: Rate,
    /// Maturity in years (typically 5-7)
    pub maturity_years: Decimal,
    /// Annual mandatory amortization as a decimal (e.g. 0.01 = 1%)
    pub amortization_pct: Rate,
    /// Years of call protection (typically 1-2)
    pub call_protection_years: u32,
    /// Premium if called during protection period (e.g. 0.02 = 2%)
    pub call_premium_pct: Rate,
    /// Maximum leverage ratio covenant (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leverage_covenant: Option<Decimal>,
    /// Minimum coverage ratio covenant (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_covenant: Option<Decimal>,
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

/// Complete output from a unitranche pricing computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitrancheOutput {
    /// Weighted average spread of FO and LO tranches (bps)
    pub blended_spread_bps: Decimal,
    /// Base rate + blended spread (decimal)
    pub blended_all_in_rate: Rate,
    /// First-out tranche details
    pub first_out: TrancheDetail,
    /// Last-out tranche details
    pub last_out: TrancheDetail,
    /// Borrower credit metrics
    pub borrower_metrics: BorrowerMetrics,
    /// Yield analysis for the lender
    pub yield_analysis: YieldAnalysis,
    /// Covenant analysis
    pub covenant_analysis: CovenantAnalysis,
}

/// Detail for a single tranche (first-out or last-out).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheDetail {
    /// Tranche label
    pub name: String,
    /// Tranche commitment amount
    pub commitment: Money,
    /// Spread in basis points
    pub spread_bps: Decimal,
    /// All-in coupon rate (base rate + spread)
    pub all_in_rate: Rate,
    /// Currently drawn amount
    pub drawn_amount: Money,
    /// Annual interest on drawn portion
    pub annual_interest: Money,
    /// Yield to maturity including OID and fees (IRR-based)
    pub yield_to_maturity: Rate,
}

/// Borrower leverage and coverage metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BorrowerMetrics {
    /// Total commitment / EBITDA
    pub total_leverage: Decimal,
    /// First-out commitment / EBITDA
    pub first_out_leverage: Decimal,
    /// Last-out commitment / EBITDA
    pub last_out_leverage: Decimal,
    /// EBITDA / total annual interest
    pub interest_coverage: Decimal,
    /// Total commitment / revenue
    pub debt_to_revenue: Decimal,
    /// Total annual interest + amortization
    pub annual_debt_service: Money,
}

/// Yield analysis from the lender perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YieldAnalysis {
    /// Blended coupon rate (cash yield)
    pub cash_yield: Rate,
    /// OID amortized over maturity (bps)
    pub oid_yield_pickup_bps: Decimal,
    /// Upfront fee amortized over maturity (bps)
    pub fee_yield_pickup_bps: Decimal,
    /// Commitment fee yield on undrawn portion
    pub undrawn_yield: Rate,
    /// All-in gross yield to lender
    pub gross_yield: Rate,
    /// Yield if called at year 3 with call premium
    pub yield_to_three_year_call: Rate,
}

/// Covenant analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovenantAnalysis {
    /// Covenant leverage limit minus actual leverage (positive = headroom)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leverage_headroom: Option<Decimal>,
    /// Actual coverage minus covenant minimum (positive = headroom)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_headroom: Option<Decimal>,
    /// Whether actual leverage exceeds the covenant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leverage_breach: Option<bool>,
    /// Whether actual coverage is below the covenant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coverage_breach: Option<bool>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Price a unitranche facility and compute FOLO economics, borrower metrics,
/// yield analysis, and covenant headroom.
pub fn price_unitranche(
    input: &UnitrancheInput,
) -> CorpFinanceResult<ComputationOutput<UnitrancheOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate ---
    validate_input(input)?;

    // --- Tranche sizing ---
    let fo_commitment = input.total_commitment * input.first_out_pct;
    let lo_commitment = input.total_commitment * (Decimal::ONE - input.first_out_pct);

    let fo_drawn = fo_commitment * input.drawn_pct;
    let lo_drawn = lo_commitment * input.drawn_pct;
    let total_drawn = input.total_commitment * input.drawn_pct;

    // --- Spread calculations ---
    let fo_spread_decimal = input.first_out_spread_bps / BPS;
    let lo_spread_decimal = input.last_out_spread_bps / BPS;

    let blended_spread_bps = input.first_out_pct * input.first_out_spread_bps
        + (Decimal::ONE - input.first_out_pct) * input.last_out_spread_bps;
    let blended_spread_decimal = blended_spread_bps / BPS;

    let fo_all_in_rate = input.base_rate + fo_spread_decimal;
    let lo_all_in_rate = input.base_rate + lo_spread_decimal;
    let blended_all_in_rate = input.base_rate + blended_spread_decimal;

    // --- Interest ---
    let fo_annual_interest = fo_drawn * fo_all_in_rate;
    let lo_annual_interest = lo_drawn * lo_all_in_rate;
    let total_annual_interest = fo_annual_interest + lo_annual_interest;

    // --- YTM for each tranche via Newton-Raphson ---
    let fo_ytm = compute_tranche_ytm(
        fo_commitment,
        fo_all_in_rate,
        input.oid_pct,
        input.upfront_fee_pct,
        input.maturity_years,
        &mut warnings,
        "First Out",
    );
    let lo_ytm = compute_tranche_ytm(
        lo_commitment,
        lo_all_in_rate,
        input.oid_pct,
        input.upfront_fee_pct,
        input.maturity_years,
        &mut warnings,
        "Last Out",
    );

    // --- Tranche details ---
    let first_out = TrancheDetail {
        name: "First Out".to_string(),
        commitment: fo_commitment,
        spread_bps: input.first_out_spread_bps,
        all_in_rate: fo_all_in_rate,
        drawn_amount: fo_drawn,
        annual_interest: fo_annual_interest,
        yield_to_maturity: fo_ytm,
    };

    let last_out = TrancheDetail {
        name: "Last Out".to_string(),
        commitment: lo_commitment,
        spread_bps: input.last_out_spread_bps,
        all_in_rate: lo_all_in_rate,
        drawn_amount: lo_drawn,
        annual_interest: lo_annual_interest,
        yield_to_maturity: lo_ytm,
    };

    // --- Borrower metrics ---
    let total_leverage = if input.borrower_ebitda.is_zero() {
        warnings.push("Borrower EBITDA is zero; leverage ratios undefined".into());
        Decimal::ZERO
    } else {
        input.total_commitment / input.borrower_ebitda
    };

    let first_out_leverage = if input.borrower_ebitda.is_zero() {
        Decimal::ZERO
    } else {
        fo_commitment / input.borrower_ebitda
    };

    let last_out_leverage = if input.borrower_ebitda.is_zero() {
        Decimal::ZERO
    } else {
        lo_commitment / input.borrower_ebitda
    };

    let interest_coverage = if total_annual_interest.is_zero() {
        warnings.push("Total annual interest is zero; coverage undefined".into());
        dec!(999)
    } else {
        input.borrower_ebitda / total_annual_interest
    };

    let debt_to_revenue = if input.borrower_revenue.is_zero() {
        warnings.push("Borrower revenue is zero; debt/revenue undefined".into());
        Decimal::ZERO
    } else {
        input.total_commitment / input.borrower_revenue
    };

    let annual_amortization = total_drawn * input.amortization_pct;
    let annual_debt_service = total_annual_interest + annual_amortization;

    let borrower_metrics = BorrowerMetrics {
        total_leverage,
        first_out_leverage,
        last_out_leverage,
        interest_coverage,
        debt_to_revenue,
        annual_debt_service,
    };

    // --- Yield analysis ---
    let cash_yield = blended_all_in_rate;

    // OID amortized straight-line over maturity (in bps)
    let oid_yield_pickup_bps = if input.maturity_years.is_zero() {
        Decimal::ZERO
    } else {
        (input.oid_pct / input.maturity_years) * BPS
    };

    // Upfront fee amortized straight-line over maturity (in bps)
    let fee_yield_pickup_bps = if input.maturity_years.is_zero() {
        Decimal::ZERO
    } else {
        (input.upfront_fee_pct / input.maturity_years) * BPS
    };

    // Commitment fee on undrawn portion
    let undrawn_pct = Decimal::ONE - input.drawn_pct;
    let undrawn_commitment = input.total_commitment * undrawn_pct;
    let commitment_fee_decimal = input.commitment_fee_bps / BPS;
    let undrawn_income = undrawn_commitment * commitment_fee_decimal;
    let undrawn_yield = if total_drawn.is_zero() {
        commitment_fee_decimal
    } else {
        undrawn_income / total_drawn
    };

    // Gross yield = cash yield + OID pickup + fee pickup + undrawn yield
    let gross_yield =
        cash_yield + oid_yield_pickup_bps / BPS + fee_yield_pickup_bps / BPS + undrawn_yield;

    // Yield to 3-year call with premium
    let yield_to_three_year_call = compute_yield_to_call(
        input.total_commitment,
        blended_all_in_rate,
        input.oid_pct,
        input.upfront_fee_pct,
        dec!(3),
        input.call_premium_pct,
        &mut warnings,
    );

    let yield_analysis = YieldAnalysis {
        cash_yield,
        oid_yield_pickup_bps,
        fee_yield_pickup_bps,
        undrawn_yield,
        gross_yield,
        yield_to_three_year_call,
    };

    // --- Covenant analysis ---
    let leverage_headroom = input.leverage_covenant.map(|cov| cov - total_leverage);
    let coverage_headroom = input.coverage_covenant.map(|cov| interest_coverage - cov);
    let leverage_breach = input.leverage_covenant.map(|cov| total_leverage > cov);
    let coverage_breach = input.coverage_covenant.map(|cov| interest_coverage < cov);

    let covenant_analysis = CovenantAnalysis {
        leverage_headroom,
        coverage_headroom,
        leverage_breach,
        coverage_breach,
    };

    let output = UnitrancheOutput {
        blended_spread_bps,
        blended_all_in_rate,
        first_out,
        last_out,
        borrower_metrics,
        yield_analysis,
        covenant_analysis,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Unitranche Pricing — FOLO structuring with blended yield analysis",
        &serde_json::json!({
            "deal_name": input.deal_name,
            "fo_lo_split": format!("{}/{}", input.first_out_pct * dec!(100), (Decimal::ONE - input.first_out_pct) * dec!(100)),
            "irr_method": "Newton-Raphson (50 iterations, iterative pow)",
            "oid_amortization": "straight-line over maturity",
            "fee_amortization": "straight-line over maturity",
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_input(input: &UnitrancheInput) -> CorpFinanceResult<()> {
    if input.total_commitment <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "total_commitment".into(),
            reason: "Total commitment must be positive".into(),
        });
    }
    if input.borrower_ebitda < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "borrower_ebitda".into(),
            reason: "Borrower EBITDA cannot be negative".into(),
        });
    }
    if input.borrower_revenue < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "borrower_revenue".into(),
            reason: "Borrower revenue cannot be negative".into(),
        });
    }
    if input.first_out_pct < Decimal::ZERO || input.first_out_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "first_out_pct".into(),
            reason: "First-out percentage must be between 0 and 1".into(),
        });
    }
    if input.drawn_pct < Decimal::ZERO || input.drawn_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "drawn_pct".into(),
            reason: "Drawn percentage must be between 0 and 1".into(),
        });
    }
    if input.maturity_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "maturity_years".into(),
            reason: "Maturity must be positive".into(),
        });
    }
    if input.base_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "base_rate".into(),
            reason: "Base rate cannot be negative".into(),
        });
    }
    if input.first_out_spread_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "first_out_spread_bps".into(),
            reason: "First-out spread cannot be negative".into(),
        });
    }
    if input.last_out_spread_bps < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "last_out_spread_bps".into(),
            reason: "Last-out spread cannot be negative".into(),
        });
    }
    if input.oid_pct < Decimal::ZERO || input.oid_pct >= Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "oid_pct".into(),
            reason: "OID must be >= 0 and < 1".into(),
        });
    }
    if input.amortization_pct < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "amortization_pct".into(),
            reason: "Amortization percentage cannot be negative".into(),
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Newton-Raphson IRR helpers
// ---------------------------------------------------------------------------

/// Compute yield-to-maturity for a single tranche using Newton-Raphson.
///
/// Cash flows:
/// - t=0: lender pays out `principal * (1 - OID)` net of upfront fee received
///   => net outflow = -principal * (1 - OID) + principal * upfront_fee
/// - t=1..N: lender receives `principal * coupon_rate`
/// - t=N: lender also receives `principal` (par repayment)
///
/// We solve for the rate r such that NPV(r) = 0.
fn compute_tranche_ytm(
    principal: Money,
    coupon_rate: Rate,
    oid_pct: Rate,
    upfront_fee_pct: Rate,
    maturity_years: Decimal,
    warnings: &mut Vec<String>,
    label: &str,
) -> Rate {
    let n = maturity_to_periods(maturity_years);
    if n == 0 {
        warnings.push(format!("{label} YTM: maturity rounds to zero periods"));
        return coupon_rate;
    }

    // Initial outflow: lender funds (principal * (1 - OID)) but receives upfront fee
    let net_outflow = principal * (Decimal::ONE - oid_pct) - principal * upfront_fee_pct;
    let annual_coupon = principal * coupon_rate;

    // Build cash flow vector: [cf_0, cf_1, ..., cf_n]
    let mut cfs = Vec::with_capacity(n + 1);
    cfs.push(-net_outflow); // negative = lender outflow
    for i in 1..=n {
        if i == n {
            cfs.push(annual_coupon + principal);
        } else {
            cfs.push(annual_coupon);
        }
    }

    match newton_raphson_irr(&cfs, coupon_rate) {
        Ok(r) => r,
        Err(_) => {
            warnings.push(format!("{label} YTM: Newton-Raphson did not converge"));
            coupon_rate
        }
    }
}

/// Compute yield-to-call: solve for IRR of cash flows assuming the loan is
/// called at the given year with a call premium.
///
/// Cash flows:
/// - t=0: -principal * (1 - OID) + principal * upfront_fee
/// - t=1..call_year: principal * coupon_rate
/// - t=call_year: also receives principal * (1 + call_premium)
fn compute_yield_to_call(
    principal: Money,
    coupon_rate: Rate,
    oid_pct: Rate,
    upfront_fee_pct: Rate,
    call_year: Decimal,
    call_premium_pct: Rate,
    warnings: &mut Vec<String>,
) -> Rate {
    let n = maturity_to_periods(call_year);
    if n == 0 {
        warnings.push("Yield to call: call year rounds to zero periods".into());
        return coupon_rate;
    }

    let net_outflow = principal * (Decimal::ONE - oid_pct) - principal * upfront_fee_pct;
    let annual_coupon = principal * coupon_rate;
    let call_repayment = principal * (Decimal::ONE + call_premium_pct);

    let mut cfs = Vec::with_capacity(n + 1);
    cfs.push(-net_outflow);
    for i in 1..=n {
        if i == n {
            cfs.push(annual_coupon + call_repayment);
        } else {
            cfs.push(annual_coupon);
        }
    }

    match newton_raphson_irr(&cfs, coupon_rate) {
        Ok(r) => r,
        Err(_) => {
            warnings.push("Yield to call: Newton-Raphson did not converge".into());
            coupon_rate
        }
    }
}

/// Newton-Raphson IRR solver using iterative multiplication for discount
/// factors (no `powd`).
fn newton_raphson_irr(cash_flows: &[Decimal], guess: Rate) -> CorpFinanceResult<Rate> {
    if cash_flows.len() < 2 {
        return Err(CorpFinanceError::InsufficientData(
            "IRR requires at least 2 cash flows".into(),
        ));
    }

    let mut rate = guess;

    for iteration in 0..NEWTON_MAX_ITERATIONS {
        let one_plus_r = Decimal::ONE + rate;
        if one_plus_r.is_zero() {
            rate += dec!(0.01);
            continue;
        }

        let mut npv_val = Decimal::ZERO;
        let mut dnpv_val = Decimal::ZERO;
        // Iterative discount factor: discount_factor = (1+r)^t built up by
        // repeated multiplication.
        let mut discount_factor = Decimal::ONE;

        for (t, cf) in cash_flows.iter().enumerate() {
            if t > 0 {
                discount_factor *= one_plus_r;
            }
            if discount_factor.is_zero() {
                continue;
            }
            npv_val += cf / discount_factor;
            if t > 0 {
                let t_dec = Decimal::from(t as u32);
                // Derivative: d/dr[ cf / (1+r)^t ] = -t * cf / (1+r)^(t+1)
                dnpv_val -= t_dec * cf / (discount_factor * one_plus_r);
            }
        }

        if npv_val.abs() < NEWTON_EPSILON {
            return Ok(rate);
        }

        if dnpv_val.is_zero() {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "unitranche_irr".into(),
                iterations: iteration,
                last_delta: npv_val,
            });
        }

        rate -= npv_val / dnpv_val;

        // Guard against divergence
        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        } else if rate > dec!(10.0) {
            rate = dec!(10.0);
        }
    }

    Err(CorpFinanceError::ConvergenceFailure {
        function: "unitranche_irr".into(),
        iterations: NEWTON_MAX_ITERATIONS,
        last_delta: Decimal::ZERO,
    })
}

/// Convert a Decimal maturity in years to a whole number of annual periods.
fn maturity_to_periods(maturity: Decimal) -> usize {
    // Round to nearest integer; floor for safety
    let rounded = maturity.round_dp(0).to_string().parse::<i64>().unwrap_or(0);
    if rounded < 0 {
        0
    } else {
        rounded as usize
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper: standard 60/40 FO/LO unitranche for testing.
    fn standard_unitranche() -> UnitrancheInput {
        UnitrancheInput {
            deal_name: "Test Unitranche".to_string(),
            total_commitment: dec!(100_000_000),
            borrower_ebitda: dec!(25_000_000),
            borrower_revenue: dec!(150_000_000),
            first_out_pct: dec!(0.60),
            first_out_spread_bps: dec!(250),
            last_out_spread_bps: dec!(650),
            base_rate: dec!(0.05),
            oid_pct: dec!(0.02),
            upfront_fee_pct: dec!(0.01),
            commitment_fee_bps: dec!(50),
            drawn_pct: dec!(1.0),
            maturity_years: dec!(5),
            amortization_pct: dec!(0.01),
            call_protection_years: 2,
            call_premium_pct: dec!(0.02),
            leverage_covenant: Some(dec!(5.0)),
            coverage_covenant: Some(dec!(2.0)),
        }
    }

    // -----------------------------------------------------------------------
    // 1. Blended spread: 60% * 250 + 40% * 650 = 150 + 260 = 410
    // -----------------------------------------------------------------------
    #[test]
    fn test_blended_spread_calculation() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let out = &result.result;

        // 0.60 * 250 + 0.40 * 650 = 150 + 260 = 410 bps
        assert_eq!(
            out.blended_spread_bps,
            dec!(410),
            "Blended spread should be 410 bps, got {}",
            out.blended_spread_bps
        );
    }

    // -----------------------------------------------------------------------
    // 2. All-in rate = base_rate + blended_spread
    // -----------------------------------------------------------------------
    #[test]
    fn test_blended_all_in_rate() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let out = &result.result;

        // base_rate = 0.05, blended spread = 410 bps = 0.041
        // all-in = 0.05 + 0.041 = 0.091
        let expected = dec!(0.05) + dec!(410) / dec!(10000);
        assert_eq!(
            out.blended_all_in_rate, expected,
            "Blended all-in rate should be {}, got {}",
            expected, out.blended_all_in_rate
        );
    }

    // -----------------------------------------------------------------------
    // 3. First-out tranche details
    // -----------------------------------------------------------------------
    #[test]
    fn test_first_out_tranche_details() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let fo = &result.result.first_out;

        assert_eq!(fo.name, "First Out");
        // Commitment = 100M * 0.60 = 60M
        assert_eq!(fo.commitment, dec!(60_000_000));
        assert_eq!(fo.spread_bps, dec!(250));
        // All-in = 0.05 + 0.025 = 0.075
        assert_eq!(fo.all_in_rate, dec!(0.075));
        // Fully drawn
        assert_eq!(fo.drawn_amount, dec!(60_000_000));
        // Annual interest = 60M * 0.075 = 4.5M
        assert_eq!(fo.annual_interest, dec!(4_500_000));
    }

    // -----------------------------------------------------------------------
    // 4. Last-out tranche details
    // -----------------------------------------------------------------------
    #[test]
    fn test_last_out_tranche_details() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let lo = &result.result.last_out;

        assert_eq!(lo.name, "Last Out");
        assert_eq!(lo.commitment, dec!(40_000_000));
        assert_eq!(lo.spread_bps, dec!(650));
        // All-in = 0.05 + 0.065 = 0.115
        assert_eq!(lo.all_in_rate, dec!(0.115));
        assert_eq!(lo.drawn_amount, dec!(40_000_000));
        // Annual interest = 40M * 0.115 = 4.6M
        assert_eq!(lo.annual_interest, dec!(4_600_000));
    }

    // -----------------------------------------------------------------------
    // 5. OID yield pickup calculation
    // -----------------------------------------------------------------------
    #[test]
    fn test_oid_yield_pickup() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let ya = &result.result.yield_analysis;

        // OID = 2% over 5 years = 0.4% per year = 40 bps
        assert_eq!(
            ya.oid_yield_pickup_bps,
            dec!(40),
            "OID yield pickup should be 40 bps, got {}",
            ya.oid_yield_pickup_bps
        );
    }

    // -----------------------------------------------------------------------
    // 6. Fee yield pickup
    // -----------------------------------------------------------------------
    #[test]
    fn test_fee_yield_pickup() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let ya = &result.result.yield_analysis;

        // Upfront fee = 1% over 5 years = 0.2% per year = 20 bps
        assert_eq!(
            ya.fee_yield_pickup_bps,
            dec!(20),
            "Fee yield pickup should be 20 bps, got {}",
            ya.fee_yield_pickup_bps
        );
    }

    // -----------------------------------------------------------------------
    // 7. Gross yield (all-in to lender)
    // -----------------------------------------------------------------------
    #[test]
    fn test_gross_yield() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let ya = &result.result.yield_analysis;

        // cash_yield = 0.091
        // oid pickup = 40 bps = 0.004
        // fee pickup = 20 bps = 0.002
        // undrawn_yield: fully drawn => undrawn = 0, so undrawn_yield = 0
        // gross = 0.091 + 0.004 + 0.002 + 0 = 0.097
        let expected = dec!(0.091) + dec!(0.004) + dec!(0.002);
        let diff = (ya.gross_yield - expected).abs();
        assert!(
            diff < dec!(0.0001),
            "Gross yield should be ~{}, got {}",
            expected,
            ya.gross_yield
        );
    }

    // -----------------------------------------------------------------------
    // 8. Borrower total leverage
    // -----------------------------------------------------------------------
    #[test]
    fn test_borrower_leverage() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let bm = &result.result.borrower_metrics;

        // 100M / 25M = 4.0x
        assert_eq!(
            bm.total_leverage,
            dec!(4),
            "Total leverage should be 4.0x, got {}",
            bm.total_leverage
        );
        // FO: 60M / 25M = 2.4x
        assert_eq!(bm.first_out_leverage, dec!(2.4));
        // LO: 40M / 25M = 1.6x
        assert_eq!(bm.last_out_leverage, dec!(1.6));
    }

    // -----------------------------------------------------------------------
    // 9. Interest coverage ratio
    // -----------------------------------------------------------------------
    #[test]
    fn test_interest_coverage() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let bm = &result.result.borrower_metrics;

        // Total interest = 4.5M + 4.6M = 9.1M
        // Coverage = 25M / 9.1M ~ 2.747...
        let total_interest = dec!(4_500_000) + dec!(4_600_000);
        let expected = dec!(25_000_000) / total_interest;
        let diff = (bm.interest_coverage - expected).abs();
        assert!(
            diff < dec!(0.001),
            "Interest coverage should be ~{}, got {}",
            expected,
            bm.interest_coverage
        );
    }

    // -----------------------------------------------------------------------
    // 10. Covenant headroom — passing
    // -----------------------------------------------------------------------
    #[test]
    fn test_covenant_headroom_passing() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let ca = &result.result.covenant_analysis;

        // Leverage covenant = 5.0x, actual = 4.0x => headroom = 1.0
        let lev_headroom = ca.leverage_headroom.unwrap();
        assert_eq!(
            lev_headroom,
            dec!(1),
            "Leverage headroom should be 1.0x, got {}",
            lev_headroom
        );
        assert_eq!(ca.leverage_breach, Some(false));

        // Coverage covenant = 2.0x, actual ~ 2.747 => headroom > 0
        let cov_headroom = ca.coverage_headroom.unwrap();
        assert!(
            cov_headroom > Decimal::ZERO,
            "Coverage headroom should be positive, got {}",
            cov_headroom
        );
        assert_eq!(ca.coverage_breach, Some(false));
    }

    // -----------------------------------------------------------------------
    // 11. Covenant headroom — failing
    // -----------------------------------------------------------------------
    #[test]
    fn test_covenant_breach() {
        let mut input = standard_unitranche();
        // Set tight covenant that will be breached
        input.leverage_covenant = Some(dec!(3.5)); // actual is 4.0x
        input.coverage_covenant = Some(dec!(5.0)); // actual is ~2.75x

        let result = price_unitranche(&input).unwrap();
        let ca = &result.result.covenant_analysis;

        assert_eq!(
            ca.leverage_breach,
            Some(true),
            "Leverage should be breached"
        );
        assert_eq!(
            ca.coverage_breach,
            Some(true),
            "Coverage should be breached"
        );

        let lev_headroom = ca.leverage_headroom.unwrap();
        assert!(
            lev_headroom < Decimal::ZERO,
            "Leverage headroom should be negative, got {}",
            lev_headroom
        );
    }

    // -----------------------------------------------------------------------
    // 12. Yield to 3-year call with premium
    // -----------------------------------------------------------------------
    #[test]
    fn test_yield_to_three_year_call() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let ya = &result.result.yield_analysis;

        // The YTC should be higher than the cash coupon because of OID + premium
        assert!(
            ya.yield_to_three_year_call > ya.cash_yield,
            "YTC ({}) should exceed cash yield ({})",
            ya.yield_to_three_year_call,
            ya.cash_yield
        );

        // Should be a reasonable rate (between 5% and 25%)
        assert!(
            ya.yield_to_three_year_call > dec!(0.05) && ya.yield_to_three_year_call < dec!(0.25),
            "YTC should be between 5% and 25%, got {}",
            ya.yield_to_three_year_call
        );
    }

    // -----------------------------------------------------------------------
    // 13. YTM for each tranche
    // -----------------------------------------------------------------------
    #[test]
    fn test_tranche_ytm() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let fo = &result.result.first_out;
        let lo = &result.result.last_out;

        // FO YTM should be higher than FO coupon rate (due to OID + fees)
        assert!(
            fo.yield_to_maturity > fo.all_in_rate,
            "FO YTM ({}) should exceed FO all-in rate ({})",
            fo.yield_to_maturity,
            fo.all_in_rate
        );

        // LO YTM should be higher than LO coupon rate
        assert!(
            lo.yield_to_maturity > lo.all_in_rate,
            "LO YTM ({}) should exceed LO all-in rate ({})",
            lo.yield_to_maturity,
            lo.all_in_rate
        );

        // LO YTM should be higher than FO YTM (higher spread tranche)
        assert!(
            lo.yield_to_maturity > fo.yield_to_maturity,
            "LO YTM ({}) should exceed FO YTM ({})",
            lo.yield_to_maturity,
            fo.yield_to_maturity
        );
    }

    // -----------------------------------------------------------------------
    // 14. Fully drawn vs partially drawn
    // -----------------------------------------------------------------------
    #[test]
    fn test_partially_drawn() {
        let mut input = standard_unitranche();
        input.drawn_pct = dec!(0.70); // 70% drawn

        let result = price_unitranche(&input).unwrap();
        let out = &result.result;

        // FO drawn = 60M * 0.70 = 42M
        assert_eq!(out.first_out.drawn_amount, dec!(42_000_000));
        // LO drawn = 40M * 0.70 = 28M
        assert_eq!(out.last_out.drawn_amount, dec!(28_000_000));

        // Undrawn yield should be positive
        assert!(
            out.yield_analysis.undrawn_yield > Decimal::ZERO,
            "Undrawn yield should be positive when partially drawn, got {}",
            out.yield_analysis.undrawn_yield
        );
    }

    // -----------------------------------------------------------------------
    // 15. Commitment fee on undrawn portion
    // -----------------------------------------------------------------------
    #[test]
    fn test_commitment_fee_undrawn() {
        let mut input = standard_unitranche();
        input.drawn_pct = dec!(0.50); // 50% drawn

        let result = price_unitranche(&input).unwrap();
        let ya = &result.result.yield_analysis;

        // Undrawn = 100M * 0.50 = 50M
        // Commitment fee = 50 bps = 0.005
        // Undrawn income = 50M * 0.005 = 250k
        // Drawn = 50M
        // Undrawn yield = 250k / 50M = 0.005
        let expected_undrawn_yield = dec!(0.005);
        let diff = (ya.undrawn_yield - expected_undrawn_yield).abs();
        assert!(
            diff < dec!(0.0001),
            "Undrawn yield should be ~0.005, got {}",
            ya.undrawn_yield
        );
    }

    // -----------------------------------------------------------------------
    // 16. Annual debt service with amortization
    // -----------------------------------------------------------------------
    #[test]
    fn test_annual_debt_service() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let bm = &result.result.borrower_metrics;

        // Total drawn = 100M (fully drawn)
        // Total interest = 9.1M (4.5M FO + 4.6M LO)
        // Amortization = 100M * 0.01 = 1M
        // Debt service = 9.1M + 1M = 10.1M
        let expected_interest = dec!(4_500_000) + dec!(4_600_000);
        let expected_amort = dec!(100_000_000) * dec!(0.01);
        let expected = expected_interest + expected_amort;
        assert_eq!(
            bm.annual_debt_service, expected,
            "Annual debt service should be {}, got {}",
            expected, bm.annual_debt_service
        );
    }

    // -----------------------------------------------------------------------
    // 17. Edge case: 100% first-out (no last-out)
    // -----------------------------------------------------------------------
    #[test]
    fn test_100_percent_first_out() {
        let mut input = standard_unitranche();
        input.first_out_pct = dec!(1.0);

        let result = price_unitranche(&input).unwrap();
        let out = &result.result;

        // LO commitment should be zero
        assert_eq!(out.last_out.commitment, Decimal::ZERO);
        assert_eq!(out.last_out.drawn_amount, Decimal::ZERO);
        // Blended spread = 100% * 250 = 250 bps
        assert_eq!(out.blended_spread_bps, dec!(250));
        // FO commitment = full 100M
        assert_eq!(out.first_out.commitment, dec!(100_000_000));
    }

    // -----------------------------------------------------------------------
    // 18. Edge case: 0% first-out (all last-out)
    // -----------------------------------------------------------------------
    #[test]
    fn test_0_percent_first_out() {
        let mut input = standard_unitranche();
        input.first_out_pct = dec!(0.0);

        let result = price_unitranche(&input).unwrap();
        let out = &result.result;

        // FO commitment should be zero
        assert_eq!(out.first_out.commitment, Decimal::ZERO);
        assert_eq!(out.first_out.drawn_amount, Decimal::ZERO);
        // Blended spread = 100% * 650 = 650 bps
        assert_eq!(out.blended_spread_bps, dec!(650));
        // LO commitment = full 100M
        assert_eq!(out.last_out.commitment, dec!(100_000_000));
    }

    // -----------------------------------------------------------------------
    // 19. High leverage deal (>6x)
    // -----------------------------------------------------------------------
    #[test]
    fn test_high_leverage_deal() {
        let mut input = standard_unitranche();
        input.borrower_ebitda = dec!(15_000_000); // 100M / 15M = 6.67x
        input.leverage_covenant = Some(dec!(7.0));

        let result = price_unitranche(&input).unwrap();
        let bm = &result.result.borrower_metrics;
        let ca = &result.result.covenant_analysis;

        // Leverage > 6x
        assert!(
            bm.total_leverage > dec!(6),
            "Leverage should be >6x, got {}",
            bm.total_leverage
        );

        // Still within 7.0x covenant
        assert_eq!(ca.leverage_breach, Some(false));
        let headroom = ca.leverage_headroom.unwrap();
        assert!(headroom > Decimal::ZERO && headroom < dec!(1.0));
    }

    // -----------------------------------------------------------------------
    // 20. Zero OID deal
    // -----------------------------------------------------------------------
    #[test]
    fn test_zero_oid_deal() {
        let mut input = standard_unitranche();
        input.oid_pct = dec!(0.0);
        input.upfront_fee_pct = dec!(0.0);

        let result = price_unitranche(&input).unwrap();
        let ya = &result.result.yield_analysis;

        assert_eq!(ya.oid_yield_pickup_bps, Decimal::ZERO);
        assert_eq!(ya.fee_yield_pickup_bps, Decimal::ZERO);

        // With no OID/fees and fully drawn, gross yield should equal cash yield
        let diff = (ya.gross_yield - ya.cash_yield).abs();
        assert!(
            diff < dec!(0.0001),
            "With zero OID/fees and fully drawn, gross yield ({}) should equal cash yield ({})",
            ya.gross_yield,
            ya.cash_yield
        );

        // YTM should be approximately equal to the coupon rate
        let fo = &result.result.first_out;
        let diff_ytm = (fo.yield_to_maturity - fo.all_in_rate).abs();
        assert!(
            diff_ytm < dec!(0.001),
            "With zero OID, FO YTM ({}) should ~= FO all-in rate ({})",
            fo.yield_to_maturity,
            fo.all_in_rate
        );
    }

    // -----------------------------------------------------------------------
    // 21. Call protection at year 1
    // -----------------------------------------------------------------------
    #[test]
    fn test_call_protection_year_1() {
        let mut input = standard_unitranche();
        input.call_protection_years = 1;
        input.call_premium_pct = dec!(0.03); // 3% premium

        let result = price_unitranche(&input).unwrap();
        let out = &result.result;

        // Call protection metadata is captured; yield-to-call uses year 3 by
        // default in our analysis. This test verifies the deal prices correctly
        // with alternative call protection parameters.
        assert_eq!(out.first_out.name, "First Out");
        assert!(out.yield_analysis.yield_to_three_year_call > dec!(0.05));
    }

    // -----------------------------------------------------------------------
    // 22. Debt to revenue ratio
    // -----------------------------------------------------------------------
    #[test]
    fn test_debt_to_revenue() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let bm = &result.result.borrower_metrics;

        // 100M / 150M = 0.6667
        let expected = dec!(100_000_000) / dec!(150_000_000);
        let diff = (bm.debt_to_revenue - expected).abs();
        assert!(
            diff < dec!(0.0001),
            "Debt/revenue should be ~{}, got {}",
            expected,
            bm.debt_to_revenue
        );
    }

    // -----------------------------------------------------------------------
    // 23. Validation: negative commitment rejected
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_negative_commitment() {
        let mut input = standard_unitranche();
        input.total_commitment = dec!(-1000);

        let result = price_unitranche(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "total_commitment");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 24. Validation: drawn_pct out of range
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_drawn_pct() {
        let mut input = standard_unitranche();
        input.drawn_pct = dec!(1.5);

        let result = price_unitranche(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "drawn_pct");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 25. Metadata populated
    // -----------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert!(result.methodology.contains("Unitranche"));
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // -----------------------------------------------------------------------
    // 26. No covenants provided
    // -----------------------------------------------------------------------
    #[test]
    fn test_no_covenants() {
        let mut input = standard_unitranche();
        input.leverage_covenant = None;
        input.coverage_covenant = None;

        let result = price_unitranche(&input).unwrap();
        let ca = &result.result.covenant_analysis;

        assert!(ca.leverage_headroom.is_none());
        assert!(ca.coverage_headroom.is_none());
        assert!(ca.leverage_breach.is_none());
        assert!(ca.coverage_breach.is_none());
    }

    // -----------------------------------------------------------------------
    // 27. Cash yield equals blended all-in rate
    // -----------------------------------------------------------------------
    #[test]
    fn test_cash_yield_equals_blended_rate() {
        let input = standard_unitranche();
        let result = price_unitranche(&input).unwrap();
        let out = &result.result;

        assert_eq!(
            out.yield_analysis.cash_yield, out.blended_all_in_rate,
            "Cash yield should equal blended all-in rate"
        );
    }

    // -----------------------------------------------------------------------
    // 28. Validation: maturity must be positive
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_zero_maturity() {
        let mut input = standard_unitranche();
        input.maturity_years = Decimal::ZERO;

        let result = price_unitranche(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "maturity_years");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // 29. Different FOLO split changes economics
    // -----------------------------------------------------------------------
    #[test]
    fn test_different_folo_split() {
        let mut input_60_40 = standard_unitranche();
        input_60_40.first_out_pct = dec!(0.60);

        let mut input_80_20 = standard_unitranche();
        input_80_20.first_out_pct = dec!(0.80);

        let result_60_40 = price_unitranche(&input_60_40).unwrap();
        let result_80_20 = price_unitranche(&input_80_20).unwrap();

        // 80/20 split should have lower blended spread (more first-out at lower spread)
        assert!(
            result_80_20.result.blended_spread_bps < result_60_40.result.blended_spread_bps,
            "80/20 blended spread ({}) should be less than 60/40 ({})",
            result_80_20.result.blended_spread_bps,
            result_60_40.result.blended_spread_bps
        );

        // 80/20: 0.80*250 + 0.20*650 = 200 + 130 = 330
        assert_eq!(result_80_20.result.blended_spread_bps, dec!(330));
    }

    // -----------------------------------------------------------------------
    // 30. Validation: OID at boundary (0 is OK, 1 is not)
    // -----------------------------------------------------------------------
    #[test]
    fn test_invalid_oid_at_100_percent() {
        let mut input = standard_unitranche();
        input.oid_pct = dec!(1.0); // 100% OID makes no sense

        let result = price_unitranche(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "oid_pct");
            }
            other => panic!("Expected InvalidInput, got {:?}", other),
        }
    }
}
