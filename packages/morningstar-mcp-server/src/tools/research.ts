import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { msFetch, CacheTTL } from '../client.js';
import {
  FairValueSchema,
  MoatRatingSchema,
  EsgRiskSchema,
  AnalystReportSchema,
  CompanyProfileSchema,
} from '../schemas/research.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerResearchTools(server: McpServer) {
  server.tool(
    'ms_fair_value',
    'Get Morningstar fair value estimate and uncertainty rating. Returns quantitative fair value per share, price/fair value ratio, uncertainty rating (Low/Medium/High/Very High), and margin of safety.',
    FairValueSchema.shape,
    async (params) => {
      const { ticker } = FairValueSchema.parse(params);
      const data = await msFetch('equity/fairvalue', {
        ticker,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_moat_rating',
    'Get economic moat rating (Wide/Narrow/None) with assessment. Returns moat classification, moat trend (Stable/Positive/Negative), competitive advantage sources, and stewardship rating.',
    MoatRatingSchema.shape,
    async (params) => {
      const { ticker } = MoatRatingSchema.parse(params);
      const data = await msFetch('equity/moat', {
        ticker,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_esg_risk',
    'Get Sustainalytics ESG risk rating and carbon metrics. Returns ESG risk score, controversy level, carbon risk score, fossil fuel involvement, and UN Global Compact compliance.',
    EsgRiskSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker } = EsgRiskSchema.parse(params);
      const data = await msFetch('esg/risk', {
        fund_id, isin, ticker,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_analyst_report',
    'Get analyst research report summary. Returns analyst note, bull/bear cases, key investment thesis, valuation summary, and recent updates from Morningstar equity analysts.',
    AnalystReportSchema.shape,
    async (params) => {
      const { ticker } = AnalystReportSchema.parse(params);
      const data = await msFetch('equity/report', {
        ticker,
      }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_company_profile',
    'Get company profile with key financial metrics. Returns company description, sector/industry, market cap, key ratios (P/E, P/B, ROE, debt/equity), and financial summary.',
    CompanyProfileSchema.shape,
    async (params) => {
      const { ticker } = CompanyProfileSchema.parse(params);
      const data = await msFetch('equity/profile', {
        ticker,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
