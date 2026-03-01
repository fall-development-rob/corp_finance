// OpenFIGI API client with caching and rate limiting

const FIGI_BASE = process.env.FIGI_BASE_URL || 'https://api.openfigi.com/v3';
const FIGI_KEY = process.env.OPENFIGI_API_KEY ?? '';
const RATE_LIMIT = Number(process.env.FIGI_RATE_LIMIT ?? 250); // requests per minute (conservative)
const DEFAULT_CACHE_TTL = Number(process.env.FIGI_CACHE_TTL ?? 600); // 10 minutes

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
  if (cache.size > 1000) {
    const now = Date.now();
    for (const [k, v] of cache) {
      if (now > v.expiresAt) cache.delete(k);
    }
  }
}

function buildHeaders(): Record<string, string> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'Accept': 'application/json',
  };
  if (FIGI_KEY) {
    headers['X-OPENFIGI-APIKEY'] = FIGI_KEY;
  }
  return headers;
}

export interface FigiRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to OpenFIGI API (for search and enumerations)
 */
export async function figiFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: FigiRequestOptions = {},
): Promise<T> {
  const base = FIGI_BASE.endsWith('/') ? FIGI_BASE : FIGI_BASE + '/';
  const url = new URL(endpoint, base);
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined) url.searchParams.set(k, String(v));
  }

  const cacheKey = `GET:${url.toString()}`;
  const ttl = options.cacheTtl ?? DEFAULT_CACHE_TTL;
  if (ttl > 0) {
    const cached = getCached(cacheKey);
    if (cached !== undefined) return cached as T;
  }

  if (isRateLimited()) {
    throw new Error(`OpenFIGI rate limit exceeded (${RATE_LIMIT} req/min). Try again shortly.`);
  }

  requestTimestamps.push(Date.now());

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);

  try {
    const res = await fetch(url.toString(), {
      method: 'GET',
      headers: buildHeaders(),
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 401) throw new Error('OpenFIGI: Invalid API key');
      if (res.status === 403) throw new Error('OpenFIGI: Access denied');
      if (res.status === 429) throw new Error('OpenFIGI: Rate limited by server');
      throw new Error(`OpenFIGI: HTTP ${res.status} — ${body.slice(0, 200)}`);
    }

    const data = await res.json() as T;
    if (ttl > 0) setCache(cacheKey, data, ttl);
    return data;
  } finally {
    clearTimeout(timeout);
  }
}

/**
 * POST request to OpenFIGI API (for mapping and filter)
 */
export async function figiPost<T = unknown>(
  endpoint: string,
  body: unknown,
  options: FigiRequestOptions = {},
): Promise<T> {
  const base = FIGI_BASE.endsWith('/') ? FIGI_BASE : FIGI_BASE + '/';
  const url = new URL(endpoint, base);

  const bodyStr = JSON.stringify(body);
  const cacheKey = `POST:${url.toString()}:${bodyStr}`;
  const ttl = options.cacheTtl ?? DEFAULT_CACHE_TTL;
  if (ttl > 0) {
    const cached = getCached(cacheKey);
    if (cached !== undefined) return cached as T;
  }

  if (isRateLimited()) {
    throw new Error(`OpenFIGI rate limit exceeded (${RATE_LIMIT} req/min). Try again shortly.`);
  }

  requestTimestamps.push(Date.now());

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const res = await fetch(url.toString(), {
      method: 'POST',
      headers: buildHeaders(),
      body: bodyStr,
      signal: controller.signal,
    });

    if (!res.ok) {
      const respBody = await res.text().catch(() => '');
      if (res.status === 401) throw new Error('OpenFIGI: Invalid API key');
      if (res.status === 403) throw new Error('OpenFIGI: Access denied');
      if (res.status === 429) throw new Error('OpenFIGI: Rate limited by server');
      throw new Error(`OpenFIGI: HTTP ${res.status} — ${respBody.slice(0, 200)}`);
    }

    const data = await res.json() as T;
    if (ttl > 0) setCache(cacheKey, data, ttl);
    return data;
  } finally {
    clearTimeout(timeout);
  }
}

/** Cache TTL presets */
export const CacheTTL = {
  SHORT: 300,      // 5 min — mapping results
  MEDIUM: 3600,    // 1 hour — search results
  LONG: 86400,     // 24 hours — enumerations
} as const;
