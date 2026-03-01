import { z } from 'zod';

export const CompanyIdentifierSchema = z.object({
  company_id: z.string().optional().describe('S&P Capital IQ company identifier'),
  ticker: z.string().optional().describe('Stock ticker symbol (e.g., AAPL, MSFT)'),
  name: z.string().optional().describe('Company name for lookup'),
});

export const PaginationSchema = z.object({
  limit: z.number().int().min(1).max(1000).default(100).describe('Max results to return'),
  offset: z.number().int().min(0).default(0).describe('Offset for pagination'),
});

export const DateRangeSchema = z.object({
  start_date: z.string().optional().describe('Start date YYYY-MM-DD'),
  end_date: z.string().optional().describe('End date YYYY-MM-DD'),
});
