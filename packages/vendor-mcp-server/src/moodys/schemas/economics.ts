import { z } from 'zod';
import { DateRangeSchema } from './common.js';

export const EconomicForecastSchema = z.object({
  country: z.string().describe('Country name or ISO code (e.g., US, GB, DE)'),
  indicator: z.string().optional().describe('Economic indicator (e.g., GDP, CPI, unemployment)'),
}).merge(DateRangeSchema);

export const CountryRiskSchema = z.object({
  country: z.string().describe('Country name or ISO code'),
});

export const IndustryOutlookSchema = z.object({
  industry: z.string().describe('Industry sector (e.g., banking, energy, healthcare)'),
  region: z.string().optional().describe('Geographic region filter (e.g., North America, EMEA, APAC)'),
});
