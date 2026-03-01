// World Bank API client with caching and polite rate limiting

const WB_BASE = process.env.WB_BASE_URL || 'https://api.worldbank.org/v2';
const POLITE_DELAY_MS = Number(process.env.WB_POLITE_DELAY_MS ?? 334); // ~3 req/sec
const DEFAULT_CACHE_TTL = Number(process.env.WB_CACHE_TTL ?? 3600); // 1 hour (data updates infrequently)

interface CacheEntry {
  data: unknown;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();
let lastRequestTime = 0;

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

async function politeWait(): Promise<void> {
  const now = Date.now();
  const elapsed = now - lastRequestTime;
  if (elapsed < POLITE_DELAY_MS) {
    await new Promise(resolve => setTimeout(resolve, POLITE_DELAY_MS - elapsed));
  }
  lastRequestTime = Date.now();
}

export interface WbRequestOptions {
  cacheTtl?: number;
}

/**
 * GET request to World Bank API. Auto-appends format=json.
 */
export async function wbFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: WbRequestOptions = {},
): Promise<T> {
  const base = WB_BASE.endsWith('/') ? WB_BASE : WB_BASE + '/';
  const url = new URL(endpoint, base);

  // World Bank API requires format=json (default is XML)
  url.searchParams.set('format', 'json');
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined) url.searchParams.set(k, String(v));
  }

  const cacheKey = url.toString();
  const ttl = options.cacheTtl ?? DEFAULT_CACHE_TTL;
  if (ttl > 0) {
    const cached = getCached(cacheKey);
    if (cached !== undefined) return cached as T;
  }

  await politeWait();

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const res = await fetch(url.toString(), {
      method: 'GET',
      headers: { 'Accept': 'application/json' },
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 429) throw new Error('World Bank: Rate limited by server');
      throw new Error(`World Bank: HTTP ${res.status} — ${body.slice(0, 200)}`);
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
  SHORT: 1800,     // 30 min — indicator data (may update)
  MEDIUM: 3600,    // 1 hour — country info
  LONG: 86400,     // 24 hours — metadata, topics, sources
  STATIC: 604800,  // 7 days — income levels, enumerations
} as const;
