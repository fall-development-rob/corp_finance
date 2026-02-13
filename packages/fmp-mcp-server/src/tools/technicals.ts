import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import { TechnicalIndicatorSchema } from '../schemas/technicals.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerTechnicalTools(server: McpServer) {
  server.tool(
    'fmp_sma',
    'Calculate Simple Moving Average (SMA) for a stock. Returns SMA values at specified timeframe and period.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/sma`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_ema',
    'Calculate Exponential Moving Average (EMA) for a stock. Returns EMA values at specified timeframe and period. More responsive to recent price changes than SMA.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/ema`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_wma',
    'Calculate Weighted Moving Average (WMA) for a stock. Returns WMA values at specified timeframe and period. Assigns higher weight to recent data points.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/wma`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_dema',
    'Calculate Double Exponential Moving Average (DEMA) for a stock. Returns DEMA values at specified timeframe and period. Reduces lag compared to traditional EMA.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/dema`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_tema',
    'Calculate Triple Exponential Moving Average (TEMA) for a stock. Returns TEMA values at specified timeframe and period. Further reduces lag compared to DEMA.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/tema`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_rsi',
    'Calculate Relative Strength Index (RSI) for a stock. Returns RSI values (0-100) at specified timeframe and period. Values above 70 suggest overbought, below 30 suggest oversold.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/rsi`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_stddev',
    'Calculate Standard Deviation for a stock. Returns standard deviation values at specified timeframe and period. Measures price volatility.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/standarddeviation`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_williams',
    'Calculate Williams %R for a stock. Returns Williams %R values (-100 to 0) at specified timeframe and period. Values above -20 suggest overbought, below -80 suggest oversold.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/williams`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_adx',
    'Calculate Average Directional Index (ADX) for a stock. Returns ADX values at specified timeframe and period. Measures trend strength regardless of direction. Values above 25 indicate a strong trend.',
    TechnicalIndicatorSchema.shape,
    async (params) => {
      const { symbol, periodLength, timeframe } = TechnicalIndicatorSchema.parse(params);
      const data = await fmpFetch(`technical-indicators/adx`, { symbol, periodLength, timeframe }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
