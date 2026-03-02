// NASA EONET (Earth Observatory Natural Event Tracker) API v3 client
// Delegates to shared createApiClient() for caching, rate limiting, and circuit breaker

import { createApiClient, CacheTTL as SharedCacheTTL } from '../../shared/circuit-breaker.js';

const client = createApiClient({
  baseUrl: 'https://eonet.gsfc.nasa.gov/api/v3',
  name: 'EONET',
  defaultCacheTtl: Number(process.env.EONET_CACHE_TTL ?? 1800),
  politeDelayMs: Number(process.env.EONET_POLITE_DELAY_MS ?? 500),
});

export interface EonetRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to NASA EONET API v3.
 */
export async function eonetFetch<T = unknown>(
  endpoint: string = 'events',
  params: Record<string, string | number | boolean | undefined> = {},
  options: EonetRequestOptions = {},
): Promise<T> {
  return client.fetch<T>(endpoint, params, { cacheTtl: options.cacheTtl });
}

/** Re-export CacheTTL so tool imports stay stable */
export const CacheTTL = {
  SHORT: 900,                       // 15 min
  MEDIUM: SharedCacheTTL.MEDIUM,    // 30 min (default)
  LONG: SharedCacheTTL.LONG,        // 1 hour — categories rarely change
} as const;
