import { z } from 'zod';

export const SectorPerformanceSchema = z.object({
  date: z.string().optional().describe('Date for snapshot (YYYY-MM-DD), defaults to latest'),
});

export const IndustryPerformanceSchema = z.object({
  date: z.string().optional().describe('Date for snapshot (YYYY-MM-DD), defaults to latest'),
});

export const IndexConstituentsSchema = z.object({
  index: z.enum(['sp500', 'nasdaq', 'dowjones']).default('sp500').describe('Market index'),
});

export const EconomicIndicatorSchema = z.object({
  name: z.string().min(1).describe('Indicator name (GDP, CPI, unemployment, etc.)'),
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});

export const TreasuryRatesSchema = z.object({
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});

export const EconomicCalendarSchema = z.object({
  from: z.string().optional().describe('Start date (YYYY-MM-DD)'),
  to: z.string().optional().describe('End date (YYYY-MM-DD)'),
});
