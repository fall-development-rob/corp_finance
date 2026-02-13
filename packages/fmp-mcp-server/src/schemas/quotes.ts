import { z } from 'zod';
import { SymbolSchema } from './common.js';

export const QuoteSchema = SymbolSchema;

export const BatchQuoteSchema = z.object({
  symbols: z.string().min(1).describe('Comma-separated ticker symbols (e.g., AAPL,MSFT,GOOGL)'),
});

export const HistoricalPriceSchema = SymbolSchema.extend({
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});

export const IntradaySchema = SymbolSchema.extend({
  interval: z.enum(['1min', '5min', '15min', '30min', '1hour', '4hour']).default('5min').describe('Chart interval'),
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});
