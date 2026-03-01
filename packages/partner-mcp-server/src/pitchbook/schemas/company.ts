import { z } from 'zod';
import { EntityIdentifierSchema, PaginationSchema } from './common.js';

export const CompanySearchSchema = z.object({
  query: z.string().min(1).describe('Company name or keyword search'),
  industry: z.string().optional().describe('Filter by industry vertical'),
}).merge(PaginationSchema);

export const CompanyProfileSchema = EntityIdentifierSchema;
