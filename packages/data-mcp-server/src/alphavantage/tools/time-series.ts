import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { avFetch, CacheTTL } from '../client.js';
import { SymbolSchema, OutputSizeSchema, IntervalSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

const IntraDaySchema = SymbolSchema.merge(IntervalSchema).merge(OutputSizeSchema);
const DailySchema = SymbolSchema.merge(OutputSizeSchema);

export function registerTimeSeriesTools(server: McpServer) {
  server.tool(
    'av_intraday',
    'Get intraday OHLCV time series (1min, 5min, 15min, 30min, 60min intervals). Returns open, high, low, close, volume for each interval. Premium endpoints may require paid plan.',
    IntraDaySchema.shape,
    async (params) => {
      const { symbol, interval, outputsize } = IntraDaySchema.parse(params);
      const data = await avFetch({
        function: 'TIME_SERIES_INTRADAY',
        symbol, interval, outputsize,
      }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_daily',
    'Get daily OHLCV time series for a stock. compact = last 100 trading days, full = 20+ years. Core endpoint for historical price analysis.',
    DailySchema.shape,
    async (params) => {
      const { symbol, outputsize } = DailySchema.parse(params);
      const data = await avFetch({
        function: 'TIME_SERIES_DAILY',
        symbol, outputsize,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_weekly',
    'Get weekly OHLCV time series for a stock. Returns last trading day of each week with adjusted OHLCV. Full 20+ year history.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await avFetch({
        function: 'TIME_SERIES_WEEKLY',
        symbol,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_monthly',
    'Get monthly OHLCV time series for a stock. Returns last trading day of each month with OHLCV and volume. Full 20+ year history.',
    SymbolSchema.shape,
    async (params) => {
      const { symbol } = SymbolSchema.parse(params);
      const data = await avFetch({
        function: 'TIME_SERIES_MONTHLY',
        symbol,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
