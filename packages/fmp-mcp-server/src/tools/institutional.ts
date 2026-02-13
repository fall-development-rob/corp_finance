import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  InstitutionalLatestSchema, InstitutionalExtractSchema, InstitutionalDatesSchema,
  InstitutionalAnalyticsByHolderSchema, HolderPerformanceSchema, HolderIndustrySchema,
  PositionsSummarySchema, IndustrySummarySchema,
} from '../schemas/institutional.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerInstitutionalTools(server: McpServer) {
  server.tool(
    'fmp_institutional_latest',
    'Get latest 13F institutional ownership filings. Returns recent SEC 13F filings showing institutional holders and their positions.',
    InstitutionalLatestSchema.shape,
    async (params) => {
      const { page, limit } = InstitutionalLatestSchema.parse(params);
      const data = await fmpFetch('institutional-ownership/latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_institutional_extract',
    'Extract holdings from a specific 13F filing by CIK, year, and quarter. Returns all stock positions held by the institution.',
    InstitutionalExtractSchema.shape,
    async (params) => {
      const { cik, year, quarter } = InstitutionalExtractSchema.parse(params);
      const data = await fmpFetch('institutional-ownership/extract', { cik, year, quarter }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_institutional_dates',
    'Get available 13F filing dates for an institutional holder by CIK. Returns list of year/quarter combinations with filings on record.',
    InstitutionalDatesSchema.shape,
    async (params) => {
      const { cik } = InstitutionalDatesSchema.parse(params);
      const data = await fmpFetch('institutional-ownership/dates', { cik }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_institutional_analytics_holder',
    'Get analytics by holder for a stock in a given quarter. Shows which institutions hold the stock and changes in their positions.',
    InstitutionalAnalyticsByHolderSchema.shape,
    async (params) => {
      const { symbol, year, quarter, page, limit } = InstitutionalAnalyticsByHolderSchema.parse(params);
      const data = await fmpFetch('institutional-ownership/extract-analytics/holder', { symbol, year, quarter, page, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_holder_performance',
    'Get holder performance summary. Returns performance metrics and portfolio returns for an institutional holder.',
    HolderPerformanceSchema.shape,
    async (params) => {
      const { cik, page } = HolderPerformanceSchema.parse(params);
      const data = await fmpFetch('institutional-ownership/holder-performance-summary', { cik, page }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_holder_industry_breakdown',
    'Get holder industry breakdown. Returns how an institutional holder allocates capital across industries for a given quarter.',
    HolderIndustrySchema.shape,
    async (params) => {
      const { cik, year, quarter } = HolderIndustrySchema.parse(params);
      const data = await fmpFetch('institutional-ownership/holder-industry-breakdown', { cik, year, quarter }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_positions_summary',
    'Get institutional positions summary for a stock. Shows aggregate institutional ownership, number of holders, and position changes.',
    PositionsSummarySchema.shape,
    async (params) => {
      const { symbol, year, quarter } = PositionsSummarySchema.parse(params);
      const data = await fmpFetch('institutional-ownership/symbol-positions-summary', { symbol, year, quarter }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_industry_ownership_summary',
    'Get industry-level institutional ownership summary. Shows aggregate institutional ownership across industries for a given quarter.',
    IndustrySummarySchema.shape,
    async (params) => {
      const { year, quarter } = IndustrySummarySchema.parse(params);
      const data = await fmpFetch('institutional-ownership/industry-summary', { year, quarter }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
