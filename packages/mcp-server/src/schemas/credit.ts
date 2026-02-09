import { z } from "zod";

export const CreditMetricsSchema = z.object({
  revenue: z.number().positive().describe("Total revenue"),
  ebitda: z.number().describe("EBITDA"),
  ebit: z.number().describe("EBIT"),
  interest_expense: z.number().min(0).describe("Total interest expense"),
  depreciation_amortisation: z
    .number()
    .min(0)
    .describe("Depreciation and amortisation"),
  total_debt: z.number().min(0).describe("Total financial debt"),
  cash: z.number().min(0).describe("Cash and cash equivalents"),
  total_assets: z.number().positive().describe("Total assets"),
  current_assets: z.number().min(0).describe("Total current assets"),
  current_liabilities: z.number().min(0).describe("Total current liabilities"),
  total_equity: z.number().describe("Total shareholders equity"),
  retained_earnings: z.number().describe("Retained earnings"),
  working_capital: z.number().describe("Net working capital"),
  operating_cash_flow: z.number().describe("Cash flow from operations"),
  capex: z.number().min(0).describe("Capital expenditure (positive number)"),
  funds_from_operations: z
    .number()
    .optional()
    .describe("FFO if available (for S&P-style analysis)"),
  lease_payments: z
    .number()
    .optional()
    .describe("Annual lease payments for fixed charge coverage"),
  preferred_dividends: z
    .number()
    .optional()
    .describe("Preferred dividend payments"),
  market_cap: z
    .number()
    .optional()
    .describe("Market capitalisation for Altman Z-score market variant"),
});

export const DebtCapacitySchema = z.object({
  ebitda: z.number().positive().describe("Current or projected EBITDA"),
  interest_rate: z
    .number()
    .min(0)
    .max(0.3)
    .describe("Expected interest rate on new debt"),
  max_leverage: z
    .number()
    .min(0)
    .max(20)
    .optional()
    .describe("Maximum Net Debt / EBITDA (e.g. 4.0x)"),
  min_interest_coverage: z
    .number()
    .min(0)
    .optional()
    .describe("Minimum EBITDA / Interest coverage (e.g. 3.0x)"),
  min_dscr: z
    .number()
    .min(0)
    .optional()
    .describe("Minimum debt service coverage ratio (e.g. 1.5x)"),
  min_ffo_to_debt: z
    .number()
    .min(0)
    .max(1)
    .optional()
    .describe("Minimum FFO / Total Debt (e.g. 0.15 = 15%)"),
  existing_debt: z
    .number()
    .min(0)
    .optional()
    .describe("Existing debt outstanding"),
  annual_amortisation: z
    .number()
    .min(0)
    .optional()
    .describe("Annual debt amortisation payment"),
  ffo: z.number().optional().describe("Funds from operations"),
});

export const CovenantTestSchema = z.object({
  covenants: z
    .array(
      z.object({
        name: z.string().describe("Covenant name (e.g. Maximum Leverage)"),
        metric: z
          .enum([
            "NetDebtToEbitda",
            "InterestCoverage",
            "Dscr",
            "DebtToEquity",
            "MinCash",
            "MaxCapex",
          ])
          .describe("Financial metric being tested"),
        threshold: z.number().describe("Covenant threshold value"),
        direction: z
          .enum(["MaxOf", "MinOf"])
          .describe("MaxOf = actual must be <= threshold; MinOf = actual must be >= threshold"),
      })
    )
    .min(1)
    .describe("List of covenant definitions to test"),
  actuals: z
    .object({
      net_debt_to_ebitda: z.number().optional().describe("Actual Net Debt / EBITDA"),
      interest_coverage: z
        .number()
        .optional()
        .describe("Actual EBITDA / Interest"),
      dscr: z.number().optional().describe("Actual debt service coverage ratio"),
      debt_to_equity: z.number().optional().describe("Actual Debt / Equity"),
      cash: z.number().optional().describe("Actual cash balance"),
      capex: z.number().optional().describe("Actual capital expenditure"),
    })
    .describe("Actual financial metrics to test against covenants"),
});
