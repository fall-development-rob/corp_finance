import { z } from 'zod';

export const CountrySchema = z.object({
  country: z.string().min(1).describe('Country code (e.g., US, GB, CN) or "all"'),
});

export const IndicatorSchema = z.object({
  indicator: z.string().min(1).describe('Indicator code (e.g., NY.GDP.MKTP.CD, FP.CPI.TOTL.ZG)'),
});

export const DateRangeSchema = z.object({
  date: z.string().optional().describe('Date range (e.g., 2010:2023)'),
});

export const PaginationSchema = z.object({
  per_page: z.number().int().min(1).max(1000).default(100).describe('Results per page'),
  page: z.number().int().min(1).default(1).describe('Page number'),
});

export const CountryIndicatorSchema = CountrySchema
  .merge(IndicatorSchema)
  .merge(DateRangeSchema)
  .merge(PaginationSchema);

export const IndicatorSearchSchema = z.object({
  query: z.string().min(1).describe('Search term for indicator name or description'),
  per_page: z.number().int().min(1).max(1000).default(50).describe('Results per page'),
  page: z.number().int().min(1).default(1).describe('Page number'),
});

export const IndicatorInfoSchema = z.object({
  indicator: z.string().min(1).describe('Indicator code (e.g., NY.GDP.MKTP.CD)'),
});

export const CountryListSchema = z.object({
  per_page: z.number().int().min(1).max(1000).default(300).describe('Results per page'),
  page: z.number().int().min(1).default(1).describe('Page number'),
});

export const MultiCountrySchema = z.object({
  countries: z.string().min(1).describe('Semicolon-separated country codes (e.g., US;GB;CN)'),
  indicator: z.string().min(1).describe('Indicator code'),
  date: z.string().optional().describe('Date range (e.g., 2010:2023)'),
  per_page: z.number().int().min(1).max(1000).default(100).describe('Results per page'),
});

export const MultiIndicatorSchema = z.object({
  country: z.string().min(1).describe('Country code (e.g., US)'),
  indicators: z.string().min(1).describe('Semicolon-separated indicator codes'),
  date: z.string().optional().describe('Date range (e.g., 2010:2023)'),
  per_page: z.number().int().min(1).max(1000).default(100).describe('Results per page'),
});

export const SourceSchema = z.object({
  source: z.string().min(1).describe('Source ID number'),
  per_page: z.number().int().min(1).max(1000).default(100).describe('Results per page'),
  page: z.number().int().min(1).default(1).describe('Page number'),
});

export const CountryPopularSchema = z.object({
  country: z.string().min(1).describe('Country code (e.g., US, GB)'),
  date: z.string().optional().describe('Date range (e.g., 2010:2023)'),
  per_page: z.number().int().min(1).max(1000).default(100).describe('Results per page'),
});
