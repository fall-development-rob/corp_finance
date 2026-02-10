import { z } from "zod";
import { CashFlowSchema, CurrencySchema } from "./common.js";

// --- ReturnsInput ---
// Rust struct: ReturnsInput in pe/returns.rs
// Fields: cash_flows, dated_cash_flows?, entry_equity, exit_equity,
//         holding_period_years?, dates?
export const ReturnsSchema = z.object({
  cash_flows: z
    .array(z.coerce.number())
    .describe(
      "Periodic cash flows for IRR calculation (index 0 = initial investment, negative)"
    ),
  dated_cash_flows: z
    .array(CashFlowSchema)
    .optional()
    .describe("Dated cash flows for XIRR calculation"),
  entry_equity: z.coerce.number().describe("Equity invested at entry"),
  exit_equity: z.coerce.number().describe("Equity received at exit"),
  holding_period_years: z
    .number()
    .optional()
    .describe("Holding period in years (for periodic IRR)"),
  dates: z
    .tuple([z.string(), z.string()])
    .optional()
    .describe(
      "Entry and exit dates as [entry, exit] ISO 8601 strings for XIRR and date-based holding period"
    ),
});

// --- AmortisationType ---
// Rust enum (externally tagged serde default):
//   Bullet           -> "Bullet"
//   StraightLine(r)  -> { "StraightLine": r }
//   Custom(vec)      -> { "Custom": [amounts] }
//   CashSweep(r)     -> { "CashSweep": r }
const AmortisationTypeSchema = z.union([
  z.literal("Bullet"),
  z.object({ StraightLine: z.coerce.number() }),
  z.object({ Custom: z.array(z.coerce.number()) }),
  z.object({ CashSweep: z.coerce.number() }),
]);

// --- DebtTrancheInput ---
// Rust struct: DebtTrancheInput in pe/debt_schedule.rs
// napi binding build_debt_schedule deserializes directly to DebtTrancheInput (single tranche)
const DebtTrancheSchema = z.object({
  name: z.string().describe("Tranche name (e.g. Senior Term Loan A)"),
  amount: z.coerce.number().positive().describe("Initial principal amount"),
  interest_rate: z.coerce.number().min(0).describe("Annual interest rate (decimal)"),
  is_floating: z.coerce.boolean().describe("Whether rate is floating (base + spread)"),
  base_rate: z
    .number()
    .optional()
    .describe("Base rate (SOFR/SONIA) if floating"),
  spread: z
    .number()
    .optional()
    .describe("Credit spread above base rate"),
  amortisation: AmortisationTypeSchema.describe(
    'Amortisation type: "Bullet", {"StraightLine": rate}, {"Custom": [amounts]}, or {"CashSweep": rate}'
  ),
  maturity_years: z
    .number()
    .int()
    .min(1)
    .describe("Years to maturity"),
  pik_rate: z
    .number()
    .optional()
    .describe("Payment-in-kind interest rate"),
  seniority: z
    .number()
    .int()
    .min(1)
    .describe("Seniority ranking (1 = most senior)"),
  commitment_fee: z
    .number()
    .optional()
    .describe("Commitment fee on undrawn revolver"),
  is_revolver: z
    .boolean()
    .describe("Whether this is a revolving credit facility"),
});

// DebtScheduleSchema matches the single DebtTrancheInput struct that the
// napi binding deserializes into.
export const DebtScheduleSchema = DebtTrancheSchema;

// --- SourcesUsesInput ---
// Rust struct: SourcesUsesInput in pe/sources_uses.rs
// Fields: enterprise_value, equity_contribution, debt_tranches (Vec<(String, Money)>),
//         transaction_fees?, financing_fees?, management_rollover?
export const SourcesUsesSchema = z.object({
  enterprise_value: z
    .number()
    .positive()
    .describe("Enterprise value of the target"),
  equity_contribution: z
    .number()
    .min(0)
    .describe("Equity contribution from sponsor"),
  debt_tranches: z
    .array(z.tuple([z.string(), z.coerce.number()]))
    .describe("Debt tranches as [name, amount] tuples"),
  transaction_fees: z
    .number()
    .optional()
    .describe("Transaction advisory fees"),
  financing_fees: z
    .number()
    .optional()
    .describe("Debt financing/arrangement fees"),
  management_rollover: z
    .number()
    .optional()
    .describe("Management equity rollover"),
});

// --- LboInput ---
// Rust struct: LboInput in pe/lbo.rs
export const LboSchema = z.object({
  entry_ev: z.coerce.number().positive().describe("Enterprise value at entry"),
  entry_ebitda: z.coerce.number().positive().describe("EBITDA at entry"),
  revenue_growth: z
    .array(z.coerce.number())
    .min(1)
    .describe("Annual revenue growth rates (decimal, e.g. 0.05 = 5%)"),
  ebitda_margin: z
    .array(z.coerce.number())
    .min(1)
    .describe("Annual EBITDA margins (decimal, e.g. 0.20 = 20%)"),
  capex_as_pct_revenue: z
    .number()
    .min(0)
    .describe("Capex as percentage of revenue"),
  nwc_as_pct_revenue: z
    .number()
    .min(0)
    .describe("Net working capital change as percentage of revenue"),
  tax_rate: z.coerce.number().min(0).max(1).describe("Corporate tax rate"),
  da_as_pct_revenue: z
    .number()
    .min(0)
    .describe("Depreciation & amortisation as percentage of revenue"),
  base_revenue: z.coerce.number().positive().describe("Revenue in the base year (year 0)"),
  tranches: z
    .array(DebtTrancheSchema)
    .min(1)
    .describe("Debt tranches in seniority order"),
  equity_contribution: z
    .number()
    .positive()
    .describe("Sponsor equity contribution"),
  cash_sweep_pct: z
    .number()
    .min(0)
    .max(1)
    .optional()
    .describe("Percentage of excess FCF used for mandatory cash sweep repayment"),
  exit_year: z.coerce.number().int().min(1).describe("Exit year (e.g. 5 for a 5-year hold)"),
  exit_multiple: z.coerce.number().positive().describe("Exit EV/EBITDA multiple"),
  transaction_fees: z
    .number()
    .optional()
    .describe("Transaction advisory fees"),
  financing_fees: z
    .number()
    .optional()
    .describe("Debt financing/arrangement fees"),
  management_rollover: z
    .number()
    .optional()
    .describe("Management equity rollover"),
  currency: CurrencySchema.optional().describe("Currency code"),
  minimum_cash: z
    .number()
    .optional()
    .describe("Minimum cash balance to maintain before optional repayments"),
});

// --- WaterfallTierType ---
// Rust enum (externally tagged serde default):
//   ReturnOfCapital             -> "ReturnOfCapital"
//   PreferredReturn { rate }    -> { "PreferredReturn": { "rate": 0.08 } }
//   CatchUp { gp_share }       -> { "CatchUp": { "gp_share": 1.0 } }
//   CarriedInterest { gp_share} -> { "CarriedInterest": { "gp_share": 0.20 } }
//   Residual { gp_share }      -> { "Residual": { "gp_share": 0.20 } }
const WaterfallTierTypeSchema = z.union([
  z.literal("ReturnOfCapital"),
  z.object({ PreferredReturn: z.object({ rate: z.coerce.number() }) }),
  z.object({ CatchUp: z.object({ gp_share: z.coerce.number() }) }),
  z.object({ CarriedInterest: z.object({ gp_share: z.coerce.number() }) }),
  z.object({ Residual: z.object({ gp_share: z.coerce.number() }) }),
]);

// --- WaterfallInput ---
// Rust struct: WaterfallInput in pe/waterfall.rs
// Fields: total_proceeds, total_invested, tiers (Vec<WaterfallTier>), gp_commitment_pct
export const WaterfallSchema = z.object({
  total_proceeds: z
    .number()
    .min(0)
    .describe("Total exit proceeds available for distribution"),
  total_invested: z
    .number()
    .positive()
    .describe("Total capital invested by the fund"),
  tiers: z
    .array(
      z.object({
        name: z.string().describe("Human-readable tier name"),
        tier_type: WaterfallTierTypeSchema.describe("Distribution logic for this tier"),
      })
    )
    .min(1)
    .describe("Ordered waterfall tiers (executed top-to-bottom)"),
  gp_commitment_pct: z
    .number()
    .min(0)
    .max(1)
    .describe("GP commitment as a fraction of fund (typically 0.01 - 0.05)"),
});

// --- AltmanInput ---
// Rust struct: AltmanInput in credit/altman.rs
// Registered via tools/pe.ts for historical reasons.
export const AltmanSchema = z.object({
  working_capital: z
    .number()
    .describe("Working capital (current assets - current liabilities)"),
  total_assets: z.coerce.number().positive().describe("Total assets"),
  retained_earnings: z.coerce.number().describe("Retained earnings"),
  ebit: z.coerce.number().describe("Earnings before interest and taxes"),
  revenue: z.coerce.number().describe("Total revenue / sales"),
  total_liabilities: z.coerce.number().positive().describe("Total liabilities"),
  market_cap: z
    .number()
    .optional()
    .describe("Market capitalization (for public companies)"),
  book_equity: z
    .number()
    .optional()
    .describe("Book value of equity (for private companies)"),
  is_public: z.coerce.boolean().describe("Whether the company is publicly traded"),
  is_manufacturing: z
    .boolean()
    .describe("Whether the company is in manufacturing"),
});
