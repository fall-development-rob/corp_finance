import { z } from 'zod';
import { CompanyIdentifierSchema, DateRangeSchema, PaginationSchema } from './common.js';

export const TranscriptSchema = CompanyIdentifierSchema.extend({
  quarter: z.string().optional().describe('Fiscal quarter (e.g., Q1, Q2, Q3, Q4)'),
  year: z.number().int().optional().describe('Fiscal year for the transcript'),
});

export const CreditRatingSchema = CompanyIdentifierSchema;

export const PeerAnalysisSchema = CompanyIdentifierSchema.extend({
  metric: z.string().optional().describe('Comparison metric (e.g., revenue_growth, ebitda_margin, roe)'),
});

export const KeyDevSchema = CompanyIdentifierSchema
  .merge(DateRangeSchema)
  .merge(PaginationSchema);

export const IndustryBenchmarkSchema = z.object({
  industry: z.string().min(1).describe('Industry name or GICS code'),
  metric: z.string().optional().describe('Benchmark metric (e.g., ev_ebitda, pe_ratio, revenue_growth)'),
});
