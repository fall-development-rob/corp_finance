// Alpha Vantage API client with caching and rate limiting
// All requests go through: https://www.alphavantage.co/query?function=FUNCTION_NAME&apikey=KEY

const AV_BASE = 'https://www.alphavantage.co/query';
const AV_KEY = process.env.ALPHA_VANTAGE_API_KEY ?? '';
const RATE_LIMIT = Number(process.env.AV_RATE_LIMIT ?? 25); // requests per day (free tier)
const DEFAULT_CACHE_TTL = Number(process.env.AV_CACHE_TTL ?? 300); // 5 minutes

interface CacheEntry {
  data: unknown;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();
let requestTimestamps: number[] = [];

function isRateLimited(): boolean {
  const now = Date.now();
  // Free tier: 25 requests/day; premium: 600+/min — use per-minute window for premium
  requestTimestamps = requestTimestamps.filter(t => now - t < 60_000);
  return requestTimestamps.length >= Math.min(RATE_LIMIT, 75);
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

export interface AvRequestOptions {
  cacheTtl?: number; // seconds, 0 to skip cache
}

export async function avFetch<T = unknown>(
  params: Record<string, string | number | boolean | undefined>,
  options: AvRequestOptions = {},
): Promise<T> {
  if (!AV_KEY) {
    throw new Error('ALPHA_VANTAGE_API_KEY environment variable is not set. Get a free key at https://www.alphavantage.co/support/#api-key');
  }

  // Build URL
  const url = new URL(AV_BASE);
  url.searchParams.set('apikey', AV_KEY);
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
    throw new Error(`Alpha Vantage rate limit exceeded. Free tier allows 25 requests/day. Consider upgrading at https://www.alphavantage.co/premium/`);
  }

  requestTimestamps.push(Date.now());

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const res = await fetch(url.toString(), {
      headers: { 'Accept': 'application/json' },
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 401 || res.status === 403) throw new Error('Alpha Vantage: Invalid API key');
      if (res.status === 429) throw new Error('Alpha Vantage: Rate limited by server');
      throw new Error(`Alpha Vantage: HTTP ${res.status} — ${body.slice(0, 200)}`);
    }

    const data = await res.json() as T;

    // Alpha Vantage returns errors inline in JSON
    const dataObj = data as Record<string, unknown>;
    if (dataObj['Error Message']) {
      throw new Error(`Alpha Vantage: ${dataObj['Error Message']}`);
    }
    if (dataObj['Note']) {
      throw new Error(`Alpha Vantage: ${dataObj['Note']}`);
    }
    if (dataObj['Information'] && typeof dataObj['Information'] === 'string' && dataObj['Information'].includes('API call frequency')) {
      throw new Error(`Alpha Vantage: Rate limited — ${dataObj['Information']}`);
    }

    if (ttl > 0) setCache(cacheKey, data, ttl);

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

/** Cache TTL presets by data type */
export const CacheTTL = {
  REALTIME: 60,       // 1 min — quotes, intraday
  SHORT: 300,         // 5 min — daily prices
  MEDIUM: 3600,       // 1 hour — search results, news
  LONG: 86400,        // 24 hours — fundamentals, company overview
  STATIC: 604800,     // 7 days — listing status, market status
} as const;
