import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import {
  yfFetch, CacheTTL,
  quoteUrl, chartUrl,
  extractQuoteResponse, extractChart, quoteSummaryUrl, extractQuoteSummary,
} from '../client.js';
import { SymbolSchema, BatchSymbolsSchema, HistoricalSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerQuoteTools(server: McpServer) {
  server.tool(
    'yf_quote',
    '[UNOFFICIAL Yahoo Finance] Get real-time quote with price, change, volume, market cap, PE, 52-week range. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteUrl([symbol]);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.REALTIME });
      const data = extractQuoteResponse(raw);
      return wrapResponse(data);
    },
  );

  server.tool(
    'yf_historical',
    '[UNOFFICIAL Yahoo Finance] Get historical OHLCV price data. Supports periods from 1 day to max and intervals from 1 minute to 3 months. May break without notice.',
    HistoricalSchema.shape,
    async (params) => {
      const { symbol, period, interval } = HistoricalSchema.parse(params);
      const url = chartUrl(symbol, { range: period, interval });
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.SHORT });
      const data = extractChart(raw);
      return wrapResponse(data);
    },
  );

  server.tool(
    'yf_summary_detail',
    '[UNOFFICIAL Yahoo Finance] Get summary details: dividend yield, PE ratio, market cap, beta, 52-week range, volume averages. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteSummaryUrl(symbol, ['summaryDetail']);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.SHORT });
      const data = extractQuoteSummary(raw);
      return wrapResponse(data);
    },
  );

  server.tool(
    'yf_fast_info',
    '[UNOFFICIAL Yahoo Finance] Get quick price snapshot: current price, change, change percent. Lightweight quote. May break without notice.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const url = quoteUrl([symbol], 'regularMarketPrice,regularMarketChange,regularMarketChangePercent,regularMarketVolume');
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.REALTIME });
      const data = extractQuoteResponse(raw);
      return wrapResponse(data);
    },
  );

  server.tool(
    'yf_batch_quotes',
    '[UNOFFICIAL Yahoo Finance] Get real-time quotes for multiple symbols in a single request. Efficient for comparing stocks. May break without notice.',
    BatchSymbolsSchema.shape,
    async (params) => {
      const { symbols } = BatchSymbolsSchema.parse(params);
      const symbolList = symbols.split(',').map(s => s.trim()).filter(Boolean);
      const url = quoteUrl(symbolList);
      const raw = await yfFetch<Record<string, unknown>>(url, { cacheTtl: CacheTTL.REALTIME });
      const data = extractQuoteResponse(raw);
      return wrapResponse(data);
    },
  );
}
