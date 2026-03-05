import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { avFetch, CacheTTL } from '../client.js';
import { SymbolSchema, SearchSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerQuoteTools(server: McpServer) {
  server.tool(
    'av_quote',
    'Get real-time stock quote from Alpha Vantage: price, change, volume, latest trading day. Use for current market data.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await avFetch({ function: 'GLOBAL_QUOTE', symbol }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_search',
    'Search Alpha Vantage for ticker symbols by company name or keyword. Returns matching symbols with name, type, region, currency, and market hours.',
    SearchSchema.shape,
    async (params) => {
      const { keywords } = SearchSchema.parse(params);
      const data = await avFetch({ function: 'SYMBOL_SEARCH', keywords }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_market_status',
    'Get current market status for major exchanges worldwide. Shows open/closed status, local time, and trading hours.',
    {},
    async () => {
      const data = await avFetch({ function: 'MARKET_STATUS' }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_top_gainers_losers',
    'Get top gainers, losers, and most actively traded US tickers. Useful for market momentum screening.',
    {},
    async () => {
      const data = await avFetch({ function: 'TOP_GAINERS_LOSERS' }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
