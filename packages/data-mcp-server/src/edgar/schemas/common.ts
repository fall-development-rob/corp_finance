import { z } from 'zod';

export const CikSchema = z.object({
  cik: z.string().min(1).describe('SEC CIK number (e.g., 0000320193 for Apple)'),
});

export const TickerSchema = z.object({
  ticker: z.string().min(1).describe('Stock ticker symbol'),
});

export const FormTypeSchema = z.object({
  form_type: z.string().optional().describe('SEC form type filter (10-K, 10-Q, 8-K, etc.)'),
});

export const TaxonomySchema = z.object({
  taxonomy: z.enum(['us-gaap', 'ifrs-full', 'dei', 'srt']).default('us-gaap').describe('XBRL taxonomy'),
});

export const ConceptSchema = z.object({
  concept: z.string().min(1).describe('XBRL concept (e.g., Revenues, NetIncomeLoss, Assets)'),
});

export const LimitSchema = z.object({
  limit: z.number().int().min(1).max(500).default(10).describe('Maximum results to return'),
});

export const YearSchema = z.object({
  year: z.number().int().min(2009).describe('Calendar year (e.g., 2024)'),
});

export const DateRangeSchema = z.object({
  start_date: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  end_date: z.string().optional().describe('End date (YYYY-MM-DD)'),
});

export const AccessionSchema = z.object({
  accession_number: z.string().min(1).describe('SEC accession number (e.g., 0000320193-23-000106)'),
});

export const SearchQuerySchema = z.object({
  query: z.string().min(1).describe('Full-text search query'),
});
