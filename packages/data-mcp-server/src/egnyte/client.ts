// Egnyte enterprise content management API client with caching and rate limiting

const EGNYTE_DOMAIN = process.env.EGNYTE_DOMAIN ?? '';
const EGNYTE_KEY = process.env.EGNYTE_API_KEY ?? '';
const RATE_LIMIT = Number(process.env.EGNYTE_RATE_LIMIT ?? 30); // per second per token, per Egnyte docs
const DEFAULT_CACHE_TTL = Number(process.env.EGNYTE_CACHE_TTL ?? 60);

interface CacheEntry {
  data: unknown;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();
let requestTimestamps: number[] = [];

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
  if (cache.size > 1000) {
    const now = Date.now();
    for (const [k, v] of cache) {
      if (now > v.expiresAt) cache.delete(k);
    }
  }
}

export interface EgnyteRequestOptions {
  cacheTtl?: number;
  method?: 'GET' | 'POST';
  body?: unknown;
}

/** Returns true when both EGNYTE_DOMAIN and EGNYTE_API_KEY are present. */
export function isCredentialed(): boolean {
  return EGNYTE_DOMAIN.length > 0 && EGNYTE_KEY.length > 0;
}

function buildBaseUrl(): string {
  if (!EGNYTE_DOMAIN) {
    throw new Error('EGNYTE_DOMAIN environment variable is not set');
  }
  return `https://${EGNYTE_DOMAIN}.egnyte.com/pubapi/v1`;
}

export async function egnyteFetch<T = unknown>(
  endpoint: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: EgnyteRequestOptions = {},
): Promise<T> {
  if (!EGNYTE_KEY) throw new Error('EGNYTE_API_KEY environment variable is not set');

  const base = buildBaseUrl();
  const method = options.method ?? 'GET';
  const cacheKey = `${endpoint}|${JSON.stringify(params)}`;
  const ttl = options.cacheTtl ?? DEFAULT_CACHE_TTL;

  if (ttl > 0 && method === 'GET') {
    const cached = getCached(cacheKey);
    if (cached !== undefined) return cached as T;
  }

  if (isRateLimited()) {
    throw new Error(`Egnyte rate limit exceeded (${RATE_LIMIT} req/s). Try again shortly.`);
  }

  requestTimestamps.push(Date.now());

  const url = new URL(`${base}${endpoint}`);
  if (method === 'GET') {
    for (const [k, v] of Object.entries(params)) {
      if (v !== undefined) url.searchParams.set(k, String(v));
    }
  }

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);

  try {
    const fetchOptions: RequestInit = {
      method,
      headers: {
        'Authorization': `Bearer ${EGNYTE_KEY}`,
        'Accept': 'application/json',
        ...(method === 'POST' ? { 'Content-Type': 'application/json' } : {}),
      },
      signal: controller.signal,
      ...(method === 'POST' && options.body !== undefined
        ? { body: JSON.stringify(options.body) }
        : {}),
    };

    const res = await fetch(url.toString(), fetchOptions);

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 401) throw new Error('Egnyte: Invalid API key');
      if (res.status === 403) throw new Error('Egnyte: Forbidden — check API key permissions');
      if (res.status === 429) throw new Error('Egnyte: Rate limited by server');
      throw new Error(`Egnyte: HTTP ${res.status} — ${body.slice(0, 200)}`);
    }

    const data = await res.json() as T;

    if (ttl > 0 && method === 'GET') setCache(cacheKey, data, ttl);

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

/** Cache TTL presets (seconds) */
export const CacheTTL = { SHORT: 30, MEDIUM: 300, LONG: 3600 } as const;
