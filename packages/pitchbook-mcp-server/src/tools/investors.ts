import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { pbFetch, CacheTTL } from '../client.js';
import {
  InvestorProfileSchema,
  FundSearchSchema,
  FundPerformanceSchema,
  LpCommitmentsSchema,
} from '../schemas/investors.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerInvestorTools(server: McpServer) {
  server.tool(
    'pb_investor_profile',
    'Get investor profile with investment strategy and track record. Returns firm description, AUM, investment preferences (stage, sector, geography), portfolio companies, recent deals, and team.',
    InvestorProfileSchema.shape,
    async (params) => {
      const { entity_id, name } = InvestorProfileSchema.parse(params);
      const data = await pbFetch('investors/profile', {
        entity_id, name,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_fund_search',
    'Search PE/VC funds by manager, strategy, and vintage. Returns matching funds with manager, strategy, vintage year, target size, close size, geography focus, and status.',
    FundSearchSchema.shape,
    async (params) => {
      const { manager, strategy, vintage, page, page_size } = FundSearchSchema.parse(params);
      const data = await pbFetch('funds/search', {
        manager, strategy, vintage, page, page_size,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_fund_performance',
    'Get fund performance metrics (IRR, TVPI, DPI, RVPI). Returns net IRR, gross IRR, TVPI, DPI, RVPI, PME benchmark comparison, quartile rank, and vintage year performance context.',
    FundPerformanceSchema.shape,
    async (params) => {
      const { fund_id } = FundPerformanceSchema.parse(params);
      const data = await pbFetch(`funds/${fund_id}/performance`, {}, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_lp_commitments',
    'Get LP commitment data and allocation breakdown. Returns LP commitments by fund, vintage, strategy, commitment size, unfunded obligations, and allocation as percentage of portfolio.',
    LpCommitmentsSchema.shape,
    async (params) => {
      const { entity_id, name, page, page_size } = LpCommitmentsSchema.parse(params);
      const data = await pbFetch('investors/commitments', {
        entity_id, name, page, page_size,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
