import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { msFetch, CacheTTL } from '../client.js';
import {
  PortfolioXraySchema,
  AssetAllocationSchema,
  PeerComparisonSchema,
} from '../schemas/portfolio.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerPortfolioTools(server: McpServer) {
  server.tool(
    'ms_portfolio_xray',
    'X-ray portfolio for style, sector, and geographic exposure. Analyzes a portfolio of holdings to show style box allocation, sector weights, geographic breakdown, stock/bond split, and risk statistics.',
    PortfolioXraySchema.shape,
    async (params) => {
      const { holdings } = PortfolioXraySchema.parse(params);
      const data = await msFetch('portfolio/xray', {
        holdings: JSON.stringify(holdings),
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_asset_allocation',
    'Get asset allocation breakdown for a fund. Returns allocation across stocks, bonds, cash, other assets, with sub-breakdowns by market cap, credit quality, and maturity.',
    AssetAllocationSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker } = AssetAllocationSchema.parse(params);
      const data = await msFetch('fund/allocation', {
        fund_id, isin, ticker,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'ms_peer_comparison',
    'Compare fund against category peers. Returns percentile rank within category, quartile placement, peer group statistics, and relative performance over multiple time periods.',
    PeerComparisonSchema.shape,
    async (params) => {
      const { fund_id, isin, ticker, category } = PeerComparisonSchema.parse(params);
      const data = await msFetch('fund/peer', {
        fund_id, isin, ticker, category,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
