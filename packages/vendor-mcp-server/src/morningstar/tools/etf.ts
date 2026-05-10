import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { msFetch, CacheTTL } from '../client.js';
import { EtfAnalyticsSchema } from '../schemas/etf.js';
import { wrapResponse } from '@robotixai/mcp-utils';

export function registerEtfTools(server: McpServer) {
  server.tool(
    'ms_etf_analytics',
    'Get ETF analytics including tracking error and premium/discount. Returns tracking error vs benchmark, premium/discount to NAV, bid-ask spread, creation/redemption data, and fund flow trends.',
    EtfAnalyticsSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker, include_tracking_error } = EtfAnalyticsSchema.parse(params);
      const data = await msFetch('etf/analytics', {
        fund_id, isin, ticker, include_tracking_error,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
