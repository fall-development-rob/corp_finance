// EIA (U.S. Energy Information Administration) API v2 client
// Delegates to shared createApiClient for caching, rate limiting, and circuit breaker

import { createApiClient } from '../../shared/circuit-breaker.js';

const EIA_BASE = process.env.EIA_BASE_URL || 'https://api.eia.gov/v2/';
const EIA_API_KEY = process.env.EIA_API_KEY || '';

const client = createApiClient({
  baseUrl: EIA_BASE,
  name: 'EIA',
  defaultCacheTtl: Number(process.env.EIA_CACHE_TTL ?? 3600),
  politeDelayMs: Number(process.env.EIA_POLITE_DELAY_MS ?? 500),
  timeout: 15_000,
});

export interface EiaRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to EIA API v2. Auth via api_key query param.
 */
export async function eiaFetch<T = unknown>(
  route: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: EiaRequestOptions = {},
): Promise<T> {
  if (!EIA_API_KEY) {
    throw new Error('EIA: EIA_API_KEY environment variable is required');
  }
  return client.fetch<T>(route, { api_key: EIA_API_KEY, ...params }, { cacheTtl: options.cacheTtl });
}

/** Cache TTL presets by data freshness */
export const CacheTTL = {
  SHORT: 900,     // 15 min — near-real-time (weekly releases)
  MEDIUM: 3600,   // 1 hour — standard energy data
  LONG: 86400,    // 24 hours — monthly aggregates
  STATIC: 604800, // 7 days — capacity/infrastructure data
} as const;
