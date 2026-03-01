import { z } from 'zod';
import { IdentifierSchema, DateRangeSchema, PaginationSchema } from './common.js';

export const ReferenceDataSchema = IdentifierSchema;

export const CorporateActionsSchema = IdentifierSchema.merge(DateRangeSchema).extend({
  type: z.string().optional().describe('Corporate action type filter (e.g., dividend, split, merger, spinoff)'),
});

export const OwnershipSchema = IdentifierSchema.merge(PaginationSchema);
