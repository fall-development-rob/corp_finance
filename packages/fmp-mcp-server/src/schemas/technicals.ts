import { z } from 'zod';

export const TechnicalIndicatorSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol (e.g., AAPL)'),
  periodLength: z.number().int().min(1).max(200).default(14).describe('Indicator period length'),
  timeframe: z.enum(['1min', '5min', '15min', '30min', '1hour', '4hour', '1day', '1week', '1month']).default('1day').describe('Chart timeframe'),
});
