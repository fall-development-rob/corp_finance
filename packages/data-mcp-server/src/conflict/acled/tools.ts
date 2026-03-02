import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { acledFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const AcledEventsSchema = z.object({
  country: z.string().describe('ISO 3166-1 alpha-2 or alpha-3 country code (e.g., UA, SYR)'),
  date_start: z.string().describe('Start date in YYYY-MM-DD format'),
  date_end: z.string().describe('End date in YYYY-MM-DD format'),
  event_type: z
    .enum([
      'battles',
      'protests',
      'riots',
      'explosions',
      'violence_against_civilians',
      'strategic_developments',
    ])
    .optional()
    .describe('Filter by event type'),
  limit: z.number().min(1).max(5000).default(250).describe('Max rows to return'),
});

const AcledFatalitiesSchema = z.object({
  country: z.string().describe('ISO 3166-1 country code'),
  date_start: z.string().describe('Start date YYYY-MM-DD'),
  date_end: z.string().describe('End date YYYY-MM-DD'),
});

const AcledCountrySummarySchema = z.object({
  country: z.string().describe('ISO 3166-1 country code'),
  days: z.number().min(1).max(365).default(90).describe('Lookback period in days (default 90)'),
});

// ---------- ACLED response typings ----------

interface AcledEvent {
  event_date: string;
  event_type: string;
  fatalities: string;
  actor1: string;
  actor2: string;
  latitude: string;
  longitude: string;
  country: string;
  notes: string;
  [key: string]: unknown;
}

interface AcledApiResponse {
  status: number;
  success: boolean;
  data: AcledEvent[];
  count: number;
}

// ---------- Helpers ----------

function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function parseFatalities(val: string): number {
  const n = Number(val);
  return Number.isFinite(n) ? n : 0;
}

// ---------- Tool registration ----------

export function registerAcledTools(server: McpServer) {
  server.tool(
    'acled_events',
    'Query ACLED conflict events by country, date range, and optional event type. Returns events with location, actors, fatalities, and descriptions.',
    AcledEventsSchema.shape,
    async (params) => {
      const { country, date_start, date_end, event_type, limit } = AcledEventsSchema.parse(params);

      const queryParams: Record<string, string | number> = {
        iso: country,
        event_date: `${date_start}|${date_end}`,
        event_date_where: 'between',
        limit,
      };
      if (event_type) {
        // ACLED expects title-cased event types with spaces
        const typeMap: Record<string, string> = {
          battles: 'Battles',
          protests: 'Protests',
          riots: 'Riots',
          explosions: 'Explosions/Remote violence',
          violence_against_civilians: 'Violence against civilians',
          strategic_developments: 'Strategic developments',
        };
        queryParams.event_type = typeMap[event_type] ?? event_type;
      }

      const raw = await acledFetch<AcledApiResponse>(queryParams, { cacheTtl: CacheTTL.MEDIUM });

      const events = (raw.data ?? []).map((e) => ({
        event_date: e.event_date,
        event_type: e.event_type,
        fatalities: parseFatalities(e.fatalities),
        actor1: e.actor1,
        actor2: e.actor2,
        latitude: Number(e.latitude),
        longitude: Number(e.longitude),
        country: e.country,
        notes: e.notes,
      }));

      return wrapResponse({
        count: events.length,
        country,
        date_range: { start: date_start, end: date_end },
        events,
      });
    },
  );

  server.tool(
    'acled_fatalities',
    'Aggregate fatality counts from ACLED by country and date range. Returns total fatalities, event count, and breakdown by event type.',
    AcledFatalitiesSchema.shape,
    async (params) => {
      const { country, date_start, date_end } = AcledFatalitiesSchema.parse(params);

      const raw = await acledFetch<AcledApiResponse>(
        {
          iso: country,
          event_date: `${date_start}|${date_end}`,
          event_date_where: 'between',
          limit: 0, // fetch all matching
        },
        { cacheTtl: CacheTTL.LONG },
      );

      const events = raw.data ?? [];
      let totalFatalities = 0;
      const fatalitiesByType: Record<string, number> = {};

      for (const e of events) {
        const f = parseFatalities(e.fatalities);
        totalFatalities += f;
        const t = e.event_type || 'unknown';
        fatalitiesByType[t] = (fatalitiesByType[t] ?? 0) + f;
      }

      return wrapResponse({
        country,
        date_range: { start: date_start, end: date_end },
        total_fatalities: totalFatalities,
        events_count: events.length,
        fatalities_by_type: fatalitiesByType,
      });
    },
  );

  server.tool(
    'acled_country_summary',
    'Composite conflict profile for a country. Returns event counts by type, total fatalities, top actors, and geographic hotspots over a configurable lookback period.',
    AcledCountrySummarySchema.shape,
    async (params) => {
      const { country, days } = AcledCountrySummarySchema.parse(params);

      const end = new Date();
      const start = new Date(end.getTime() - days * 24 * 60 * 60 * 1000);

      const raw = await acledFetch<AcledApiResponse>(
        {
          iso: country,
          event_date: `${formatDate(start)}|${formatDate(end)}`,
          event_date_where: 'between',
          limit: 0,
        },
        { cacheTtl: CacheTTL.LONG },
      );

      const events = raw.data ?? [];

      // Event counts by type
      const eventCountsByType: Record<string, number> = {};
      let totalFatalities = 0;
      const actorCounts: Record<string, number> = {};
      const locationBuckets: Record<string, { lat: number; lon: number; count: number }> = {};

      for (const e of events) {
        // Count by type
        const t = e.event_type || 'unknown';
        eventCountsByType[t] = (eventCountsByType[t] ?? 0) + 1;

        // Fatalities
        totalFatalities += parseFatalities(e.fatalities);

        // Actor frequency
        if (e.actor1) actorCounts[e.actor1] = (actorCounts[e.actor1] ?? 0) + 1;
        if (e.actor2) actorCounts[e.actor2] = (actorCounts[e.actor2] ?? 0) + 1;

        // Geographic clustering (round to 1 decimal for grouping)
        const lat = Math.round(Number(e.latitude) * 10) / 10;
        const lon = Math.round(Number(e.longitude) * 10) / 10;
        const geoKey = `${lat},${lon}`;
        if (!locationBuckets[geoKey]) {
          locationBuckets[geoKey] = { lat, lon, count: 0 };
        }
        locationBuckets[geoKey].count += 1;
      }

      // Top 10 actors
      const topActors = Object.entries(actorCounts)
        .sort((a, b) => b[1] - a[1])
        .slice(0, 10)
        .map(([name, count]) => ({ name, event_count: count }));

      // Top 10 geographic hotspots
      const geographicHotspots = Object.values(locationBuckets)
        .sort((a, b) => b.count - a.count)
        .slice(0, 10)
        .map(({ lat, lon, count }) => ({ latitude: lat, longitude: lon, event_count: count }));

      return wrapResponse({
        country,
        period_days: days,
        date_range: { start: formatDate(start), end: formatDate(end) },
        total_events: events.length,
        total_fatalities: totalFatalities,
        event_counts_by_type: eventCountsByType,
        top_actors: topActors,
        geographic_hotspots: geographicHotspots,
      });
    },
  );
}
