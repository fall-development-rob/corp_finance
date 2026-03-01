import { z } from 'zod';
import { DateRangeSchema } from './common.js';

export const YieldCurveSchema = DateRangeSchema.merge(z.object({
  limit: z.number().int().min(1).max(1000).default(1).describe('Number of observation dates to return (most recent first)'),
}));

export const SpreadSchema = z.object({
  series_id_long: z.string().min(1).describe('FRED series ID for the long-term rate (e.g., DGS10)'),
  series_id_short: z.string().min(1).describe('FRED series ID for the short-term rate (e.g., DGS2)'),
  observation_start: z.string().optional().describe('Start date YYYY-MM-DD'),
  observation_end: z.string().optional().describe('End date YYYY-MM-DD'),
  limit: z.number().int().min(1).max(100000).default(100).describe('Max observations'),
});
