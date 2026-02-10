import { z } from "zod";
import { CurrencySchema } from "./common.js";

export const WaccSchema = z.object({
  risk_free_rate: z
    .number()
    .min(0)
    .max(0.2)
    .describe("10Y government bond yield (e.g. 0.04 = 4%)"),
  equity_risk_premium: z
    .number()
    .min(0)
    .max(0.15)
    .describe("Equity risk premium (e.g. 0.055 = 5.5%)"),
  beta: z.coerce.number().min(0).max(5).describe("Levered equity beta"),
  cost_of_debt: z
    .number()
    .min(0)
    .max(0.3)
    .describe("Pre-tax cost of debt (e.g. 0.06 = 6%)"),
  tax_rate: z
    .number()
    .min(0)
    .max(0.5)
    .describe("Corporate tax rate (e.g. 0.25 = 25%)"),
  debt_weight: z
    .number()
    .min(0)
    .max(1)
    .describe("Debt / (Debt + Equity) weight"),
  equity_weight: z
    .number()
    .min(0)
    .max(1)
    .describe("Equity / (Debt + Equity) weight"),
  size_premium: z
    .number()
    .optional()
    .describe("Small-cap premium if applicable"),
  country_risk_premium: z
    .number()
    .optional()
    .describe("Emerging market or country risk premium"),
  specific_risk_premium: z
    .number()
    .optional()
    .describe("Company-specific risk premium"),
  unlevered_beta: z
    .number()
    .optional()
    .describe("Unlevered (asset) beta for Hamada re-levering"),
  target_debt_equity: z
    .number()
    .optional()
    .describe("Target debt-to-equity ratio for Hamada re-levering"),
});

const CompanyMetricsSchema = z.object({
  enterprise_value: z.coerce.number().optional().describe("Enterprise value"),
  market_cap: z.coerce.number().optional().describe("Market capitalisation"),
  revenue: z.coerce.number().optional().describe("Total revenue / sales"),
  ebitda: z.coerce.number().optional().describe("EBITDA"),
  ebit: z.coerce.number().optional().describe("EBIT / operating income"),
  net_income: z.coerce.number().optional().describe("Net income"),
  book_value: z.coerce.number().optional().describe("Book value of equity"),
  eps: z.coerce.number().optional().describe("Earnings per share"),
  eps_growth_rate: z
    .number()
    .optional()
    .describe("Expected EPS growth rate (for PEG ratio)"),
  share_price: z.coerce.number().optional().describe("Share price"),
});

export const DcfSchema = z.object({
  base_revenue: z.coerce.number().positive().describe("Base year revenue"),
  revenue_growth_rates: z
    .array(z.coerce.number().min(-1).max(2))
    .describe("Annual revenue growth rates for each forecast year"),
  ebitda_margin: z
    .number()
    .min(0)
    .max(1)
    .describe("EBITDA margin as decimal (e.g. 0.15 = 15%)"),
  ebit_margin: z
    .number()
    .min(0)
    .max(1)
    .optional()
    .describe("EBIT margin if D&A requires separate treatment"),
  da_as_pct_revenue: z
    .number()
    .min(0)
    .max(0.5)
    .optional()
    .describe("Depreciation & amortisation as % of revenue"),
  capex_as_pct_revenue: z
    .number()
    .min(0)
    .max(0.5)
    .describe("Capital expenditure as % of revenue"),
  nwc_as_pct_revenue: z
    .number()
    .min(-0.5)
    .max(0.5)
    .describe("Net working capital change as % of revenue"),
  tax_rate: z
    .number()
    .min(0)
    .max(0.5)
    .describe("Corporate tax rate"),
  wacc: z
    .number()
    .min(0.001)
    .max(0.3)
    .describe("Weighted average cost of capital"),
  wacc_input: WaccSchema.optional().describe(
    "If provided, WACC is computed from these inputs (overrides wacc field)"
  ),
  terminal_method: z
    .enum(["GordonGrowth", "ExitMultiple", "Both"])
    .describe("Terminal value calculation methodology"),
  terminal_growth_rate: z
    .number()
    .min(0)
    .max(0.1)
    .optional()
    .describe("Perpetual growth rate for Gordon Growth model"),
  terminal_exit_multiple: z
    .number()
    .min(0)
    .max(50)
    .optional()
    .describe("Exit EV/EBITDA multiple for terminal value"),
  currency: CurrencySchema.describe("Reporting currency"),
  forecast_years: z
    .number()
    .int()
    .min(1)
    .max(30)
    .optional()
    .describe("Number of explicit forecast years (default 10)"),
  mid_year_convention: z
    .boolean()
    .optional()
    .describe("Use mid-year discounting convention (default true)"),
  net_debt: z
    .number()
    .optional()
    .describe("Net debt to bridge EV to equity value"),
  minority_interest: z
    .number()
    .optional()
    .describe("Minority interest deducted from equity value"),
  shares_outstanding: z
    .number()
    .positive()
    .optional()
    .describe("Diluted shares outstanding for per-share value"),
});

export const CompsSchema = z.object({
  target_name: z.string().describe("Target company name"),
  target_metrics: CompanyMetricsSchema.describe(
    "Financial metrics of the company being valued"
  ),
  comparables: z
    .array(
      z.object({
        name: z.string().describe("Comparable company name"),
        metrics: CompanyMetricsSchema.describe("Financial metrics"),
        include: z
          .boolean()
          .describe(
            "Include in analysis (set false for outliers)"
          ),
      })
    )
    .min(1)
    .describe("List of comparable companies"),
  multiples: z
    .array(
      z.enum([
        "EvEbitda",
        "EvRevenue",
        "EvEbit",
        "PriceEarnings",
        "PriceBook",
        "Peg",
      ])
    )
    .min(1)
    .describe("Valuation multiples to compute"),
  currency: CurrencySchema.describe("Reporting currency"),
});
