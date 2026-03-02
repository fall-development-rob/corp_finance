import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { pbFetch, CacheTTL } from '../client.js';
import {
  DealSearchSchema,
  DealDetailsSchema,
  ComparableDealsSchema,
} from '../schemas/deals.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerDealTools(server: McpServer) {
  server.tool(
    'pb_deal_search',
    'Search PE/VC deal activity by type, industry, and size. Returns deals with company, deal type, size, date, investors, round, pre/post-money valuation, and status.',
    DealSearchSchema.shape,
    async (params) => {
      const { deal_type, industry, min_size, max_size, start_date, end_date, page, page_size } = DealSearchSchema.parse(params);
      const data = await pbFetch('deals/search', {
        deal_type, industry, min_size, max_size, start_date, end_date, page, page_size,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_deal_details',
    'Get detailed deal information including multiples and structure. Returns full deal terms, EV/EBITDA multiples, financing structure, participating investors, advisors, and deal rationale.',
    DealDetailsSchema.shape,
    async (params) => {
      const { deal_id } = DealDetailsSchema.parse(params);
      const data = await pbFetch(`deals/${deal_id}`, {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_comparable_deals',
    'Find comparable transactions for benchmarking. Returns similar deals in the same industry with valuation multiples, deal sizes, and dates for comp analysis.',
    ComparableDealsSchema.shape,
    async (params) => {
      const { industry, deal_size, start_date, end_date, page, page_size } = ComparableDealsSchema.parse(params);
      const data = await pbFetch('deals/comparable', {
        industry, deal_size, start_date, end_date, page, page_size,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
