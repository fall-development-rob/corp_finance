// CoinGecko API client with strict polite rate limiting (free tier)
// Delegates to shared createApiClient for caching, rate limiting, and circuit breaker

import { createApiClient } from '../../shared/circuit-breaker.js';

const COINGECKO_BASE = process.env.COINGECKO_BASE_URL || 'https://api.coingecko.com/api/v3';
const COINGECKO_API_KEY = process.env.COINGECKO_API_KEY || '';

const client = createApiClient({
  baseUrl: COINGECKO_BASE,
  name: 'CoinGecko',
  defaultCacheTtl: Number(process.env.COINGECKO_CACHE_TTL ?? 300),
  politeDelayMs: Number(process.env.COINGECKO_POLITE_DELAY_MS ?? 2000),
  timeout: 15_000,
  headers: () => {
    const h: Record<string, string> = {};
    if (COINGECKO_API_KEY) h['x-cg-demo-api-key'] = COINGECKO_API_KEY;
    return h;
  },
});

export interface CoinGeckoRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to CoinGecko API. Optional API key via x-cg-demo-api-key header.
 */
export async function coinGeckoFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: CoinGeckoRequestOptions = {},
): Promise<T> {
  return client.fetch<T>(endpoint, params, { cacheTtl: options.cacheTtl });
}

/** Cache TTL presets */
export const CacheTTL = {
  SHORT: 120,    // 2 min — live price data
  MEDIUM: 300,   // 5 min — general queries (default)
  LONG: 900,     // 15 min — less volatile data
} as const;
