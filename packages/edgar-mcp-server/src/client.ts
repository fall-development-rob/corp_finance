// SEC EDGAR API client with caching and rate limiting
// Two base URLs: data.sec.gov (structured XBRL) and efts.sec.gov (full-text search)
// No API key required — only User-Agent header (SEC requirement)
// Hard rate limit: 10 requests per second

const EDGAR_BASE = 'https://data.sec.gov/';
const EFTS_BASE = 'https://efts.sec.gov/LATEST/';
const USER_AGENT = process.env.EDGAR_USER_AGENT || 'CFA-Agent/1.0 research@robotixai.com';
const RATE_LIMIT = 10; // requests per second (SEC hard limit)
const DEFAULT_CACHE_TTL = Number(process.env.EDGAR_CACHE_TTL ?? 300); // 5 minutes

interface CacheEntry {
  data: unknown;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();
let requestTimestamps: number[] = [];

/** Pad CIK to 10 digits with leading zeros */
export function padCik(cik: string): string {
  const numeric = cik.replace(/\D/g, '');
  return numeric.padStart(10, '0');
}

function isRateLimited(): boolean {
  const now = Date.now();
  requestTimestamps = requestTimestamps.filter(t => now - t < 1_000);
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

export interface EdgarRequestOptions {
  cacheTtl?: number; // seconds, 0 to skip cache
}

async function doFetch<T = unknown>(
  baseUrl: string,
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: EdgarRequestOptions = {},
): Promise<T> {
  // Build URL
  const url = new URL(endpoint, baseUrl);
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

  // Rate limit — 10 req/sec
  if (isRateLimited()) {
    throw new Error('EDGAR rate limit exceeded (10 req/sec). Try again shortly.');
  }

  requestTimestamps.push(Date.now());

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const res = await fetch(url.toString(), {
      headers: {
        'User-Agent': USER_AGENT,
        'Accept': 'application/json',
      },
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 403) throw new Error('EDGAR: Forbidden — check User-Agent header');
      if (res.status === 404) throw new Error(`EDGAR: Not found — ${endpoint}`);
      if (res.status === 429) throw new Error('EDGAR: Rate limited by server — reduce request frequency');
      throw new Error(`EDGAR: HTTP ${res.status} — ${body.slice(0, 200)}`);
    }

    const data = await res.json() as T;

    if (ttl > 0) setCache(cacheKey, data, ttl);

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

/** Fetch from data.sec.gov (structured XBRL/filing data) */
export async function edgarFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: EdgarRequestOptions = {},
): Promise<T> {
  return doFetch<T>(EDGAR_BASE, endpoint, params, options);
}

/** Fetch from efts.sec.gov (full-text search) */
export async function eftsFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: EdgarRequestOptions = {},
): Promise<T> {
  return doFetch<T>(EFTS_BASE, endpoint, params, options);
}

/** Cache TTL presets by data type */
export const CacheTTL = {
  REALTIME: 30,       // quotes, intraday
  SHORT: 300,         // 5 min — search results, recent filings
  MEDIUM: 3600,       // 1 hour — submission histories, company facts
  LONG: 86400,        // 24 hours — company concepts, frames
  STATIC: 604800,     // 7 days — ticker mappings, SIC codes
} as const;
