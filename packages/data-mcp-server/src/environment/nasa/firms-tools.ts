import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { firmsFetch, CacheTTL } from './firms-client.js';
import { wrapResponse } from '../../shared/types.js';

// --- Zod schemas ---

const FirmsFiresSchema = z.object({
  country: z
    .string()
    .length(3)
    .describe('ISO3 country code (e.g., USA, BRA, AUS)'),
  days: z
    .number()
    .int()
    .min(1)
    .max(10)
    .default(1)
    .describe('Number of days of data (1-10, default 1)'),
});

const FirmsCountryFiresSchema = z.object({
  country: z
    .string()
    .length(3)
    .describe('ISO3 country code (e.g., USA, BRA, AUS)'),
  days: z
    .number()
    .int()
    .min(1)
    .max(10)
    .default(1)
    .describe('Number of days of data (1-10, default 1)'),
});

// --- helpers ---

function normalizeConfidence(raw: string): 'low' | 'nominal' | 'high' {
  const lower = raw.toLowerCase().trim();
  if (lower === 'h' || lower === 'high') return 'high';
  if (lower === 'l' || lower === 'low') return 'low';
  return 'nominal';
}

// --- tool registration ---

export function registerFirmsTools(server: McpServer) {
  // 1. firms_fires — Active fire detections by country
  server.tool(
    'firms_fires',
    'Active fire detections from NASA FIRMS by country (ISO3 code) and days (1-10). Returns latitude, longitude, brightness, frp (fire radiative power), confidence (low/nominal/high), acq_date, daynight.',
    FirmsFiresSchema.shape,
    async (params) => {
      const { country, days } = FirmsFiresSchema.parse(params);
      const detections = await firmsFetch(country, days, { cacheTtl: CacheTTL.MEDIUM });

      const fires = detections.map(d => ({
        latitude: d.latitude,
        longitude: d.longitude,
        brightness: d.brightness,
        frp: d.frp,
        confidence: normalizeConfidence(d.confidence),
        acq_date: d.acq_date,
        daynight: d.daynight,
      }));

      return wrapResponse({
        country,
        days,
        count: fires.length,
        fires,
      });
    },
  );

  // 2. firms_country_fires — Aggregate fire statistics
  server.tool(
    'firms_country_fires',
    'Aggregate fire statistics from NASA FIRMS for a country. Returns total_detections, high_confidence_count, avg_frp, max_frp, detections_by_day.',
    FirmsCountryFiresSchema.shape,
    async (params) => {
      const { country, days } = FirmsCountryFiresSchema.parse(params);
      const detections = await firmsFetch(country, days, { cacheTtl: CacheTTL.MEDIUM });

      let totalFrp = 0;
      let maxFrp = 0;
      let highConfidenceCount = 0;
      const detectionsByDay: Record<string, number> = {};

      for (const d of detections) {
        totalFrp += d.frp;
        if (d.frp > maxFrp) maxFrp = d.frp;

        const conf = normalizeConfidence(d.confidence);
        if (conf === 'high') highConfidenceCount++;

        const day = d.acq_date || 'unknown';
        detectionsByDay[day] = (detectionsByDay[day] ?? 0) + 1;
      }

      const totalDetections = detections.length;
      const avgFrp = totalDetections > 0 ? Math.round((totalFrp / totalDetections) * 100) / 100 : 0;

      return wrapResponse({
        country,
        days,
        total_detections: totalDetections,
        high_confidence_count: highConfidenceCount,
        avg_frp: avgFrp,
        max_frp: Math.round(maxFrp * 100) / 100,
        detections_by_day: detectionsByDay,
      });
    },
  );
}
