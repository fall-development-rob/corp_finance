import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { spFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  FinancialsSchema,
  EstimatesSchema,
  SegmentDataSchema,
} from '../schemas/financials.js';

export function registerFinancialTools(server: McpServer) {
  server.tool(
    'sp_financials',
    'Get standardized financial statements (income statement, balance sheet, cash flow) with key line items. Supports annual and quarterly periods. Use for fundamental financial analysis.',
    FinancialsSchema.shape,
    async (params) => {
      const { company_id, ticker, name, period, limit } = FinancialsSchema.parse(params);
      const data = await spFetch('companies/financials', {
        company_id,
        ticker,
        name,
        period,
        limit,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_estimates',
    'Get consensus analyst estimates and forecasts including revenue, EPS, EBITDA projections with high/low/mean/median. Use for forward-looking valuation and earnings expectations.',
    EstimatesSchema.shape,
    async (params) => {
      const { company_id, ticker, name, metric } = EstimatesSchema.parse(params);
      const data = await spFetch('companies/estimates', {
        company_id,
        ticker,
        name,
        metric,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_segment_data',
    'Get business segment breakdown with revenue, operating income, and margins by segment. Use to understand business mix, growth drivers, and segment-level profitability.',
    SegmentDataSchema.shape,
    async (params) => {
      const { company_id, ticker, name, fiscal_year } = SegmentDataSchema.parse(params);
      const data = await spFetch('companies/segments', {
        company_id,
        ticker,
        name,
        fiscal_year,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
