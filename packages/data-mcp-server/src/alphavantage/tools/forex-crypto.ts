import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { avFetch, CacheTTL } from '../client.js';
import { ForexPairSchema, CryptoSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerForexCryptoTools(server: McpServer) {
  // ── Forex ────────────────────────────────────────────────────────────

  server.tool(
    'av_fx_rate',
    'Get real-time exchange rate for a currency pair. Returns bid/ask prices and last refreshed time. Covers 150+ physical and digital currencies.',
    ForexPairSchema.shape,
    async (params) => {
      const { from_currency, to_currency } = ForexPairSchema.parse(params);
      const data = await avFetch({
        function: 'CURRENCY_EXCHANGE_RATE',
        from_currency, to_currency,
      }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_fx_daily',
    'Get daily OHLC time series for a forex pair. Returns open, high, low, close for each trading day. Full history available.',
    ForexPairSchema.shape,
    async (params) => {
      const { from_currency, to_currency } = ForexPairSchema.parse(params);
      const data = await avFetch({
        function: 'FX_DAILY',
        from_symbol: from_currency,
        to_symbol: to_currency,
        outputsize: 'compact',
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_fx_monthly',
    'Get monthly OHLC time series for a forex pair. Returns last trading day of each month. Full history for long-term FX trend analysis.',
    ForexPairSchema.shape,
    async (params) => {
      const { from_currency, to_currency } = ForexPairSchema.parse(params);
      const data = await avFetch({
        function: 'FX_MONTHLY',
        from_symbol: from_currency,
        to_symbol: to_currency,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  // ── Crypto ───────────────────────────────────────────────────────────

  server.tool(
    'av_crypto_rate',
    'Get real-time exchange rate for a cryptocurrency. Returns bid/ask in the specified market currency.',
    CryptoSchema.shape,
    async (params) => {
      const { symbol, market } = CryptoSchema.parse(params);
      const data = await avFetch({
        function: 'CURRENCY_EXCHANGE_RATE',
        from_currency: symbol,
        to_currency: market,
      }, { cacheTtl: CacheTTL.REALTIME });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_crypto_daily',
    'Get daily OHLCV time series for a cryptocurrency in the specified market. Returns open, high, low, close, volume, and market cap.',
    CryptoSchema.shape,
    async (params) => {
      const { symbol, market } = CryptoSchema.parse(params);
      const data = await avFetch({
        function: 'DIGITAL_CURRENCY_DAILY',
        symbol, market,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_crypto_monthly',
    'Get monthly OHLCV time series for a cryptocurrency. Full history for long-term crypto trend analysis and market cap tracking.',
    CryptoSchema.shape,
    async (params) => {
      const { symbol, market } = CryptoSchema.parse(params);
      const data = await avFetch({
        function: 'DIGITAL_CURRENCY_MONTHLY',
        symbol, market,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
