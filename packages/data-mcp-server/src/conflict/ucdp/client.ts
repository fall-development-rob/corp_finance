// UCDP (Uppsala Conflict Data Program) API client
// Delegates to shared createApiClient for caching, rate limiting, circuit breaker

import { createApiClient, CacheTTL } from '../../shared/circuit-breaker.js';

export { CacheTTL };

const client = createApiClient({
  baseUrl: process.env.UCDP_BASE_URL || 'https://ucdpapi.pcr.uu.se/api/',
  name: 'UCDP',
  defaultCacheTtl: Number(process.env.UCDP_CACHE_TTL ?? 3600),
  politeDelayMs: Number(process.env.UCDP_POLITE_DELAY_MS ?? 334),
});

export interface UcdpRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to UCDP API. No authentication required.
 */
export async function ucdpFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: UcdpRequestOptions = {},
): Promise<T> {
  return client.fetch<T>(endpoint, params, { cacheTtl: options.cacheTtl });
}
