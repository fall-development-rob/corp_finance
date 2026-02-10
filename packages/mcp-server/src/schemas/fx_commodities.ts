import { z } from "zod";

export const FxForwardSchema = z.object({
  spot_rate: z.coerce.number().positive().describe("Current spot FX rate (domestic per foreign, e.g. 1.10 USD/EUR)"),
  domestic_rate: z.coerce.number().min(-0.1).max(0.3).describe("Domestic risk-free rate (annualised, decimal)"),
  foreign_rate: z.coerce.number().min(-0.1).max(0.3).describe("Foreign risk-free rate (annualised, decimal)"),
  time_to_expiry: z.coerce.number().positive().describe("Time to delivery in years"),
  notional_foreign: z.coerce.number().positive().describe("Notional amount in foreign currency"),
  forward_type: z.enum(["Deliverable", "NonDeliverable"]).describe("Deliverable or non-deliverable (NDF)"),
});

export const CrossRateSchema = z.object({
  rate1: z.coerce.number().positive().describe("First exchange rate, e.g. 1.10"),
  rate1_pair: z.string().describe("First pair label, e.g. USD/EUR"),
  rate2: z.coerce.number().positive().describe("Second exchange rate, e.g. 150.0"),
  rate2_pair: z.string().describe("Second pair label, e.g. USD/JPY"),
  target_pair: z.string().describe("Target pair, e.g. EUR/JPY"),
});

export const CommodityForwardSchema = z.object({
  spot_price: z.coerce.number().positive().describe("Current spot price"),
  risk_free_rate: z.coerce.number().min(0).max(0.2).describe("Annualised risk-free rate"),
  storage_cost_rate: z.coerce.number().min(0).max(0.2).describe("Annual storage cost as % of spot"),
  convenience_yield: z.coerce.number().min(0).max(0.5).describe("Annual convenience yield"),
  time_to_expiry: z.coerce.number().positive().describe("Time to expiry in years"),
  commodity_type: z.enum(["Energy", "Metals", "Agriculture", "Precious"]).describe("Type of commodity"),
});

export const CommodityCurveSchema = z.object({
  spot_price: z.coerce.number().positive().describe("Current spot price"),
  futures_prices: z.array(z.object({
    expiry_months: z.coerce.number().int().min(1).describe("Months until expiry"),
    price: z.coerce.number().positive().describe("Observed futures price"),
    open_interest: z.coerce.number().int().min(0).optional().describe("Open interest (optional)"),
  })).min(1).describe("Futures term structure sorted by expiry"),
  risk_free_rate: z.coerce.number().min(0).max(0.2).describe("Annualised risk-free rate"),
  storage_cost_rate: z.coerce.number().min(0).max(0.2).describe("Annual storage cost as % of spot"),
});
