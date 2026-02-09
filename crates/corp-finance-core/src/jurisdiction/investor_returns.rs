use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::*;
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single layer in the fee/cost waterfall, showing its annual drag,
/// total dollar cost over the holding period, and share of the total drag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostLayer {
    pub name: String,
    pub annual_rate: Rate,
    pub total_cost: Money,
    pub pct_of_total_drag: Rate,
}

/// Input for the investor net returns calculator.
///
/// Models the full fee/cost waterfall from gross return down to the
/// investor's after-tax net return, including fund-level fees, FoF
/// overlay, withholding tax drag, blocker entity costs, and personal tax.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorNetReturnsInput {
    /// Annualised gross return (e.g. 0.15 = 15%)
    pub gross_return: Rate,
    /// Total investment amount
    pub investment_amount: Money,
    /// Holding period in years (can be fractional, e.g. 3.5)
    pub holding_period_years: Decimal,
    /// Annual management fee as a rate
    pub management_fee: Rate,
    /// Performance fee rate (applied to gain above hurdle)
    pub performance_fee: Rate,
    /// Hurdle rate for the performance fee (None = no hurdle)
    pub hurdle_rate: Option<Rate>,
    /// Annual fund operating expenses as a percentage of NAV
    pub fund_expenses_pct: Rate,
    /// Fund-of-funds additional management fee layer
    pub fof_management_fee: Option<Rate>,
    /// Fund-of-funds performance fee layer
    pub fof_performance_fee: Option<Rate>,
    /// Annual withholding tax drag on returns
    pub wht_drag: Rate,
    /// Annual blocker entity maintenance cost as a rate
    pub blocker_cost: Option<Rate>,
    /// Personal/institutional tax rate on gains
    pub investor_tax_rate: Option<Rate>,
    /// Currency for the output
    pub currency: Option<Currency>,
}

/// Output of the investor net returns calculator.
///
/// Shows the return after each successive fee/cost layer and the
/// terminal dollar amounts, MOICs, and a detailed cost breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorNetReturnsOutput {
    pub gross_return: Rate,
    pub after_management_fee: Rate,
    pub after_performance_fee: Rate,
    pub after_fund_expenses: Rate,
    pub after_fof_fees: Option<Rate>,
    pub after_wht: Rate,
    pub after_blocker: Option<Rate>,
    pub after_tax: Option<Rate>,
    /// Final annualised net return
    pub net_return: Rate,
    /// gross_return - net_return
    pub total_fee_drag: Rate,
    /// Fee drag expressed in basis points
    pub total_fee_drag_bps: Decimal,
    /// investment_amount * (1 + gross_return)^holding_period
    pub gross_amount: Money,
    /// investment_amount * (1 + net_return)^holding_period
    pub net_amount: Money,
    /// gross_amount - net_amount
    pub fees_paid: Money,
    /// Breakdown of each cost layer
    pub cost_breakdown: Vec<CostLayer>,
    /// gross_amount / investment_amount
    pub gross_moic: Multiple,
    /// net_amount / investment_amount
    pub net_moic: Multiple,
}

// ---------------------------------------------------------------------------
// Main calculation
// ---------------------------------------------------------------------------

/// Calculate investor net returns through the full fee/cost waterfall.
///
/// Applies management fees, performance fees (with optional hurdle),
/// fund expenses, optional fund-of-funds overlay, withholding tax drag,
/// blocker costs, and personal/institutional tax in a layer-by-layer
/// deduction from the gross return to arrive at the net annualised return.
pub fn calculate_investor_net_returns(
    input: &InvestorNetReturnsInput,
) -> CorpFinanceResult<ComputationOutput<InvestorNetReturnsOutput>> {
    let start = Instant::now();
    let warnings: Vec<String> = Vec::new();

    // ------------------------------------------------------------------
    // 1. Validate inputs
    // ------------------------------------------------------------------
    validate_input(input)?;

    // ------------------------------------------------------------------
    // 2. Layer-by-layer fee deduction waterfall
    // ------------------------------------------------------------------
    let gross = input.gross_return;

    // Management fee
    let after_mgmt = gross - input.management_fee;

    // Performance fee (applied to gain above hurdle)
    let hurdle = input.hurdle_rate.unwrap_or(Decimal::ZERO);
    let gain_above_hurdle = (after_mgmt - hurdle).max(Decimal::ZERO);
    let perf_fee_drag = gain_above_hurdle * input.performance_fee;
    let after_perf = after_mgmt - perf_fee_drag;

    // Fund expenses
    let after_expenses = after_perf - input.fund_expenses_pct;

    // FoF fees (optional double layer)
    let after_fof: Option<Rate>;
    let rate_after_fof_or_expenses: Rate;

    if let (Some(fof_mgmt), Some(fof_perf)) = (input.fof_management_fee, input.fof_performance_fee)
    {
        let after_fof_mgmt = after_expenses - fof_mgmt;
        let fof_gain_above_hurdle = (after_fof_mgmt - hurdle).max(Decimal::ZERO);
        let fof_perf_drag = fof_gain_above_hurdle * fof_perf;
        let fof_net = after_fof_mgmt - fof_perf_drag;
        after_fof = Some(fof_net);
        rate_after_fof_or_expenses = fof_net;
    } else if let Some(fof_mgmt) = input.fof_management_fee {
        let fof_net = after_expenses - fof_mgmt;
        after_fof = Some(fof_net);
        rate_after_fof_or_expenses = fof_net;
    } else {
        after_fof = None;
        rate_after_fof_or_expenses = after_expenses;
    }

    // WHT drag
    let after_wht = rate_after_fof_or_expenses - input.wht_drag;

    // Blocker cost
    let after_blocker: Option<Rate>;
    let rate_after_blocker_or_wht: Rate;

    if let Some(blocker) = input.blocker_cost {
        let net = after_wht - blocker;
        after_blocker = Some(net);
        rate_after_blocker_or_wht = net;
    } else {
        after_blocker = None;
        rate_after_blocker_or_wht = after_wht;
    }

    // Tax (applied only on gains, not the full return)
    let after_tax: Option<Rate>;
    let net_return: Rate;

    if let Some(tax_rate) = input.investor_tax_rate {
        if rate_after_blocker_or_wht > Decimal::ZERO {
            let taxed = rate_after_blocker_or_wht * (Decimal::ONE - tax_rate);
            after_tax = Some(taxed);
            net_return = taxed;
        } else {
            after_tax = Some(rate_after_blocker_or_wht);
            net_return = rate_after_blocker_or_wht;
        }
    } else {
        after_tax = None;
        net_return = rate_after_blocker_or_wht;
    }

    // ------------------------------------------------------------------
    // 3. Terminal values
    // ------------------------------------------------------------------
    let holding = input.holding_period_years;
    let investment = input.investment_amount;

    let gross_amount = investment * compound(Decimal::ONE + gross, holding);
    let net_amount = investment * compound(Decimal::ONE + net_return, holding);
    let fees_paid = gross_amount - net_amount;

    // ------------------------------------------------------------------
    // 4. MOICs
    // ------------------------------------------------------------------
    let gross_moic = gross_amount / investment;
    let net_moic = net_amount / investment;

    // ------------------------------------------------------------------
    // 5. Fee drag
    // ------------------------------------------------------------------
    let total_fee_drag = gross - net_return;
    let total_fee_drag_bps = total_fee_drag * dec!(10000);

    // ------------------------------------------------------------------
    // 6. Cost breakdown
    // ------------------------------------------------------------------
    let cost_breakdown = build_cost_breakdown(
        input,
        &CostDrags {
            mgmt_drag: input.management_fee,
            perf_drag: perf_fee_drag,
            expense_drag: input.fund_expenses_pct,
            fof_drag: compute_fof_drag(input, after_expenses, hurdle),
            wht_drag: input.wht_drag,
            blocker_drag: input.blocker_cost,
            tax_drag: compute_tax_drag(rate_after_blocker_or_wht, input.investor_tax_rate),
            total_fee_drag,
        },
    );

    // ------------------------------------------------------------------
    // 7. Assemble output
    // ------------------------------------------------------------------
    let output = InvestorNetReturnsOutput {
        gross_return: gross,
        after_management_fee: after_mgmt,
        after_performance_fee: after_perf,
        after_fund_expenses: after_expenses,
        after_fof_fees: after_fof,
        after_wht,
        after_blocker,
        after_tax,
        net_return,
        total_fee_drag,
        total_fee_drag_bps,
        gross_amount,
        net_amount,
        fees_paid,
        cost_breakdown,
        gross_moic,
        net_moic,
    };

    let elapsed = start.elapsed().as_micros() as u64;
    Ok(with_metadata(
        "Investor Net Returns Calculator: Layer-by-layer fee waterfall from gross to net",
        &serde_json::json!({
            "gross_return": input.gross_return.to_string(),
            "investment_amount": input.investment_amount.to_string(),
            "holding_period_years": input.holding_period_years.to_string(),
            "management_fee": input.management_fee.to_string(),
            "performance_fee": input.performance_fee.to_string(),
            "hurdle_rate": input.hurdle_rate.map(|r| r.to_string()),
            "fund_expenses_pct": input.fund_expenses_pct.to_string(),
            "wht_drag": input.wht_drag.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn validate_input(input: &InvestorNetReturnsInput) -> CorpFinanceResult<()> {
    if input.investment_amount <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "investment_amount".into(),
            reason: "Investment amount must be positive".into(),
        });
    }
    if input.holding_period_years <= Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "holding_period_years".into(),
            reason: "Holding period must be greater than zero".into(),
        });
    }
    if input.gross_return <= dec!(-1) {
        return Err(CorpFinanceError::InvalidInput {
            field: "gross_return".into(),
            reason: "Gross return must be greater than -1 (total loss maximum)".into(),
        });
    }
    if input.management_fee < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "management_fee".into(),
            reason: "Management fee must be non-negative".into(),
        });
    }
    if input.performance_fee < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "performance_fee".into(),
            reason: "Performance fee must be non-negative".into(),
        });
    }
    if input.fund_expenses_pct < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "fund_expenses_pct".into(),
            reason: "Fund expenses must be non-negative".into(),
        });
    }
    if input.wht_drag < Decimal::ZERO {
        return Err(CorpFinanceError::InvalidInput {
            field: "wht_drag".into(),
            reason: "WHT drag must be non-negative".into(),
        });
    }
    if let Some(hr) = input.hurdle_rate {
        if hr < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "hurdle_rate".into(),
                reason: "Hurdle rate must be non-negative".into(),
            });
        }
    }
    if let Some(fof) = input.fof_management_fee {
        if fof < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "fof_management_fee".into(),
                reason: "FoF management fee must be non-negative".into(),
            });
        }
    }
    if let Some(fof) = input.fof_performance_fee {
        if fof < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "fof_performance_fee".into(),
                reason: "FoF performance fee must be non-negative".into(),
            });
        }
    }
    if let Some(bc) = input.blocker_cost {
        if bc < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "blocker_cost".into(),
                reason: "Blocker cost must be non-negative".into(),
            });
        }
    }
    if let Some(tr) = input.investor_tax_rate {
        if tr < Decimal::ZERO {
            return Err(CorpFinanceError::InvalidInput {
                field: "investor_tax_rate".into(),
                reason: "Investor tax rate must be non-negative".into(),
            });
        }
    }
    Ok(())
}

/// Compound (1+r)^n using `powd` for fractional exponents.
fn compound(base: Decimal, exponent: Decimal) -> Decimal {
    if base <= Decimal::ZERO {
        return Decimal::ZERO;
    }
    base.powd(exponent)
}

/// Intermediate struct to pass individual cost drags to the breakdown builder.
struct CostDrags {
    mgmt_drag: Rate,
    perf_drag: Rate,
    expense_drag: Rate,
    fof_drag: Option<Rate>,
    wht_drag: Rate,
    blocker_drag: Option<Rate>,
    tax_drag: Option<Rate>,
    total_fee_drag: Rate,
}

/// Compute the combined FoF drag (mgmt + perf) if FoF fees apply.
fn compute_fof_drag(
    input: &InvestorNetReturnsInput,
    after_expenses: Rate,
    hurdle: Rate,
) -> Option<Rate> {
    if let Some(fof_mgmt) = input.fof_management_fee {
        let mut drag = fof_mgmt;
        if let Some(fof_perf) = input.fof_performance_fee {
            let after_fof_mgmt = after_expenses - fof_mgmt;
            let fof_gain_above_hurdle = (after_fof_mgmt - hurdle).max(Decimal::ZERO);
            drag += fof_gain_above_hurdle * fof_perf;
        }
        Some(drag)
    } else {
        None
    }
}

/// Compute the tax drag as the difference between pre-tax and post-tax return.
fn compute_tax_drag(pre_tax_return: Rate, tax_rate: Option<Rate>) -> Option<Rate> {
    tax_rate.map(|tr| {
        if pre_tax_return > Decimal::ZERO {
            pre_tax_return * tr
        } else {
            Decimal::ZERO
        }
    })
}

/// Build the cost breakdown vector with one entry per applicable cost layer.
fn build_cost_breakdown(input: &InvestorNetReturnsInput, drags: &CostDrags) -> Vec<CostLayer> {
    let mut layers: Vec<CostLayer> = Vec::new();
    let holding = input.holding_period_years;
    let investment = input.investment_amount;
    let gross = input.gross_return;

    // Dollar cost of a given annual drag: difference in terminal value
    let dollar_cost = |drag: Rate| -> Money {
        let with_full = compound(Decimal::ONE + gross, holding);
        let without_drag = compound(Decimal::ONE + (gross - drag), holding);
        investment * (with_full - without_drag)
    };

    // Percentage of total drag (avoid division by zero)
    let pct_of_total = |drag: Rate| -> Rate {
        if drags.total_fee_drag > Decimal::ZERO {
            drag / drags.total_fee_drag
        } else {
            Decimal::ZERO
        }
    };

    // Management fee
    if drags.mgmt_drag > Decimal::ZERO {
        layers.push(CostLayer {
            name: "Management Fee".to_string(),
            annual_rate: drags.mgmt_drag,
            total_cost: dollar_cost(drags.mgmt_drag),
            pct_of_total_drag: pct_of_total(drags.mgmt_drag),
        });
    }

    // Performance fee
    if drags.perf_drag > Decimal::ZERO {
        layers.push(CostLayer {
            name: "Performance Fee".to_string(),
            annual_rate: drags.perf_drag,
            total_cost: dollar_cost(drags.perf_drag),
            pct_of_total_drag: pct_of_total(drags.perf_drag),
        });
    }

    // Fund expenses
    if drags.expense_drag > Decimal::ZERO {
        layers.push(CostLayer {
            name: "Fund Expenses".to_string(),
            annual_rate: drags.expense_drag,
            total_cost: dollar_cost(drags.expense_drag),
            pct_of_total_drag: pct_of_total(drags.expense_drag),
        });
    }

    // FoF fees
    if let Some(fof_drag) = drags.fof_drag {
        if fof_drag > Decimal::ZERO {
            layers.push(CostLayer {
                name: "Fund-of-Funds Fees".to_string(),
                annual_rate: fof_drag,
                total_cost: dollar_cost(fof_drag),
                pct_of_total_drag: pct_of_total(fof_drag),
            });
        }
    }

    // WHT drag
    if drags.wht_drag > Decimal::ZERO {
        layers.push(CostLayer {
            name: "Withholding Tax Drag".to_string(),
            annual_rate: drags.wht_drag,
            total_cost: dollar_cost(drags.wht_drag),
            pct_of_total_drag: pct_of_total(drags.wht_drag),
        });
    }

    // Blocker cost
    if let Some(blocker) = drags.blocker_drag {
        if blocker > Decimal::ZERO {
            layers.push(CostLayer {
                name: "Blocker Entity Cost".to_string(),
                annual_rate: blocker,
                total_cost: dollar_cost(blocker),
                pct_of_total_drag: pct_of_total(blocker),
            });
        }
    }

    // Tax
    if let Some(tax_drag) = drags.tax_drag {
        if tax_drag > Decimal::ZERO {
            layers.push(CostLayer {
                name: "Investor Tax".to_string(),
                annual_rate: tax_drag,
                total_cost: dollar_cost(tax_drag),
                pct_of_total_drag: pct_of_total(tax_drag),
            });
        }
    }

    layers
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Helper to create a baseline input with no optional fee layers.
    fn base_input() -> InvestorNetReturnsInput {
        InvestorNetReturnsInput {
            gross_return: dec!(0.15),
            investment_amount: dec!(1_000_000),
            holding_period_years: dec!(5),
            management_fee: dec!(0.02),
            performance_fee: dec!(0.20),
            hurdle_rate: Some(dec!(0.08)),
            fund_expenses_pct: dec!(0.005),
            fof_management_fee: None,
            fof_performance_fee: None,
            wht_drag: dec!(0.0),
            blocker_cost: None,
            investor_tax_rate: None,
            currency: Some(Currency::USD),
        }
    }

    // ------------------------------------------------------------------
    // Test 1: Basic net returns with no extras
    // ------------------------------------------------------------------
    #[test]
    fn test_basic_net_returns_no_extras() {
        let input = base_input();
        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.gross_return, dec!(0.15));

        assert!(
            out.net_return < out.gross_return,
            "Net return ({}) should be less than gross ({})",
            out.net_return,
            out.gross_return
        );

        assert!(out.net_return > Decimal::ZERO);
        assert!(out.net_amount < out.gross_amount);
        assert!(out.fees_paid > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // Test 2: Management fee only
    // ------------------------------------------------------------------
    #[test]
    fn test_management_fee_only() {
        let mut input = base_input();
        input.performance_fee = dec!(0.0);
        input.fund_expenses_pct = dec!(0.0);
        input.hurdle_rate = None;

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.after_management_fee, dec!(0.13));
        assert_eq!(out.after_performance_fee, out.after_management_fee);
        assert_eq!(out.after_fund_expenses, out.after_performance_fee);
        assert_eq!(out.net_return, dec!(0.13));
    }

    // ------------------------------------------------------------------
    // Test 3: Performance fee with hurdle
    // ------------------------------------------------------------------
    #[test]
    fn test_performance_fee_with_hurdle() {
        let mut input = base_input();
        input.management_fee = dec!(0.0);
        input.fund_expenses_pct = dec!(0.0);
        // gross=15%, hurdle=8%, perf fee=20%
        // gain above hurdle = 15% - 8% = 7%
        // perf fee drag = 7% * 20% = 1.4%
        // after perf = 15% - 1.4% = 13.6%

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.after_management_fee, dec!(0.15));
        assert_eq!(out.after_performance_fee, dec!(0.136));
        assert_eq!(out.net_return, dec!(0.136));
    }

    // ------------------------------------------------------------------
    // Test 4: Performance fee no hurdle
    // ------------------------------------------------------------------
    #[test]
    fn test_performance_fee_no_hurdle() {
        let mut input = base_input();
        input.management_fee = dec!(0.0);
        input.fund_expenses_pct = dec!(0.0);
        input.hurdle_rate = None;
        // gross=15%, no hurdle, perf fee=20%
        // gain above hurdle = 15% - 0% = 15%
        // perf fee drag = 15% * 20% = 3%
        // after perf = 15% - 3% = 12%

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.after_performance_fee, dec!(0.12));
        assert_eq!(out.net_return, dec!(0.12));
    }

    // ------------------------------------------------------------------
    // Test 5: Fund expenses deduction
    // ------------------------------------------------------------------
    #[test]
    fn test_fund_expenses_deduction() {
        let mut input = base_input();
        input.management_fee = dec!(0.0);
        input.performance_fee = dec!(0.0);
        input.hurdle_rate = None;
        input.fund_expenses_pct = dec!(0.005);

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.after_fund_expenses, dec!(0.145));
        assert_eq!(out.net_return, dec!(0.145));
    }

    // ------------------------------------------------------------------
    // Test 6: FoF double-layer fees
    // ------------------------------------------------------------------
    #[test]
    fn test_fof_double_layer_fees() {
        let mut input = base_input();
        input.management_fee = dec!(0.02);
        input.performance_fee = dec!(0.20);
        input.hurdle_rate = Some(dec!(0.08));
        input.fund_expenses_pct = dec!(0.005);
        input.fof_management_fee = Some(dec!(0.01));
        input.fof_performance_fee = Some(dec!(0.10));

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert!(out.after_fof_fees.is_some());

        let after_fof = out.after_fof_fees.unwrap();
        assert!(
            after_fof < out.after_fund_expenses,
            "After FoF ({}) should be less than after fund expenses ({})",
            after_fof,
            out.after_fund_expenses
        );
    }

    // ------------------------------------------------------------------
    // Test 7: WHT drag applied
    // ------------------------------------------------------------------
    #[test]
    fn test_wht_drag_applied() {
        let mut input = base_input();
        input.management_fee = dec!(0.0);
        input.performance_fee = dec!(0.0);
        input.fund_expenses_pct = dec!(0.0);
        input.hurdle_rate = None;
        input.wht_drag = dec!(0.01);

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert_eq!(out.after_wht, dec!(0.14));
        assert_eq!(out.net_return, dec!(0.14));
    }

    // ------------------------------------------------------------------
    // Test 8: Blocker cost applied
    // ------------------------------------------------------------------
    #[test]
    fn test_blocker_cost_applied() {
        let mut input = base_input();
        input.management_fee = dec!(0.0);
        input.performance_fee = dec!(0.0);
        input.fund_expenses_pct = dec!(0.0);
        input.hurdle_rate = None;
        input.blocker_cost = Some(dec!(0.005));

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert!(out.after_blocker.is_some());
        assert_eq!(out.after_blocker.unwrap(), dec!(0.145));
        assert_eq!(out.net_return, dec!(0.145));
    }

    // ------------------------------------------------------------------
    // Test 9: Investor tax applied
    // ------------------------------------------------------------------
    #[test]
    fn test_investor_tax_applied() {
        let mut input = base_input();
        input.management_fee = dec!(0.0);
        input.performance_fee = dec!(0.0);
        input.fund_expenses_pct = dec!(0.0);
        input.hurdle_rate = None;
        input.investor_tax_rate = Some(dec!(0.20));

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        // Tax on gains: 15% * (1 - 20%) = 12%
        assert!(out.after_tax.is_some());
        assert_eq!(out.after_tax.unwrap(), dec!(0.12));
        assert_eq!(out.net_return, dec!(0.12));
    }

    // ------------------------------------------------------------------
    // Test 10: Total fee drag in basis points
    // ------------------------------------------------------------------
    #[test]
    fn test_total_fee_drag_bps() {
        let input = base_input();
        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        let expected_bps = out.total_fee_drag * dec!(10000);
        assert_eq!(out.total_fee_drag_bps, expected_bps);
        assert!(out.total_fee_drag_bps > Decimal::ZERO);
    }

    // ------------------------------------------------------------------
    // Test 11: Cost breakdown sums to total drag
    // ------------------------------------------------------------------
    #[test]
    fn test_cost_breakdown_sums_to_total() {
        let mut input = base_input();
        input.wht_drag = dec!(0.005);
        input.blocker_cost = Some(dec!(0.002));
        input.investor_tax_rate = Some(dec!(0.15));

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        // Sum of individual annual_rate drags should equal total_fee_drag
        let sum_annual: Decimal = out.cost_breakdown.iter().map(|c| c.annual_rate).sum();
        assert_eq!(sum_annual, out.total_fee_drag);

        // pct_of_total_drag should sum to ~1.0
        let sum_pct: Decimal = out.cost_breakdown.iter().map(|c| c.pct_of_total_drag).sum();
        let diff = (sum_pct - Decimal::ONE).abs();
        assert!(
            diff < dec!(0.0001),
            "Sum of pct_of_total_drag ({}) should be ~1.0",
            sum_pct
        );
    }

    // ------------------------------------------------------------------
    // Test 12: Gross vs net MOIC
    // ------------------------------------------------------------------
    #[test]
    fn test_gross_vs_net_moic() {
        let input = base_input();
        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert!(out.gross_moic > Decimal::ONE);
        assert!(out.net_moic > Decimal::ONE);
        assert!(
            out.net_moic < out.gross_moic,
            "Net MOIC ({}) should be < gross MOIC ({})",
            out.net_moic,
            out.gross_moic
        );

        let expected_gross_moic = out.gross_amount / dec!(1_000_000);
        let expected_net_moic = out.net_amount / dec!(1_000_000);
        assert_eq!(out.gross_moic, expected_gross_moic);
        assert_eq!(out.net_moic, expected_net_moic);
    }

    // ------------------------------------------------------------------
    // Test 13: Zero investment error
    // ------------------------------------------------------------------
    #[test]
    fn test_zero_investment_error() {
        let mut input = base_input();
        input.investment_amount = Decimal::ZERO;

        let result = calculate_investor_net_returns(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "investment_amount");
            }
            other => panic!("Expected InvalidInput for investment_amount, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // Test 14: Negative holding period error
    // ------------------------------------------------------------------
    #[test]
    fn test_negative_holding_period_error() {
        let mut input = base_input();
        input.holding_period_years = dec!(-1);

        let result = calculate_investor_net_returns(&input);
        assert!(result.is_err());

        match result.unwrap_err() {
            CorpFinanceError::InvalidInput { field, .. } => {
                assert_eq!(field, "holding_period_years");
            }
            other => panic!("Expected InvalidInput for holding_period_years, got: {other}"),
        }
    }

    // ------------------------------------------------------------------
    // Test 15: Metadata populated
    // ------------------------------------------------------------------
    #[test]
    fn test_metadata_populated() {
        let input = base_input();
        let result = calculate_investor_net_returns(&input).unwrap();

        assert!(!result.methodology.is_empty());
        assert_eq!(result.metadata.precision, "rust_decimal_128bit");
        assert!(!result.metadata.version.is_empty());
    }

    // ------------------------------------------------------------------
    // Test 16: Full waterfall with all layers
    // ------------------------------------------------------------------
    #[test]
    fn test_full_waterfall_all_layers() {
        let mut input = base_input();
        input.fof_management_fee = Some(dec!(0.01));
        input.fof_performance_fee = Some(dec!(0.10));
        input.wht_drag = dec!(0.005);
        input.blocker_cost = Some(dec!(0.003));
        input.investor_tax_rate = Some(dec!(0.20));

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        assert!(out.after_management_fee < out.gross_return);
        assert!(out.after_performance_fee <= out.after_management_fee);
        assert!(out.after_fund_expenses < out.after_performance_fee);
        assert!(out.after_fof_fees.unwrap() < out.after_fund_expenses);
        assert!(out.after_wht < out.after_fof_fees.unwrap());
        assert!(out.after_blocker.unwrap() < out.after_wht);
        assert!(out.after_tax.unwrap() < out.after_blocker.unwrap());
        assert_eq!(out.net_return, out.after_tax.unwrap());
    }

    // ------------------------------------------------------------------
    // Test 17: Fractional holding period
    // ------------------------------------------------------------------
    #[test]
    fn test_fractional_holding_period() {
        let mut input = base_input();
        input.holding_period_years = dec!(3.5);
        input.performance_fee = dec!(0.0);
        input.fund_expenses_pct = dec!(0.0);
        input.hurdle_rate = None;

        let result = calculate_investor_net_returns(&input).unwrap();
        let out = &result.result;

        let expected_gross = dec!(1_000_000) * (Decimal::ONE + dec!(0.15)).powd(dec!(3.5));
        let diff = (out.gross_amount - expected_gross).abs();
        assert!(
            diff < dec!(0.01),
            "Gross amount ({}) should match expected ({})",
            out.gross_amount,
            expected_gross
        );
    }
}
