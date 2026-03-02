import { z } from 'zod';
import { IdentifierSchema, PaginationSchema, DateRangeSchema } from './common.js';

export const CompanySearchSchema = z.object({
  query: z.string().min(1).describe('Company name or ticker to search for'),
  exchange: z.string().optional().describe('Filter by exchange (e.g., NYSE, LSE, XETRA)'),
}).merge(PaginationSchema);

export const FundamentalsSchema = IdentifierSchema.extend({
  period: z.enum(['annual', 'quarterly']).optional().describe('Reporting period type'),
  limit: z.number().int().min(1).max(100).optional().describe('Number of periods to return'),
});

export const EsgScoresSchema = IdentifierSchema;

export const NewsSchema = z.object({
  query: z.string().optional().describe('Search keywords for news articles'),
  ric: z.string().optional().describe('Filter news by Reuters Instrument Code'),
}).merge(PaginationSchema);

export const OptionsChainSchema = IdentifierSchema.extend({
  expiration: z.string().optional().describe('Options expiration date YYYY-MM-DD'),
});

export const EconomicIndicatorsSchema = z.object({
  country: z.string().describe('Country code (e.g., US, GB, DE, JP)'),
  indicator: z.string().optional().describe('Specific indicator code (e.g., GDP, CPI, UNEMP)'),
}).merge(DateRangeSchema);
