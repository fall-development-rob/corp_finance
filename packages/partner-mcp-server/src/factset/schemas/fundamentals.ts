import { z } from 'zod';
import { IdentifierSchema, PaginationSchema } from './common.js';

export const FundamentalsSchema = IdentifierSchema.extend({
  metrics: z.array(z.string()).optional().describe('Fundamental metrics to retrieve (e.g., FF_SALES, FF_EBITDA, FF_NET_INC)'),
  period: z.enum(['annual', 'quarterly', 'ltm']).optional().describe('Reporting period type'),
});

export const EstimatesSchema = IdentifierSchema.extend({
  metrics: z.array(z.string()).optional().describe('Estimate metrics (e.g., FE_ESTIMATE_EPS, FE_ESTIMATE_REVENUE)'),
  periodicity: z.enum(['ANN', 'QTR']).optional().describe('Estimate periodicity: ANN=annual, QTR=quarterly'),
});

export const CompanySearchSchema = z.object({
  query: z.string().describe('Company name or keyword to search'),
  exchange: z.string().optional().describe('Filter by exchange code (e.g., NYSE, NASDAQ, LSE)'),
}).merge(PaginationSchema);
