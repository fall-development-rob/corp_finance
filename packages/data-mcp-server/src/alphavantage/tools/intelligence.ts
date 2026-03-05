import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { avFetch, CacheTTL } from '../client.js';
import { NewsSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerIntelligenceTools(server: McpServer) {
  server.tool(
    'av_news_sentiment',
    'Get market news with AI-powered sentiment analysis from Alpha Vantage. Filter by tickers and/or topics. Returns articles with title, summary, source, sentiment score, and relevance. Topics: blockchain, earnings, ipo, mergers_and_acquisitions, financial_markets, economy_fiscal, economy_monetary, economy_macro, energy_transportation, finance, life_sciences, manufacturing, real_estate, retail_wholesale, technology.',
    NewsSchema.shape,
    async (params) => {
      const { tickers, topics, sort, limit } = NewsSchema.parse(params);
      const fetchParams: Record<string, string | number> = {
        function: 'NEWS_SENTIMENT',
        sort,
        limit,
      };
      if (tickers) fetchParams.tickers = tickers;
      if (topics) fetchParams.topics = topics;
      const data = await avFetch(fetchParams, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
