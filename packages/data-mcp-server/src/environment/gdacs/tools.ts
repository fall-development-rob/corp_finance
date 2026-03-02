import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { gdacsFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// --- internal types for GDACS response ---

interface GdacsEvent {
  alertid?: string | number;
  eventtype?: string;
  alertlevel?: string;
  country?: string;
  iso3?: string;
  fromdate?: string;
  todate?: string;
  eventdate?: string;
  name?: string;
  description?: string;
  htmldescription?: string;
  severity?: Record<string, unknown>;
  population?: Record<string, unknown>;
  vulnerability?: Record<string, unknown>;
  geo_lat?: number;
  geo_lng?: number;
  alertscore?: number;
  episodealertlevel?: string;
  episodealertscore?: number;
  [key: string]: unknown;
}

interface GdacsResponse {
  features?: GdacsEvent[];
  [key: string]: unknown;
}

// --- helpers ---

function normalizeSeverity(event: GdacsEvent): string {
  const level = (
    event.alertlevel ??
    event.episodealertlevel ??
    ''
  ).toString().toLowerCase();
  if (level.includes('red')) return 'red';
  if (level.includes('orange')) return 'orange';
  if (level.includes('green')) return 'green';
  return level || 'unknown';
}

function normalizeEventType(raw: string | undefined): string {
  if (!raw) return 'unknown';
  const upper = raw.toUpperCase();
  const mapping: Record<string, string> = {
    EQ: 'EQ', FL: 'FL', TC: 'TC', VO: 'VO', DR: 'DR', WF: 'WF',
    EARTHQUAKE: 'EQ', FLOOD: 'FL', CYCLONE: 'TC',
    VOLCANO: 'VO', DROUGHT: 'DR', WILDFIRE: 'WF',
  };
  return mapping[upper] ?? upper;
}

function extractPopulation(event: GdacsEvent): number {
  const pop = event.population;
  if (!pop) return 0;
  if (typeof pop === 'number') return pop;
  if (typeof pop === 'object') {
    const val =
      (pop as Record<string, unknown>)['value'] ??
      (pop as Record<string, unknown>)['affected'] ??
      (pop as Record<string, unknown>)['total'] ??
      0;
    return typeof val === 'number' ? val : Number(val) || 0;
  }
  return 0;
}

function extractEvents(data: GdacsResponse): GdacsEvent[] {
  if (Array.isArray(data.features)) return data.features;
  // Some endpoints nest under .events or .items
  for (const key of ['events', 'items', 'results']) {
    const candidate = (data as Record<string, unknown>)[key];
    if (Array.isArray(candidate)) return candidate as GdacsEvent[];
  }
  // If the response itself is an array
  if (Array.isArray(data)) return data as unknown as GdacsEvent[];
  return [];
}

function toAlertRecord(event: GdacsEvent) {
  return {
    alertid: event.alertid ?? null,
    eventtype: normalizeEventType(event.eventtype),
    severity: normalizeSeverity(event),
    country: event.country ?? event.iso3 ?? null,
    lat: event.geo_lat ?? null,
    lon: event.geo_lng ?? null,
    eventdate: event.eventdate ?? event.fromdate ?? null,
    population_affected: extractPopulation(event),
    alertscore: event.alertscore ?? event.episodealertscore ?? null,
  };
}

// --- Zod schemas ---

const GdacsEventsSchema = z.object({
  hazard_type: z
    .enum(['earthquake', 'flood', 'cyclone', 'volcano', 'drought', 'wildfire'])
    .describe('Type of natural hazard to filter by'),
  country: z
    .string()
    .optional()
    .describe('ISO3 country code to filter events (optional)'),
});

const GdacsCountryExposureSchema = z.object({
  country: z
    .string()
    .describe('ISO3 country code to aggregate exposure for'),
});

// --- tool registration ---

export function registerGdacsTools(server: McpServer) {
  // 1. gdacs_alerts — Current global disaster alerts (Orange/Red only)
  server.tool(
    'gdacs_alerts',
    'Current global disaster alerts from GDACS. Filters to Orange and Red severity only (excludes Green). Returns alertid, eventtype, severity, country, lat, lon, eventdate, population_affected, alertscore.',
    {},
    async () => {
      const data = await gdacsFetch<GdacsResponse>({ cacheTtl: CacheTTL.SHORT });
      const events = extractEvents(data);
      const alerts = events
        .map(toAlertRecord)
        .filter(a => a.severity === 'orange' || a.severity === 'red');
      return wrapResponse({ count: alerts.length, alerts });
    },
  );

  // 2. gdacs_events — Historical events by type and optional country
  server.tool(
    'gdacs_events',
    'Historical GDACS events filtered by hazard type and optional country. Returns matching events with alertid, eventtype, severity, country, coordinates, date, population affected.',
    GdacsEventsSchema.shape,
    async (params) => {
      const { hazard_type, country } = GdacsEventsSchema.parse(params);
      const data = await gdacsFetch<GdacsResponse>({ cacheTtl: CacheTTL.MEDIUM });
      const events = extractEvents(data);

      const typeCode = normalizeEventType(hazard_type);
      const filtered = events
        .map(toAlertRecord)
        .filter(a => {
          if (a.eventtype !== typeCode) return false;
          if (country) {
            const c = country.toUpperCase();
            const eventCountry = (a.country ?? '').toUpperCase();
            if (!eventCountry.includes(c)) return false;
          }
          return true;
        });

      return wrapResponse({ count: filtered.length, events: filtered });
    },
  );

  // 3. gdacs_country_exposure — Aggregate disaster exposure for a country
  server.tool(
    'gdacs_country_exposure',
    'Aggregate disaster exposure for a country. Returns events_by_type, max_severity, total_population_affected, most_recent_event.',
    GdacsCountryExposureSchema.shape,
    async (params) => {
      const { country } = GdacsCountryExposureSchema.parse(params);
      const data = await gdacsFetch<GdacsResponse>({ cacheTtl: CacheTTL.MEDIUM });
      const events = extractEvents(data);

      const countryUpper = country.toUpperCase();
      const countryEvents = events
        .map(toAlertRecord)
        .filter(a => {
          const ec = (a.country ?? '').toUpperCase();
          return ec.includes(countryUpper);
        });

      // Count events by type
      const eventsByType: Record<string, number> = {};
      let totalPopulation = 0;
      const severityRank: Record<string, number> = { green: 1, orange: 2, red: 3, unknown: 0 };
      let maxSeverity = 'unknown';
      let maxSeverityRank = 0;
      let mostRecent: ReturnType<typeof toAlertRecord> | null = null;
      let mostRecentDate = '';

      for (const ev of countryEvents) {
        const type = ev.eventtype;
        eventsByType[type] = (eventsByType[type] ?? 0) + 1;
        totalPopulation += ev.population_affected;

        const rank = severityRank[ev.severity] ?? 0;
        if (rank > maxSeverityRank) {
          maxSeverityRank = rank;
          maxSeverity = ev.severity;
        }

        const d = ev.eventdate ?? '';
        if (d > mostRecentDate) {
          mostRecentDate = d;
          mostRecent = ev;
        }
      }

      return wrapResponse({
        country,
        total_events: countryEvents.length,
        events_by_type: eventsByType,
        max_severity: maxSeverity,
        total_population_affected: totalPopulation,
        most_recent_event: mostRecent,
      });
    },
  );
}
