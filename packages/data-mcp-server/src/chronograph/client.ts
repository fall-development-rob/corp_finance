// Chronograph PE/VC portfolio monitoring API client with caching and rate limiting

const CHRONOGRAPH_BASE = process.env.CHRONOGRAPH_BASE_URL || 'https://api.chronograph.pe/v1';
const CHRONOGRAPH_KEY = process.env.CHRONOGRAPH_API_KEY ?? '';
const RATE_LIMIT = Number(process.env.CHRONOGRAPH_RATE_LIMIT ?? 60); // requests per minute
const DEFAULT_CACHE_TTL = Number(process.env.CHRONOGRAPH_CACHE_TTL ?? 300); // 5 minutes

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

export interface ChronographRequestOptions {
  cacheTtl?: number; // seconds, 0 to skip cache
}

/** Returns true when CHRONOGRAPH_API_KEY is present in the environment. */
export function isCredentialed(): boolean {
  return CHRONOGRAPH_KEY.length > 0;
}

export async function chronographFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: ChronographRequestOptions = {},
): Promise<T> {
  if (!CHRONOGRAPH_KEY) {
    throw new Error('CHRONOGRAPH_API_KEY environment variable is not set');
  }

  const base = CHRONOGRAPH_BASE.endsWith('/') ? CHRONOGRAPH_BASE.slice(0, -1) : CHRONOGRAPH_BASE;
  const url = new URL(`${base}${endpoint}`);
  for (const [k, v] of Object.entries(params)) {
    if (v !== undefined) url.searchParams.set(k, String(v));
  }

  const cacheKey = `${endpoint}|${JSON.stringify(params)}`;
  const ttl = options.cacheTtl ?? DEFAULT_CACHE_TTL;
  if (ttl > 0) {
    const cached = getCached(cacheKey);
    if (cached !== undefined) return cached as T;
  }

  if (isRateLimited()) {
    throw new Error(`Chronograph rate limit exceeded (${RATE_LIMIT} req/min). Try again shortly.`);
  }

  requestTimestamps.push(Date.now());

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);

  try {
    const res = await fetch(url.toString(), {
      headers: {
        'Authorization': `Bearer ${CHRONOGRAPH_KEY}`,
        'Accept': 'application/json',
      },
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 401) throw new Error('Chronograph: Invalid API key');
      if (res.status === 403) throw new Error('Chronograph: Forbidden — check API key permissions');
      if (res.status === 429) throw new Error('Chronograph: Rate limited by server');
      throw new Error(`Chronograph: HTTP ${res.status} — ${body.slice(0, 200)}`);
    }

    const data = await res.json() as T;

    if (ttl > 0) setCache(cacheKey, data, ttl);

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

/** Cache TTL presets (seconds) */
export const CacheTTL = {
  SHORT: 60,
  MEDIUM: 300,
  LONG: 3600,
} as const;
