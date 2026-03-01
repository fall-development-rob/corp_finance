import { z } from 'zod';

export const IdentifierSchema = z.object({
  ric: z.string().optional().describe('Reuters Instrument Code (e.g., AAPL.O, VOD.L, IBM.N)'),
  isin: z.string().optional().describe('International Securities Identification Number (e.g., US0378331005)'),
  sedol: z.string().optional().describe('Stock Exchange Daily Official List code (e.g., 2046251)'),
});

export const DateRangeSchema = z.object({
  start_date: z.string().optional().describe('Start date in YYYY-MM-DD format'),
  end_date: z.string().optional().describe('End date in YYYY-MM-DD format'),
});

export const PaginationSchema = z.object({
  limit: z.number().int().min(1).max(10000).default(100).describe('Maximum number of results to return'),
  offset: z.number().int().min(0).default(0).describe('Number of results to skip for pagination'),
});
