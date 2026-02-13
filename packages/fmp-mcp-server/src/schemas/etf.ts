import { z } from 'zod';

export const EtfSymbolSchema = z.object({
  symbol: z.string().min(1).describe('ETF or mutual fund ticker symbol (e.g., SPY, VWO)'),
});

export const EtfAssetExposureSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker to find ETF exposure for (e.g., AAPL)'),
});

export const FundDisclosureSchema = z.object({
  symbol: z.string().min(1).describe('Fund ticker symbol'),
  year: z.number().int().min(2000).describe('Disclosure year'),
  quarter: z.number().int().min(1).max(4).describe('Disclosure quarter (1-4)'),
});

export const FundDisclosureSearchSchema = z.object({
  name: z.string().min(1).describe('Fund name to search for'),
});

export const FundDisclosureDatesSchema = z.object({
  symbol: z.string().min(1).describe('Fund ticker symbol'),
});

export const FundDisclosureLatestSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker to find fund holders for'),
});
