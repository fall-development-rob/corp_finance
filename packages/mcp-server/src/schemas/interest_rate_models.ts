import { z } from "zod";

const ZeroRatePointSchema = z.object({
  maturity: z.coerce.number().describe("Time to maturity in years"),
  rate: z.coerce.number().describe("Zero rate as decimal (e.g. 0.05 = 5%)"),
});

const VasicekInputSchema = z.object({
  mean_reversion_speed: z.coerce.number().describe("Speed of mean reversion (a)"),
  long_term_rate: z.coerce.number().describe("Long-term mean rate (b)"),
  volatility: z.coerce.number().describe("Rate volatility (sigma)"),
  current_rate: z.coerce.number().describe("Current short rate (r0)"),
  time_horizon: z.coerce.number().describe("Time horizon in years"),
  time_steps: z.coerce.number().int().describe("Number of time steps"),
});

const CirInputSchema = z.object({
  mean_reversion_speed: z.coerce.number().describe("Speed of mean reversion (a)"),
  long_term_rate: z.coerce.number().describe("Long-term mean rate (b)"),
  volatility: z.coerce.number().describe("Rate volatility (sigma)"),
  current_rate: z.coerce.number().describe("Current short rate (r0)"),
  time_horizon: z.coerce.number().describe("Time horizon in years"),
  time_steps: z.coerce.number().int().describe("Number of time steps"),
});

const HullWhiteInputSchema = z.object({
  mean_reversion_speed: z.coerce.number().describe("Speed of mean reversion (a)"),
  volatility: z.coerce.number().describe("Rate volatility (sigma)"),
  current_rate: z.coerce.number().describe("Current short rate (r0)"),
  time_horizon: z.coerce.number().describe("Time horizon in years"),
  time_steps: z.coerce.number().int().describe("Number of time steps"),
  market_zero_rates: z.array(ZeroRatePointSchema).describe("Market zero rate curve"),
});

export const ShortRateSchema = z.object({
  model: z.discriminatedUnion("type", [
    z.object({ type: z.literal("Vasicek"), ...VasicekInputSchema.shape }),
    z.object({ type: z.literal("Cir"), ...CirInputSchema.shape }),
    z.object({ type: z.literal("HullWhite"), ...HullWhiteInputSchema.shape }),
  ]).describe("Short rate model selection"),
});

const MarketRateSchema = z.object({
  maturity: z.coerce.number().describe("Time to maturity in years"),
  rate: z.coerce.number().describe("Observed market rate as decimal"),
});

const NelsonSiegelParamsSchema = z.object({
  beta0: z.coerce.number().describe("Level parameter"),
  beta1: z.coerce.number().describe("Slope parameter"),
  beta2: z.coerce.number().describe("Curvature parameter"),
  lambda: z.coerce.number().describe("Decay parameter"),
});

const NelsonSiegelInputSchema = z.object({
  market_rates: z.array(MarketRateSchema).describe("Observed market rates"),
  initial_params: NelsonSiegelParamsSchema.optional().describe("Optional initial parameters"),
});

const SvenssonParamsSchema = z.object({
  beta0: z.coerce.number(),
  beta1: z.coerce.number(),
  beta2: z.coerce.number(),
  beta3: z.coerce.number(),
  lambda1: z.coerce.number(),
  lambda2: z.coerce.number(),
});

const SvenssonInputSchema = z.object({
  market_rates: z.array(MarketRateSchema).describe("Observed market rates"),
  initial_params: SvenssonParamsSchema.optional().describe("Optional initial parameters"),
});

const BootstrapInstrumentSchema = z.object({
  maturity: z.coerce.number().describe("Maturity in years"),
  coupon_rate: z.coerce.number().describe("Annual coupon rate"),
  price: z.coerce.number().describe("Market price"),
  instrument_type: z.enum(["ZeroCoupon", "ParBond", "Swap"]).describe("Instrument type"),
});

const BootstrapInputSchema = z.object({
  instruments: z.array(BootstrapInstrumentSchema).describe("Market instruments"),
});

export const TermStructureSchema = z.object({
  model: z.discriminatedUnion("type", [
    z.object({ type: z.literal("NelsonSiegel"), ...NelsonSiegelInputSchema.shape }),
    z.object({ type: z.literal("Svensson"), ...SvenssonInputSchema.shape }),
    z.object({ type: z.literal("Bootstrap"), ...BootstrapInputSchema.shape }),
  ]).describe("Term structure model selection"),
});
