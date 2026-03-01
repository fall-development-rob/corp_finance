import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { moodysFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  StructuredFinanceSchema,
  MunicipalScoreSchema,
  CompanyFinancialsSchema,
} from '../schemas/structured.js';

function resolveIssuerParams(params: { issuer_id?: string; ticker?: string; name?: string }): Record<string, string | undefined> {
  return {
    issuer_id: params.issuer_id,
    ticker: params.ticker,
    name: params.name,
  };
}

export function registerStructuredTools(server: McpServer) {
  // 1. Structured finance
  server.tool(
    'moodys_structured_finance',
    'Get structured finance deal performance data (CMBS/RMBS/ABS/CLO) from Moody\'s. Returns deal-level metrics including collateral performance, tranche ratings, delinquency rates, and loss projections. Use for structured credit analysis and surveillance.',
    StructuredFinanceSchema.shape,
    async (params) => {
      const parsed = StructuredFinanceSchema.parse(params);
      const data = await moodysFetch(
        'structured-finance/v1/deals',
        {
          deal_id: parsed.deal_id,
          asset_class: parsed.asset_class,
          limit: parsed.limit,
          offset: parsed.offset,
        },
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  // 2. Municipal score
  server.tool(
    'moodys_municipal_score',
    'Get municipal credit scoring and analysis from Moody\'s. Returns credit quality indicators for municipal issuers including financial position, debt burden, economic base, and governance factors. Use for muni credit analysis and portfolio construction.',
    MunicipalScoreSchema.shape,
    async (params) => {
      const parsed = MunicipalScoreSchema.parse(params);
      const data = await moodysFetch(
        'municipal/v1/scores',
        {
          issuer: parsed.issuer,
          state: parsed.state,
        },
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  // 3. Company financials
  server.tool(
    'moodys_company_financials',
    'Get Moody\'s-adjusted financial metrics for an issuer. Returns standardized financial statements with Moody\'s analytical adjustments for leases, pensions, hybrid securities, and off-balance-sheet items. Includes key ratios (leverage, coverage, margins). Use for cross-company comparisons and credit assessment.',
    CompanyFinancialsSchema.shape,
    async (params) => {
      const parsed = CompanyFinancialsSchema.parse(params);
      const data = await moodysFetch(
        'financials/v1/company',
        {
          ...resolveIssuerParams(parsed),
          period: parsed.period,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );
}
