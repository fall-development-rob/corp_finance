import { z } from 'zod';

export const SymbolSchema = z.object({
  symbol: z.string().min(1).describe('Stock ticker symbol (e.g., AAPL, MSFT, IBM)'),
});

export const OutputSizeSchema = z.object({
  outputsize: z.enum(['compact', 'full']).default('compact').describe('compact = latest 100 data points, full = full history (20+ years)'),
});

export const IntervalSchema = z.object({
  interval: z.enum(['1min', '5min', '15min', '30min', '60min']).default('5min').describe('Time interval between data points'),
});

export const SearchSchema = z.object({
  keywords: z.string().min(1).describe('Search keywords (company name or ticker)'),
});

export const ForexPairSchema = z.object({
  from_currency: z.string().min(1).describe('Source currency code (e.g., EUR, GBP, JPY)'),
  to_currency: z.string().min(1).describe('Destination currency code (e.g., USD, EUR)'),
});

export const CryptoSchema = z.object({
  symbol: z.string().min(1).describe('Crypto symbol (e.g., BTC, ETH)'),
  market: z.string().default('USD').describe('Exchange market (e.g., USD, EUR, CNY)'),
});

export const CommodityIntervalSchema = z.object({
  interval: z.enum(['daily', 'weekly', 'monthly']).default('monthly').describe('Data interval'),
});

export const EconIntervalSchema = z.object({
  interval: z.enum(['quarterly', 'annual']).default('annual').describe('Reporting interval'),
});

export const TreasuryMaturitySchema = z.object({
  maturity: z.enum(['3month', '2year', '5year', '7year', '10year', '30year']).default('10year').describe('Treasury bond maturity'),
});

export const TechnicalSchema = SymbolSchema.merge(z.object({
  interval: z.enum(['1min', '5min', '15min', '30min', '60min', 'daily', 'weekly', 'monthly']).default('daily').describe('Time interval'),
  time_period: z.number().int().min(1).max(500).default(20).describe('Number of data points for calculation (e.g., 20 for 20-day SMA)'),
  series_type: z.enum(['close', 'open', 'high', 'low']).default('close').describe('Price type for calculation'),
}));

export const NewsSchema = z.object({
  tickers: z.string().optional().describe('Comma-separated tickers to filter (e.g., AAPL,MSFT). Omit for general market news.'),
  topics: z.string().optional().describe('Topics to filter: blockchain, earnings, ipo, mergers_and_acquisitions, financial_markets, economy_fiscal, economy_monetary, economy_macro, energy_transportation, finance, life_sciences, manufacturing, real_estate, retail_wholesale, technology'),
  sort: z.enum(['LATEST', 'EARLIEST', 'RELEVANCE']).default('LATEST').describe('Sort order'),
  limit: z.number().int().min(1).max(1000).default(50).describe('Max results'),
});

export const EarningsCalendarSchema = z.object({
  horizon: z.enum(['3month', '6month', '12month']).default('3month').describe('Lookahead period for upcoming earnings'),
  symbol: z.string().optional().describe('Optional ticker to filter for a specific company'),
});
