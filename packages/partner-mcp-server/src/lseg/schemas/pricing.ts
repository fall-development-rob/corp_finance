import { z } from 'zod';
import { IdentifierSchema, DateRangeSchema } from './common.js';

export const HistoricalPricingSchema = IdentifierSchema.merge(DateRangeSchema).extend({
  interval: z.enum(['daily', 'weekly', 'monthly']).optional().describe('Price bar interval'),
});

export const IntradayPricingSchema = IdentifierSchema.extend({
  interval: z.enum(['1min', '5min', '15min', '1hr']).optional().describe('Intraday bar interval'),
});

export const BondPricingSchema = IdentifierSchema;

export const FxRatesSchema = z.object({
  base: z.string().describe('Base currency code (e.g., USD, EUR, GBP)'),
  quotes: z.array(z.string()).optional().describe('Quote currency codes (e.g., ["EUR","GBP","JPY"])'),
}).merge(DateRangeSchema);
