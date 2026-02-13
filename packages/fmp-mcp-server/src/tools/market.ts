import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import { SearchSchema, ScreenerSchema } from '../schemas/common.js';
import {
  SectorPerformanceSchema, IndustryPerformanceSchema, IndexConstituentsSchema,
  EconomicIndicatorSchema, TreasuryRatesSchema, EconomicCalendarSchema,
} from '../schemas/market.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerMarketTools(server: McpServer) {
  server.tool(
    'fmp_search_symbol',
    'Search for stocks by ticker symbol. Returns matching tickers with company name, exchange, and type. Use when you know approximate ticker.',
    SearchSchema.shape,
    async (params) => {
      const { query, limit, exchange } = SearchSchema.parse(params);
      const data = await fmpFetch('search-symbol', { query, limit, exchange }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_search_name',
    'Search for stocks by company name. Returns matching companies with ticker, exchange, and type. Use when you know company name but not ticker.',
    SearchSchema.shape,
    async (params) => {
      const { query, limit, exchange } = SearchSchema.parse(params);
      const data = await fmpFetch('search-name', { query, limit, exchange }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_stock_screener',
    'Screen stocks by market cap, sector, industry, exchange, and country. Returns filtered list of companies matching criteria. Use for peer set building.',
    ScreenerSchema.shape,
    async (params) => {
      const parsed = ScreenerSchema.parse(params);
      const fmpParams: Record<string, string | number | boolean | undefined> = {};
      if (parsed.market_cap_more_than) fmpParams.marketCapMoreThan = parsed.market_cap_more_than;
      if (parsed.market_cap_less_than) fmpParams.marketCapLessThan = parsed.market_cap_less_than;
      if (parsed.sector) fmpParams.sector = parsed.sector;
      if (parsed.industry) fmpParams.industry = parsed.industry;
      if (parsed.exchange) fmpParams.exchange = parsed.exchange;
      if (parsed.country) fmpParams.country = parsed.country;
      fmpParams.limit = parsed.limit;
      const data = await fmpFetch('company-screener', fmpParams, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sector_performance',
    'Get sector performance snapshot showing returns for all market sectors. Useful for sector rotation analysis.',
    SectorPerformanceSchema.shape,
    async (params) => {
      const { date } = SectorPerformanceSchema.parse(params);
      const data = await fmpFetch('sector-performance-snapshot', { date }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_industry_performance',
    'Get industry-level performance snapshot. More granular than sector performance for drill-down analysis.',
    IndustryPerformanceSchema.shape,
    async (params) => {
      const { date } = IndustryPerformanceSchema.parse(params);
      const data = await fmpFetch('industry-performance-snapshot', { date }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_index_constituents',
    'Get current constituents of major market indices (S&P 500, Nasdaq 100, Dow Jones). Returns all member companies.',
    IndexConstituentsSchema.shape,
    async (params) => {
      const { index } = IndexConstituentsSchema.parse(params);
      const endpoint = index === 'sp500' ? 'sp500-constituent'
        : index === 'nasdaq' ? 'nasdaq-constituent'
        : 'dowjones-constituent';
      const data = await fmpFetch(endpoint, {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_treasury_rates',
    'Get US Treasury rates across maturities (1M, 3M, 6M, 1Y, 2Y, 5Y, 10Y, 30Y). Essential for risk-free rate in WACC/CAPM.',
    TreasuryRatesSchema.shape,
    async (params) => {
      const { from, to } = TreasuryRatesSchema.parse(params);
      const data = await fmpFetch('treasury-rates', { from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_economic_indicators',
    'Get economic indicator data (GDP, CPI, unemployment, etc.) with historical values. Use for macroeconomic analysis.',
    EconomicIndicatorSchema.shape,
    async (params) => {
      const { name, from, to } = EconomicIndicatorSchema.parse(params);
      const data = await fmpFetch('economic-indicators', { name, from, to }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_economic_calendar',
    'Get upcoming economic events calendar (FOMC, CPI releases, employment, etc.). Use for event-driven macro analysis.',
    EconomicCalendarSchema.shape,
    async (params) => {
      const { from, to } = EconomicCalendarSchema.parse(params);
      const data = await fmpFetch('economic-calendar', { from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
