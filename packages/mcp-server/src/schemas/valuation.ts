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
  beta: z.number().min(0).max(5).describe("Levered equity beta"),
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
});

export const DcfSchema = z.object({
  base_revenue: z.number().positive().describe("Base year revenue"),
  revenue_growth_rates: z
    .array(z.number().min(-1).max(2))
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
  currency: CurrencySchema.optional(),
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
  net_debt: z.number().optional().describe("Net debt to bridge EV to equity value"),
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
  target: z
    .object({
      enterprise_value: z.number().optional().describe("Target enterprise value"),
      market_cap: z.number().optional().describe("Target market capitalisation"),
      revenue: z.number().optional().describe("Target LTM revenue"),
      ebitda: z.number().optional().describe("Target LTM EBITDA"),
      ebit: z.number().optional().describe("Target LTM EBIT"),
      net_income: z.number().optional().describe("Target LTM net income"),
      book_value: z.number().optional().describe("Target book value of equity"),
      earnings_growth: z
        .number()
        .optional()
        .describe("Target forward earnings growth rate"),
    })
    .describe("Financial metrics of the company being valued"),
  comparables: z
    .array(
      z.object({
        name: z.string().describe("Comparable company name"),
        metrics: z.object({
          enterprise_value: z.number().optional().describe("Comparable EV"),
          market_cap: z.number().optional().describe("Comparable market cap"),
          revenue: z.number().optional().describe("Comparable LTM revenue"),
          ebitda: z.number().optional().describe("Comparable LTM EBITDA"),
          ebit: z.number().optional().describe("Comparable LTM EBIT"),
          net_income: z.number().optional().describe("Comparable LTM net income"),
          book_value: z.number().optional().describe("Comparable book value"),
          earnings_growth: z
            .number()
            .optional()
            .describe("Comparable earnings growth"),
        }),
        include: z
          .boolean()
          .optional()
          .describe("Include in analysis (default true, set false for outliers)"),
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
        "PegRatio",
      ])
    )
    .min(1)
    .describe("Valuation multiples to compute"),
});
