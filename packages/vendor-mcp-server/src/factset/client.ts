// FactSet API client with Basic auth, caching, and rate limiting

const FACTSET_BASE = process.env.FACTSET_BASE_URL || 'https://api.factset.com/';
const FACTSET_USERNAME = process.env.FACTSET_USERNAME ?? '';
const FACTSET_API_KEY = process.env.FACTSET_API_KEY ?? '';
const RATE_LIMIT = Number(process.env.FACTSET_RATE_LIMIT ?? 60); // requests per minute
const DEFAULT_CACHE_TTL = Number(process.env.FACTSET_CACHE_TTL ?? 300); // 5 minutes

interface CacheEntry {
  data: unknown;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();
let requestTimestamps: number[] = [];

function getAuthHeader(): string {
  return 'Basic ' + Buffer.from(FACTSET_USERNAME + ':' + FACTSET_API_KEY).toString('base64');
}

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
  // Evict expired entries if cache grows too large
  if (cache.size > 1000) {
    const now = Date.now();
    for (const [k, v] of cache) {
      if (now > v.expiresAt) cache.delete(k);
    }
  }
}

function ensureCredentials(): void {
  if (!FACTSET_USERNAME || !FACTSET_API_KEY) {
    throw new Error('FACTSET_USERNAME and FACTSET_API_KEY environment variables must be set');
  }
}

export interface FactsetRequestOptions {
  cacheTtl?: number; // seconds, 0 to skip cache
}

export async function factsetFetch<T = unknown>(
  path: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: FactsetRequestOptions = {},
): Promise<T> {
  ensureCredentials();

  // Build URL
  const base = FACTSET_BASE.endsWith('/') ? FACTSET_BASE : FACTSET_BASE + '/';
  const url = new URL(path, base);
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
    throw new Error(`FactSet rate limit exceeded (${RATE_LIMIT} req/min). Try again shortly.`);
  }

  requestTimestamps.push(Date.now());

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const res = await fetch(url.toString(), {
      headers: {
        'Accept': 'application/json',
        'Authorization': getAuthHeader(),
      },
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 401) throw new Error('FactSet: Invalid credentials — check FACTSET_USERNAME and FACTSET_API_KEY');
      if (res.status === 403) throw new Error('FactSet: Forbidden — insufficient permissions for this endpoint');
      if (res.status === 429) throw new Error('FactSet: Rate limited by server');
      throw new Error(`FactSet: HTTP ${res.status} — ${body.slice(0, 200)}`);
    }

    const data = await res.json() as T;

    if (ttl > 0) setCache(cacheKey, data, ttl);

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

export async function factsetPost<T = unknown>(
  path: string,
  body: unknown,
  options: FactsetRequestOptions = {},
): Promise<T> {
  ensureCredentials();

  const base = FACTSET_BASE.endsWith('/') ? FACTSET_BASE : FACTSET_BASE + '/';
  const url = new URL(path, base);

  // Check cache (POST bodies hashed via JSON)
  const cacheKey = url.toString() + '::POST::' + JSON.stringify(body);
  const ttl = options.cacheTtl ?? DEFAULT_CACHE_TTL;
  if (ttl > 0) {
    const cached = getCached(cacheKey);
    if (cached !== undefined) return cached as T;
  }

  // Rate limit
  if (isRateLimited()) {
    throw new Error(`FactSet rate limit exceeded (${RATE_LIMIT} req/min). Try again shortly.`);
  }

  requestTimestamps.push(Date.now());

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 30_000);

  try {
    const res = await fetch(url.toString(), {
      method: 'POST',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
        'Authorization': getAuthHeader(),
      },
      body: JSON.stringify(body),
      signal: controller.signal,
    });

    if (!res.ok) {
      const text = await res.text().catch(() => '');
      if (res.status === 401) throw new Error('FactSet: Invalid credentials — check FACTSET_USERNAME and FACTSET_API_KEY');
      if (res.status === 403) throw new Error('FactSet: Forbidden — insufficient permissions for this endpoint');
      if (res.status === 429) throw new Error('FactSet: Rate limited by server');
      throw new Error(`FactSet: HTTP ${res.status} — ${text.slice(0, 200)}`);
    }

    const data = await res.json() as T;

    if (ttl > 0) setCache(cacheKey, data, ttl);

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

export function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

/** Cache TTL presets by data type */
export const CacheTTL = {
  REALTIME: 30,       // quotes, intraday
  SHORT: 300,         // 5 min — fundamentals, estimates
  MEDIUM: 3600,       // 1 hour — search results, ownership
  LONG: 86400,        // 24 hours — company info, events
  STATIC: 604800,     // 7 days — people, supply chain
} as const;
