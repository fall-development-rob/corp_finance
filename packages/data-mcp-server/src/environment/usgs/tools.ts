import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { usgsFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// --- GeoJSON types for USGS response ---

interface UsgsProperties {
  mag?: number;
  place?: string;
  time?: number;
  updated?: number;
  tz?: number;
  url?: string;
  detail?: string;
  felt?: number;
  cdi?: number;
  mmi?: number;
  alert?: string;
  status?: string;
  tsunami?: number;
  sig?: number;
  net?: string;
  code?: string;
  ids?: string;
  sources?: string;
  types?: string;
  nst?: number;
  dmin?: number;
  rms?: number;
  gap?: number;
  magType?: string;
  type?: string;
  title?: string;
  [key: string]: unknown;
}

interface UsgsFeature {
  id: string;
  properties: UsgsProperties;
  geometry: {
    type: string;
    coordinates: [number, number, number]; // [lon, lat, depth_km]
  };
}

interface UsgsGeoJsonResponse {
  type: string;
  metadata: {
    generated: number;
    url: string;
    title: string;
    status: number;
    api: string;
    count: number;
  };
  features: UsgsFeature[];
}

// --- helpers ---

function toEarthquakeRecord(feature: UsgsFeature) {
  const p = feature.properties;
  const [longitude, latitude, depth_km] = feature.geometry.coordinates;
  return {
    id: feature.id,
    magnitude: p.mag ?? null,
    place: p.place ?? null,
    time: p.time ? new Date(p.time).toISOString() : null,
    depth_km,
    latitude,
    longitude,
    tsunami: (p.tsunami ?? 0) === 1,
    alert_level: p.alert ?? null,
  };
}

function toSignificantRecord(feature: UsgsFeature) {
  const base = toEarthquakeRecord(feature);
  const p = feature.properties;
  return {
    ...base,
    felt_reports: p.felt ?? null,
    max_intensity_cdi: p.cdi ?? null,
    max_instrument_mmi: p.mmi ?? null,
    significance: p.sig ?? null,
    pager_alert: p.alert ?? null,
    title: p.title ?? null,
  };
}

// --- Zod schemas ---

const EarthquakeQuerySchema = z.object({
  minmagnitude: z
    .number()
    .min(0)
    .max(10)
    .default(4.5)
    .describe('Minimum earthquake magnitude (default 4.5)'),
  starttime: z
    .string()
    .optional()
    .describe('Start date in ISO 8601 format (e.g., 2024-01-01)'),
  endtime: z
    .string()
    .optional()
    .describe('End date in ISO 8601 format (e.g., 2024-12-31)'),
  limit: z
    .number()
    .int()
    .min(1)
    .max(500)
    .default(50)
    .describe('Maximum number of results (default 50, max 500)'),
});

// --- tool registration ---

export function registerUsgsTools(server: McpServer) {
  // 1. usgs_earthquakes — Query earthquakes by magnitude, date range, limit
  server.tool(
    'usgs_earthquakes',
    'Query USGS earthquakes by minimum magnitude, date range, and limit. Returns id, magnitude, place, time, depth_km, latitude, longitude, tsunami flag, alert level (PAGER).',
    EarthquakeQuerySchema.shape,
    async (params) => {
      const { minmagnitude, starttime, endtime, limit } = EarthquakeQuerySchema.parse(params);
      const queryParams: Record<string, string | number> = {
        minmagnitude,
        limit,
        orderby: 'time',
      };
      if (starttime) queryParams.starttime = starttime;
      if (endtime) queryParams.endtime = endtime;

      const data = await usgsFetch<UsgsGeoJsonResponse>(queryParams, {
        cacheTtl: CacheTTL.MEDIUM,
      });

      const earthquakes = (data.features ?? []).map(toEarthquakeRecord);

      return wrapResponse({
        count: earthquakes.length,
        total_available: data.metadata?.count ?? earthquakes.length,
        earthquakes,
      });
    },
  );

  // 2. usgs_significant — Significant recent earthquakes (M6.0+)
  server.tool(
    'usgs_significant',
    'List USGS-curated significant recent earthquakes (M6.0+). Returns id, magnitude, place, time, depth_km, latitude, longitude, tsunami flag, PAGER alert level, felt reports, intensity estimates.',
    {},
    async () => {
      const data = await usgsFetch<UsgsGeoJsonResponse>(
        {
          minmagnitude: 6.0,
          orderby: 'time',
          limit: 20,
        },
        { cacheTtl: CacheTTL.LONG },
      );

      const earthquakes = (data.features ?? []).map(toSignificantRecord);

      return wrapResponse({
        count: earthquakes.length,
        earthquakes,
      });
    },
  );
}
