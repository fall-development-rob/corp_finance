import { z } from 'zod';
import { CompanyIdentifierSchema, PaginationSchema, DateRangeSchema } from './common.js';

export const CompanySearchSchema = z.object({
  query: z.string().min(1).describe('Company name or keyword to search for'),
}).merge(PaginationSchema);

export const TearsheetSchema = CompanyIdentifierSchema;

export const CapitalStructureSchema = CompanyIdentifierSchema.merge(DateRangeSchema);

export const OwnershipSchema = CompanyIdentifierSchema.merge(PaginationSchema);
