import { z } from "zod";

export const StructuredNoteSchema = z.object({
  note_type: z.enum(["CapitalProtected", "YieldEnhancement", "Participation", "CreditLinked"]).describe("Note type"),
  notional: z.coerce.number().positive().describe("Notional amount"),
  maturity_years: z.coerce.number().positive().describe("Maturity in years"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate"),
  underlying_price: z.coerce.number().positive().describe("Underlying asset price"),
  underlying_volatility: z.coerce.number().min(0).describe("Underlying volatility"),
  protection_level: z.coerce.number().optional().describe("Capital protection level"),
  participation_rate: z.coerce.number().optional().describe("Participation rate"),
  cap_level: z.coerce.number().optional().describe("Cap level"),
  barrier_level: z.coerce.number().optional().describe("Barrier level"),
  coupon_rate: z.coerce.number().optional().describe("Coupon rate"),
  strike_pct: z.coerce.number().optional().describe("Strike as pct of spot"),
  floor_level: z.coerce.number().optional().describe("Floor level"),
  reference_entity: z.string().optional().describe("Reference entity for credit-linked"),
  credit_spread_bps: z.coerce.number().optional().describe("Credit spread in basis points"),
  recovery_rate: z.coerce.number().optional().describe("Recovery rate"),
  default_probability: z.coerce.number().optional().describe("Default probability"),
});

const AutocallableParamsSchema = z.object({
  notional: z.coerce.number().positive().describe("Notional amount"),
  underlying_price: z.coerce.number().positive().describe("Underlying price"),
  volatility: z.coerce.number().min(0).describe("Volatility"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate"),
  dividend_yield: z.coerce.number().optional().describe("Dividend yield"),
  maturity_years: z.coerce.number().positive().describe("Maturity in years"),
  observation_frequency: z.coerce.number().int().positive().describe("Observations per year"),
  autocall_barrier: z.coerce.number().describe("Autocall barrier as fraction of initial price"),
  coupon_per_period: z.coerce.number().describe("Coupon per observation period"),
  coupon_barrier: z.coerce.number().describe("Coupon barrier as fraction of initial price"),
  ki_barrier: z.coerce.number().describe("Knock-in barrier as fraction of initial price"),
  ki_strike: z.coerce.number().describe("Put strike if knock-in triggered"),
});

const BarrierOptionParamsSchema = z.object({
  spot: z.coerce.number().positive().describe("Spot price"),
  strike: z.coerce.number().positive().describe("Strike price"),
  barrier: z.coerce.number().positive().describe("Barrier level"),
  barrier_type: z.enum(["UpAndIn", "UpAndOut", "DownAndIn", "DownAndOut"]).describe("Barrier type"),
  option_type: z.string().describe("Call or Put"),
  volatility: z.coerce.number().min(0).describe("Volatility"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate"),
  dividend_yield: z.coerce.number().optional().describe("Dividend yield"),
  time_to_expiry: z.coerce.number().positive().describe("Time to expiry in years"),
  rebate: z.coerce.number().optional().describe("Rebate amount"),
});

const DigitalOptionParamsSchema = z.object({
  spot: z.coerce.number().positive().describe("Spot price"),
  strike: z.coerce.number().positive().describe("Strike price"),
  digital_type: z.enum(["CashOrNothing", "AssetOrNothing"]).describe("Digital type"),
  option_type: z.string().describe("Call or Put"),
  payout: z.coerce.number().positive().describe("Payout amount"),
  volatility: z.coerce.number().min(0).describe("Volatility"),
  risk_free_rate: z.coerce.number().describe("Risk-free rate"),
  dividend_yield: z.coerce.number().optional().describe("Dividend yield"),
  time_to_expiry: z.coerce.number().positive().describe("Time to expiry in years"),
});

export const ExoticProductSchema = z.object({
  product_type: z.enum(["Autocallable", "BarrierOption", "DigitalOption"]).describe("Exotic product type"),
  autocallable: AutocallableParamsSchema.optional().describe("Autocallable parameters"),
  barrier_option: BarrierOptionParamsSchema.optional().describe("Barrier option parameters"),
  digital_option: DigitalOptionParamsSchema.optional().describe("Digital option parameters"),
});
