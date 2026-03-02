// GDACS (Global Disaster Alerting Coordination System) API client
// Delegates to shared createApiClient() for caching, rate limiting, and circuit breaker

import { createApiClient, CacheTTL as SharedCacheTTL } from '../../shared/circuit-breaker.js';

const client = createApiClient({
  baseUrl: 'https://www.gdacs.org/gdacsapi/api/events',
  name: 'GDACS',
  defaultCacheTtl: Number(process.env.GDACS_CACHE_TTL ?? 600),
  politeDelayMs: Number(process.env.GDACS_POLITE_DELAY_MS ?? 500),
});

export interface GdacsRequestOptions {
  cacheTtl?: number;
}

/**
 * Fetch the GDACS event list (MAP endpoint). Returns JSON with events array.
 */
export async function gdacsFetch<T = unknown>(
  options: GdacsRequestOptions = {},
): Promise<T> {
  return client.fetch<T>('geteventlist/MAP', {}, { cacheTtl: options.cacheTtl });
}

/** Re-export CacheTTL so tool imports stay stable */
export const CacheTTL = {
  SHORT: SharedCacheTTL.SHORT,      // 5 min — active alerts
  MEDIUM: 600,                      // 10 min — event list (default)
  LONG: SharedCacheTTL.MEDIUM,      // 30 min — historical data
} as const;
