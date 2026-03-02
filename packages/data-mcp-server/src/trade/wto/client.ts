// WTO (World Trade Organization) API client
// Delegates to shared createApiClient for caching, rate limiting, and circuit breaker

import { createApiClient } from '../../shared/circuit-breaker.js';

const WTO_BASE = process.env.WTO_BASE_URL || 'https://api.wto.org/timeseries/v1/';
const WTO_API_KEY = process.env.WTO_API_KEY || '';

const client = createApiClient({
  baseUrl: WTO_BASE,
  name: 'WTO',
  defaultCacheTtl: Number(process.env.WTO_CACHE_TTL ?? 3600),
  politeDelayMs: Number(process.env.WTO_POLITE_DELAY_MS ?? 500),
  timeout: 20_000,
  headers: () => {
    const h: Record<string, string> = {};
    if (WTO_API_KEY) h['Ocp-Apim-Subscription-Key'] = WTO_API_KEY;
    return h;
  },
});

export interface WtoRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to WTO Timeseries API. Optional auth via Ocp-Apim-Subscription-Key header.
 */
export async function wtoFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: WtoRequestOptions = {},
): Promise<T> {
  return client.fetch<T>(endpoint, params, { cacheTtl: options.cacheTtl });
}

/** Cache TTL presets by data freshness */
export const CacheTTL = {
  SHORT: 1800,    // 30 min — active trade data
  MEDIUM: 3600,   // 1 hour — tariff/barrier queries
  LONG: 86400,    // 24 hours — historical trade stats
  STATIC: 604800, // 7 days — indicator metadata, reporter lists
} as const;
