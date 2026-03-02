// ACLED (Armed Conflict Location & Event Data) API client
// Delegates to shared createApiClient for caching, rate limiting, circuit breaker

import { createApiClient, CacheTTL } from '../../shared/circuit-breaker.js';

export { CacheTTL };

const ACLED_BASE = process.env.ACLED_BASE_URL || 'https://api.acleddata.com/acled/read';
const ACLED_API_KEY = process.env.ACLED_API_KEY || '';
const ACLED_EMAIL = process.env.ACLED_EMAIL || '';

const client = createApiClient({
  baseUrl: ACLED_BASE,
  name: 'ACLED',
  defaultCacheTtl: Number(process.env.ACLED_CACHE_TTL ?? 900),
  politeDelayMs: Number(process.env.ACLED_POLITE_DELAY_MS ?? 500),
});

export interface AcledRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to ACLED API. Auto-appends key and email for auth.
 */
export async function acledFetch<T = unknown>(
  params: Record<string, string | number | boolean | undefined> = {},
  options: AcledRequestOptions = {},
): Promise<T> {
  if (!ACLED_API_KEY || !ACLED_EMAIL) {
    throw new Error('ACLED: ACLED_API_KEY and ACLED_EMAIL environment variables are required');
  }

  return client.fetch<T>('', { key: ACLED_API_KEY, email: ACLED_EMAIL, ...params }, {
    cacheTtl: options.cacheTtl,
  });
}
