import { z } from "zod";

const VolQuoteSchema = z.object({
  strike: z.coerce.number().positive().describe("Option strike price"),
  expiry: z.coerce.number().positive().describe("Time to expiry in years"),
  implied_vol: z.coerce.number().positive().describe("Observed implied volatility (decimal, e.g. 0.20 for 20%)"),
  option_type: z.enum(["Call", "Put"]).describe("Option type"),
  bid_vol: z.coerce.number().positive().optional().describe("Bid implied vol (optional)"),
  ask_vol: z.coerce.number().positive().optional().describe("Ask implied vol (optional)"),
});

export const ImpliedVolSurfaceSchema = z.object({
  spot_price: z.coerce.number().positive().describe("Current spot price of the underlying"),
  risk_free_rate: z.coerce.number().describe("Annualized risk-free rate (decimal)"),
  dividend_yield: z.coerce.number().describe("Continuous dividend yield (decimal)"),
  market_quotes: z.array(VolQuoteSchema).min(1).describe("Market option vol quotes"),
  interpolation_method: z.enum([
    "Linear",
    "CubicSpline",
    "SVI",
  ]).describe("Surface interpolation method"),
  extrapolation: z.boolean().describe("Whether to extrapolate beyond observed strikes/expiries"),
  target_strikes: z.array(z.coerce.number().positive()).optional().describe("Specific strikes to evaluate (optional, defaults to observed)"),
  target_expiries: z.array(z.coerce.number().positive()).optional().describe("Specific expiries to evaluate (optional, defaults to observed)"),
});

const SabrVolPointSchema = z.object({
  strike: z.coerce.number().positive().describe("Option strike price"),
  implied_vol: z.coerce.number().positive().describe("Market implied volatility at this strike"),
});

export const SabrCalibrationSchema = z.object({
  forward_price: z.coerce.number().positive().describe("Forward price of the underlying"),
  expiry: z.coerce.number().positive().describe("Time to expiry in years"),
  market_vols: z.array(SabrVolPointSchema).min(1).describe("Market implied vol observations"),
  initial_alpha: z.coerce.number().positive().optional().describe("Initial guess for alpha (optional)"),
  initial_rho: z.coerce.number().min(-0.999).max(0.999).optional().describe("Initial guess for rho correlation (-1,1) (optional)"),
  initial_nu: z.coerce.number().positive().optional().describe("Initial guess for nu vol-of-vol (optional)"),
  beta: z.coerce.number().min(0).max(1).describe("SABR beta parameter (0=normal, 0.5=CIR, 1=lognormal)"),
  target_strikes: z.array(z.coerce.number().positive()).optional().describe("Strikes at which to evaluate calibrated model (optional)"),
});
