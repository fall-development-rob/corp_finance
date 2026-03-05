import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { avFetch, CacheTTL } from '../client.js';
import { TechnicalSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerTechnicalTools(server: McpServer) {
  server.tool(
    'av_sma',
    'Get Simple Moving Average (SMA) for a stock. Configure interval, time period, and series type. Use for trend identification.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval, time_period, series_type } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'SMA', symbol, interval, time_period, series_type,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_ema',
    'Get Exponential Moving Average (EMA) for a stock. Weights recent prices more heavily than SMA. Use for responsive trend signals.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval, time_period, series_type } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'EMA', symbol, interval, time_period, series_type,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_rsi',
    'Get Relative Strength Index (RSI) for a stock. Momentum oscillator measuring speed and magnitude of price changes. Values above 70 = overbought, below 30 = oversold.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval, time_period, series_type } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'RSI', symbol, interval, time_period, series_type,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_macd',
    'Get MACD (Moving Average Convergence Divergence) for a stock. Trend-following momentum indicator showing relationship between two EMAs. Returns MACD line, signal line, and histogram.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval, series_type } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'MACD', symbol, interval, series_type,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_bbands',
    'Get Bollinger Bands for a stock. Volatility bands placed above and below a moving average. Returns upper band, middle band (SMA), and lower band.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval, time_period, series_type } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'BBANDS', symbol, interval, time_period, series_type,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_stoch',
    'Get Stochastic Oscillator (STOCH) for a stock. Compares closing price to price range over a period. Returns SlowK and SlowD values.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'STOCH', symbol, interval,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_adx',
    'Get Average Directional Index (ADX) for a stock. Measures trend strength regardless of direction. Values above 25 indicate a strong trend.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval, time_period } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'ADX', symbol, interval, time_period,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_obv',
    'Get On Balance Volume (OBV) for a stock. Cumulative volume indicator that relates volume to price change. Use for confirming price trends.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'OBV', symbol, interval,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_vwap',
    'Get Volume Weighted Average Price (VWAP) for a stock (intraday intervals only). Benchmark price representing the average price weighted by volume.',
    TechnicalSchema.shape,
    async (params) => {
      const { symbol, interval } = TechnicalSchema.parse(params);
      const data = await avFetch({
        function: 'VWAP', symbol, interval,
      }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );
}
