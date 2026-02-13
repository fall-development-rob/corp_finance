import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// We need to test the client module, but it reads env vars at module level.
// Set env vars before import.
process.env.FMP_API_KEY = 'test-key-123';
process.env.FMP_BASE_URL = 'https://fmp-test.local/stable';
process.env.FMP_RATE_LIMIT = '10';

// Dynamic import after env setup
const { fmpFetch, CacheTTL } = await import('../src/client.js');

describe('FMP Client', () => {
  const mockFetch = vi.fn();

  beforeEach(() => {
    vi.stubGlobal('fetch', mockFetch);
    mockFetch.mockReset();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  describe('fmpFetch', () => {
    it('makes request with correct URL and API key', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ price: 150.00 }),
      });

      const result = await fmpFetch('quote', { symbol: 'AAPL' }, { cacheTtl: 0 });

      expect(mockFetch).toHaveBeenCalledTimes(1);
      const url = new URL(mockFetch.mock.calls[0][0]);
      expect(url.pathname).toContain('quote');
      expect(url.searchParams.get('apikey')).toBe('test-key-123');
      expect(url.searchParams.get('symbol')).toBe('AAPL');
      expect(result).toEqual({ price: 150.00 });
    });

    it('caches responses when cacheTtl > 0', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ cached: true }),
      });

      // First call - hits network
      await fmpFetch('profile', { symbol: 'MSFT' }, { cacheTtl: 60 });
      expect(mockFetch).toHaveBeenCalledTimes(1);

      // Second call - should hit cache
      const cached = await fmpFetch('profile', { symbol: 'MSFT' }, { cacheTtl: 60 });
      expect(mockFetch).toHaveBeenCalledTimes(1); // Still 1 - no new fetch
      expect(cached).toEqual({ cached: true });
    });

    it('skips cache when cacheTtl is 0', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ fresh: true }),
      });

      await fmpFetch('quote', { symbol: 'AAPL' }, { cacheTtl: 0 });
      await fmpFetch('quote', { symbol: 'AAPL' }, { cacheTtl: 0 });
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });

    it('throws on 401 with invalid API key message', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
        text: async () => 'Invalid API key',
      });

      await expect(fmpFetch('quote', { symbol: 'AAPL' }, { cacheTtl: 0 }))
        .rejects.toThrow('FMP: Invalid API key');
    });

    it('throws on 403 with plan limit message', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 403,
        text: async () => 'Upgrade your plan',
      });

      await expect(fmpFetch('quote', { symbol: 'AAPL' }, { cacheTtl: 0 }))
        .rejects.toThrow('FMP: Endpoint not available on your plan');
    });

    it('throws on 429 rate limit', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 429,
        text: async () => 'Too many requests',
      });

      await expect(fmpFetch('quote', { symbol: 'AAPL' }, { cacheTtl: 0 }))
        .rejects.toThrow('FMP: Rate limited by server');
    });

    it('throws generic error for other HTTP errors', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        text: async () => 'Internal Server Error',
      });

      await expect(fmpFetch('quote', { symbol: 'AAPL' }, { cacheTtl: 0 }))
        .rejects.toThrow('FMP: HTTP 500');
    });

    it('passes additional params as query parameters', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ([]),
      });

      await fmpFetch('income-statement',
        { symbol: 'AAPL', period: 'quarter', limit: 4 },
        { cacheTtl: 0 },
      );

      const url = new URL(mockFetch.mock.calls[0][0]);
      expect(url.searchParams.get('symbol')).toBe('AAPL');
      expect(url.searchParams.get('period')).toBe('quarter');
      expect(url.searchParams.get('limit')).toBe('4');
    });

    it('omits undefined params', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ([]),
      });

      await fmpFetch('search-symbol',
        { query: 'AAPL', exchange: undefined },
        { cacheTtl: 0 },
      );

      const url = new URL(mockFetch.mock.calls[0][0]);
      expect(url.searchParams.get('query')).toBe('AAPL');
      expect(url.searchParams.has('exchange')).toBe(false);
    });
  });

  describe('CacheTTL', () => {
    it('defines expected TTL values', () => {
      expect(CacheTTL.REALTIME).toBe(30);
      expect(CacheTTL.SHORT).toBe(300);
      expect(CacheTTL.MEDIUM).toBe(3600);
      expect(CacheTTL.LONG).toBe(86400);
      expect(CacheTTL.STATIC).toBe(604800);
    });
  });
});
