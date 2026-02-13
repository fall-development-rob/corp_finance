import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import { QuoteSchema, BatchQuoteSchema, HistoricalPriceSchema, IntradaySchema } from '../schemas/quotes.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerQuoteTools(server: McpServer) {
  server.tool(
    'fmp_quote',
    'Get real-time stock quote with price, change, volume, market cap, PE, and 52-week range. Use for current market snapshot of any security.',
    QuoteSchema.shape,
    async (params) => {
      const { symbol } = QuoteSchema.parse(params);
      const data = await fmpFetch('quote', { symbol }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_batch_quote',
    'Get real-time quotes for multiple symbols in a single request. Returns array of quote objects. Efficient for comparing stocks.',
    BatchQuoteSchema.shape,
    async (params) => {
      const { symbols } = BatchQuoteSchema.parse(params);
      const data = await fmpFetch('batch-quote', { symbols }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_quote_short',
    'Get abbreviated stock quote (price and volume only). Lightweight alternative to full quote.',
    QuoteSchema.shape,
    async (params) => {
      const { symbol } = QuoteSchema.parse(params);
      const data = await fmpFetch('quote-short', { symbol }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_historical_price',
    'Get end-of-day historical stock prices with OHLCV data, adjusted prices, change percent, VWAP, and label. Use for charting and backtesting.',
    HistoricalPriceSchema.shape,
    async (params) => {
      const { symbol, from, to } = HistoricalPriceSchema.parse(params);
      const data = await fmpFetch(`historical-price-eod/full`, { symbol, from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_intraday_chart',
    'Get intraday price data at specified intervals (1min, 5min, 15min, 30min, 1hour, 4hour). Returns OHLCV candles for technical analysis.',
    IntradaySchema.shape,
    async (params) => {
      const { symbol, interval, from, to } = IntradaySchema.parse(params);
      const data = await fmpFetch(`historical-chart/${interval}`, { symbol, from, to }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );
}
