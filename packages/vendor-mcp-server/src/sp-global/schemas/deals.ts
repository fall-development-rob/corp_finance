import { z } from 'zod';
import { CompanyIdentifierSchema, DateRangeSchema, PaginationSchema } from './common.js';

export const MaDealsSchema = z.object({
  acquirer: z.string().optional().describe('Acquirer company name or identifier'),
  target: z.string().optional().describe('Target company name or identifier'),
  status: z.string().optional().describe('Deal status filter (e.g., completed, pending, withdrawn)'),
}).merge(DateRangeSchema).merge(PaginationSchema);

export const FundingDigestSchema = CompanyIdentifierSchema.merge(DateRangeSchema);
