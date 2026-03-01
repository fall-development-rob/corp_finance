// Moody's Analytics API client with OAuth2, caching, and rate limiting

const MOODYS_BASE = process.env.MOODYS_BASE_URL || 'https://api.moodys.com/';
const MOODYS_TOKEN_URL = process.env.MOODYS_TOKEN_URL || 'https://api.moodys.com/oauth2/token';
const MOODYS_CLIENT_ID = process.env.MOODYS_CLIENT_ID ?? '';
const MOODYS_CLIENT_SECRET = process.env.MOODYS_CLIENT_SECRET ?? '';
const RATE_LIMIT = Number(process.env.MOODYS_RATE_LIMIT ?? 30); // requests per minute
const DEFAULT_CACHE_TTL = Number(process.env.MOODYS_CACHE_TTL ?? 300); // 5 minutes

// --- OAuth2 Token Management ---

let tokenCache: { accessToken: string; expiresAt: number } | null = null;

async function getAccessToken(): Promise<string> {
  if (!MOODYS_CLIENT_ID || !MOODYS_CLIENT_SECRET) {
    throw new Error('MOODYS_CLIENT_ID and MOODYS_CLIENT_SECRET environment variables must be set');
  }

  // Return cached token if still valid (refresh 30s before expiry)
  if (tokenCache && Date.now() < tokenCache.expiresAt - 30_000) {
    return tokenCache.accessToken;
  }

  const body = new URLSearchParams({
    grant_type: 'client_credentials',
    client_id: MOODYS_CLIENT_ID,
    client_secret: MOODYS_CLIENT_SECRET,
  });

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);

  try {
    const res = await fetch(MOODYS_TOKEN_URL, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Accept': 'application/json',
      },
      body: body.toString(),
      signal: controller.signal,
    });

    if (!res.ok) {
      const text = await res.text().catch(() => '');
      if (res.status === 401) throw new Error('Moodys: Invalid client credentials');
      if (res.status === 403) throw new Error('Moodys: Forbidden — check client permissions');
      throw new Error(`Moodys token error: HTTP ${res.status} — ${text.slice(0, 200)}`);
    }

    const data = await res.json() as { access_token: string; expires_in: number };
    tokenCache = {
      accessToken: data.access_token,
      expiresAt: Date.now() + data.expires_in * 1000,
    };

    return tokenCache.accessToken;
  } finally {
    clearTimeout(timeout);
  }
}

// --- LRU Cache ---

interface CacheEntry {
  data: unknown;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();

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

// --- Sliding-Window Rate Limiter ---

let requestTimestamps: number[] = [];

function isRateLimited(): boolean {
  const now = Date.now();
  requestTimestamps = requestTimestamps.filter(t => now - t < 60_000);
  return requestTimestamps.length >= RATE_LIMIT;
}

// --- Main Fetch ---

export interface MoodysRequestOptions {
  cacheTtl?: number; // seconds, 0 to skip cache
}

export async function moodysFetch<T = unknown>(
  path: string,
  params: Record<string, string | number | boolean | undefined> = {},
  options: MoodysRequestOptions = {},
): Promise<T> {
  // Build URL
  const base = MOODYS_BASE.endsWith('/') ? MOODYS_BASE : MOODYS_BASE + '/';
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
    throw new Error(`Moodys rate limit exceeded (${RATE_LIMIT} req/min). Try again shortly.`);
  }

  requestTimestamps.push(Date.now());

  // Get OAuth2 access token
  const accessToken = await getAccessToken();

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const res = await fetch(url.toString(), {
      headers: {
        'Authorization': `Bearer ${accessToken}`,
        'Accept': 'application/json',
      },
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 401) {
        // Token may have expired — clear cache and retry once
        tokenCache = null;
        throw new Error('Moodys: Unauthorized — token may have expired');
      }
      if (res.status === 403) throw new Error('Moodys: Forbidden — check entitlements');
      if (res.status === 429) throw new Error('Moodys: Rate limited by server');
      throw new Error(`Moodys: HTTP ${res.status} — ${body.slice(0, 200)}`);
    }

    const data = await res.json() as T;

    if (ttl > 0) setCache(cacheKey, data, ttl);

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

// --- Response Wrapper ---

export function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

// --- Cache TTL Presets ---

export const CacheTTL = {
  REALTIME: 30,       // quotes, intraday
  SHORT: 300,         // 5 min — recent ratings, news
  MEDIUM: 3600,       // 1 hour — forecasts, search results
  LONG: 86400,        // 24 hours — reference data, issuer profiles
  STATIC: 604800,     // 7 days — transition matrices, historical defaults
} as const;
