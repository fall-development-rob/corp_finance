// Yahoo Finance API client — UNOFFICIAL, no API key required
// Uses cookie + crumb authentication mechanism
// Aggressive caching recommended due to undocumented rate limits

const YF_BASE_CHART = 'https://query1.finance.yahoo.com/v8/finance/chart';
const YF_BASE_QUOTE_SUMMARY = 'https://query2.finance.yahoo.com/v10/finance/quoteSummary';
const YF_BASE_OPTIONS = 'https://query1.finance.yahoo.com/v7/finance/options';
const YF_QUOTE_V6 = 'https://query2.finance.yahoo.com/v6/finance/quote';

// ── Crumb / Cookie auth ──────────────────────────────────────────────

let crumb = '';
let cookie = '';
let crumbExpiresAt = 0;
const CRUMB_TTL_MS = 30 * 60 * 1000; // refresh crumb every 30 minutes

const USER_AGENTS = [
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15',
  'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0',
];

function randomUserAgent(): string {
  return USER_AGENTS[Math.floor(Math.random() * USER_AGENTS.length)];
}

async function refreshCrumb(): Promise<void> {
  // Step 1: fetch a cookie from Yahoo
  const cookieRes = await fetch('https://fc.yahoo.com', { redirect: 'manual' });
  const setCookie = cookieRes.headers.get('set-cookie');
  if (setCookie) {
    cookie = setCookie.split(';')[0];
  }

  // Step 2: fetch crumb using the cookie
  const crumbRes = await fetch('https://query2.finance.yahoo.com/v1/test/getcrumb', {
    headers: {
      'Cookie': cookie,
      'User-Agent': randomUserAgent(),
    },
  });

  if (!crumbRes.ok) {
    throw new Error(`Yahoo Finance: failed to fetch crumb — HTTP ${crumbRes.status}`);
  }

  crumb = await crumbRes.text();
  crumbExpiresAt = Date.now() + CRUMB_TTL_MS;
}

async function ensureCrumb(): Promise<void> {
  if (!crumb || Date.now() > crumbExpiresAt) {
    await refreshCrumb();
  }
}

// ── Cache ────────────────────────────────────────────────────────────

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
  // Evict expired entries when cache grows large
  if (cache.size > 1000) {
    const now = Date.now();
    for (const [k, v] of cache) {
      if (now > v.expiresAt) cache.delete(k);
    }
  }
}

// ── Rate limiting (self-imposed) ─────────────────────────────────────

const RATE_LIMIT = 5; // requests per second (conservative)
let requestTimestamps: number[] = [];

function isRateLimited(): boolean {
  const now = Date.now();
  requestTimestamps = requestTimestamps.filter(t => now - t < 1_000);
  return requestTimestamps.length >= RATE_LIMIT;
}

async function waitForSlot(): Promise<void> {
  while (isRateLimited()) {
    await new Promise(resolve => setTimeout(resolve, 200));
  }
}

// ── Core fetch ───────────────────────────────────────────────────────

export interface YfRequestOptions {
  cacheTtl?: number;   // seconds, 0 to skip cache
  needsCrumb?: boolean; // default true — some endpoints may not need crumb
}

export async function yfFetch<T = unknown>(
  url: string,
  options: YfRequestOptions = {},
): Promise<T> {
  const ttl = options.cacheTtl ?? CacheTTL.SHORT;
  const needsCrumb = options.needsCrumb ?? true;

  // Check cache first
  const cacheKey = url;
  if (ttl > 0) {
    const cached = getCached(cacheKey);
    if (cached !== undefined) return cached as T;
  }

  // Rate limit
  await waitForSlot();

  // Ensure crumb is fresh
  if (needsCrumb) {
    await ensureCrumb();
  }

  // Append crumb to URL if needed
  const separator = url.includes('?') ? '&' : '?';
  const fetchUrl = needsCrumb ? `${url}${separator}crumb=${encodeURIComponent(crumb)}` : url;

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const res = await fetch(fetchUrl, {
      headers: {
        'Cookie': cookie,
        'User-Agent': randomUserAgent(),
        'Accept': 'application/json',
      },
      signal: controller.signal,
    });

    // On 401/403, refresh crumb and retry once
    if ((res.status === 401 || res.status === 403) && needsCrumb) {
      clearTimeout(timeout);
      await refreshCrumb();
      return yfFetchRetry<T>(url, ttl);
    }

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      if (res.status === 429) {
        throw new Error('Yahoo Finance: rate limited by server — try again shortly');
      }
      throw new Error(`Yahoo Finance: HTTP ${res.status} — ${body.slice(0, 300)}`);
    }

    const data = await res.json() as T;

    if (ttl > 0) setCache(cacheKey, data, ttl);
    requestTimestamps.push(Date.now());

    return data;
  } finally {
    clearTimeout(timeout);
  }
}

async function yfFetchRetry<T>(url: string, ttl: number): Promise<T> {
  await waitForSlot();

  const separator = url.includes('?') ? '&' : '?';
  const fetchUrl = `${url}${separator}crumb=${encodeURIComponent(crumb)}`;

  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);

  try {
    const res = await fetch(fetchUrl, {
      headers: {
        'Cookie': cookie,
        'User-Agent': randomUserAgent(),
        'Accept': 'application/json',
      },
      signal: controller.signal,
    });

    if (!res.ok) {
      const body = await res.text().catch(() => '');
      throw new Error(`Yahoo Finance: HTTP ${res.status} after crumb refresh — ${body.slice(0, 300)}`);
    }

    const data = await res.json() as T;
    if (ttl > 0) setCache(url, data, ttl);
    requestTimestamps.push(Date.now());
    return data;
  } finally {
    clearTimeout(timeout);
  }
}

// ── Helper: build quoteSummary URL ───────────────────────────────────

export function quoteSummaryUrl(symbol: string, modules: string[]): string {
  const mods = modules.join(',');
  return `${YF_BASE_QUOTE_SUMMARY}/${encodeURIComponent(symbol)}?modules=${mods}`;
}

export function chartUrl(symbol: string, params: Record<string, string>): string {
  const qs = new URLSearchParams(params).toString();
  return `${YF_BASE_CHART}/${encodeURIComponent(symbol)}?${qs}`;
}

export function optionsUrl(symbol: string, date?: number): string {
  const base = `${YF_BASE_OPTIONS}/${encodeURIComponent(symbol)}`;
  return date ? `${base}?date=${date}` : base;
}

export function quoteUrl(symbols: string[], fields?: string): string {
  let url = `${YF_QUOTE_V6}?symbols=${symbols.map(s => encodeURIComponent(s)).join(',')}`;
  if (fields) url += `&fields=${fields}`;
  return url;
}

// ── Extractors for nested Yahoo responses ────────────────────────────

export function extractQuoteSummary(data: Record<string, unknown>): unknown {
  const qs = data.quoteSummary as Record<string, unknown> | undefined;
  if (!qs) return data;
  const result = qs.result as unknown[] | undefined;
  if (!result || result.length === 0) {
    const error = qs.error as Record<string, string> | undefined;
    throw new Error(`Yahoo Finance: ${error?.description ?? 'no data returned for symbol'}`);
  }
  return result[0];
}

export function extractChart(data: Record<string, unknown>): unknown {
  const chart = data.chart as Record<string, unknown> | undefined;
  if (!chart) return data;
  const result = chart.result as unknown[] | undefined;
  if (!result || result.length === 0) {
    const error = chart.error as Record<string, string> | undefined;
    throw new Error(`Yahoo Finance: ${error?.description ?? 'no chart data returned'}`);
  }
  return result[0];
}

export function extractQuoteResponse(data: Record<string, unknown>): unknown {
  const qr = data.quoteResponse as Record<string, unknown> | undefined;
  if (!qr) return data;
  const result = qr.result as unknown[] | undefined;
  if (!result) {
    const error = qr.error as Record<string, string> | undefined;
    throw new Error(`Yahoo Finance: ${error?.description ?? 'no quote data returned'}`);
  }
  return result;
}

export function extractOptions(data: Record<string, unknown>): unknown {
  const optionChain = data.optionChain as Record<string, unknown> | undefined;
  if (!optionChain) return data;
  const result = optionChain.result as unknown[] | undefined;
  if (!result || result.length === 0) {
    const error = optionChain.error as Record<string, string> | undefined;
    throw new Error(`Yahoo Finance: ${error?.description ?? 'no options data returned'}`);
  }
  return result[0];
}

// ── Cache TTL presets ────────────────────────────────────────────────

/** Cache TTL presets by data type */
export const CacheTTL = {
  REALTIME: 60,       // 1 min — quotes, fast info
  SHORT: 300,         // 5 min — options, intraday
  MEDIUM: 3600,       // 1 hour — financial statements, earnings
  LONG: 86400,        // 24 hours — profiles, analyst targets
} as const;
