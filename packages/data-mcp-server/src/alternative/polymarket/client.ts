// Polymarket Gamma API client
// Delegates to shared createApiClient for caching, rate limiting, and circuit breaker

import { createApiClient } from '../../shared/circuit-breaker.js';

const POLYMARKET_BASE = process.env.POLYMARKET_BASE_URL || 'https://gamma-api.polymarket.com';

const client = createApiClient({
  baseUrl: POLYMARKET_BASE,
  name: 'Polymarket',
  defaultCacheTtl: Number(process.env.POLYMARKET_CACHE_TTL ?? 120),
  politeDelayMs: Number(process.env.POLYMARKET_POLITE_DELAY_MS ?? 500),
  timeout: 15_000,
});

export interface PolymarketRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to Polymarket Gamma API. No auth required.
 */
export async function polymarketFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: PolymarketRequestOptions = {},
): Promise<T> {
  return client.fetch<T>(endpoint, params, { cacheTtl: options.cacheTtl });
}

/** Cache TTL presets */
export const CacheTTL = {
  SHORT: 60,     // 1 min — live odds
  MEDIUM: 120,   // 2 min — event listings (default)
  LONG: 600,     // 10 min — aggregated data
} as const;
