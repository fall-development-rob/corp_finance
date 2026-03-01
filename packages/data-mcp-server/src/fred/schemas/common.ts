import { z } from 'zod';

export const SeriesIdSchema = z.object({
  series_id: z.string().min(1).describe('FRED series ID (e.g., DGS10, CPIAUCSL, FEDFUNDS)'),
});

export const DateRangeSchema = z.object({
  observation_start: z.string().optional().describe('Start date YYYY-MM-DD'),
  observation_end: z.string().optional().describe('End date YYYY-MM-DD'),
});

export const LimitSchema = z.object({
  limit: z.number().int().min(1).max(100000).default(1000).describe('Max observations'),
});

export const FrequencySchema = z.object({
  frequency: z.enum(['d', 'w', 'bw', 'm', 'q', 'sa', 'a']).optional().describe('Frequency aggregation: d=daily, w=weekly, bw=biweekly, m=monthly, q=quarterly, sa=semiannual, a=annual'),
});

export const SearchSchema = z.object({
  search_text: z.string().min(1).describe('Search keywords'),
  limit: z.number().int().min(1).max(1000).default(50).describe('Max results'),
});
