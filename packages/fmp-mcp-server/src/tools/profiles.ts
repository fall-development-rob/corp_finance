import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import { SymbolSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerProfileTools(server: McpServer) {
  server.tool(
    'fmp_company_profile',
    'Get comprehensive company profile: description, sector, industry, CEO, employees, market cap, beta, average volume, website, and exchange listing. Essential starting point for company research.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await fmpFetch('profile', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_stock_peers',
    'Get list of stock peer companies based on sector, industry, and market cap. Use for building comparable company sets for comps analysis.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await fmpFetch('stock-peers', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_key_executives',
    'Get list of company executives with name, title, pay, and year born. Use for corporate governance analysis.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await fmpFetch('key-executives', { symbol }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_market_cap',
    'Get current and historical market capitalization for a company.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await fmpFetch('market-capitalization', { symbol }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
