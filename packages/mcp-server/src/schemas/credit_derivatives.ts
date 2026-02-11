import { z } from "zod";

export const CdsPricingSchema = z.object({
  reference_entity: z.string().describe("Reference entity name"),
  notional: z.coerce.number().positive().describe("Notional amount"),
  spread_bps: z.coerce.number().positive().describe("CDS spread in basis points"),
  recovery_rate: z.coerce.number().min(0).max(1).describe("Expected recovery rate (0-1)"),
  risk_free_rate: z.coerce.number().describe("Risk-free discount rate"),
  maturity_years: z.coerce.number().int().positive().describe("CDS tenor in years (1-30)"),
  payment_frequency: z.coerce.number().int().positive().describe("Premium payments per year (1, 2, or 4)"),
  default_probability: z.coerce.number().min(0).max(1).optional().describe("Annual hazard rate / default probability"),
  market_spread_bps: z.coerce.number().optional().describe("Market spread for MTM calculation"),
  counterparty_rating: z.string().optional().describe("Counterparty credit rating"),
});

const ExposurePointSchema = z.object({
  time_years: z.coerce.number().min(0).describe("Time in years"),
  expected_exposure: z.coerce.number().min(0).describe("Expected positive exposure"),
  potential_future_exposure: z.coerce.number().min(0).optional().describe("Potential future exposure"),
});

export const CvaCalculationSchema = z.object({
  trade_description: z.string().describe("Trade description"),
  expected_exposure_profile: z.array(ExposurePointSchema).min(1).describe("Time-bucketed expected positive exposure"),
  counterparty_default_probability: z.coerce.number().min(0).max(1).describe("Annual PD of counterparty"),
  counterparty_recovery_rate: z.coerce.number().min(0).max(1).describe("Counterparty recovery rate"),
  own_default_probability: z.coerce.number().min(0).max(1).optional().describe("Own PD for DVA"),
  own_recovery_rate: z.coerce.number().min(0).max(1).optional().describe("Own recovery rate"),
  risk_free_rate: z.coerce.number().describe("Risk-free discount rate"),
  netting_benefit: z.coerce.number().min(0).max(1).optional().describe("Netting benefit reduction ratio"),
  collateral_threshold: z.coerce.number().min(0).optional().describe("Collateral posting threshold"),
});
