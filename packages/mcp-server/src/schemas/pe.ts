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
