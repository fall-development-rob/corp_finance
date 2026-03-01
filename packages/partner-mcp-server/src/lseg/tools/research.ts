import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { lsegFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  CompanySearchSchema,
  FundamentalsSchema,
  EsgScoresSchema,
  NewsSchema,
  OptionsChainSchema,
  EconomicIndicatorsSchema,
} from '../schemas/research.js';

function resolveIdentifier(params: { ric?: string; isin?: string; sedol?: string }): string {
  if (params.ric) return params.ric;
  if (params.isin) return params.isin;
  if (params.sedol) return params.sedol;
  throw new Error('At least one identifier (ric, isin, or sedol) is required');
}

export function registerResearchTools(server: McpServer) {
  // 1. Company search
  server.tool(
    'lseg_company_search',
    'Search for companies by name or identifier. Returns matching entities with RIC, ISIN, exchange, and sector. Use to discover instruments before querying data.',
    CompanySearchSchema.shape,
    async (params) => {
      const parsed = CompanySearchSchema.parse(params);
      const data = await lsegFetch(
        'discovery/search/v1',
        {
          q: parsed.query,
          exchange: parsed.exchange,
          top: parsed.limit,
          skip: parsed.offset,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );

  // 2. Fundamentals
  server.tool(
    'lseg_fundamentals',
    'Get company fundamental financial data including income statement, balance sheet, and cash flow items. Returns revenue, earnings, margins, ratios, and per-share data.',
    FundamentalsSchema.shape,
    async (params) => {
      const parsed = FundamentalsSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `data-store/v1/financial-statements/${encodeURIComponent(identifier)}`,
        {
          period: parsed.period,
          limit: parsed.limit,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );

  // 3. ESG scores
  server.tool(
    'lseg_esg_scores',
    'Get ESG scores and sustainability metrics including environmental, social, and governance pillars. Returns LSEG ESG combined score, category scores, and controversy flags.',
    EsgScoresSchema.shape,
    async (params) => {
      const parsed = EsgScoresSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `environmental-social-governance/v2/views/scores-full/${encodeURIComponent(identifier)}`,
        {},
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  // 4. News
  server.tool(
    'lseg_news',
    'Search financial news headlines and stories. Filter by keyword or instrument RIC. Returns headline, timestamp, source, and story body. Use for event-driven analysis and sentiment.',
    NewsSchema.shape,
    async (params) => {
      const parsed = NewsSchema.parse(params);
      const data = await lsegFetch(
        'news/v1/headlines',
        {
          query: parsed.query,
          ric: parsed.ric,
          top: parsed.limit,
          skip: parsed.offset,
        },
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  // 5. Options chain
  server.tool(
    'lseg_options_chain',
    'Get options chain data with Greeks for a given underlying. Returns calls and puts with strike, expiration, bid/ask, delta, gamma, theta, vega, and implied volatility.',
    OptionsChainSchema.shape,
    async (params) => {
      const parsed = OptionsChainSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `quantitative-analytics/v1/options-chains/${encodeURIComponent(identifier)}`,
        {
          expiration: parsed.expiration,
        },
        { cacheTtl: CacheTTL.REALTIME },
      );
      return wrapResponse(data);
    },
  );

  // 6. Economic indicators
  server.tool(
    'lseg_economic_indicators',
    'Get macro economic indicator time series by country. Returns GDP, CPI, unemployment, PMI, trade balance, and other indicators with historical values and release dates.',
    EconomicIndicatorsSchema.shape,
    async (params) => {
      const parsed = EconomicIndicatorsSchema.parse(params);
      const data = await lsegFetch(
        'data-store/v1/economic-indicators',
        {
          country: parsed.country,
          indicator: parsed.indicator,
          start: parsed.start_date,
          end: parsed.end_date,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );
}
