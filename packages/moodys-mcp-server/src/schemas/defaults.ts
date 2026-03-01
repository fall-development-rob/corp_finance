import { z } from 'zod';
import { DateRangeSchema } from './common.js';

export const DefaultRatesSchema = z.object({
  rating: z.string().optional().describe('Rating category (e.g., Aaa, Baa1, B2)'),
  sector: z.string().optional().describe('Industry sector filter'),
  horizon: z.number().int().min(1).max(30).optional().describe('Default rate horizon in years'),
}).merge(DateRangeSchema);

export const RecoveryRatesSchema = z.object({
  seniority: z.string().optional().describe('Debt seniority (e.g., senior_secured, senior_unsecured, subordinated)'),
  sector: z.string().optional().describe('Industry sector filter'),
}).merge(DateRangeSchema);

export const TransitionMatrixSchema = z.object({
  from_rating: z.string().optional().describe('Starting rating to filter (e.g., Aaa, Baa1)'),
  horizon: z.number().int().min(1).max(20).optional().default(1).describe('Transition horizon in years (default 1)'),
});
