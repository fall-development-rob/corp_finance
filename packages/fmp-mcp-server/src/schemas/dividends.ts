import { z } from 'zod';

export const DividendsSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol (e.g., AAPL)'),
});

export const DividendsCalendarSchema = z.object({
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});

export const SplitsSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
});

export const SplitsCalendarSchema = z.object({
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});

export const IpoCalendarSchema = z.object({
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});

export const EarningsTranscriptLatestSchema = z.object({
  page: z.number().int().min(0).default(0).describe('Page number'),
  limit: z.number().int().min(1).max(100).default(20).describe('Results per page'),
});

export const EarningsTranscriptDatesSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol'),
});
