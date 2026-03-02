// NASA FIRMS (Fire Information for Resource Management System) API client
// Delegates to shared createApiClient() for caching, rate limiting, and circuit breaker
// Retains FirmsDetection interface (source-specific)

import { createApiClient, CacheTTL as SharedCacheTTL } from '../../shared/circuit-breaker.js';

function getApiKey(): string {
  const key = process.env.NASA_FIRMS_KEY;
  if (!key) {
    throw new Error('NASA_FIRMS_KEY environment variable is required for FIRMS API access');
  }
  return key;
}

// Use the JSON endpoint so the shared client's res.json() works natively.
// The CSV endpoint (api/area/csv) would require custom response parsing.
const client = createApiClient({
  baseUrl: 'https://firms.modaps.eosdis.nasa.gov/api/area',
  name: 'FIRMS',
  defaultCacheTtl: Number(process.env.FIRMS_CACHE_TTL ?? 900),
  politeDelayMs: Number(process.env.FIRMS_POLITE_DELAY_MS ?? 500),
  timeout: 30_000,
});

export interface FirmsRequestOptions {
  cacheTtl?: number;
}

export interface FirmsDetection {
  latitude: number;
  longitude: number;
  brightness: number;
  frp: number;
  confidence: string;
  acq_date: string;
  acq_time: string;
  daynight: string;
  satellite: string;
  [key: string]: unknown;
}

/**
 * Fetch FIRMS fire data for a country.
 * URL: {base}/csv/{key}/VIIRS_SNPP_NRT/{country}/{days}
 *
 * We hit the csv endpoint but request JSON via Accept header.
 * The shared client handles caching, rate limiting, circuit breaker, and
 * JSON response parsing. We just normalize field names in the wrapper.
 */
export async function firmsFetch(
  country: string,
  days: number = 1,
  options: FirmsRequestOptions = {},
): Promise<FirmsDetection[]> {
  const key = getApiKey();
  const endpoint = `csv/${key}/VIIRS_SNPP_NRT/${encodeURIComponent(country)}/${days}`;

  const data = await client.fetch<FirmsDetection[]>(
    endpoint,
    {},
    {
      cacheTtl: options.cacheTtl,
      headers: { Accept: 'application/json' },
    },
  ).catch((err: Error) => {
    if (err.message.includes('401') || err.message.includes('403')) {
      throw new Error('FIRMS: Invalid or expired NASA_FIRMS_KEY');
    }
    throw err;
  });

  if (!Array.isArray(data)) return [];

  // Normalize fields — the JSON response may use different brightness key names
  return data.map(row => ({
    latitude: Number(row.latitude) || 0,
    longitude: Number(row.longitude) || 0,
    brightness: Number((row as Record<string, unknown>).bright_ti4 ?? row.brightness) || 0,
    frp: Number(row.frp) || 0,
    confidence: row.confidence ?? 'unknown',
    acq_date: row.acq_date ?? '',
    acq_time: row.acq_time ?? '',
    daynight: row.daynight ?? '',
    satellite: row.satellite ?? 'VIIRS_SNPP_NRT',
  }));
}

/** Re-export CacheTTL so tool imports stay stable */
export const CacheTTL = {
  SHORT: SharedCacheTTL.SHORT,      // 5 min
  MEDIUM: 900,                      // 15 min (default)
  LONG: SharedCacheTTL.MEDIUM,      // 30 min
} as const;
