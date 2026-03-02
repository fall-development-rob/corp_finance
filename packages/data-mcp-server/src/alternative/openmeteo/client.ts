// Open-Meteo Archive API client
// Delegates to shared createApiClient for caching, rate limiting, and circuit breaker

import { createApiClient } from '../../shared/circuit-breaker.js';

const OPENMETEO_BASE =
  process.env.OPENMETEO_BASE_URL || 'https://archive-api.open-meteo.com/v1/archive';

const client = createApiClient({
  baseUrl: OPENMETEO_BASE,
  name: 'Open-Meteo',
  defaultCacheTtl: Number(process.env.OPENMETEO_CACHE_TTL ?? 3600),
  politeDelayMs: Number(process.env.OPENMETEO_POLITE_DELAY_MS ?? 200),
  timeout: 15_000,
});

export interface OpenMeteoRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to Open-Meteo Archive API. No auth required.
 * Params are appended to the base URL as query parameters.
 */
export async function openMeteoFetch<T = unknown>(
  params: Record<string, string | number | boolean | undefined>,
  options: OpenMeteoRequestOptions = {},
): Promise<T> {
  return client.fetch<T>('', params, { cacheTtl: options.cacheTtl });
}

/** Cache TTL presets */
export const CacheTTL = {
  SHORT: 1800,   // 30 min — recent observation window
  MEDIUM: 3600,  // 1 hour — standard (default)
  LONG: 86400,   // 24 hours — baseline climatology
} as const;
