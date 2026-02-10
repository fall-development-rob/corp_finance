import { z } from "zod";

// --- CreditMetricsInput ---
// Rust struct: CreditMetricsInput in credit/metrics.rs
// All fields match exactly as-is. No changes needed.
export const CreditMetricsSchema = z.object({
  revenue: z.coerce.number().positive().describe("Total revenue"),
  ebitda: z.coerce.number().describe("EBITDA"),
  ebit: z.coerce.number().describe("EBIT"),
  interest_expense: z.coerce.number().min(0).describe("Total interest expense"),
  depreciation_amortisation: z
    .number()
    .min(0)
    .describe("Depreciation and amortisation"),
  total_debt: z.coerce.number().min(0).describe("Total financial debt"),
  cash: z.coerce.number().min(0).describe("Cash and cash equivalents"),
  total_assets: z.coerce.number().positive().describe("Total assets"),
  current_assets: z.coerce.number().min(0).describe("Total current assets"),
  current_liabilities: z.coerce.number().min(0).describe("Total current liabilities"),
  total_equity: z.coerce.number().describe("Total shareholders equity"),
  retained_earnings: z.coerce.number().describe("Retained earnings"),
  working_capital: z.coerce.number().describe("Net working capital"),
  operating_cash_flow: z.coerce.number().describe("Cash flow from operations"),
  capex: z.coerce.number().min(0).describe("Capital expenditure (positive number)"),
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
    .describe("Market capitalisation for EV calculation"),
});

// --- DebtCapacityInput ---
// Rust struct: DebtCapacityInput in credit/capacity.rs
// All fields match exactly as-is. No changes needed.
export const DebtCapacitySchema = z.object({
  ebitda: z.coerce.number().positive().describe("Current or projected EBITDA"),
  interest_rate: z
    .number()
    .min(0)
    .describe("Expected interest rate on new debt"),
  max_leverage: z
    .number()
    .positive()
    .optional()
    .describe("Maximum Net Debt / EBITDA (e.g. 4.0x)"),
  min_interest_coverage: z
    .number()
    .positive()
    .optional()
    .describe("Minimum EBITDA / Interest coverage (e.g. 3.0x)"),
  min_dscr: z
    .number()
    .positive()
    .optional()
    .describe("Minimum debt service coverage ratio (e.g. 1.5x)"),
  min_ffo_to_debt: z
    .number()
    .min(0)
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
  ffo: z.coerce.number().optional().describe("Funds from operations"),
});

// --- CreditMetricsOutput ---
// Rust struct: CreditMetricsOutput in credit/metrics.rs
// Used as the `actuals` field in CovenantTestInput.
// The CreditRating enum has serde(rename) for variants like "AA+", "AA-", etc.
const CreditRatingSchema = z.enum([
  "AAA",
  "AA+",
  "AA",
  "AA-",
  "A+",
  "A",
  "A-",
  "BBB+",
  "BBB",
  "BBB-",
  "BB+",
  "BB",
  "BB-",
  "B+",
  "B",
  "B-",
  "CCC+",
  "CCC",
  "CCC-",
  "CC",
  "C",
  "D",
]);

const CreditMetricsOutputSchema = z.object({
  net_debt: z.coerce.number(),
  net_debt_to_ebitda: z.coerce.number(),
  total_debt_to_ebitda: z.coerce.number(),
  debt_to_equity: z.coerce.number(),
  debt_to_assets: z.coerce.number(),
  net_debt_to_ev: z.coerce.number().optional(),
  interest_coverage: z.coerce.number(),
  ebit_coverage: z.coerce.number(),
  fixed_charge_coverage: z.coerce.number().optional(),
  dscr: z.coerce.number(),
  ffo_to_debt: z.coerce.number().optional(),
  ocf_to_debt: z.coerce.number(),
  fcf_to_debt: z.coerce.number(),
  fcf: z.coerce.number(),
  cash_conversion: z.coerce.number(),
  current_ratio: z.coerce.number(),
  quick_ratio: z.coerce.number(),
  cash_to_debt: z.coerce.number(),
  implied_rating: CreditRatingSchema,
  rating_rationale: z.array(z.string()),
});

// --- CovenantMetric ---
// Rust enum: CovenantMetric in credit/covenants.rs
// Variants: NetDebtToEbitda, InterestCoverage, Dscr, DebtToEquity, MinCash, MaxCapex, Custom(String)
// Custom is externally tagged: { "Custom": "some_metric" }
const CovenantMetricSchema = z.union([
  z.enum([
    "NetDebtToEbitda",
    "InterestCoverage",
    "Dscr",
    "DebtToEquity",
    "MinCash",
    "MaxCapex",
  ]),
  z.object({ Custom: z.string() }),
]);

// --- CovenantTestInput ---
// Rust struct: CovenantTestInput in credit/covenants.rs
// Fields: covenants (Vec<Covenant>), actuals (CreditMetricsOutput)
export const CovenantTestSchema = z.object({
  covenants: z
    .array(
      z.object({
        name: z.string().describe("Covenant name (e.g. Maximum Leverage)"),
        metric: CovenantMetricSchema.describe("Financial metric being tested"),
        threshold: z.coerce.number().describe("Covenant threshold value"),
        direction: z
          .enum(["MaxOf", "MinOf"])
          .describe(
            "MaxOf = actual must be <= threshold; MinOf = actual must be >= threshold"
          ),
      })
    )
    .min(1)
    .describe("List of covenant definitions to test"),
  actuals: CreditMetricsOutputSchema.describe(
    "Actual CreditMetricsOutput to test against covenants"
  ),
});
