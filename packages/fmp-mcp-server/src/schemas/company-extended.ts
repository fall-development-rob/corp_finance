import { z } from 'zod';

export const CikSearchSchema = z.object({
  cik: z.string().min(1).describe('CIK number (e.g., 320193)'),
});

export const CusipSearchSchema = z.object({
  cusip: z.string().min(1).describe('CUSIP identifier (e.g., 037833100)'),
});

export const IsinSearchSchema = z.object({
  isin: z.string().min(1).describe('ISIN identifier (e.g., US0378331005)'),
});

export const ExchangeVariantsSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
});

export const SymbolOnlySchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol (e.g., AAPL)'),
});

export const BatchSymbolsSchema = z.object({
  symbols: z.string().min(1).describe('Comma-separated ticker symbols'),
});

export const PageLimitSchema = z.object({
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(1000).default(100).describe('Results per page'),
});

export const DelistedSchema = z.object({
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(500).default(100).describe('Results per page'),
});

export const MaSearchSchema = z.object({
  name: z.string().min(1).describe('Company name to search M&A for'),
});

export const EmptySchema = z.object({});

export const ExchangeQuoteSchema = z.object({
  exchange: z.string().min(1).describe('Exchange code (e.g., NASDAQ, NYSE)'),
});

export const HistoricalPriceLightSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});
