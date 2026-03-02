import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { spFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  MaDealsSchema,
  FundingDigestSchema,
} from '../schemas/deals.js';

export function registerDealTools(server: McpServer) {
  server.tool(
    'sp_ma_deals',
    'Search M&A deal flow by acquirer, target, or date range. Returns deal details including value, status, multiples, and advisors. Use for deal screening and M&A market analysis.',
    MaDealsSchema.shape,
    async (params) => {
      const { acquirer, target, status, start_date, end_date, limit, offset } = MaDealsSchema.parse(params);
      const data = await spFetch('deals/ma', {
        acquirer,
        target,
        status,
        start_date,
        end_date,
        limit,
        offset,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_funding_digest',
    'Get funding rounds and capital raises for a company including round type, amount, investors, and valuation. Use for venture capital and growth equity analysis.',
    FundingDigestSchema.shape,
    async (params) => {
      const { company_id, ticker, name, start_date, end_date } = FundingDigestSchema.parse(params);
      const data = await spFetch('companies/funding', {
        company_id,
        ticker,
        name,
        start_date,
        end_date,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
