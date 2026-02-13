import { z } from 'zod';

export const SymbolSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol (e.g., AAPL, MSFT)'),
});

export const SymbolPeriodSchema = SymbolSchema.extend({
  period: z.enum(['annual', 'quarter']).default('annual').describe('Reporting period'),
  limit: z.number().int().min(1).max(120).default(4).describe('Number of periods to return'),
});

export const SymbolLimitSchema = SymbolSchema.extend({
  limit: z.number().int().min(1).max(500).default(10).describe('Maximum results'),
});

export const DateRangeSchema = z.object({
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});

export const SearchSchema = z.object({
  query: z.string().min(1).describe('Search query (ticker or company name)'),
  limit: z.number().int().min(1).max(100).default(10).describe('Maximum results'),
  exchange: z.string().optional().describe('Filter by exchange (e.g., NYSE, NASDAQ)'),
});

export const ScreenerSchema = z.object({
  market_cap_more_than: z.number().optional().describe('Minimum market cap'),
  market_cap_less_than: z.number().optional().describe('Maximum market cap'),
  sector: z.string().optional().describe('Sector filter'),
  industry: z.string().optional().describe('Industry filter'),
  exchange: z.string().optional().describe('Exchange filter (NYSE, NASDAQ, etc.)'),
  country: z.string().optional().describe('Country filter (US, GB, etc.)'),
  limit: z.number().int().min(1).max(1000).default(50).describe('Maximum results'),
});
