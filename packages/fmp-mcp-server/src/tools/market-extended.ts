import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  HistoricalSectorSchema,
  HistoricalIndustrySchema,
  PeSnapshotSchema,
  ExchangeSchema,
  EmptySchema,
} from '../schemas/market-extended.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerMarketExtendedTools(server: McpServer) {
  // ── Market Performance ─────────────────────────────────────────────

  server.tool(
    'fmp_historical_sector_performance',
    'Get historical performance data for a specific sector over time. Use for sector trend analysis.',
    HistoricalSectorSchema.shape,
    async (params) => {
      const { sector } = HistoricalSectorSchema.parse(params);
      const data = await fmpFetch('historical-sector-performance', { sector }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_industry_performance',
    'Get historical performance data for a specific industry over time. Use for industry trend analysis.',
    HistoricalIndustrySchema.shape,
    async (params) => {
      const { industry } = HistoricalIndustrySchema.parse(params);
      const data = await fmpFetch('historical-industry-performance', { industry }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_sector_pe',
    'Get PE ratio snapshot across all sectors. Useful for relative valuation and identifying over/undervalued sectors.',
    PeSnapshotSchema.shape,
    async (params) => {
      const { date } = PeSnapshotSchema.parse(params);
      const data = await fmpFetch('sector-pe-snapshot', { date }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_industry_pe',
    'Get PE ratio snapshot across all industries. Useful for relative valuation at industry level.',
    PeSnapshotSchema.shape,
    async (params) => {
      const { date } = PeSnapshotSchema.parse(params);
      const data = await fmpFetch('industry-pe-snapshot', { date }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_sector_pe',
    'Get historical PE ratios for a specific sector. Track valuation trends over time.',
    HistoricalSectorSchema.shape,
    async (params) => {
      const { sector } = HistoricalSectorSchema.parse(params);
      const data = await fmpFetch('historical-sector-pe', { sector }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_industry_pe',
    'Get historical PE ratios for a specific industry. Track valuation trends over time.',
    HistoricalIndustrySchema.shape,
    async (params) => {
      const { industry } = HistoricalIndustrySchema.parse(params);
      const data = await fmpFetch('historical-industry-pe', { industry }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_biggest_gainers',
    'Get top gaining stocks today by percentage change. Use for momentum screening and market sentiment.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('biggest-gainers', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_biggest_losers',
    'Get top losing stocks today by percentage change. Use for contrarian screening and risk monitoring.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('biggest-losers', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_most_active',
    'Get most actively traded stocks today by volume. Use for liquidity analysis and market activity monitoring.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('most-actives', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_market_risk_premium',
    'Get current market risk premium by country. Essential for CAPM and cost of equity calculations.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('market-risk-premium', {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  // ── Market Hours ───────────────────────────────────────────────────

  server.tool(
    'fmp_exchange_hours',
    'Get trading hours for a specific exchange including open/close times and timezone. Use for scheduling and market availability checks.',
    ExchangeSchema.shape,
    async (params) => {
      const { exchange } = ExchangeSchema.parse(params);
      const data = await fmpFetch('exchange-market-hours', { exchange }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_exchange_holidays',
    'Get market holidays for a specific exchange. Use to check if markets are closed on a given date.',
    ExchangeSchema.shape,
    async (params) => {
      const { exchange } = ExchangeSchema.parse(params);
      const data = await fmpFetch('holidays-by-exchange', { exchange }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_all_exchange_hours',
    'Get trading hours for all exchanges worldwide. Comprehensive view of global market schedules.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('all-exchange-market-hours', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  // ── Extended Indexes ───────────────────────────────────────────────

  server.tool(
    'fmp_index_list',
    'Get list of all available market indexes with symbols and names. Use to discover trackable indexes.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('index-list', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_sp500_constituent',
    'Get historical changes to S&P 500 constituents (additions and removals). Use for index rebalancing analysis.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('historical-sp500-constituent', {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_nasdaq_constituent',
    'Get historical changes to Nasdaq 100 constituents (additions and removals). Use for index rebalancing analysis.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('historical-nasdaq-constituent', {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_dowjones_constituent',
    'Get historical changes to Dow Jones constituents (additions and removals). Use for index rebalancing analysis.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('historical-dowjones-constituent', {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  // ── Commodities ────────────────────────────────────────────────────

  server.tool(
    'fmp_commodities_list',
    'Get list of all available commodity symbols (gold, oil, natural gas, etc.). Use to discover tradeable commodities.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('commodities-list', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_commodity_quotes',
    'Get real-time quotes for all commodities in a single call. Use for commodity market overview and portfolio exposure checks.',
    EmptySchema.shape,
    async (params) => {
      EmptySchema.parse(params);
      const data = await fmpFetch('batch-commodity-quotes', {}, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );
}
