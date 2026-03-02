// USASpending API client
// Delegates to shared createApiClient for caching, rate limiting, and circuit breaker

import { createApiClient } from '../../shared/circuit-breaker.js';

const USA_SPENDING_BASE = process.env.USA_SPENDING_BASE_URL || 'https://api.usaspending.gov/api/v2/';

const client = createApiClient({
  baseUrl: USA_SPENDING_BASE,
  name: 'USASpending',
  defaultCacheTtl: Number(process.env.USA_SPENDING_CACHE_TTL ?? 900),
  politeDelayMs: Number(process.env.USA_SPENDING_POLITE_DELAY_MS ?? 200),
  timeout: 20_000,
});

export interface UsaSpendingRequestOptions {
  cacheTtl?: number;
}

/**
 * Request to USASpending API. No auth required.
 * Supports both GET (body undefined) and POST (body provided).
 */
export async function usaSpendingFetch<T = unknown>(
  endpoint: string,
  body?: Record<string, unknown>,
  options: UsaSpendingRequestOptions = {},
): Promise<T> {
  if (body !== undefined) {
    return client.fetch<T>(endpoint, {}, { cacheTtl: options.cacheTtl, method: 'POST', body });
  }
  return client.fetch<T>(endpoint, {}, { cacheTtl: options.cacheTtl });
}

/** Cache TTL presets by data freshness */
export const CacheTTL = {
  SHORT: 300,     // 5 min — active contract searches
  MEDIUM: 900,    // 15 min — standard queries
  LONG: 3600,     // 1 hour — agency summaries
  STATIC: 86400,  // 24 hours — historical/fiscal year data
} as const;
