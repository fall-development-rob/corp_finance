import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { factsetPost, wrapResponse, CacheTTL } from '../client.js';
import {
  FundamentalsSchema,
  EstimatesSchema,
  CompanySearchSchema,
} from '../schemas/fundamentals.js';

export function registerFundamentalsTools(server: McpServer) {
  server.tool(
    'factset_fundamentals',
    'Get company fundamental data (revenue, EBITDA, margins, ratios) from FactSet. Supports 70,000+ public companies worldwide. Use for financial analysis, valuation, and screening.',
    FundamentalsSchema.shape,
    async (params) => {
      const { ids, metrics, period } = FundamentalsSchema.parse(params);
      const body: Record<string, unknown> = { ids };
      if (metrics) body.metrics = metrics;
      if (period) body.periodicity = period;
      const data = await factsetPost('formula-api/v1/time-series', body, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_estimates',
    'Get consensus broker estimates and forecasts from FactSet. Includes EPS, revenue, EBITDA estimates with mean, median, high, low, and number of analysts. Use for forward-looking analysis.',
    EstimatesSchema.shape,
    async (params) => {
      const { ids, metrics, periodicity } = EstimatesSchema.parse(params);
      const body: Record<string, unknown> = { ids };
      if (metrics) body.metrics = metrics;
      if (periodicity) body.periodicity = periodicity;
      const data = await factsetPost('estimates/v2/rolling-consensus', body, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_company_search',
    'Search for companies across 70,000+ public entities in the FactSet universe. Returns company name, ticker, exchange, country, and FactSet identifiers. Use to discover and resolve company identifiers.',
    CompanySearchSchema.shape,
    async (params) => {
      const { query, exchange, limit, offset } = CompanySearchSchema.parse(params);
      const body: Record<string, unknown> = { pattern: query, limit, offset };
      if (exchange) body.exchanges = [exchange];
      const data = await factsetPost('idsearch/v1/idsearch', body, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
