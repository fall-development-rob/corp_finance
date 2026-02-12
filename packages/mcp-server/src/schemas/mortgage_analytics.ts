import { z } from "zod";

const PsaInputSchema = z.object({
  psa_speed: z.coerce.number().describe("PSA speed (e.g. 150 for 150% PSA)"),
  loan_age_months: z.coerce.number().int().describe("Current loan age in months"),
  remaining_months: z.coerce.number().int().describe("Remaining months to maturity"),
  original_balance: z.coerce.number().describe("Original loan balance"),
  current_balance: z.coerce.number().describe("Current outstanding balance"),
  mortgage_rate: z.coerce.number().describe("Annual mortgage rate as decimal"),
});

const CprInputSchema = z.object({
  annual_cpr: z.coerce.number().describe("Annual conditional prepayment rate"),
  loan_age_months: z.coerce.number().int().describe("Current loan age in months"),
  remaining_months: z.coerce.number().int().describe("Remaining months"),
  original_balance: z.coerce.number().describe("Original balance"),
  current_balance: z.coerce.number().describe("Current balance"),
  mortgage_rate: z.coerce.number().describe("Annual mortgage rate"),
});

const RefinancingInputSchema = z.object({
  mortgage_rate: z.coerce.number().describe("Borrower's mortgage rate"),
  market_rate: z.coerce.number().describe("Current market mortgage rate"),
  base_cpr: z.coerce.number().describe("Base CPR without incentive"),
  incentive_multiplier: z.coerce.number().describe("Multiplier for rate differential"),
  burnout_factor: z.coerce.number().describe("Burnout decay factor (0-1)"),
  loan_age_months: z.coerce.number().int().describe("Loan age in months"),
  remaining_months: z.coerce.number().int().describe("Remaining months"),
  original_balance: z.coerce.number().describe("Original balance"),
  current_balance: z.coerce.number().describe("Current balance"),
});

export const PrepaymentSchema = z.object({
  model: z.discriminatedUnion("type", [
    z.object({ type: z.literal("Psa"), ...PsaInputSchema.shape }),
    z.object({ type: z.literal("Cpr"), ...CprInputSchema.shape }),
    z.object({ type: z.literal("Refinancing"), ...RefinancingInputSchema.shape }),
  ]).describe("Prepayment model selection"),
});

const ZeroRatePointSchema = z.object({
  maturity: z.coerce.number().describe("Maturity in years"),
  rate: z.coerce.number().describe("Zero rate"),
});

const PassThroughInputSchema = z.object({
  original_balance: z.coerce.number().describe("Original pool balance"),
  current_balance: z.coerce.number().describe("Current pool balance"),
  mortgage_rate: z.coerce.number().describe("Gross mortgage rate (WAC)"),
  pass_through_rate: z.coerce.number().describe("Net pass-through rate to investors"),
  servicing_fee: z.coerce.number().describe("Annual servicing fee rate"),
  remaining_months: z.coerce.number().int().describe("Remaining months"),
  psa_speed: z.coerce.number().describe("PSA prepayment speed"),
  settlement_delay_days: z.coerce.number().int().describe("Settlement delay in days"),
});

const OasInputSchema = z.object({
  market_price: z.coerce.number().describe("Market price of MBS"),
  pass_through_input: PassThroughInputSchema.describe("Pass-through parameters for cash flow generation"),
  benchmark_zero_rates: z.array(ZeroRatePointSchema).describe("Benchmark zero rate curve"),
  spread_search_min: z.coerce.number().optional().describe("Min spread search range"),
  spread_search_max: z.coerce.number().optional().describe("Max spread search range"),
});

const MbsDurationInputSchema = z.object({
  pass_through_input: PassThroughInputSchema.describe("Pass-through parameters"),
  yield_bps: z.coerce.number().describe("Current yield in basis points"),
  shock_bps: z.coerce.number().describe("Yield shock in basis points for duration calc"),
});

export const MbsAnalyticsSchema = z.object({
  model: z.discriminatedUnion("type", [
    z.object({ type: z.literal("PassThrough"), ...PassThroughInputSchema.shape }),
    z.object({ type: z.literal("Oas"), ...OasInputSchema.shape }),
    z.object({ type: z.literal("Duration"), ...MbsDurationInputSchema.shape }),
  ]).describe("MBS analytics model selection"),
});
