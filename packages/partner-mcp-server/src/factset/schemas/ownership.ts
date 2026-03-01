import { z } from 'zod';
import { IdentifierSchema, PaginationSchema } from './common.js';

export const OwnershipSchema = IdentifierSchema.merge(PaginationSchema);

export const InstitutionalSchema = IdentifierSchema.extend({
  holder_type: z.string().optional().describe('Filter by holder type (e.g., mutual_fund, hedge_fund, pension)'),
}).merge(PaginationSchema);
