import { z } from "zod";
import { CurrencySchema } from "./common.js";

export const FundFeeSchema = z.object({
  fund_size: z.number().positive().describe("Total fund size / commitments"),
  management_fee_rate: z.number().min(0).max(0.05).describe("Annual management fee rate (e.g. 0.02 for 2%)"),
  management_fee_basis: z.enum(["CommittedCapital", "InvestedCapital", "NetAssetValue"]).describe("Basis for management fee calculation"),
  performance_fee_rate: z.number().min(0).max(0.5).describe("Carried interest rate (e.g. 0.20 for 20%)"),
  hurdle_rate: z.number().min(0).max(0.2).describe("Preferred return / hurdle rate"),
  catch_up_rate: z.number().min(0).max(1).describe("GP catch-up rate (1.0 = 100% catch-up)"),
  waterfall_type: z.enum(["European", "American"]).describe("Waterfall type for carry calculation"),
  gp_commitment_pct: z.number().min(0).max(0.1).describe("GP co-investment as % of fund"),
  clawback: z.boolean().describe("Whether GP clawback provision exists"),
  fund_life_years: z.number().int().min(1).max(20).describe("Total fund life in years"),
  investment_period_years: z.number().int().min(1).max(10).describe("Investment period in years"),
  gross_irr_assumption: z.number().min(-0.5).max(1).describe("Assumed gross IRR for projections"),
  gross_moic_assumption: z.number().min(0).max(10).describe("Assumed gross MOIC for projections"),
  annual_fund_expenses: z.number().min(0).describe("Annual fund operating expenses"),
  currency: CurrencySchema.optional(),
});
