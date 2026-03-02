// USGS Earthquake Hazards Program API client
// Delegates to shared createApiClient() for caching, rate limiting, and circuit breaker

import { createApiClient, CacheTTL as SharedCacheTTL } from '../../shared/circuit-breaker.js';

const client = createApiClient({
  baseUrl: 'https://earthquake.usgs.gov/fdsnws/event/1',
  name: 'USGS',
  defaultCacheTtl: Number(process.env.USGS_CACHE_TTL ?? 300),
  politeDelayMs: Number(process.env.USGS_POLITE_DELAY_MS ?? 500),
});

export interface UsgsRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to USGS Earthquake API. Auto-appends format=geojson.
 */
export async function usgsFetch<T = unknown>(
  params: Record<string, string | number | boolean | undefined> = {},
  options: UsgsRequestOptions = {},
): Promise<T> {
  return client.fetch<T>('query', { format: 'geojson', ...params }, { cacheTtl: options.cacheTtl });
}

/** Re-export CacheTTL so tool imports stay stable */
export const CacheTTL = {
  SHORT: 180,                       // 3 min — recent quakes
  MEDIUM: SharedCacheTTL.SHORT,     // 5 min — standard queries (default)
  LONG: 900,                        // 15 min — significant events
} as const;
