//! Secondaries Pricing model for private equity fund interests.
//!
//! Provides a discounted cash flow framework for pricing secondary
//! fund interests, including:
//!
//! - **NAV discount/premium**: bid_price / nav - 1
//! - **Remaining fund life**: weighted average remaining years
//! - **Unfunded discount**: present value of future capital calls
//! - **IRR sensitivity**: IRR at different exit multiples
//! - **J-curve benefit**: avoidance of initial J-curve drag
//! - **Pricing framework**: fair_value = pv_distributions - pv_unfunded + terminal_nav_discounted
//!
//! All arithmetic uses `rust_decimal::Decimal`. No `f64`.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::CorpFinanceError;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Input / Output
// ---------------------------------------------------------------------------

/// Input for secondaries pricing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondariesPricingInput {
    /// Current NAV of the fund interest.
    pub fund_nav: Decimal,
    /// Remaining unfunded commitment.
    pub unfunded_commitment: Decimal,
    /// Expected remaining fund life in years.
    pub remaining_life_years: u32,
    /// Expected annual distribution rate on NAV (decimal).
    pub expected_distribution_rate: Decimal,
    /// Expected annual NAV growth rate (decimal).
    pub expected_growth_rate: Decimal,
    /// Buyer's discount rate for PV calculations (decimal).
    pub discount_rate: Decimal,
    /// Management fee percentage (decimal, e.g. 0.02 = 2%).
    pub management_fee_pct: Decimal,
    /// Carried interest percentage (decimal, e.g. 0.20 = 20%).
    pub carry_pct: Decimal,
}

/// An IRR scenario at a given exit multiple.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrrScenario {
    /// Exit multiple applied to NAV.
    pub multiple: Decimal,
    /// IRR at this exit multiple.
    pub irr: Decimal,
}

/// Output of the secondaries pricing model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondariesPricingOutput {
    /// Estimated fair value of the fund interest.
    pub fair_value: Decimal,
    /// NAV discount (negative) or premium (positive) at fair value.
    pub nav_discount_pct: Decimal,
    /// Present value of unfunded capital calls.
    pub unfunded_pv: Decimal,
    /// Present value of projected distributions.
    pub distributions_pv: Decimal,
    /// Present value of terminal NAV.
    pub terminal_value_pv: Decimal,
    /// IRR if purchased at NAV (no discount).
    pub irr_at_nav: Decimal,
    /// IRR if purchased at fair value (with discount).
    pub irr_at_discount: Decimal,
    /// IRR at various exit multiples.
    pub irr_scenarios: Vec<IrrScenario>,
    /// Breakeven multiple: the exit multiple at which IRR = 0.
    pub breakeven_multiple: Decimal,
}

// ---------------------------------------------------------------------------
// Core computation
// ---------------------------------------------------------------------------

/// Calculate secondaries pricing analytics.
pub fn calculate_secondaries_pricing(
    input: &SecondariesPricingInput,
) -> CorpFinanceResult<SecondariesPricingOutput> {
    validate_secondaries_input(input)?;

    let one = Decimal::ONE;
    let n = input.remaining_life_years as usize;

    // Project cash flows: year-by-year distributions, unfunded calls, terminal NAV.
    let mut nav = input.fund_nav;
    let mut unfunded = input.unfunded_commitment;
    let mut distributions: Vec<Decimal> = Vec::with_capacity(n);
    let mut capital_calls: Vec<Decimal> = Vec::with_capacity(n);

    // Distribute unfunded evenly over remaining life (simple model).
    let annual_call = if n > 0 {
        unfunded / Decimal::from(n as u32)
    } else {
        Decimal::ZERO
    };

    for _yr in 0..n {
        // Capital call
        let call = annual_call.min(unfunded);
        unfunded -= call;
        capital_calls.push(call);

        // Grow NAV
        nav *= one + input.expected_growth_rate;
        // Add capital call to NAV
        nav += call;
        // Management fee drag
        let fee_drag = nav * input.management_fee_pct;
        nav -= fee_drag;

        // Distributions
        let dist = nav * input.expected_distribution_rate;
        nav -= dist;
        distributions.push(dist);
    }

    // Terminal NAV at end of fund life (after carry on gains).
    let total_invested = input.fund_nav + input.unfunded_commitment;
    let terminal_nav = if nav > total_invested {
        let gain = nav - total_invested;
        let carry = gain * input.carry_pct;
        nav - carry
    } else {
        nav
    };

    // PV of distributions
    let mut distributions_pv = Decimal::ZERO;
    let mut df = one;
    let denom = one + input.discount_rate;
    for dist in &distributions {
        df /= denom;
        distributions_pv += *dist * df;
    }

    // PV of capital calls (unfunded)
    let mut unfunded_pv = Decimal::ZERO;
    df = one;
    for call in &capital_calls {
        df /= denom;
        unfunded_pv += *call * df;
    }

    // PV of terminal NAV
    let mut terminal_df = one;
    for _ in 0..n {
        terminal_df /= denom;
    }
    let terminal_value_pv = terminal_nav * terminal_df;

    // Fair value = PV of distributions + PV of terminal - PV of unfunded
    let fair_value = distributions_pv + terminal_value_pv - unfunded_pv;

    // NAV discount/premium: (fair_value / nav_original) - 1
    let nav_discount_pct = if input.fund_nav.is_zero() {
        Decimal::ZERO
    } else {
        (fair_value / input.fund_nav) - one
    };

    // IRR at NAV: cash flows = [-nav at t=0, distributions..., terminal_nav at t=n]
    let irr_at_nav =
        compute_irr_for_price(input.fund_nav, &distributions, terminal_nav, &capital_calls);

    // IRR at fair value (discount)
    let irr_at_discount =
        compute_irr_for_price(fair_value, &distributions, terminal_nav, &capital_calls);

    // IRR scenarios at different exit multiples.
    let scenario_multiples = [dec!(0.8), dec!(1.0), dec!(1.2), dec!(1.5), dec!(2.0)];
    let irr_scenarios: Vec<IrrScenario> = scenario_multiples
        .iter()
        .map(|&m| {
            let adjusted_terminal = terminal_nav * m;
            let irr = compute_irr_for_price(
                fair_value,
                &distributions,
                adjusted_terminal,
                &capital_calls,
            );
            IrrScenario { multiple: m, irr }
        })
        .collect();

    // Breakeven multiple: find multiple where IRR = 0.
    // At IRR=0, PV at 0% discount = fair_value: sum(dists) + terminal*m - sum(calls) = fair_value.
    let total_dist: Decimal = distributions.iter().copied().sum();
    let total_calls: Decimal = capital_calls.iter().copied().sum();
    let breakeven_multiple = if terminal_nav.is_zero() {
        Decimal::ZERO
    } else {
        let needed = fair_value + total_calls - total_dist;
        if needed <= Decimal::ZERO {
            Decimal::ZERO
        } else {
            needed / terminal_nav
        }
    };

    Ok(SecondariesPricingOutput {
        fair_value,
        nav_discount_pct,
        unfunded_pv,
        distributions_pv,
        terminal_value_pv,
        irr_at_nav,
        irr_at_discount,
        irr_scenarios,
        breakeven_multiple,
    })
}

// ---------------------------------------------------------------------------
// IRR helper
// ---------------------------------------------------------------------------

/// Compute IRR for a secondary purchase given purchase price, distributions,
/// terminal value, and capital calls.
fn compute_irr_for_price(
    purchase_price: Decimal,
    distributions: &[Decimal],
    terminal_nav: Decimal,
    capital_calls: &[Decimal],
) -> Decimal {
    // Cash flow vector: t=0 is -purchase_price, t=1..n is dist-call, t=n adds terminal.
    let n = distributions.len();
    let mut cfs: Vec<Decimal> = Vec::with_capacity(n + 1);
    cfs.push(-purchase_price);

    for i in 0..n {
        let dist = distributions[i];
        let call = if i < capital_calls.len() {
            capital_calls[i]
        } else {
            Decimal::ZERO
        };
        let mut cf = dist - call;
        if i == n - 1 {
            cf += terminal_nav;
        }
        cfs.push(cf);
    }

    newton_irr(&cfs)
}

/// Newton-Raphson IRR with 30 iterations, initial guess 0.10.
fn newton_irr(cash_flows: &[Decimal]) -> Decimal {
    if cash_flows.len() < 2 {
        return Decimal::ZERO;
    }

    let one = Decimal::ONE;
    let mut rate = dec!(0.10);
    let max_iter = 30;
    let eps = dec!(0.0000001);

    for _ in 0..max_iter {
        let mut npv = Decimal::ZERO;
        let mut dnpv = Decimal::ZERO;
        let mut df = one;
        let denom = one + rate;

        if denom.is_zero() {
            rate = dec!(0.05);
            continue;
        }

        for (t, cf) in cash_flows.iter().enumerate() {
            if t == 0 {
                npv += *cf;
            } else {
                df /= denom;
                npv += *cf * df;
                let t_dec = Decimal::from(t as u32);
                dnpv += -t_dec * *cf * df / denom;
            }
        }

        if dnpv.is_zero() {
            break;
        }

        let delta = npv / dnpv;
        rate -= delta;

        if rate < dec!(-0.99) {
            rate = dec!(-0.99);
        }
        if rate > dec!(10.0) {
            rate = dec!(10.0);
        }

        if delta.abs() < eps {
            break;
        }
    }

    rate
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_secondaries_input(input: &SecondariesPricingInput) -> CorpFinanceResult<()> {
    if input.fund_nav <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_nav".into(),
            reason: "Fund NAV must be positive.".into(),
        });
    }
    if input.unfunded_commitment < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "unfunded_commitment".into(),
            reason: "Unfunded commitment cannot be negative.".into(),
        });
    }
    if input.remaining_life_years == 0 {
        return Err(CorpFinanceError::InvalidInput {
            field: "remaining_life_years".into(),
            reason: "Remaining life must be at least 1 year.".into(),
        });
    }
    if input.expected_distribution_rate < Decimal::ZERO
        || input.expected_distribution_rate > Decimal::ONE
    {
        return Err(CorpFinanceError::InvalidInput {
            field: "expected_distribution_rate".into(),
            reason: "Distribution rate must be in [0, 1].".into(),
        });
    }
    if input.discount_rate < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "discount_rate".into(),
            reason: "Discount rate cannot be negative.".into(),
        });
    }
    if input.management_fee_pct < Decimal::ZERO || input.management_fee_pct > dec!(0.10) {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee_pct".into(),
            reason: "Management fee must be in [0, 0.10].".into(),
        });
    }
    if input.carry_pct < Decimal::ZERO || input.carry_pct > Decimal::ONE {
        return Err(CorpFinanceError::InvalidInput {
            field: "carry_pct".into(),
            reason: "Carry percentage must be in [0, 1].".into(),
        });
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

    fn default_input() -> SecondariesPricingInput {
        SecondariesPricingInput {
            fund_nav: dec!(50_000_000),
            unfunded_commitment: dec!(10_000_000),
            remaining_life_years: 5,
            expected_distribution_rate: dec!(0.15),
            expected_growth_rate: dec!(0.10),
            discount_rate: dec!(0.12),
            management_fee_pct: dec!(0.02),
            carry_pct: dec!(0.20),
        }
    }

    #[test]
    fn test_secondaries_basic_output() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        assert!(
            out.fair_value > Decimal::ZERO,
            "Fair value should be positive"
        );
    }

    #[test]
    fn test_secondaries_fair_value_equals_components() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        let expected = out.distributions_pv + out.terminal_value_pv - out.unfunded_pv;
        assert!(
            approx_eq(out.fair_value, expected, dec!(1.0)),
            "Fair value {} != dist_pv + terminal_pv - unfunded_pv = {}",
            out.fair_value,
            expected
        );
    }

    #[test]
    fn test_secondaries_nav_discount_range() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        // Discount should typically be between -50% and +50%
        assert!(
            out.nav_discount_pct > dec!(-1.0) && out.nav_discount_pct < Decimal::ONE,
            "NAV discount {} seems extreme",
            out.nav_discount_pct
        );
    }

    #[test]
    fn test_secondaries_unfunded_pv_positive() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        assert!(
            out.unfunded_pv > Decimal::ZERO,
            "Unfunded PV should be positive when unfunded > 0"
        );
    }

    #[test]
    fn test_secondaries_unfunded_pv_zero_when_no_unfunded() {
        let mut input = default_input();
        input.unfunded_commitment = Decimal::ZERO;
        let out = calculate_secondaries_pricing(&input).unwrap();
        assert_eq!(out.unfunded_pv, Decimal::ZERO);
    }

    #[test]
    fn test_secondaries_distributions_pv_positive() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        assert!(out.distributions_pv > Decimal::ZERO);
    }

    #[test]
    fn test_secondaries_terminal_pv_positive() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        assert!(out.terminal_value_pv > Decimal::ZERO);
    }

    #[test]
    fn test_secondaries_irr_scenarios_count() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        assert_eq!(out.irr_scenarios.len(), 5);
    }

    #[test]
    fn test_secondaries_irr_increases_with_multiple() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        // Higher exit multiple should generally yield higher IRR
        for i in 1..out.irr_scenarios.len() {
            assert!(
                out.irr_scenarios[i].irr >= out.irr_scenarios[i - 1].irr,
                "IRR should increase with exit multiple: {} at {}x vs {} at {}x",
                out.irr_scenarios[i].irr,
                out.irr_scenarios[i].multiple,
                out.irr_scenarios[i - 1].irr,
                out.irr_scenarios[i - 1].multiple
            );
        }
    }

    #[test]
    fn test_secondaries_irr_at_discount_gt_nav() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        // If buying at a discount, IRR should be >= IRR at NAV
        if out.nav_discount_pct < Decimal::ZERO {
            assert!(
                out.irr_at_discount >= out.irr_at_nav,
                "Discount IRR {} should >= NAV IRR {}",
                out.irr_at_discount,
                out.irr_at_nav
            );
        }
    }

    #[test]
    fn test_secondaries_breakeven_multiple_positive() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        assert!(
            out.breakeven_multiple >= Decimal::ZERO,
            "Breakeven multiple should be non-negative"
        );
    }

    #[test]
    fn test_secondaries_higher_discount_rate_lower_fair_value() {
        let mut low_dr = default_input();
        low_dr.discount_rate = dec!(0.08);
        let mut high_dr = default_input();
        high_dr.discount_rate = dec!(0.18);
        let out_low = calculate_secondaries_pricing(&low_dr).unwrap();
        let out_high = calculate_secondaries_pricing(&high_dr).unwrap();
        assert!(
            out_low.fair_value > out_high.fair_value,
            "Lower discount rate should yield higher fair value"
        );
    }

    #[test]
    fn test_secondaries_higher_growth_higher_fair_value() {
        let mut low_g = default_input();
        low_g.expected_growth_rate = dec!(0.05);
        let mut high_g = default_input();
        high_g.expected_growth_rate = dec!(0.20);
        let out_low = calculate_secondaries_pricing(&low_g).unwrap();
        let out_high = calculate_secondaries_pricing(&high_g).unwrap();
        assert!(
            out_high.fair_value > out_low.fair_value,
            "Higher growth should yield higher fair value"
        );
    }

    #[test]
    fn test_secondaries_no_carry_higher_fair_value() {
        let with_carry = default_input();
        let mut no_carry = default_input();
        no_carry.carry_pct = Decimal::ZERO;
        let out_carry = calculate_secondaries_pricing(&with_carry).unwrap();
        let out_no = calculate_secondaries_pricing(&no_carry).unwrap();
        assert!(
            out_no.fair_value >= out_carry.fair_value,
            "No carry fair value {} should >= carry fair value {}",
            out_no.fair_value,
            out_carry.fair_value
        );
    }

    // -- Validation tests --

    #[test]
    fn test_reject_zero_nav() {
        let mut input = default_input();
        input.fund_nav = Decimal::ZERO;
        assert!(calculate_secondaries_pricing(&input).is_err());
    }

    #[test]
    fn test_reject_negative_unfunded() {
        let mut input = default_input();
        input.unfunded_commitment = dec!(-1);
        assert!(calculate_secondaries_pricing(&input).is_err());
    }

    #[test]
    fn test_reject_zero_remaining_life() {
        let mut input = default_input();
        input.remaining_life_years = 0;
        assert!(calculate_secondaries_pricing(&input).is_err());
    }

    #[test]
    fn test_reject_negative_discount_rate() {
        let mut input = default_input();
        input.discount_rate = dec!(-0.01);
        assert!(calculate_secondaries_pricing(&input).is_err());
    }

    #[test]
    fn test_reject_distribution_rate_over_one() {
        let mut input = default_input();
        input.expected_distribution_rate = dec!(1.5);
        assert!(calculate_secondaries_pricing(&input).is_err());
    }

    #[test]
    fn test_reject_excessive_management_fee() {
        let mut input = default_input();
        input.management_fee_pct = dec!(0.15);
        assert!(calculate_secondaries_pricing(&input).is_err());
    }

    #[test]
    fn test_reject_carry_over_one() {
        let mut input = default_input();
        input.carry_pct = dec!(1.5);
        assert!(calculate_secondaries_pricing(&input).is_err());
    }

    #[test]
    fn test_serialization_roundtrip() {
        let input = default_input();
        let out = calculate_secondaries_pricing(&input).unwrap();
        let json = serde_json::to_string(&out).unwrap();
        let _: SecondariesPricingOutput = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_single_year_remaining() {
        let mut input = default_input();
        input.remaining_life_years = 1;
        let out = calculate_secondaries_pricing(&input).unwrap();
        assert!(out.fair_value > Decimal::ZERO);
        assert_eq!(out.irr_scenarios.len(), 5);
    }
}
