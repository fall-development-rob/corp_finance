import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { polymarketFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const PolymarketEventsSchema = z.object({
  query: z
    .string()
    .optional()
    .describe('Keyword to filter events (optional)'),
  limit: z
    .number()
    .min(1)
    .max(100)
    .default(20)
    .describe('Max events to return (default 20)'),
});

const PolymarketOddsSchema = z.object({
  event_id: z
    .string()
    .optional()
    .describe('Numeric event ID'),
  slug: z
    .string()
    .optional()
    .describe('Event slug (alternative to event_id)'),
});

const PolymarketGeopoliticalSchema = z.object({
  limit: z
    .number()
    .min(1)
    .max(100)
    .default(20)
    .describe('Max events to return (default 20)'),
});

// ---------- Response typings ----------

interface PolymarketMarket {
  id: string;
  question: string;
  outcomePrices: string;
  volume: string;
  liquidity: string;
  endDate: string;
  outcomes: string;
  [key: string]: unknown;
}

interface PolymarketEvent {
  id: string;
  slug: string;
  title: string;
  description: string;
  markets: PolymarketMarket[];
  [key: string]: unknown;
}

// ---------- Helpers ----------

function parseOutcomePrices(raw: string): number[] {
  try {
    const parsed: unknown = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed.map(Number);
  } catch { /* ignore parse errors */ }
  return [];
}

function parseOutcomes(raw: string): string[] {
  try {
    const parsed: unknown = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed.map(String);
  } catch { /* ignore parse errors */ }
  return [];
}

function formatMarket(m: PolymarketMarket) {
  const prices = parseOutcomePrices(m.outcomePrices);
  const outcomes = parseOutcomes(m.outcomes);
  return {
    id: m.id,
    question: m.question,
    outcomes: outcomes.map((name, i) => ({
      name,
      price: prices[i] ?? null,
    })),
    volume: m.volume,
    liquidity: m.liquidity,
    end_date: m.endDate,
  };
}

function formatEvent(e: PolymarketEvent) {
  return {
    id: e.id,
    slug: e.slug,
    title: e.title,
    description: e.description,
    markets: (e.markets ?? []).map(formatMarket),
  };
}

// ---------- Tool registration ----------

export function registerPolymarketTools(server: McpServer) {
  server.tool(
    'polymarket_events',
    'Search active prediction markets on Polymarket. Returns events with market odds, volume, and liquidity.',
    PolymarketEventsSchema.shape,
    async (params) => {
      const { query, limit } = PolymarketEventsSchema.parse(params);

      const queryParams: Record<string, string | number> = {
        closed: 'false',
        limit,
      };

      const events = await polymarketFetch<PolymarketEvent[]>(
        'events',
        queryParams,
        { cacheTtl: CacheTTL.MEDIUM },
      );

      let filtered = events ?? [];
      if (query) {
        const q = query.toLowerCase();
        filtered = filtered.filter(
          (e) =>
            e.title?.toLowerCase().includes(q) ||
            e.description?.toLowerCase().includes(q),
        );
      }

      return wrapResponse({
        count: filtered.length,
        query: query ?? null,
        events: filtered.map(formatEvent),
      });
    },
  );

  server.tool(
    'polymarket_odds',
    'Get current odds for a specific Polymarket event. Provide event_id or slug. Returns outcomes with prices, volume, and liquidity.',
    PolymarketOddsSchema.shape,
    async (params) => {
      const { event_id, slug } = PolymarketOddsSchema.parse(params);

      if (!event_id && !slug) {
        throw new Error('Polymarket: Provide either event_id or slug');
      }

      let event: PolymarketEvent | undefined;

      if (slug) {
        const events = await polymarketFetch<PolymarketEvent[]>(
          'events',
          { slug },
          { cacheTtl: CacheTTL.SHORT },
        );
        event = (events ?? [])[0];
      } else if (event_id) {
        const events = await polymarketFetch<PolymarketEvent[]>(
          'events',
          { id: event_id },
          { cacheTtl: CacheTTL.SHORT },
        );
        event = (events ?? [])[0];
      }

      if (!event) {
        throw new Error(
          `Polymarket: Event not found (event_id=${event_id}, slug=${slug})`,
        );
      }

      const markets = (event.markets ?? []).map((m) => {
        const prices = parseOutcomePrices(m.outcomePrices);
        const outcomes = parseOutcomes(m.outcomes);
        return {
          question: m.question,
          outcomes: outcomes.map((name, i) => ({
            name,
            price: prices[i] ?? null,
          })),
          volume_24h: m.volume,
          liquidity: m.liquidity,
          end_date: m.endDate,
        };
      });

      return wrapResponse({
        event_id: event.id,
        slug: event.slug,
        title: event.title,
        markets,
      });
    },
  );

  server.tool(
    'polymarket_geopolitical',
    'Pre-filtered geopolitical prediction markets from Polymarket. Searches politics, geopolitics, elections, conflict, and sanctions tags. Returns curated list with odds and volume.',
    PolymarketGeopoliticalSchema.shape,
    async (params) => {
      const { limit } = PolymarketGeopoliticalSchema.parse(params);

      const GEOPOLITICAL_TAGS = ['politics', 'geopolitics', 'elections', 'conflict', 'sanctions'];

      // Fetch events for each geopolitical tag in parallel
      const tagResults = await Promise.all(
        GEOPOLITICAL_TAGS.map((tag) =>
          polymarketFetch<PolymarketEvent[]>(
            'events',
            { closed: 'false', tag, limit: 50 },
            { cacheTtl: CacheTTL.MEDIUM },
          ).catch(() => [] as PolymarketEvent[]),
        ),
      );

      // Deduplicate by event id
      const seen = new Set<string>();
      const allEvents: PolymarketEvent[] = [];
      for (const batch of tagResults) {
        for (const event of batch ?? []) {
          if (!seen.has(event.id)) {
            seen.add(event.id);
            allEvents.push(event);
          }
        }
      }

      // Build curated list
      const curated = allEvents.slice(0, limit).map((e) => {
        const primaryMarket = (e.markets ?? [])[0];
        const prices = primaryMarket
          ? parseOutcomePrices(primaryMarket.outcomePrices)
          : [];
        return {
          title: e.title,
          slug: e.slug,
          yes_price: prices[0] ?? null,
          volume: primaryMarket?.volume ?? '0',
          end_date: primaryMarket?.endDate ?? null,
        };
      });

      return wrapResponse({
        count: curated.length,
        tags_searched: GEOPOLITICAL_TAGS,
        events: curated,
      });
    },
  );
}
