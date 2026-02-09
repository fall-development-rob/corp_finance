import { z } from "zod";
import { CurrencySchema } from "./common.js";

export const ReturnsSchema = z.object({
  cash_flows: z
    .array(z.number())
    .min(2)
    .describe("Cash flow series (negative for investments, positive for proceeds)"),
  dates: z
    .array(z.string())
    .optional()
    .describe("ISO 8601 dates for XIRR (one per cash flow)"),
  guess: z
    .number()
    .min(-0.99)
    .max(10)
    .optional()
    .describe("Initial IRR guess for Newton-Raphson solver (default 0.10)"),
});

export const DebtScheduleSchema = z.object({
  tranches: z
    .array(
      z.object({
        name: z.string().describe("Tranche name (e.g. Senior Term Loan A)"),
        amount: z.number().positive().describe("Initial principal amount"),
        interest_rate: z
          .number()
          .min(0)
          .max(0.3)
          .describe("Annual interest rate"),
        is_floating: z
          .boolean()
          .optional()
          .describe("Whether rate is floating (base + spread)"),
        base_rate: z
          .number()
          .optional()
          .describe("Base rate (SOFR/SONIA) if floating"),
        spread: z
          .number()
          .optional()
          .describe("Credit spread above base rate"),
        amortisation_type: z
          .enum(["Bullet", "StraightLine", "CashSweep"])
          .describe("Amortisation schedule type"),
        amortisation_rate: z
          .number()
          .min(0)
          .max(1)
          .optional()
          .describe("Annual amortisation rate for StraightLine, sweep % for CashSweep"),
        maturity_years: z
          .number()
          .int()
          .min(1)
          .max(30)
          .describe("Years to maturity"),
        pik_rate: z
          .number()
          .min(0)
          .max(0.2)
          .optional()
          .describe("Payment-in-kind interest rate"),
        seniority: z
          .number()
          .int()
          .min(1)
          .describe("Seniority ranking (1 = most senior)"),
        is_revolver: z
          .boolean()
          .optional()
          .describe("Whether this is a revolving credit facility"),
        commitment_fee: z
          .number()
          .min(0)
          .max(0.05)
          .optional()
          .describe("Commitment fee on undrawn revolver"),
      })
    )
    .min(1)
    .describe("Debt tranches to schedule"),
  projection_years: z
    .number()
    .int()
    .min(1)
    .max(30)
    .describe("Number of years to project"),
  annual_cash_available: z
    .array(z.number().min(0))
    .optional()
    .describe("Annual free cash flow available for debt service (for cash sweep)"),
});

export const SourcesUsesSchema = z.object({
  sources: z
    .array(
      z.object({
        name: z.string().describe("Source of funds (e.g. Senior Debt, Sponsor Equity)"),
        amount: z.number().positive().describe("Amount contributed"),
      })
    )
    .min(1)
    .describe("Sources of transaction financing"),
  uses: z
    .array(
      z.object({
        name: z.string().describe("Use of funds (e.g. Enterprise Value, Transaction Fees)"),
        amount: z.number().positive().describe("Amount used"),
      })
    )
    .min(1)
    .describe("Uses of transaction financing"),
  currency: CurrencySchema.optional(),
});

export const LboSchema = z.object({
  entry_ev: z.number().positive().describe("Enterprise value at entry"),
  entry_ebitda: z.number().positive().describe("EBITDA at entry"),
  base_revenue: z.number().positive().describe("Base year revenue"),
  revenue_growth: z.array(z.number()).min(1).describe("Annual revenue growth rates"),
  ebitda_margin: z.array(z.number().min(0).max(1)).min(1).describe("Annual EBITDA margins"),
  capex_as_pct_revenue: z.number().min(0).max(0.5).describe("Capex as % of revenue"),
  nwc_as_pct_revenue: z.number().min(0).max(0.5).describe("Net working capital as % of revenue"),
  tax_rate: z.number().min(0).max(0.5).describe("Corporate tax rate"),
  da_as_pct_revenue: z.number().min(0).max(0.3).describe("D&A as % of revenue"),
  tranches: z.array(z.object({
    name: z.string(),
    amount: z.number().positive(),
    interest_rate: z.number().min(0).max(0.3),
    is_floating: z.boolean().optional(),
    base_rate: z.number().optional(),
    spread: z.number().optional(),
    amortisation: z.object({
      type: z.enum(["Bullet", "StraightLine", "CashSweep"]),
      rate: z.number().optional(),
    }),
    maturity_years: z.number().int().min(1).max(30),
    pik_rate: z.number().optional(),
    seniority: z.number().int().min(1),
    is_revolver: z.boolean().optional(),
    commitment_fee: z.number().optional(),
  })).min(1).describe("Debt tranches"),
  equity_contribution: z.number().positive().describe("Sponsor equity"),
  cash_sweep_pct: z.number().min(0).max(1).optional().describe("Cash sweep percentage"),
  exit_year: z.number().int().min(1).max(15).describe("Exit year"),
  exit_multiple: z.number().positive().describe("Exit EV/EBITDA multiple"),
  transaction_fees: z.number().optional(),
  financing_fees: z.number().optional(),
  management_rollover: z.number().optional(),
  minimum_cash: z.number().optional(),
});

export const WaterfallSchema = z.object({
  total_proceeds: z.number().min(0).describe("Total fund/deal proceeds to distribute"),
  total_invested: z.number().positive().describe("Total capital invested"),
  tiers: z.array(z.object({
    name: z.string().describe("Tier name"),
    tier_type: z.object({
      type: z.enum(["ReturnOfCapital", "PreferredReturn", "CatchUp", "CarriedInterest", "Residual"]),
      rate: z.number().optional().describe("Rate for PreferredReturn"),
      gp_share: z.number().optional().describe("GP share for CatchUp/CarriedInterest/Residual"),
    }),
  })).min(1).describe("Distribution tiers in priority order"),
  gp_commitment_pct: z.number().min(0).max(0.1).describe("GP commitment as % of fund"),
});

export const AltmanSchema = z.object({
  working_capital: z.number().describe("Working capital (current assets - current liabilities)"),
  total_assets: z.number().positive().describe("Total assets"),
  retained_earnings: z.number().describe("Retained earnings"),
  ebit: z.number().describe("Earnings before interest and taxes"),
  revenue: z.number().describe("Total revenue / sales"),
  total_liabilities: z.number().positive().describe("Total liabilities"),
  market_cap: z.number().optional().describe("Market capitalization (for public companies)"),
  book_equity: z.number().optional().describe("Book value of equity (for private companies)"),
  is_public: z.boolean().describe("Whether the company is publicly traded"),
  is_manufacturing: z.boolean().describe("Whether the company is in manufacturing"),
});
