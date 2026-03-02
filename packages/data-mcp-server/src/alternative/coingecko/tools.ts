import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { coinGeckoFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';
import type { CacheEntry } from '../../shared/types.js';

// ---------- Fear & Greed cache (separate API) ----------

const FNG_BASE = 'https://api.alternative.me/fng/';
const FNG_CACHE_TTL = 300; // 5 min

let fngCacheEntry: CacheEntry | null = null;

function getFngCached(): unknown | undefined {
  if (!fngCacheEntry) return undefined;
  if (Date.now() > fngCacheEntry.expiresAt) {
    fngCacheEntry = null;
    return undefined;
  }
  return fngCacheEntry.data;
}

function setFngCache(data: unknown): void {
  fngCacheEntry = { data, expiresAt: Date.now() + FNG_CACHE_TTL * 1000 };
}

// ---------- Schemas ----------

const FearGreedSchema = z.object({
  limit: z
    .number()
    .min(1)
    .max(365)
    .default(30)
    .describe('Number of days of history to return (default 30)'),
});

const StablecoinHealthSchema = z.object({
  coins: z
    .array(z.string())
    .default(['tether', 'usd-coin', 'dai'])
    .describe('CoinGecko coin IDs to monitor (default: tether, usd-coin, dai)'),
});

// ---------- Response typings ----------

interface FngDataPoint {
  value: string;
  value_classification: string;
  timestamp: string;
  time_until_update?: string;
}

interface FngResponse {
  name: string;
  data: FngDataPoint[];
  metadata: { error: string | null };
}

interface CoinGeckoPriceEntry {
  usd: number;
  usd_24h_change?: number;
}

type CoinGeckoPriceResponse = Record<string, CoinGeckoPriceEntry>;

// ---------- Helpers ----------

type SentimentLabel =
  | 'Extreme Fear'
  | 'Fear'
  | 'Neutral'
  | 'Greed'
  | 'Extreme Greed';

function classifySentiment(value: number): SentimentLabel {
  if (value <= 20) return 'Extreme Fear';
  if (value <= 40) return 'Fear';
  if (value <= 60) return 'Neutral';
  if (value <= 80) return 'Greed';
  return 'Extreme Greed';
}

// ---------- Tool registration ----------

export function registerCoinGeckoTools(server: McpServer) {
  server.tool(
    'coingecko_fear_greed',
    'Crypto Fear & Greed index. Returns current value (0-100), classification, and history for the specified number of days.',
    FearGreedSchema.shape,
    async (params) => {
      const { limit } = FearGreedSchema.parse(params);

      // Check cache first
      let data = getFngCached() as FngResponse | undefined;

      if (!data) {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 15_000);

        try {
          const url = `${FNG_BASE}?limit=${limit}&format=json`;
          const res = await fetch(url, {
            method: 'GET',
            headers: { 'Accept': 'application/json' },
            signal: controller.signal,
          });

          if (!res.ok) {
            const body = await res.text().catch(() => '');
            throw new Error(`Fear & Greed API: HTTP ${res.status} — ${body.slice(0, 200)}`);
          }

          data = await res.json() as FngResponse;
          setFngCache(data);
        } finally {
          clearTimeout(timeout);
        }
      }

      const points = (data?.data ?? []).slice(0, limit);

      const current = points[0];
      const currentValue = current ? Number(current.value) : null;

      const history = points.map((p) => ({
        date: new Date(Number(p.timestamp) * 1000).toISOString().slice(0, 10),
        value: Number(p.value),
        classification: p.value_classification,
      }));

      return wrapResponse({
        current_value: currentValue,
        classification: currentValue !== null
          ? classifySentiment(currentValue)
          : null,
        history,
      });
    },
  );

  server.tool(
    'coingecko_stablecoin_health',
    'Stablecoin peg monitoring. Checks current price vs $1.00 peg and flags depegging (>0.5% deviation). Returns price, deviation, and 24h change.',
    StablecoinHealthSchema.shape,
    async (params) => {
      const { coins } = StablecoinHealthSchema.parse(params);

      const ids = coins.join(',');
      const data = await coinGeckoFetch<CoinGeckoPriceResponse>(
        'simple/price',
        {
          ids,
          vs_currencies: 'usd',
          include_24hr_change: true,
        },
        { cacheTtl: CacheTTL.SHORT },
      );

      const PEG_TARGET = 1.0;
      const DEPEG_THRESHOLD = 0.005; // 0.5%

      const results = coins.map((coin) => {
        const entry = data[coin];
        if (!entry) {
          return {
            coin,
            current_price: null,
            peg_target: PEG_TARGET,
            deviation_pct: null,
            is_depegged: null,
            change_24h: null,
            error: `No data returned for ${coin}`,
          };
        }

        const price = entry.usd;
        const deviation = Math.abs(price - PEG_TARGET) / PEG_TARGET;
        const deviationPct = Math.round(deviation * 10000) / 100; // 2 decimal places

        return {
          coin,
          current_price: price,
          peg_target: PEG_TARGET,
          deviation_pct: deviationPct,
          is_depegged: deviation > DEPEG_THRESHOLD,
          change_24h: entry.usd_24h_change ?? null,
        };
      });

      return wrapResponse({
        timestamp: new Date().toISOString(),
        peg_target: PEG_TARGET,
        depeg_threshold_pct: 0.5,
        coins: results,
      });
    },
  );
}
