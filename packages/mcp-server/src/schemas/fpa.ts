import { z } from "zod";

export const VarianceSchema = z.object({
  period_name: z.string().describe("Reporting period label (e.g. 'Q1 2024', 'FY 2024')"),
  revenue_lines: z.array(z.object({
    name: z.string().describe("Product or segment name"),
    budget_units: z.number().min(0).describe("Budgeted volume (units)"),
    budget_price: z.number().min(0).describe("Budgeted price per unit"),
    actual_units: z.number().min(0).describe("Actual volume (units)"),
    actual_price: z.number().min(0).describe("Actual price per unit"),
  })).describe("Revenue line items with price/volume detail"),
  cost_lines: z.array(z.object({
    name: z.string().describe("Cost category name (e.g. 'COGS', 'SGA')"),
    budget_amount: z.number().min(0).describe("Budgeted cost amount"),
    actual_amount: z.number().min(0).describe("Actual cost amount"),
    cost_type: z.enum(["Fixed", "Variable", "SemiVariable"]).describe("Cost behaviour classification"),
    variable_cost_per_unit: z.number().min(0).optional().describe("Variable cost per unit"),
  })).describe("Cost line items"),
  budget_total_revenue: z.number().min(0).describe("Budget total revenue (top-level)"),
  budget_total_costs: z.number().min(0).describe("Budget total costs (top-level)"),
  prior_period: z.object({
    revenue: z.number().describe("Prior period revenue"),
    costs: z.number().describe("Prior period costs"),
    profit: z.number().describe("Prior period profit"),
  }).optional().describe("Prior period for YoY comparison"),
});

export const BreakevenSchema = z.object({
  product_name: z.string().describe("Product name"),
  selling_price: z.number().positive().describe("Selling price per unit"),
  variable_cost_per_unit: z.number().min(0).describe("Variable cost per unit"),
  fixed_costs: z.number().min(0).describe("Total fixed costs"),
  current_volume: z.number().min(0).describe("Current units sold"),
  target_profit: z.number().optional().describe("Optional profit target for target-volume calc"),
  scenarios: z.array(z.object({
    name: z.string().describe("Scenario name"),
    price_change_pct: z.number().optional().describe("Price change percentage"),
    variable_cost_change_pct: z.number().optional().describe("Variable cost change percentage"),
    fixed_cost_change_pct: z.number().optional().describe("Fixed cost change percentage"),
  })).optional().describe("What-if scenarios"),
});

export const WorkingCapitalSchema = z.object({
  company_name: z.string().describe("Company identifier"),
  periods: z.array(z.object({
    period_name: z.string().describe("Period label (e.g. 'Q1 2024')"),
    revenue: z.number().positive().describe("Total revenue for the period"),
    cogs: z.number().min(0).describe("Cost of goods sold"),
    accounts_receivable: z.number().min(0).describe("Accounts receivable balance"),
    inventory: z.number().min(0).describe("Inventory balance"),
    accounts_payable: z.number().min(0).describe("Accounts payable balance"),
    other_current_assets: z.number().min(0).describe("Prepaid expenses, other current assets"),
    other_current_liabilities: z.number().min(0).describe("Accrued expenses, other current liabilities"),
    days_in_period: z.number().int().positive().describe("Days in the period (90 for quarter, 365 for year)"),
  })).describe("Multiple periods for trend analysis"),
  industry_benchmarks: z.object({
    dso_median: z.number().min(0).describe("Industry DSO median"),
    dio_median: z.number().min(0).describe("Industry DIO median"),
    dpo_median: z.number().min(0).describe("Industry DPO median"),
    ccc_median: z.number().describe("Industry CCC median"),
  }).optional().describe("Industry benchmarks for peer comparison"),
  cost_of_capital: z.number().min(0).max(1).describe("WACC for financing savings calculation"),
});

export const RollingForecastSchema = z.object({
  company_name: z.string().describe("Company name"),
  historical_periods: z.array(z.object({
    period_name: z.string().describe("Period label"),
    revenue: z.number().min(0).describe("Revenue"),
    cogs: z.number().min(0).describe("Cost of goods sold"),
    operating_expenses: z.number().min(0).describe("Operating expenses"),
    capex: z.number().min(0).describe("Capital expenditures"),
    depreciation: z.number().min(0).describe("Depreciation"),
  })).describe("At least 4 periods of historical data"),
  forecast_periods: z.number().int().positive().describe("Number of periods to forecast forward"),
  revenue_growth_rate: z.number().describe("Assumed revenue growth rate per period"),
  drivers: z.object({
    cogs_pct_revenue: z.number().min(0).max(1).optional().describe("COGS as % of revenue"),
    opex_pct_revenue: z.number().min(0).max(1).optional().describe("OpEx as % of revenue"),
    capex_pct_revenue: z.number().min(0).max(1).optional().describe("CapEx as % of revenue"),
    depreciation_pct_ppe: z.number().min(0).optional().describe("Depreciation as % of PP&E"),
    tax_rate: z.number().min(0).max(1).describe("Corporate tax rate"),
  }).describe("Driver assumptions"),
});
