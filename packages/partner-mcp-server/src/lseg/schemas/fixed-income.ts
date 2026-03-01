import { z } from 'zod';
import { DateRangeSchema } from './common.js';

export const YieldCurveSchema = z.object({
  currency: z.string().default('USD').describe('Currency for the yield curve (e.g., USD, EUR, GBP)'),
  date: z.string().optional().describe('Curve date in YYYY-MM-DD format (defaults to latest)'),
});

export const CreditSpreadsSchema = z.object({
  rating: z.string().optional().describe('Credit rating filter (e.g., AAA, AA, A, BBB, BB, B)'),
  sector: z.string().optional().describe('Industry sector filter (e.g., Financials, Technology, Energy)'),
}).merge(DateRangeSchema);
