// FRED API client with caching and rate limiting

const FRED_BASE = process.env.FRED_BASE_URL || 'https://api.stlouisfed.org/fred/';
const FRED_KEY = process.env.FRED_API_KEY ?? '';
const RATE_LIMIT = Number(process.env.FRED_RATE_LIMIT ?? 120); // requests per minute
const DEFAULT_CACHE_TTL = Number(process.env.FRED_CACHE_TTL ?? 300); // 5 minutes

interface CacheEntry {
  data: unknown;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();
let requestTimestamps: number[] = [];

function isRateLimited(): boolean {
  const now = Date.now();
  requestTimestamps = requestTimestamps.filter(t => now - t < 60_000);
  return requestTimestamps.length >= RATE_LIMIT;
}

function getCached(key: string): unknown | undefined {
  const entry = cache.get(key);
  if (!entry) return undefined;
  if (Date.now() > entry.expiresAt) {
    cache.delete(key);
    return undefined;
  }
  return entry.data;
}

function setCache(key: string, data: unknown, ttlSeconds: number): void {
  cache.set(key, { data, expiresAt: Date.now() + ttlSeconds * 1000 });
  // Evict old entries if cache grows too large
  if (cache.size > 1000) {
    const now = Date.now();
    for (const [k, v] of cache) {
      if (now > v.expiresAt) cache.delete(k);
    }
  }
}

export interface FredRequestOptions {
  cacheTtl?: number; // seconds, 0 to skip cache
}

export async function fredFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: FredRequestOptions = {},
): Promise<T> {
  if (!FRED_KEY) {
    throw new Error('FRED_API_KEY environment variable is not set');
  }

  // Build URL
  const url = new URL(endpoint, FRED_BASE.endsWith('/') ? FRED_BASE : FRED_BASE + '/');
  url.searchParams.set('api_key', FRED_KEY);
  url.searchParams.set('file_type', 'json');
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined) url.searchParams.set(k, String(v));
  }

  // Check cache
  const cacheKey = url.toString();
  const ttl = options.cacheTtl ?? DEFAULT_CACHE_TTL;
  if (ttl > 0) {
    const cached = getCached(cacheKey);
    if (cached !== undefined) return cached as T;
  }

  // Rate limit
  if (isRateLimited()) {
    throw new Error(`FRED rate limit exceeded (${RATE_LIMIT} req/min). Try again shortly.`);
  }

  requestTimestamps.push(Date.now());

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);

  try {
    const res = await fetch(url.toString(), {
      headers: { 'Accept': 'application/json' },
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 401) throw new Error('FRED: Invalid API key');
      if (res.status === 403) throw new Error('FRED: Forbidden — check API key permissions');
      if (res.status === 429) throw new Error('FRED: Rate limited by server');
      throw new Error(`FRED: HTTP ${res.status} — ${body.slice(0, 200)}`);
    }

    const data = await res.json() as T;

    if (ttl > 0) setCache(cacheKey, data, ttl);

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

/** Cache TTL presets by data type */
export const CacheTTL = {
  REALTIME: 30,       // quotes, intraday
  SHORT: 300,         // 5 min — observations, recent data
  MEDIUM: 3600,       // 1 hour — search results, release dates
  LONG: 86400,        // 24 hours — series info, tags, release metadata
  STATIC: 604800,     // 7 days — categories, tag lists
} as const;
