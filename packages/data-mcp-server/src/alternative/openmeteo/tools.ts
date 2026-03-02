import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { openMeteoFetch, CacheTTL } from './client.js';
import type { AnomalySeverity } from '../../shared/types.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const ClimateAnomalySchema = z.object({
  latitude: z
    .number()
    .min(-90)
    .max(90)
    .describe('Latitude of the location (-90 to 90)'),
  longitude: z
    .number()
    .min(-180)
    .max(180)
    .describe('Longitude of the location (-180 to 180)'),
  baseline_start: z
    .string()
    .default('1991-01-01')
    .describe('Baseline period start date YYYY-MM-DD (default 1991-01-01)'),
  baseline_end: z
    .string()
    .default('2020-12-31')
    .describe('Baseline period end date YYYY-MM-DD (default 2020-12-31)'),
  observation_start: z
    .string()
    .optional()
    .describe('Observation period start YYYY-MM-DD (default 30 days ago)'),
  observation_end: z
    .string()
    .optional()
    .describe('Observation period end YYYY-MM-DD (default today)'),
});

// ---------- Response typings ----------

interface OpenMeteoDailyResponse {
  daily: {
    time: string[];
    temperature_2m_mean: (number | null)[];
    precipitation_sum: (number | null)[];
  };
  [key: string]: unknown;
}

// ---------- Helpers ----------

function formatDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function mean(arr: number[]): number {
  if (arr.length === 0) return 0;
  return arr.reduce((sum, v) => sum + v, 0) / arr.length;
}

function stddev(arr: number[], avg: number): number {
  if (arr.length < 2) return 0;
  const variance = arr.reduce((sum, v) => sum + (v - avg) ** 2, 0) / (arr.length - 1);
  return Math.sqrt(variance);
}

function classifySeverity(sigmas: number): AnomalySeverity {
  const abs = Math.abs(sigmas);
  if (abs < 1) return 'normal';
  if (abs < 2) return 'moderate';
  if (abs < 3) return 'severe';
  return 'extreme';
}

function filterNulls(arr: (number | null)[]): number[] {
  return arr.filter((v): v is number => v !== null && Number.isFinite(v));
}

// ---------- Tool registration ----------

export function registerOpenMeteoTools(server: McpServer) {
  server.tool(
    'openmeteo_climate_anomaly',
    'ERA5 climate anomaly analysis for a location. Compares observation period temperatures and precipitation against a baseline climatology. Returns deltas and severity classification (normal/moderate/severe/extreme based on standard deviations).',
    ClimateAnomalySchema.shape,
    async (params) => {
      const parsed = ClimateAnomalySchema.parse(params);
      const { latitude, longitude, baseline_start, baseline_end } = parsed;

      // Default observation window: last 30 days
      const now = new Date();
      const thirtyDaysAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
      const observationStart = parsed.observation_start ?? formatDate(thirtyDaysAgo);
      const observationEnd = parsed.observation_end ?? formatDate(now);

      const dailyVars = 'temperature_2m_mean,precipitation_sum';

      // Fetch baseline and observation periods in parallel
      const [baselineData, observationData] = await Promise.all([
        openMeteoFetch<OpenMeteoDailyResponse>(
          {
            latitude,
            longitude,
            start_date: baseline_start,
            end_date: baseline_end,
            daily: dailyVars,
          },
          { cacheTtl: CacheTTL.LONG },
        ),
        openMeteoFetch<OpenMeteoDailyResponse>(
          {
            latitude,
            longitude,
            start_date: observationStart,
            end_date: observationEnd,
            daily: dailyVars,
          },
          { cacheTtl: CacheTTL.SHORT },
        ),
      ]);

      // Extract baseline statistics
      const baselineTemps = filterNulls(baselineData.daily?.temperature_2m_mean ?? []);
      const baselinePrecip = filterNulls(baselineData.daily?.precipitation_sum ?? []);

      const avgTempBaseline = mean(baselineTemps);
      const stdTempBaseline = stddev(baselineTemps, avgTempBaseline);
      const avgPrecipBaseline = mean(baselinePrecip);
      const stdPrecipBaseline = stddev(baselinePrecip, avgPrecipBaseline);

      // Extract observation statistics
      const obsTemps = filterNulls(observationData.daily?.temperature_2m_mean ?? []);
      const obsPrecip = filterNulls(observationData.daily?.precipitation_sum ?? []);

      const avgTempObs = mean(obsTemps);
      const avgPrecipObs = mean(obsPrecip);

      // Compute deltas
      const tempDeltaC = Math.round((avgTempObs - avgTempBaseline) * 100) / 100;
      const precipDeltaPct =
        avgPrecipBaseline > 0
          ? Math.round(((avgPrecipObs - avgPrecipBaseline) / avgPrecipBaseline) * 10000) / 100
          : null;

      // Sigma-based severity (temperature)
      const tempSigmas =
        stdTempBaseline > 0
          ? Math.round((tempDeltaC / stdTempBaseline) * 100) / 100
          : 0;
      const tempSeverity = classifySeverity(tempSigmas);

      // Sigma-based severity (precipitation)
      const precipDeltaAbs = avgPrecipObs - avgPrecipBaseline;
      const precipSigmas =
        stdPrecipBaseline > 0
          ? Math.round((precipDeltaAbs / stdPrecipBaseline) * 100) / 100
          : 0;
      const precipSeverity = classifySeverity(precipSigmas);

      // Overall severity = worst of the two
      const severityRank: Record<AnomalySeverity, number> = {
        normal: 0,
        moderate: 1,
        severe: 2,
        extreme: 3,
      };
      const overallSeverity: AnomalySeverity =
        severityRank[tempSeverity] >= severityRank[precipSeverity]
          ? tempSeverity
          : precipSeverity;

      return wrapResponse({
        location: { latitude, longitude },
        baseline: {
          period: { start: baseline_start, end: baseline_end },
          days: baselineTemps.length,
          avg_temp_c: Math.round(avgTempBaseline * 100) / 100,
          avg_precip_mm: Math.round(avgPrecipBaseline * 100) / 100,
        },
        observation: {
          period: { start: observationStart, end: observationEnd },
          days: obsTemps.length,
          avg_temp_c: Math.round(avgTempObs * 100) / 100,
          avg_precip_mm: Math.round(avgPrecipObs * 100) / 100,
        },
        anomaly: {
          temp_delta_c: tempDeltaC,
          temp_sigmas: tempSigmas,
          temp_severity: tempSeverity,
          precip_delta_pct: precipDeltaPct,
          precip_sigmas: precipSigmas,
          precip_severity: precipSeverity,
          severity: overallSeverity,
        },
      });
    },
  );
}
