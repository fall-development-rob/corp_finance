// UNHCR Population Data API client
// Delegates to shared createApiClient for caching, rate limiting, and circuit breaker

import { createApiClient } from '../../shared/circuit-breaker.js';

const UNHCR_BASE = process.env.UNHCR_BASE_URL || 'https://api.unhcr.org/population/v1';

const client = createApiClient({
  baseUrl: UNHCR_BASE,
  name: 'UNHCR',
  defaultCacheTtl: Number(process.env.UNHCR_CACHE_TTL ?? 3600),
  politeDelayMs: Number(process.env.UNHCR_POLITE_DELAY_MS ?? 1000),
  timeout: 20_000,
});

export interface UnhcrRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to UNHCR Population API. No auth required.
 */
export async function unhcrFetch<T = unknown>(
  params: Record<string, string | number | boolean | undefined> = {},
  options: UnhcrRequestOptions = {},
): Promise<T> {
  return client.fetch<T>('population/', params, { cacheTtl: options.cacheTtl });
}

/** Cache TTL presets */
export const CacheTTL = {
  SHORT: 1800,   // 30 min — recent queries
  MEDIUM: 3600,  // 1 hour — standard (default)
  LONG: 86400,   // 24 hours — historical data (updates annually)
} as const;
