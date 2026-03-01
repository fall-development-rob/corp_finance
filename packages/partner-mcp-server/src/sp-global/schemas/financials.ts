import { z } from 'zod';
import { CompanyIdentifierSchema } from './common.js';

export const FinancialsSchema = CompanyIdentifierSchema.extend({
  period: z.enum(['annual', 'quarterly']).optional().describe('Reporting period: annual or quarterly'),
  limit: z.number().int().min(1).max(40).optional().describe('Number of periods to return'),
});

export const EstimatesSchema = CompanyIdentifierSchema.extend({
  metric: z.string().optional().describe('Specific metric to retrieve (e.g., revenue, eps, ebitda)'),
});

export const SegmentDataSchema = CompanyIdentifierSchema.extend({
  fiscal_year: z.number().int().optional().describe('Fiscal year to retrieve segment data for'),
});
