use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::error::CorpFinanceError;
use crate::types::{with_metadata, ComputationOutput, Money, Rate};
use crate::CorpFinanceResult;

// ---------------------------------------------------------------------------
// Types — Variance Analysis
// ---------------------------------------------------------------------------

/// Cost classification for variance decomposition.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CostType {
    Fixed,
    Variable,
    SemiVariable,
}

/// A single revenue line item with budget vs actual figures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueLine {
    /// Product or segment name, e.g. "Product A", "Region EMEA"
    pub name: String,
    /// Budgeted volume (units)
    pub budget_units: Decimal,
    /// Budgeted price per unit
    pub budget_price: Decimal,
    /// Actual volume (units)
    pub actual_units: Decimal,
    /// Actual price per unit
    pub actual_price: Decimal,
}

/// A single cost line item with budget vs actual figures.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostLine {
    /// Cost category name, e.g. "COGS", "SGA", "R&D"
    pub name: String,
    /// Budgeted cost amount
    pub budget_amount: Money,
    /// Actual cost amount
    pub actual_amount: Money,
    /// Cost behaviour classification
    pub cost_type: CostType,
    /// Variable cost per unit (for variable / semi-variable costs)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_cost_per_unit: Option<Decimal>,
}

/// Optional prior-period data for year-over-year comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorPeriod {
    pub revenue: Money,
    pub costs: Money,
    pub profit: Money,
}

/// Input for budget-vs-actual variance analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceInput {
    /// Reporting period label, e.g. "Q1 2024", "FY 2024"
    pub period_name: String,
    /// Revenue line items with price/volume detail
    pub revenue_lines: Vec<RevenueLine>,
    /// Cost line items
    pub cost_lines: Vec<CostLine>,
    /// Budget total revenue (top-level)
    pub budget_total_revenue: Money,
    /// Budget total costs (top-level)
    pub budget_total_costs: Money,
    /// Prior period for YoY comparison
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prior_period: Option<PriorPeriod>,
}

// ---------------------------------------------------------------------------
// Output types — Variance Analysis
// ---------------------------------------------------------------------------

/// Revenue variance with price/volume/mix decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevenueVariance {
    pub budget_revenue: Money,
    pub actual_revenue: Money,
    /// actual - budget
    pub total_variance: Money,
    /// total_variance / budget_revenue as a rate
    pub total_variance_pct: Rate,
    /// Sum of (actual_price - budget_price) * actual_units across all lines
    pub price_variance: Money,
    /// Sum of (actual_units - budget_units) * budget_price across all lines
    pub volume_variance: Money,
    /// Residual: total - price - volume
    pub mix_variance: Money,
    /// True when actual > budget (favorable for revenue)
    pub favorable: bool,
}

/// Cost variance summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostVariance {
    pub budget_costs: Money,
    pub actual_costs: Money,
    /// actual - budget (positive = unfavorable for costs)
    pub total_variance: Money,
    pub total_variance_pct: Rate,
    /// True when actual < budget (favorable for costs)
    pub favorable: bool,
}

/// Profit variance with margin analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitVariance {
    /// budget_revenue - budget_costs
    pub budget_profit: Money,
    /// actual_revenue - actual_costs
    pub actual_profit: Money,
    pub total_variance: Money,
    pub total_variance_pct: Rate,
    /// Budget profit margin percentage
    pub margin_budget: Rate,
    /// Actual profit margin percentage
    pub margin_actual: Rate,
}

/// Per-line variance detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineVariance {
    pub name: String,
    /// "Revenue" or "Cost"
    pub line_type: String,
    pub budget: Money,
    pub actual: Money,
    pub variance: Money,
    pub variance_pct: Rate,
    pub favorable: bool,
}

/// Year-over-year comparison metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YoyComparison {
    pub prior_revenue: Money,
    pub current_revenue: Money,
    pub revenue_growth_pct: Rate,
    pub prior_profit: Money,
    pub current_profit: Money,
    pub profit_growth_pct: Rate,
    /// (current_margin - prior_margin) * 10_000
    pub margin_expansion_bps: Decimal,
}

/// Full variance analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarianceOutput {
    pub revenue_variance: RevenueVariance,
    pub cost_variance: CostVariance,
    pub profit_variance: ProfitVariance,
    pub line_detail: Vec<LineVariance>,
    pub yoy_comparison: Option<YoyComparison>,
}

// ---------------------------------------------------------------------------
// Types — Break-even Analysis
// ---------------------------------------------------------------------------

/// A scenario override for what-if break-even analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioOverride {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_change_pct: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variable_cost_change_pct: Option<Rate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_cost_change_pct: Option<Rate>,
}

/// Input for break-even and operating leverage analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakevenInput {
    pub product_name: String,
    /// Selling price per unit
    pub selling_price: Money,
    /// Variable cost per unit
    pub variable_cost_per_unit: Money,
    /// Total fixed costs
    pub fixed_costs: Money,
    /// Current units sold
    pub current_volume: Decimal,
    /// Optional profit target for target-volume calculation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_profit: Option<Money>,
    /// What-if scenarios
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scenarios: Option<Vec<ScenarioOverride>>,
}

/// Result for a single break-even scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub name: String,
    pub breakeven_units: Decimal,
    pub breakeven_revenue: Money,
    pub profit_at_current_volume: Money,
    pub margin_of_safety_pct: Rate,
}

/// Full break-even analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakevenOutput {
    /// Price - variable cost per unit
    pub contribution_margin: Money,
    /// CM / price
    pub contribution_margin_pct: Rate,
    /// Fixed costs / contribution margin
    pub breakeven_units: Decimal,
    /// breakeven_units * price
    pub breakeven_revenue: Money,
    /// (price - VC) * volume - FC
    pub current_profit: Money,
    /// current_volume - breakeven_units
    pub margin_of_safety_units: Decimal,
    /// MOS / current_volume
    pub margin_of_safety_pct: Rate,
    /// Degree of operating leverage: CM * volume / profit
    pub operating_leverage: Decimal,
    /// (FC + target_profit) / CM
    pub target_volume: Option<Decimal>,
    pub scenario_results: Vec<ScenarioResult>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Safe percentage: returns Decimal::ZERO when the denominator is zero.
fn safe_pct(numerator: Decimal, denominator: Decimal) -> Decimal {
    if denominator == dec!(0) {
        Decimal::ZERO
    } else {
        numerator / denominator
    }
}

// ---------------------------------------------------------------------------
// Function 1: analyze_variance
// ---------------------------------------------------------------------------

/// Perform budget-vs-actual variance analysis with price/volume/mix
/// decomposition, cost variance, profit variance, line-level detail,
/// and optional year-over-year comparison.
pub fn analyze_variance(
    input: &VarianceInput,
) -> CorpFinanceResult<ComputationOutput<VarianceOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Compute actual revenue from line items ---
    let actual_revenue: Decimal = input
        .revenue_lines
        .iter()
        .map(|r| r.actual_units * r.actual_price)
        .sum();

    let budget_revenue_from_lines: Decimal = input
        .revenue_lines
        .iter()
        .map(|r| r.budget_units * r.budget_price)
        .sum();

    // Warn if the sum of budget line items diverges from the stated total
    if budget_revenue_from_lines != input.budget_total_revenue && !input.revenue_lines.is_empty() {
        warnings.push(format!(
            "Sum of budget revenue lines ({}) differs from stated budget_total_revenue ({})",
            budget_revenue_from_lines, input.budget_total_revenue
        ));
    }

    // --- Price / Volume / Mix decomposition ---
    let price_variance: Decimal = input
        .revenue_lines
        .iter()
        .map(|r| (r.actual_price - r.budget_price) * r.actual_units)
        .sum();

    let volume_variance: Decimal = input
        .revenue_lines
        .iter()
        .map(|r| (r.actual_units - r.budget_units) * r.budget_price)
        .sum();

    let total_revenue_variance = actual_revenue - input.budget_total_revenue;
    let mix_variance = total_revenue_variance - price_variance - volume_variance;

    let revenue_variance = RevenueVariance {
        budget_revenue: input.budget_total_revenue,
        actual_revenue,
        total_variance: total_revenue_variance,
        total_variance_pct: safe_pct(total_revenue_variance, input.budget_total_revenue),
        price_variance,
        volume_variance,
        mix_variance,
        favorable: actual_revenue > input.budget_total_revenue,
    };

    // --- Cost variance ---
    let actual_costs: Decimal = input.cost_lines.iter().map(|c| c.actual_amount).sum();
    let budget_costs_from_lines: Decimal = input.cost_lines.iter().map(|c| c.budget_amount).sum();

    if budget_costs_from_lines != input.budget_total_costs && !input.cost_lines.is_empty() {
        warnings.push(format!(
            "Sum of budget cost lines ({}) differs from stated budget_total_costs ({})",
            budget_costs_from_lines, input.budget_total_costs
        ));
    }

    let total_cost_variance = actual_costs - input.budget_total_costs;
    let cost_variance = CostVariance {
        budget_costs: input.budget_total_costs,
        actual_costs,
        total_variance: total_cost_variance,
        total_variance_pct: safe_pct(total_cost_variance, input.budget_total_costs),
        favorable: actual_costs < input.budget_total_costs,
    };

    // --- Profit variance ---
    let budget_profit = input.budget_total_revenue - input.budget_total_costs;
    let actual_profit = actual_revenue - actual_costs;
    let total_profit_variance = actual_profit - budget_profit;

    let profit_variance = ProfitVariance {
        budget_profit,
        actual_profit,
        total_variance: total_profit_variance,
        total_variance_pct: safe_pct(total_profit_variance, budget_profit),
        margin_budget: safe_pct(budget_profit, input.budget_total_revenue),
        margin_actual: safe_pct(actual_profit, actual_revenue),
    };

    // --- Line-level detail ---
    let mut line_detail: Vec<LineVariance> = Vec::new();

    for r in &input.revenue_lines {
        let budget = r.budget_units * r.budget_price;
        let actual = r.actual_units * r.actual_price;
        let variance = actual - budget;
        line_detail.push(LineVariance {
            name: r.name.clone(),
            line_type: "Revenue".to_string(),
            budget,
            actual,
            variance,
            variance_pct: safe_pct(variance, budget),
            favorable: actual > budget,
        });
    }

    for c in &input.cost_lines {
        let variance = c.actual_amount - c.budget_amount;
        line_detail.push(LineVariance {
            name: c.name.clone(),
            line_type: "Cost".to_string(),
            budget: c.budget_amount,
            actual: c.actual_amount,
            variance,
            variance_pct: safe_pct(variance, c.budget_amount),
            favorable: c.actual_amount < c.budget_amount,
        });
    }

    // --- YoY comparison ---
    let yoy_comparison = input.prior_period.as_ref().map(|pp| {
        let current_margin = safe_pct(actual_profit, actual_revenue);
        let prior_margin = safe_pct(pp.profit, pp.revenue);
        YoyComparison {
            prior_revenue: pp.revenue,
            current_revenue: actual_revenue,
            revenue_growth_pct: safe_pct(actual_revenue - pp.revenue, pp.revenue),
            prior_profit: pp.profit,
            current_profit: actual_profit,
            profit_growth_pct: safe_pct(actual_profit - pp.profit, pp.profit),
            margin_expansion_bps: (current_margin - prior_margin) * dec!(10000),
        }
    });

    let output = VarianceOutput {
        revenue_variance,
        cost_variance,
        profit_variance,
        line_detail,
        yoy_comparison,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Budget vs Actual Variance Analysis with Price/Volume/Mix Decomposition",
        &serde_json::json!({
            "period": input.period_name,
            "revenue_lines": input.revenue_lines.len(),
            "cost_lines": input.cost_lines.len(),
            "has_prior_period": input.prior_period.is_some(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

// ---------------------------------------------------------------------------
// Function 2: analyze_breakeven
// ---------------------------------------------------------------------------

/// Compute break-even point, margin of safety, degree of operating leverage,
/// optional target volume, and what-if scenario results.
pub fn analyze_breakeven(
    input: &BreakevenInput,
) -> CorpFinanceResult<ComputationOutput<BreakevenOutput>> {
    let start = Instant::now();
    let mut warnings: Vec<String> = Vec::new();

    // --- Validate inputs ---
    if input.selling_price <= dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "selling_price".to_string(),
            reason: "Selling price must be positive".to_string(),
        });
    }

    if input.variable_cost_per_unit < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "variable_cost_per_unit".to_string(),
            reason: "Variable cost per unit cannot be negative".to_string(),
        });
    }

    if input.fixed_costs < dec!(0) {
        return Err(CorpFinanceError::InvalidInput {
            field: "fixed_costs".to_string(),
            reason: "Fixed costs cannot be negative".to_string(),
        });
    }

    let contribution_margin = input.selling_price - input.variable_cost_per_unit;

    if contribution_margin <= dec!(0) {
        return Err(CorpFinanceError::FinancialImpossibility(
            "Contribution margin is zero or negative — break-even is unreachable".to_string(),
        ));
    }

    let contribution_margin_pct = contribution_margin / input.selling_price;

    // Break-even units = FC / CM
    let breakeven_units = input.fixed_costs / contribution_margin;
    let breakeven_revenue = breakeven_units * input.selling_price;

    // Current profit = CM * volume - FC
    let current_profit = contribution_margin * input.current_volume - input.fixed_costs;

    // Margin of safety
    let margin_of_safety_units = input.current_volume - breakeven_units;
    let margin_of_safety_pct = if input.current_volume == dec!(0) {
        warnings.push("Current volume is zero; margin of safety is undefined".to_string());
        Decimal::ZERO
    } else {
        margin_of_safety_units / input.current_volume
    };

    // Degree of operating leverage = Total CM / Profit
    let total_cm = contribution_margin * input.current_volume;
    let operating_leverage = if current_profit == dec!(0) {
        warnings
            .push("Current profit is zero; operating leverage is undefined (set to 0)".to_string());
        Decimal::ZERO
    } else {
        total_cm / current_profit
    };

    // Target volume
    let target_volume = input
        .target_profit
        .map(|tp| (input.fixed_costs + tp) / contribution_margin);

    // --- Scenario analysis ---
    let scenario_results = match &input.scenarios {
        Some(scenarios) => scenarios
            .iter()
            .map(|s| compute_scenario(input, s))
            .collect::<CorpFinanceResult<Vec<ScenarioResult>>>()?,
        None => Vec::new(),
    };

    let output = BreakevenOutput {
        contribution_margin,
        contribution_margin_pct,
        breakeven_units,
        breakeven_revenue,
        current_profit,
        margin_of_safety_units,
        margin_of_safety_pct,
        operating_leverage,
        target_volume,
        scenario_results,
    };

    let elapsed = start.elapsed().as_micros() as u64;

    Ok(with_metadata(
        "Break-even Analysis with Operating Leverage and Scenario Modelling",
        &serde_json::json!({
            "product": input.product_name,
            "selling_price": input.selling_price.to_string(),
            "variable_cost_per_unit": input.variable_cost_per_unit.to_string(),
            "fixed_costs": input.fixed_costs.to_string(),
            "current_volume": input.current_volume.to_string(),
        }),
        warnings,
        elapsed,
        output,
    ))
}

/// Compute a single break-even what-if scenario.
fn compute_scenario(
    base: &BreakevenInput,
    overrides: &ScenarioOverride,
) -> CorpFinanceResult<ScenarioResult> {
    let price = apply_pct_change(base.selling_price, overrides.price_change_pct);
    let vc = apply_pct_change(
        base.variable_cost_per_unit,
        overrides.variable_cost_change_pct,
    );
    let fc = apply_pct_change(base.fixed_costs, overrides.fixed_cost_change_pct);

    let cm = price - vc;
    if cm <= dec!(0) {
        return Err(CorpFinanceError::FinancialImpossibility(format!(
            "Scenario '{}': contribution margin is zero or negative after overrides",
            overrides.name
        )));
    }

    let be_units = fc / cm;
    let be_revenue = be_units * price;
    let profit = cm * base.current_volume - fc;
    let mos_pct = if base.current_volume == dec!(0) {
        Decimal::ZERO
    } else {
        (base.current_volume - be_units) / base.current_volume
    };

    Ok(ScenarioResult {
        name: overrides.name.clone(),
        breakeven_units: be_units,
        breakeven_revenue: be_revenue,
        profit_at_current_volume: profit,
        margin_of_safety_pct: mos_pct,
    })
}

/// Apply an optional percentage change to a base value.
/// e.g. base=100, change=Some(0.10) => 110
fn apply_pct_change(base: Decimal, change_pct: Option<Rate>) -> Decimal {
    match change_pct {
        Some(pct) => base * (dec!(1) + pct),
        None => base,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn basic_variance_input() -> VarianceInput {
        VarianceInput {
            period_name: "Q1 2024".to_string(),
            revenue_lines: vec![
                RevenueLine {
                    name: "Product A".to_string(),
                    budget_units: dec!(100),
                    budget_price: dec!(10),
                    actual_units: dec!(110),
                    actual_price: dec!(11),
                },
                RevenueLine {
                    name: "Product B".to_string(),
                    budget_units: dec!(200),
                    budget_price: dec!(5),
                    actual_units: dec!(190),
                    actual_price: dec!(5),
                },
            ],
            cost_lines: vec![
                CostLine {
                    name: "COGS".to_string(),
                    budget_amount: dec!(800),
                    actual_amount: dec!(750),
                    cost_type: CostType::Variable,
                    variable_cost_per_unit: Some(dec!(3)),
                },
                CostLine {
                    name: "SGA".to_string(),
                    budget_amount: dec!(300),
                    actual_amount: dec!(320),
                    cost_type: CostType::Fixed,
                    variable_cost_per_unit: None,
                },
            ],
            // budget_total_revenue = 100*10 + 200*5 = 2000
            budget_total_revenue: dec!(2000),
            // budget_total_costs = 800 + 300 = 1100
            budget_total_costs: dec!(1100),
            prior_period: None,
        }
    }

    fn basic_breakeven_input() -> BreakevenInput {
        BreakevenInput {
            product_name: "Widget".to_string(),
            selling_price: dec!(50),
            variable_cost_per_unit: dec!(30),
            fixed_costs: dec!(10000),
            current_volume: dec!(1000),
            target_profit: None,
            scenarios: None,
        }
    }

    // -----------------------------------------------------------------------
    // Variance analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_revenue_variance_favorable() {
        // actual_revenue = 110*11 + 190*5 = 1210 + 950 = 2160
        // budget = 2000 => variance = +160, favorable
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        assert_eq!(result.result.revenue_variance.actual_revenue, dec!(2160));
        assert_eq!(result.result.revenue_variance.total_variance, dec!(160));
        assert!(result.result.revenue_variance.favorable);
    }

    #[test]
    fn test_revenue_variance_unfavorable() {
        let mut input = basic_variance_input();
        // Make actual worse: lower prices and lower units
        input.revenue_lines[0].actual_units = dec!(80);
        input.revenue_lines[0].actual_price = dec!(9);
        input.revenue_lines[1].actual_units = dec!(180);
        input.revenue_lines[1].actual_price = dec!(4);
        // actual = 80*9 + 180*4 = 720 + 720 = 1440 < 2000
        let result = analyze_variance(&input).unwrap();
        assert_eq!(result.result.revenue_variance.actual_revenue, dec!(1440));
        assert!(result.result.revenue_variance.total_variance < dec!(0));
        assert!(!result.result.revenue_variance.favorable);
    }

    #[test]
    fn test_price_volume_mix_decomposition_sums_to_total() {
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        let rv = &result.result.revenue_variance;
        let reconstructed = rv.price_variance + rv.volume_variance + rv.mix_variance;
        assert_eq!(reconstructed, rv.total_variance);
    }

    #[test]
    fn test_price_variance_calculation() {
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        // price_variance = (11-10)*110 + (5-5)*190 = 110 + 0 = 110
        assert_eq!(result.result.revenue_variance.price_variance, dec!(110));
    }

    #[test]
    fn test_volume_variance_calculation() {
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        // volume_variance = (110-100)*10 + (190-200)*5 = 100 + (-50) = 50
        assert_eq!(result.result.revenue_variance.volume_variance, dec!(50));
    }

    #[test]
    fn test_cost_variance_favorable() {
        // actual_costs = 750 + 320 = 1070 < 1100 => favorable
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        assert_eq!(result.result.cost_variance.actual_costs, dec!(1070));
        assert_eq!(result.result.cost_variance.total_variance, dec!(-30));
        assert!(result.result.cost_variance.favorable);
    }

    #[test]
    fn test_cost_variance_unfavorable() {
        let mut input = basic_variance_input();
        input.cost_lines[0].actual_amount = dec!(900);
        input.cost_lines[1].actual_amount = dec!(350);
        // actual = 1250 > 1100
        let result = analyze_variance(&input).unwrap();
        assert_eq!(result.result.cost_variance.actual_costs, dec!(1250));
        assert!(result.result.cost_variance.total_variance > dec!(0));
        assert!(!result.result.cost_variance.favorable);
    }

    #[test]
    fn test_profit_variance_with_margins() {
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        let pv = &result.result.profit_variance;
        // budget_profit = 2000 - 1100 = 900
        // actual_profit = 2160 - 1070 = 1090
        assert_eq!(pv.budget_profit, dec!(900));
        assert_eq!(pv.actual_profit, dec!(1090));
        assert_eq!(pv.total_variance, dec!(190));
        // margin_budget = 900/2000 = 0.45
        assert_eq!(pv.margin_budget, dec!(0.45));
        // margin_actual = 1090/2160 = 0.50462962962962962962962962963
        assert!(pv.margin_actual > dec!(0.50));
    }

    #[test]
    fn test_yoy_comparison_growth_rates() {
        let mut input = basic_variance_input();
        input.prior_period = Some(PriorPeriod {
            revenue: dec!(1800),
            costs: dec!(1000),
            profit: dec!(800),
        });
        let result = analyze_variance(&input).unwrap();
        let yoy = result.result.yoy_comparison.as_ref().unwrap();
        assert_eq!(yoy.prior_revenue, dec!(1800));
        assert_eq!(yoy.current_revenue, dec!(2160));
        // revenue_growth = (2160 - 1800) / 1800 = 360/1800 = 0.2
        assert_eq!(yoy.revenue_growth_pct, dec!(0.2));
    }

    #[test]
    fn test_margin_expansion_calculation() {
        let mut input = basic_variance_input();
        input.prior_period = Some(PriorPeriod {
            revenue: dec!(1800),
            costs: dec!(1000),
            profit: dec!(800),
        });
        let result = analyze_variance(&input).unwrap();
        let yoy = result.result.yoy_comparison.as_ref().unwrap();
        // prior_margin = 800/1800 = 0.44444...
        // current_margin = 1090/2160 = 0.50462962...
        // expansion_bps = (0.50462962... - 0.44444...) * 10000 = ~601.85 bps
        assert!(yoy.margin_expansion_bps > dec!(500));
        assert!(yoy.margin_expansion_bps < dec!(700));
    }

    #[test]
    fn test_line_by_line_detail() {
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        // 2 revenue lines + 2 cost lines = 4
        assert_eq!(result.result.line_detail.len(), 4);
        assert_eq!(result.result.line_detail[0].line_type, "Revenue");
        assert_eq!(result.result.line_detail[0].name, "Product A");
        assert_eq!(result.result.line_detail[2].line_type, "Cost");
        assert_eq!(result.result.line_detail[2].name, "COGS");
    }

    // -----------------------------------------------------------------------
    // Break-even analysis tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_breakeven_basic_calculation() {
        let input = basic_breakeven_input();
        let result = analyze_breakeven(&input).unwrap();
        // CM = 50 - 30 = 20, BE = 10000 / 20 = 500
        assert_eq!(result.result.breakeven_units, dec!(500));
        assert_eq!(result.result.breakeven_revenue, dec!(25000));
    }

    #[test]
    fn test_breakeven_high_fixed_costs() {
        let mut input = basic_breakeven_input();
        input.fixed_costs = dec!(30000);
        let result = analyze_breakeven(&input).unwrap();
        // BE = 30000 / 20 = 1500 > 500
        assert_eq!(result.result.breakeven_units, dec!(1500));
    }

    #[test]
    fn test_contribution_margin_calculation() {
        let input = basic_breakeven_input();
        let result = analyze_breakeven(&input).unwrap();
        assert_eq!(result.result.contribution_margin, dec!(20));
        // CM% = 20/50 = 0.4
        assert_eq!(result.result.contribution_margin_pct, dec!(0.4));
    }

    #[test]
    fn test_margin_of_safety_positive() {
        let input = basic_breakeven_input();
        let result = analyze_breakeven(&input).unwrap();
        // MOS = 1000 - 500 = 500, MOS% = 500/1000 = 0.5
        assert_eq!(result.result.margin_of_safety_units, dec!(500));
        assert_eq!(result.result.margin_of_safety_pct, dec!(0.5));
    }

    #[test]
    fn test_margin_of_safety_negative() {
        let mut input = basic_breakeven_input();
        input.current_volume = dec!(300); // below breakeven of 500
        let result = analyze_breakeven(&input).unwrap();
        // MOS = 300 - 500 = -200, MOS% = -200/300
        assert!(result.result.margin_of_safety_units < dec!(0));
        assert!(result.result.margin_of_safety_pct < dec!(0));
    }

    #[test]
    fn test_operating_leverage_dol() {
        let input = basic_breakeven_input();
        let result = analyze_breakeven(&input).unwrap();
        // profit = 20*1000 - 10000 = 10000
        // total_cm = 20*1000 = 20000
        // DOL = 20000 / 10000 = 2
        assert_eq!(result.result.operating_leverage, dec!(2));
    }

    #[test]
    fn test_target_volume_with_profit_target() {
        let mut input = basic_breakeven_input();
        input.target_profit = Some(dec!(20000));
        let result = analyze_breakeven(&input).unwrap();
        // target = (10000 + 20000) / 20 = 1500
        assert_eq!(result.result.target_volume, Some(dec!(1500)));
    }

    #[test]
    fn test_scenario_price_increase_lowers_breakeven() {
        let mut input = basic_breakeven_input();
        input.scenarios = Some(vec![ScenarioOverride {
            name: "Price +10%".to_string(),
            price_change_pct: Some(dec!(0.10)),
            variable_cost_change_pct: None,
            fixed_cost_change_pct: None,
        }]);
        let result = analyze_breakeven(&input).unwrap();
        let scenario = &result.result.scenario_results[0];
        // new_price = 50 * 1.10 = 55, CM = 55-30 = 25, BE = 10000/25 = 400 < 500
        assert_eq!(scenario.breakeven_units, dec!(400));
        assert!(scenario.breakeven_units < result.result.breakeven_units);
    }

    #[test]
    fn test_scenario_cost_increase_raises_breakeven() {
        let mut input = basic_breakeven_input();
        input.scenarios = Some(vec![ScenarioOverride {
            name: "VC +25%".to_string(),
            price_change_pct: None,
            variable_cost_change_pct: Some(dec!(0.25)),
            fixed_cost_change_pct: None,
        }]);
        let result = analyze_breakeven(&input).unwrap();
        let scenario = &result.result.scenario_results[0];
        // new_vc = 30 * 1.25 = 37.5, CM = 50 - 37.5 = 12.5, BE = 10000/12.5 = 800 > 500
        assert_eq!(scenario.breakeven_units, dec!(800));
        assert!(scenario.breakeven_units > result.result.breakeven_units);
    }

    #[test]
    fn test_edge_zero_volume_breakeven() {
        let mut input = basic_breakeven_input();
        input.current_volume = dec!(0);
        let result = analyze_breakeven(&input).unwrap();
        // MOS should be zero (cannot divide by zero volume)
        assert_eq!(result.result.margin_of_safety_pct, dec!(0));
        // Profit = 20*0 - 10000 = -10000
        assert_eq!(result.result.current_profit, dec!(-10000));
        // Operating leverage should be zero (profit is negative => warning)
        // Actually profit is -10000, DOL = 0 / -10000 = 0
        // But we have total_cm = 0, so 0 / -10000 = 0
        assert_eq!(result.result.operating_leverage, dec!(0));
    }

    #[test]
    fn test_multiple_revenue_lines_different_variances() {
        let input = VarianceInput {
            period_name: "FY 2024".to_string(),
            revenue_lines: vec![
                RevenueLine {
                    name: "Premium".to_string(),
                    budget_units: dec!(50),
                    budget_price: dec!(100),
                    actual_units: dec!(60),
                    actual_price: dec!(105), // favorable: more units, higher price
                },
                RevenueLine {
                    name: "Standard".to_string(),
                    budget_units: dec!(200),
                    budget_price: dec!(20),
                    actual_units: dec!(180),
                    actual_price: dec!(18), // unfavorable: fewer units, lower price
                },
                RevenueLine {
                    name: "Economy".to_string(),
                    budget_units: dec!(500),
                    budget_price: dec!(5),
                    actual_units: dec!(520),
                    actual_price: dec!(5), // slight favorable: more units, same price
                },
            ],
            cost_lines: vec![CostLine {
                name: "Total COGS".to_string(),
                budget_amount: dec!(5000),
                actual_amount: dec!(4800),
                cost_type: CostType::Variable,
                variable_cost_per_unit: None,
            }],
            // budget = 50*100 + 200*20 + 500*5 = 5000 + 4000 + 2500 = 11500
            budget_total_revenue: dec!(11500),
            budget_total_costs: dec!(5000),
            prior_period: None,
        };

        let result = analyze_variance(&input).unwrap();

        // actual_revenue = 60*105 + 180*18 + 520*5 = 6300 + 3240 + 2600 = 12140
        assert_eq!(result.result.revenue_variance.actual_revenue, dec!(12140));
        assert_eq!(result.result.revenue_variance.total_variance, dec!(640));
        assert!(result.result.revenue_variance.favorable);

        // Check decomposition sums
        let rv = &result.result.revenue_variance;
        assert_eq!(
            rv.price_variance + rv.volume_variance + rv.mix_variance,
            rv.total_variance
        );

        // 3 revenue lines + 1 cost line = 4 detail lines
        assert_eq!(result.result.line_detail.len(), 4);

        // Premium line should be favorable
        assert!(result.result.line_detail[0].favorable);
        // Standard line should be unfavorable
        assert!(!result.result.line_detail[1].favorable);
    }

    #[test]
    fn test_zero_contribution_margin_error() {
        let mut input = basic_breakeven_input();
        input.variable_cost_per_unit = dec!(50); // same as price
        let result = analyze_breakeven(&input);
        assert!(result.is_err());
        match result.unwrap_err() {
            CorpFinanceError::FinancialImpossibility(msg) => {
                assert!(msg.contains("Contribution margin"));
            }
            other => panic!("Expected FinancialImpossibility, got {:?}", other),
        }
    }

    #[test]
    fn test_variance_percentage_calculation() {
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        // total_variance_pct = 160 / 2000 = 0.08
        assert_eq!(
            result.result.revenue_variance.total_variance_pct,
            dec!(0.08)
        );
    }

    #[test]
    fn test_breakeven_zero_fixed_costs() {
        let mut input = basic_breakeven_input();
        input.fixed_costs = dec!(0);
        let result = analyze_breakeven(&input).unwrap();
        // BE = 0 / 20 = 0 units
        assert_eq!(result.result.breakeven_units, dec!(0));
        assert_eq!(result.result.breakeven_revenue, dec!(0));
        // Profit = 20*1000 - 0 = 20000
        assert_eq!(result.result.current_profit, dec!(20000));
    }

    #[test]
    fn test_yoy_no_prior_period() {
        let input = basic_variance_input();
        let result = analyze_variance(&input).unwrap();
        assert!(result.result.yoy_comparison.is_none());
    }

    #[test]
    fn test_breakeven_scenario_fixed_cost_change() {
        let mut input = basic_breakeven_input();
        input.scenarios = Some(vec![ScenarioOverride {
            name: "FC +50%".to_string(),
            price_change_pct: None,
            variable_cost_change_pct: None,
            fixed_cost_change_pct: Some(dec!(0.50)),
        }]);
        let result = analyze_breakeven(&input).unwrap();
        let scenario = &result.result.scenario_results[0];
        // new_fc = 10000 * 1.50 = 15000, BE = 15000/20 = 750
        assert_eq!(scenario.breakeven_units, dec!(750));
    }
}
