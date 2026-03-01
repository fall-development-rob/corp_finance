import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { lsegFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  HistoricalPricingSchema,
  IntradayPricingSchema,
  BondPricingSchema,
  FxRatesSchema,
} from '../schemas/pricing.js';

function resolveIdentifier(params: { ric?: string; isin?: string; sedol?: string }): string {
  if (params.ric) return params.ric;
  if (params.isin) return params.isin;
  if (params.sedol) return params.sedol;
  throw new Error('At least one identifier (ric, isin, or sedol) is required');
}

export function registerPricingTools(server: McpServer) {
  // 1. Historical prices
  server.tool(
    'lseg_historical_prices',
    'Get historical daily/weekly/monthly prices for equities and bonds. Returns OHLCV data with volume and adjusted close. Use for price trend analysis, backtesting, and charting.',
    HistoricalPricingSchema.shape,
    async (params) => {
      const parsed = HistoricalPricingSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `historical-pricing/v1/views/interday-summaries/${encodeURIComponent(identifier)}`,
        {
          start: parsed.start_date,
          end: parsed.end_date,
          interval: parsed.interval,
        },
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  // 2. Intraday prices
  server.tool(
    'lseg_intraday_prices',
    'Get intraday price bars at 1-minute, 5-minute, 15-minute, or 1-hour intervals. Returns timestamped OHLCV data for short-term trading analysis.',
    IntradayPricingSchema.shape,
    async (params) => {
      const parsed = IntradayPricingSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `historical-pricing/v1/views/intraday-summaries/${encodeURIComponent(identifier)}`,
        {
          interval: parsed.interval,
        },
        { cacheTtl: CacheTTL.REALTIME },
      );
      return wrapResponse(data);
    },
  );

  // 3. Bond pricing
  server.tool(
    'lseg_bond_pricing',
    'Get bond pricing data including yield, spread, duration, convexity, and accrued interest. Use for fixed income valuation and relative value analysis.',
    BondPricingSchema.shape,
    async (params) => {
      const parsed = BondPricingSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `quantitative-analytics/v1/financial-contracts/${encodeURIComponent(identifier)}`,
        {},
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  // 4. FX rates
  server.tool(
    'lseg_fx_rates',
    'Get FX exchange rates and crosses. Returns spot rates, bid/ask spreads, and historical rates for currency pairs. Use for FX risk analysis and hedging.',
    FxRatesSchema.shape,
    async (params) => {
      const parsed = FxRatesSchema.parse(params);
      const quotesParam = parsed.quotes ? parsed.quotes.join(',') : undefined;
      const data = await lsegFetch(
        `quantitative-analytics/v1/fx-cross-rates`,
        {
          base: parsed.base,
          quotes: quotesParam,
          start: parsed.start_date,
          end: parsed.end_date,
        },
        { cacheTtl: CacheTTL.REALTIME },
      );
      return wrapResponse(data);
    },
  );
}
