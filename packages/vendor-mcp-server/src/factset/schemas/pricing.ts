import { z } from 'zod';
import { IdentifierSchema, DateRangeSchema } from './common.js';

export const PricesSchema = IdentifierSchema.merge(DateRangeSchema).extend({
  frequency: z.enum(['D', 'W', 'M']).optional().describe('Price frequency: D=daily, W=weekly, M=monthly'),
});

export const BondPricingSchema = IdentifierSchema;
