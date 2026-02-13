import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import { PageLimitSchema, SymbolNewsSchema } from '../schemas/news.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerNewsTools(server: McpServer) {
  server.tool(
    'fmp_fmp_articles',
    'Get FMP editorial articles and analysis. Returns paginated list of financial articles.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('fmp-articles', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_news_general',
    'Get general financial news from multiple sources. Returns latest headlines and summaries across all markets.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('news/general-latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_news_press_releases',
    'Get latest press releases from public companies. Returns recent corporate announcements and filings.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('news/press-releases-latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_news_stock',
    'Get latest stock market news. Returns recent news articles related to equities and stock markets.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('news/stock-latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_news_crypto',
    'Get latest cryptocurrency news. Returns recent articles about Bitcoin, Ethereum, and other digital assets.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('news/crypto-latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_news_forex',
    'Get latest forex market news. Returns recent articles about currency pairs and foreign exchange markets.',
    PageLimitSchema.shape,
    async (params) => {
      const { page, limit } = PageLimitSchema.parse(params);
      const data = await fmpFetch('news/forex-latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_search_press_releases',
    'Search press releases by ticker symbol. Returns corporate announcements for specific companies.',
    SymbolNewsSchema.shape,
    async (params) => {
      const { symbols, page, limit } = SymbolNewsSchema.parse(params);
      const data = await fmpFetch('news/press-releases', { symbols, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_search_stock_news',
    'Search stock news by ticker symbol. Returns news articles for specific companies or tickers.',
    SymbolNewsSchema.shape,
    async (params) => {
      const { symbols, page, limit } = SymbolNewsSchema.parse(params);
      const data = await fmpFetch('news/stock', { symbols, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_search_crypto_news',
    'Search cryptocurrency news by symbol. Returns news articles for specific crypto assets.',
    SymbolNewsSchema.shape,
    async (params) => {
      const { symbols, page, limit } = SymbolNewsSchema.parse(params);
      const data = await fmpFetch('news/crypto', { symbols, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_search_forex_news',
    'Search forex news by currency pair symbol. Returns news articles for specific currency pairs.',
    SymbolNewsSchema.shape,
    async (params) => {
      const { symbols, page, limit } = SymbolNewsSchema.parse(params);
      const data = await fmpFetch('news/forex', { symbols, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
