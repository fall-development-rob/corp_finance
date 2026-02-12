//! CLO Tranche Analytics.
//!
//! Computes individual tranche-level metrics:
//! - Yield to Maturity / Yield to Call / Yield to Worst (Newton-Raphson)
//! - Weighted Average Life (WAL)
//! - Spread Duration (finite difference)
//! - Breakeven Default Rate (binary search)
//! - Equity IRR (Newton-Raphson)
//! - Cash-on-Cash Return
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output types
// ---------------------------------------------------------------------------

/// A single cash flow for a tranche.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheCashFlow {
    /// Period number (1-indexed).
    pub period: u32,
    /// Interest payment.
    pub interest: Decimal,
    /// Principal payment.
    pub principal: Decimal,
}

/// Input for tranche analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheAnalyticsInput {
    /// Tranche name/identifier.
    pub tranche_name: String,
    /// Projected cash flows.
    pub cash_flows: Vec<TrancheCashFlow>,
    /// Initial investment amount.
    pub initial_investment: Decimal,
    /// Current price as percentage of par (e.g. 99.5 = 99.5%).
    pub price: Decimal,
    /// Period at which the tranche can be called (optional, 0 = no call).
    pub call_date_period: u32,
    /// Reference rate for spread calculations.
    pub reference_rate: Decimal,
}

/// Output of tranche analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrancheAnalyticsOutput {
    /// Yield to maturity (annualized, decimal).
    pub yield_to_maturity: Decimal,
    /// Yield to call (annualized, decimal). None if no call date.
    pub yield_to_call: Option<Decimal>,
    /// Yield to worst (min of YTM and YTC).
    pub yield_to_worst: Decimal,
    /// Weighted average life (years, assuming 4 periods per year).
    pub wal: Decimal,
    /// Spread duration (sensitivity to 1bp spread change).
    pub spread_duration: Decimal,
    /// Breakeven CDR (annual) at which tranche begins taking losses.
    pub breakeven_cdr: Option<Decimal>,
    /// Equity IRR (if this is an equity tranche). None for rated tranches.
    pub equity_irr: Option<Decimal>,
    /// Cash-on-cash return (annualized). None for rated tranches.
    pub cash_on_cash: Option<Decimal>,
}

// ---------------------------------------------------------------------------
// Newton-Raphson yield solver
// ---------------------------------------------------------------------------

/// Solve for the periodic yield that equates PV of cash flows to price.
///
/// PV(y) = sum[ CF_t / (1+y)^t ] = price * par
/// We solve for y such that PV(y) - target = 0.
fn newton_yield(
    cash_flows: &[(u32, Decimal)],
    target_pv: Decimal,
    max_iter: u32,
) -> CorpFinanceResult<Decimal> {
    let mut y = dec!(0.02); // initial guess (periodic)

    for _iter in 0..max_iter {
        let mut pv = Decimal::ZERO;
        let mut dpv = Decimal::ZERO;

        for &(t, cf) in cash_flows {
            if cf.is_zero() {
                continue;
            }
            // discount factor = 1/(1+y)^t using iterative multiplication
            let mut df = Decimal::ONE;
            for _ in 0..t {
                let denom = Decimal::ONE + y;
                if denom.is_zero() {
                    return Err(CorpFinanceError::DivisionByZero {
                        context: "Yield solver: (1+y) is zero.".into(),
                    });
                }
                df /= denom;
            }
            pv += cf * df;
            // dpv/dy = -t * cf / (1+y)^(t+1)
            let denom_plus = Decimal::ONE + y;
            if denom_plus.is_zero() {
                return Err(CorpFinanceError::DivisionByZero {
                    context: "Yield solver derivative: (1+y) is zero.".into(),
                });
            }
            dpv -= Decimal::from(t) * cf * df / denom_plus;
        }

        let f_val = pv - target_pv;

        if f_val.abs() < dec!(0.0000001) {
            return Ok(y);
        }

        if dpv.is_zero() {
            break;
        }

        y -= f_val / dpv;

        // Clamp to avoid divergence
        if y < dec!(-0.5) {
            y = dec!(-0.5);
        }
        if y > dec!(2.0) {
            y = dec!(2.0);
        }
    }

    Ok(y)
}

/// Convert periodic yield to annualized (assuming 4 periods per year).
fn annualize_yield(periodic: Decimal, periods_per_year: u32) -> Decimal {
    periodic * Decimal::from(periods_per_year)
}

// ---------------------------------------------------------------------------
// Newton-Raphson IRR solver
// ---------------------------------------------------------------------------

/// Solve for IRR given cash flows (first is negative investment).
fn newton_irr(cash_flows: &[Decimal], max_iter: u32) -> CorpFinanceResult<Decimal> {
    if cash_flows.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "No cash flows for IRR.".into(),
        ));
    }

    let mut r = dec!(0.10); // initial guess (periodic)

    for _iter in 0..max_iter {
        let mut npv = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;

        for (t, cf) in cash_flows.iter().enumerate() {
            if cf.is_zero() {
                continue;
            }
            // df = 1/(1+r)^t
            let mut df = Decimal::ONE;
            for _ in 0..t {
                let denom = Decimal::ONE + r;
                if denom.is_zero() {
                    return Err(CorpFinanceError::DivisionByZero {
                        context: "IRR solver: (1+r) is zero.".into(),
                    });
                }
                df /= denom;
            }
            npv += *cf * df;
            // d(npv)/dr = -t * cf / (1+r)^(t+1)
            if t > 0 {
                let denom = Decimal::ONE + r;
                if denom.is_zero() {
                    return Err(CorpFinanceError::DivisionByZero {
                        context: "IRR solver derivative: (1+r) is zero.".into(),
                    });
                }
                dnpv -= Decimal::from(t as u32) * *cf * df / denom;
            }
        }

        if npv.abs() < dec!(0.0000001) {
            return Ok(r);
        }

        if dnpv.is_zero() {
            break;
        }

        r -= npv / dnpv;

        if r < dec!(-0.99) {
            r = dec!(-0.99);
        }
        if r > dec!(10.0) {
            r = dec!(10.0);
        }
    }

    Ok(r)
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

/// Compute tranche analytics.
pub fn calculate_tranche_analytics(
    input: &TrancheAnalyticsInput,
) -> CorpFinanceResult<TrancheAnalyticsOutput> {
    validate_tranche_input(input)?;

    let periods_per_year: u32 = 4; // quarterly

    // Build total cash flow per period (interest + principal)
    let total_cfs: Vec<(u32, Decimal)> = input
        .cash_flows
        .iter()
        .map(|cf| (cf.period, cf.interest + cf.principal))
        .collect();

    // Target PV = price/100 * initial_investment
    let target_pv = input.price / dec!(100) * input.initial_investment;

    // --- Yield to Maturity ---
    let ytm_periodic = newton_yield(&total_cfs, target_pv, 30)?;
    let yield_to_maturity = annualize_yield(ytm_periodic, periods_per_year);

    // --- Yield to Call ---
    let yield_to_call = if input.call_date_period > 0 {
        let call_cfs: Vec<(u32, Decimal)> = input
            .cash_flows
            .iter()
            .filter(|cf| cf.period <= input.call_date_period)
            .map(|cf| (cf.period, cf.interest + cf.principal))
            .collect();

        // At call, remaining par is returned
        let prin_paid_to_call: Decimal = input
            .cash_flows
            .iter()
            .filter(|cf| cf.period <= input.call_date_period)
            .map(|cf| cf.principal)
            .sum();
        let remaining_par = input.initial_investment - prin_paid_to_call;

        let mut call_cfs_with_redemption = call_cfs;
        // Add remaining par to the last call period
        if let Some(last) = call_cfs_with_redemption.last_mut() {
            if last.0 == input.call_date_period {
                last.1 += remaining_par;
            }
        } else {
            call_cfs_with_redemption.push((input.call_date_period, remaining_par));
        }

        let ytc_periodic = newton_yield(&call_cfs_with_redemption, target_pv, 30)?;
        Some(annualize_yield(ytc_periodic, periods_per_year))
    } else {
        None
    };

    // --- Yield to Worst ---
    let yield_to_worst = match yield_to_call {
        Some(ytc) => {
            if ytc < yield_to_maturity {
                ytc
            } else {
                yield_to_maturity
            }
        }
        None => yield_to_maturity,
    };

    // --- WAL ---
    let total_principal: Decimal = input.cash_flows.iter().map(|cf| cf.principal).sum();
    let wal = if total_principal.is_zero() {
        Decimal::ZERO
    } else {
        let weighted_time: Decimal = input
            .cash_flows
            .iter()
            .map(|cf| cf.principal * Decimal::from(cf.period) / Decimal::from(periods_per_year))
            .sum();
        weighted_time / total_principal
    };

    // --- Spread Duration ---
    // Finite difference: dP/ds ~ [PV(s-1bp) - PV(s+1bp)] / (2 * 1bp * par)
    let one_bp = dec!(0.0001);
    let base_rate = ytm_periodic;

    let pv_down = {
        let r = base_rate - one_bp / Decimal::from(periods_per_year);
        let mut pv = Decimal::ZERO;
        for &(t, cf) in &total_cfs {
            let mut df = Decimal::ONE;
            for _ in 0..t {
                df /= Decimal::ONE + r;
            }
            pv += cf * df;
        }
        pv
    };

    let pv_up = {
        let r = base_rate + one_bp / Decimal::from(periods_per_year);
        let mut pv = Decimal::ZERO;
        for &(t, cf) in &total_cfs {
            let mut df = Decimal::ONE;
            for _ in 0..t {
                df /= Decimal::ONE + r;
            }
            pv += cf * df;
        }
        pv
    };

    let spread_duration = if target_pv.is_zero() {
        Decimal::ZERO
    } else {
        (pv_down - pv_up) / (dec!(2) * one_bp * target_pv)
    };

    // --- Breakeven CDR ---
    // Binary search: find annual CDR where cumulative losses exhaust subordination
    // For a rated tranche, subordination = initial_investment (simplified)
    // For equity, breakeven is not meaningful
    let is_equity = input.tranche_name.to_uppercase().contains("EQUITY");

    let breakeven_cdr = if !is_equity {
        // Simplified: total_principal represents tranche par
        // Breakeven when total losses >= subordination
        // Loss in each period ~= pool * CDR_periodic * (1 - recovery)
        // We approximate: loss ~ initial_investment fraction
        // Binary search from 0% to 100% CDR
        let mut lo = Decimal::ZERO;
        let mut hi = Decimal::ONE;
        let mut result = Decimal::ONE;

        for _ in 0..30 {
            let mid = (lo + hi) / dec!(2);
            // Approximate cumulative loss ratio
            // For simplicity: if CDR_annual = mid, then over n periods
            // cumulative_default_pct ~ 1 - (1 - mid * period_frac)^n
            let period_frac = Decimal::ONE / Decimal::from(periods_per_year);
            let periodic_survival = Decimal::ONE - mid * period_frac;
            let n = input.cash_flows.len() as u32;

            let mut survival = Decimal::ONE;
            for _ in 0..n {
                survival *= periodic_survival;
                if survival < Decimal::ZERO {
                    survival = Decimal::ZERO;
                    break;
                }
            }
            let cumulative_default = Decimal::ONE - survival;
            // Assume 40% recovery
            let cumulative_loss = cumulative_default * dec!(0.60);

            // Subordination fraction (price-based)
            let sub_frac = Decimal::ONE - input.price / dec!(100);
            let sub_frac = if sub_frac < Decimal::ZERO {
                Decimal::ZERO
            } else {
                sub_frac
            };

            // Extra subordination from structure (simplified to 30% of par)
            let structural_sub = dec!(0.30);
            let total_sub = sub_frac + structural_sub;

            if cumulative_loss > total_sub {
                hi = mid;
                result = mid;
            } else {
                lo = mid;
                result = mid;
            }
        }
        Some(result)
    } else {
        None
    };

    // --- Equity IRR ---
    let equity_irr = if is_equity {
        // Build equity cash flows: negative initial, then distributions
        let mut eq_cfs: Vec<Decimal> = vec![-input.initial_investment];
        for cf in &input.cash_flows {
            eq_cfs.push(cf.interest + cf.principal);
        }
        let irr_periodic = newton_irr(&eq_cfs, 30)?;
        Some(annualize_yield(irr_periodic, periods_per_year))
    } else {
        None
    };

    // --- Cash-on-Cash ---
    let cash_on_cash = if is_equity && !input.initial_investment.is_zero() {
        let total_dist: Decimal = input
            .cash_flows
            .iter()
            .map(|cf| cf.interest + cf.principal)
            .sum();
        let num_years =
            Decimal::from(input.cash_flows.len() as u32) / Decimal::from(periods_per_year);
        if num_years.is_zero() {
            Some(Decimal::ZERO)
        } else {
            let annual_dist = total_dist / num_years;
            Some(annual_dist / input.initial_investment)
        }
    } else {
        None
    };

    Ok(TrancheAnalyticsOutput {
        yield_to_maturity,
        yield_to_call,
        yield_to_worst,
        wal,
        spread_duration,
        breakeven_cdr,
        equity_irr,
        cash_on_cash,
    })
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_tranche_input(input: &TrancheAnalyticsInput) -> CorpFinanceResult<()> {
    if input.cash_flows.is_empty() {
        return Err(CorpFinanceError::InsufficientData(
            "At least one cash flow is required.".into(),
        ));
    }
    if input.initial_investment <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "initial_investment".into(),
            reason: "Initial investment must be positive.".into(),
        });
    }
    if input.price <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "price".into(),
            reason: "Price must be positive.".into(),
        });
    }
    for cf in &input.cash_flows {
        if cf.interest < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "cash_flows.interest".into(),
                reason: "Interest cannot be negative.".into(),
            });
        }
        if cf.principal < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "cash_flows.principal".into(),
                reason: "Principal cannot be negative.".into(),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn approx_eq(a: Decimal, b: Decimal, eps: Decimal) -> bool {
        (a - b).abs() < eps
    }

    fn sample_rated_cfs() -> Vec<TrancheCashFlow> {
        // 20 quarterly periods, constant coupon, bullet principal at maturity
        let coupon = dec!(1_500_000); // quarterly coupon
        let par = dec!(100_000_000);
        let mut cfs = Vec::new();
        for p in 1..=20 {
            let principal = if p == 20 { par } else { Decimal::ZERO };
            cfs.push(TrancheCashFlow {
                period: p,
                interest: coupon,
                principal,
            });
        }
        cfs
    }

    fn sample_rated_input() -> TrancheAnalyticsInput {
        TrancheAnalyticsInput {
            tranche_name: "AAA".into(),
            cash_flows: sample_rated_cfs(),
            initial_investment: dec!(100_000_000),
            price: dec!(100),
            call_date_period: 8,
            reference_rate: dec!(0.05),
        }
    }

    fn sample_equity_cfs() -> Vec<TrancheCashFlow> {
        // 20 quarterly periods with distributions
        let mut cfs = Vec::new();
        for p in 1..=20 {
            cfs.push(TrancheCashFlow {
                period: p,
                interest: dec!(2_000_000), // residual interest
                principal: if p == 20 {
                    dec!(50_000_000)
                } else {
                    Decimal::ZERO
                },
            });
        }
        cfs
    }

    fn sample_equity_input() -> TrancheAnalyticsInput {
        TrancheAnalyticsInput {
            tranche_name: "Equity".into(),
            cash_flows: sample_equity_cfs(),
            initial_investment: dec!(50_000_000),
            price: dec!(100),
            call_date_period: 0,
            reference_rate: dec!(0.05),
        }
    }

    #[test]
    fn test_ytm_at_par_approximately_coupon_rate() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        // Coupon = 1.5M per quarter on 100M = 1.5% quarterly = 6% annual
        assert!(
            approx_eq(out.yield_to_maturity, dec!(0.06), dec!(0.005)),
            "YTM {} should be ~0.06",
            out.yield_to_maturity
        );
    }

    #[test]
    fn test_ytc_present_when_call_date_set() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(out.yield_to_call.is_some());
    }

    #[test]
    fn test_ytc_none_when_no_call() {
        let mut input = sample_rated_input();
        input.call_date_period = 0;
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(out.yield_to_call.is_none());
    }

    #[test]
    fn test_ytw_equals_min_of_ytm_ytc() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        if let Some(ytc) = out.yield_to_call {
            let expected = ytc.min(out.yield_to_maturity);
            assert_eq!(out.yield_to_worst, expected);
        }
    }

    #[test]
    fn test_ytw_equals_ytm_when_no_call() {
        let mut input = sample_rated_input();
        input.call_date_period = 0;
        let out = calculate_tranche_analytics(&input).unwrap();
        assert_eq!(out.yield_to_worst, out.yield_to_maturity);
    }

    #[test]
    fn test_wal_bullet_at_maturity() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        // All principal at period 20, WAL = 20/4 = 5.0 years
        assert!(
            approx_eq(out.wal, dec!(5.0), dec!(0.01)),
            "WAL {} should be ~5.0",
            out.wal
        );
    }

    #[test]
    fn test_wal_amortizing() {
        let mut input = sample_rated_input();
        // Equal principal in each period
        let per_period = dec!(5_000_000);
        input.cash_flows = (1..=20)
            .map(|p| TrancheCashFlow {
                period: p,
                interest: dec!(1_000_000),
                principal: per_period,
            })
            .collect();
        let out = calculate_tranche_analytics(&input).unwrap();
        // WAL should be less than 5.0 for amortizing
        assert!(
            out.wal < dec!(5.0),
            "Amortizing WAL {} should be < 5.0",
            out.wal
        );
    }

    #[test]
    fn test_spread_duration_positive() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(
            out.spread_duration > Decimal::ZERO,
            "Spread duration should be positive"
        );
    }

    #[test]
    fn test_breakeven_cdr_exists_for_rated() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(out.breakeven_cdr.is_some());
    }

    #[test]
    fn test_breakeven_cdr_none_for_equity() {
        let input = sample_equity_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(out.breakeven_cdr.is_none());
    }

    #[test]
    fn test_breakeven_cdr_in_valid_range() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        if let Some(cdr) = out.breakeven_cdr {
            assert!(cdr >= Decimal::ZERO && cdr <= Decimal::ONE);
        }
    }

    #[test]
    fn test_equity_irr_present() {
        let input = sample_equity_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(out.equity_irr.is_some());
    }

    #[test]
    fn test_equity_irr_positive() {
        let input = sample_equity_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(
            out.equity_irr.unwrap() > Decimal::ZERO,
            "Equity IRR should be positive"
        );
    }

    #[test]
    fn test_equity_irr_none_for_rated() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(out.equity_irr.is_none());
    }

    #[test]
    fn test_cash_on_cash_present_for_equity() {
        let input = sample_equity_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(out.cash_on_cash.is_some());
    }

    #[test]
    fn test_cash_on_cash_none_for_rated() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(out.cash_on_cash.is_none());
    }

    #[test]
    fn test_cash_on_cash_positive() {
        let input = sample_equity_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        assert!(
            out.cash_on_cash.unwrap() > Decimal::ZERO,
            "Cash-on-cash should be positive"
        );
    }

    #[test]
    fn test_discount_price_higher_yield() {
        let par_input = sample_rated_input();
        let par_out = calculate_tranche_analytics(&par_input).unwrap();

        let mut disc_input = sample_rated_input();
        disc_input.price = dec!(95); // discount
        let disc_out = calculate_tranche_analytics(&disc_input).unwrap();

        assert!(
            disc_out.yield_to_maturity > par_out.yield_to_maturity,
            "Discount price should give higher yield"
        );
    }

    #[test]
    fn test_reject_empty_cash_flows() {
        let mut input = sample_rated_input();
        input.cash_flows = vec![];
        assert!(calculate_tranche_analytics(&input).is_err());
    }

    #[test]
    fn test_reject_zero_investment() {
        let mut input = sample_rated_input();
        input.initial_investment = Decimal::ZERO;
        assert!(calculate_tranche_analytics(&input).is_err());
    }

    #[test]
    fn test_reject_zero_price() {
        let mut input = sample_rated_input();
        input.price = Decimal::ZERO;
        assert!(calculate_tranche_analytics(&input).is_err());
    }

    #[test]
    fn test_reject_negative_interest() {
        let mut input = sample_rated_input();
        input.cash_flows[0].interest = dec!(-100);
        assert!(calculate_tranche_analytics(&input).is_err());
    }

    #[test]
    fn test_reject_negative_principal() {
        let mut input = sample_rated_input();
        input.cash_flows[0].principal = dec!(-100);
        assert!(calculate_tranche_analytics(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = sample_rated_input();
        let out = calculate_tranche_analytics(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: TrancheAnalyticsOutput = serde_json::from_str(&json).unwrap();
    }
}
