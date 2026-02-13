import { z } from 'zod';

export const HistoricalSectorSchema = z.object({
  sector: z.string().min(1).describe('Sector name (e.g., Energy, Technology, Healthcare)'),
});

export const HistoricalIndustrySchema = z.object({
  industry: z.string().min(1).describe('Industry name (e.g., Biotechnology, Software)'),
});

export const PeSnapshotSchema = z.object({
  date: z.string().optional().describe('Date for snapshot (YYYY-MM-DD), defaults to latest'),
});

export const ExchangeSchema = z.object({
  exchange: z.string().min(1).describe('Exchange code (e.g., NASDAQ, NYSE, LSE)'),
});

export const HistoricalIndexSchema = z.object({
  index: z.enum(['sp500', 'nasdaq', 'dowjones']).describe('Market index'),
});

export const EmptySchema = z.object({});
