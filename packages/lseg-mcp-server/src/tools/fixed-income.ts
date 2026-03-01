import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { lsegFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  YieldCurveSchema,
  CreditSpreadsSchema,
} from '../schemas/fixed-income.js';

export function registerFixedIncomeTools(server: McpServer) {
  // 1. Yield curve
  server.tool(
    'lseg_yield_curve',
    'Get sovereign yield curve data by currency. Returns term structure with tenors (3M, 6M, 1Y, 2Y, 5Y, 10Y, 30Y) and corresponding yields. Use for rate analysis, curve shape, and spread calculations.',
    YieldCurveSchema.shape,
    async (params) => {
      const parsed = YieldCurveSchema.parse(params);
      const data = await lsegFetch(
        'quantitative-analytics/v1/curves/yield',
        {
          currency: parsed.currency,
          date: parsed.date,
        },
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  // 2. Credit spreads
  server.tool(
    'lseg_credit_spreads',
    'Get credit spread data by rating and sector. Returns OAS, Z-spread, and benchmark spreads across the credit quality spectrum. Use for relative value and credit risk analysis.',
    CreditSpreadsSchema.shape,
    async (params) => {
      const parsed = CreditSpreadsSchema.parse(params);
      const data = await lsegFetch(
        'quantitative-analytics/v1/curves/credit-spreads',
        {
          rating: parsed.rating,
          sector: parsed.sector,
          start: parsed.start_date,
          end: parsed.end_date,
        },
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );
}
