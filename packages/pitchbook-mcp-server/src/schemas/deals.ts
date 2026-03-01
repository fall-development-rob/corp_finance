import { z } from 'zod';
import { DateRangeSchema, PaginationSchema } from './common.js';

export const DealSearchSchema = z.object({
  deal_type: z.string().optional().describe('Deal type (PE, VC, M&A, IPO)'),
  industry: z.string().optional().describe('Industry vertical filter'),
  min_size: z.number().optional().describe('Minimum deal size in millions'),
  max_size: z.number().optional().describe('Maximum deal size in millions'),
}).merge(DateRangeSchema).merge(PaginationSchema);

export const DealDetailsSchema = z.object({
  deal_id: z.string().min(1).describe('PitchBook deal ID'),
});

export const ComparableDealsSchema = z.object({
  industry: z.string().min(1).describe('Industry vertical for comparable deals'),
  deal_size: z.number().optional().describe('Target deal size in millions for filtering'),
}).merge(DateRangeSchema).merge(PaginationSchema);
