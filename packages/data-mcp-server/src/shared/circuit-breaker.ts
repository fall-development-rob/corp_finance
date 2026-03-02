// Reusable API client with circuit breaker, caching, and polite rate limiting
// Follows the data-mcp-server/wb/client.ts pattern but generic and reusable

import type { CacheEntry } from './types.js';

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

export interface ApiClientConfig {
  /** Base URL for the API (no trailing slash) */
  baseUrl: string;
  /** Human-readable source name for error messages */
  name: string;
  /** Default cache TTL in seconds (0 = no cache) */
  defaultCacheTtl: number;
  /** Minimum delay between requests in ms (polite rate limiting) */
  politeDelayMs: number;
  /** Request timeout in ms (default 15000) */
  timeout?: number;
  /** Optional function returning headers (e.g. for auth tokens) */
  headers?: () => Record<string, string>;
}

export interface RequestOptions {
  /** Override the default cache TTL for this request (seconds, 0 = skip cache) */
  cacheTtl?: number;
  /** Extra headers for this specific request */
  headers?: Record<string, string>;
  /** HTTP method override (default GET) */
  method?: 'GET' | 'POST';
  /** Request body for POST requests */
  body?: unknown;
}

// ---------------------------------------------------------------------------
// Circuit breaker states
// ---------------------------------------------------------------------------

type CircuitState = 'closed' | 'open' | 'half-open';

const FAILURE_THRESHOLD = 5;
const RECOVERY_MS = 60_000; // 60 seconds before half-open

// ---------------------------------------------------------------------------
// Cache helpers
// ---------------------------------------------------------------------------

function getCached(cache: Map<string, CacheEntry>, key: string): unknown | undefined {
  const entry = cache.get(key);
  if (!entry) return undefined;
  if (Date.now() > entry.expiresAt) {
    cache.delete(key);
    return undefined;
  }
  return entry.data;
}

function setCache(
  cache: Map<string, CacheEntry>,
  key: string,
  data: unknown,
  ttlSeconds: number,
): void {
  cache.set(key, { data, expiresAt: Date.now() + ttlSeconds * 1000 });
  // Evict expired entries when cache grows beyond 1000
  if (cache.size > 1000) {
    const now = Date.now();
    for (const [k, v] of cache) {
      if (now > v.expiresAt) cache.delete(k);
    }
  }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export interface ApiClient {
  fetch: <T = unknown>(
    endpoint: string,
    params?: Record<string, string | number | boolean | undefined>,
    options?: RequestOptions,
  ) => Promise<T>;
  /** Expose cache for testing / manual invalidation */
  cache: Map<string, CacheEntry>;
}

export function createApiClient(config: ApiClientConfig): ApiClient {
  const cache = new Map<string, CacheEntry>();
  let lastRequestTime = 0;

  // Circuit breaker state
  let circuitState: CircuitState = 'closed';
  let consecutiveFailures = 0;
  let lastFailureTime = 0;

  const timeout = config.timeout ?? 15_000;

  // -- Polite wait ----------------------------------------------------------

  async function politeWait(): Promise<void> {
    const now = Date.now();
    const elapsed = now - lastRequestTime;
    if (elapsed < config.politeDelayMs) {
      await new Promise(resolve => setTimeout(resolve, config.politeDelayMs - elapsed));
    }
    lastRequestTime = Date.now();
  }

  // -- Circuit breaker helpers ----------------------------------------------

  function checkCircuit(): void {
    if (circuitState === 'open') {
      const elapsed = Date.now() - lastFailureTime;
      if (elapsed >= RECOVERY_MS) {
        circuitState = 'half-open';
      } else {
        throw new Error(
          `${config.name}: Circuit breaker OPEN — ${FAILURE_THRESHOLD} consecutive failures. ` +
            `Retry in ${Math.ceil((RECOVERY_MS - elapsed) / 1000)}s`,
        );
      }
    }
  }

  function onSuccess(): void {
    consecutiveFailures = 0;
    circuitState = 'closed';
  }

  function onFailure(): void {
    consecutiveFailures++;
    lastFailureTime = Date.now();
    if (consecutiveFailures >= FAILURE_THRESHOLD) {
      circuitState = 'open';
    }
  }

  // -- Fetch ----------------------------------------------------------------

  async function apiFetch<T = unknown>(
    endpoint: string,
    params: Record<string, string | number | boolean | undefined> = {},
    options: RequestOptions = {},
  ): Promise<T> {
    // Build URL
    const base = config.baseUrl.endsWith('/') ? config.baseUrl : config.baseUrl + '/';
    const url = new URL(endpoint, base);

    for (const [k, v] of Object.entries(params)) {
      if (v !== undefined) url.searchParams.set(k, String(v));
    }

    // Cache lookup
    const method = options.method ?? 'GET';
    const cacheKey = method === 'GET' ? url.toString() : '';
    const ttl = options.cacheTtl ?? config.defaultCacheTtl;

    if (method === 'GET' && ttl > 0) {
      const cached = getCached(cache, cacheKey);
      if (cached !== undefined) return cached as T;
    }

    // Circuit breaker check
    checkCircuit();

    // Polite rate limiting
    await politeWait();

    // Build headers
    const mergedHeaders: Record<string, string> = {
      Accept: 'application/json',
      ...(config.headers ? config.headers() : {}),
      ...(options.headers ?? {}),
    };

    // Timeout via AbortController
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), timeout);

    try {
      const fetchInit: RequestInit = {
        method,
        headers: mergedHeaders,
        signal: controller.signal,
      };

      if (method === 'POST' && options.body !== undefined) {
        fetchInit.body = JSON.stringify(options.body);
        mergedHeaders['Content-Type'] = 'application/json';
      }

      const res = await globalThis.fetch(url.toString(), fetchInit);

      if (!res.ok) {
        const body = await res.text().catch(() => '');
        if (res.status === 429) {
          onFailure();
          throw new Error(`${config.name}: Rate limited (429) — back off`);
        }
        onFailure();
        throw new Error(`${config.name}: HTTP ${res.status} — ${body.slice(0, 200)}`);
      }

      const data = (await res.json()) as T;

      // Success — reset circuit breaker
      onSuccess();

      // Store in cache
      if (method === 'GET' && ttl > 0) {
        setCache(cache, cacheKey, data, ttl);
      }

      return data;
    } finally {
      clearTimeout(timer);
    }
  }

  return { fetch: apiFetch, cache };
}

// ---------------------------------------------------------------------------
// Cache TTL presets (seconds)
// ---------------------------------------------------------------------------

export const CacheTTL = {
  REALTIME: 60,        // 1 min — live conflict / seismic data
  SHORT: 300,          // 5 min — frequently updating feeds
  MEDIUM: 1800,        // 30 min — event summaries
  LONG: 3600,          // 1 hour — country-level aggregates
  DAILY: 86400,        // 24 hours — reference data, metadata
  STATIC: 604800,      // 7 days — enumerations, schemas
} as const;
