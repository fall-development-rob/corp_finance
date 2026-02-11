import { z } from "zod";

const CommodityPriceSchema = z.object({
  name: z.string(),
  price: z.coerce.number(),
  unit: z.string(),
  volume: z.coerce.number(),
});

export const CommoditySpreadSchema = z.object({
  spread_type: z.enum(["Crack", "Crush", "Spark", "Calendar", "Location", "Quality"]),
  input_prices: z.array(CommodityPriceSchema),
  output_prices: z.array(CommodityPriceSchema),
  conversion_ratios: z.array(z.coerce.number()),
  processing_cost: z.coerce.number().optional(),
  fixed_costs: z.coerce.number().optional(),
  capacity_utilization: z.coerce.number().optional(),
  heat_rate: z.coerce.number().optional(),
  carbon_price: z.coerce.number().optional(),
  emission_factor: z.coerce.number().optional(),
  historical_spreads: z.array(z.coerce.number()).optional(),
});

const FuturesPriceSchema = z.object({
  month: z.coerce.number(),
  price: z.coerce.number(),
  open_interest: z.coerce.number().optional(),
});

const SeasonalFactorSchema = z.object({
  month: z.coerce.number(),
  factor: z.coerce.number(),
});

export const StorageEconomicsSchema = z.object({
  spot_price: z.coerce.number(),
  futures_prices: z.array(FuturesPriceSchema),
  storage_cost_per_unit_month: z.coerce.number(),
  financing_rate: z.coerce.number(),
  insurance_cost_pct: z.coerce.number().optional(),
  handling_cost: z.coerce.number().optional(),
  max_storage_capacity: z.coerce.number().optional(),
  current_inventory: z.coerce.number().optional(),
  injection_rate: z.coerce.number().optional(),
  withdrawal_rate: z.coerce.number().optional(),
  seasonal_factors: z.array(SeasonalFactorSchema).optional(),
  commodity_name: z.string(),
});
