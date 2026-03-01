import { z } from 'zod';

export const SymbolSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol (e.g., AAPL, MSFT)'),
});

export const BatchSymbolsSchema = z.object({
  symbols: z.string().min(1).describe('Comma-separated ticker symbols (e.g., AAPL,MSFT,GOOGL)'),
});

export const PeriodSchema = z.object({
  period: z.enum([
    '1d', '5d', '1mo', '3mo', '6mo', '1y', '2y', '5y', '10y', 'ytd', 'max',
  ]).default('1y').describe('Date range period'),
});

export const IntervalSchema = z.object({
  interval: z.enum([
    '1m', '2m', '5m', '15m', '30m', '60m', '90m', '1h', '1d', '5d', '1wk', '1mo', '3mo',
  ]).default('1d').describe('Data interval/granularity'),
});

export const HistoricalSchema = SymbolSchema.merge(PeriodSchema).merge(IntervalSchema);

export const OptionsDateSchema = SymbolSchema.extend({
  date: z.number().int().optional().describe('Expiration date as Unix epoch timestamp (seconds). Omit for nearest expiry.'),
});
