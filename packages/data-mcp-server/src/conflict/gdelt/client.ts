// GDELT (Global Database of Events, Language, and Tone) API client
// Delegates to shared createApiClient for caching, rate limiting, circuit breaker

import { createApiClient, CacheTTL } from '../../shared/circuit-breaker.js';

export { CacheTTL };

const client = createApiClient({
  baseUrl: process.env.GDELT_BASE_URL || 'https://api.gdeltproject.org/api/v2/doc/doc',
  name: 'GDELT',
  defaultCacheTtl: Number(process.env.GDELT_CACHE_TTL ?? 300),
  politeDelayMs: Number(process.env.GDELT_POLITE_DELAY_MS ?? 200),
});

export interface GdeltRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to GDELT DOC 2.0 API. Auto-appends format=json.
 */
export async function gdeltFetch<T = unknown>(
  params: Record<string, string | number | boolean | undefined> = {},
  options: GdeltRequestOptions = {},
): Promise<T> {
  return client.fetch<T>('', { format: 'json', ...params }, { cacheTtl: options.cacheTtl });
}
