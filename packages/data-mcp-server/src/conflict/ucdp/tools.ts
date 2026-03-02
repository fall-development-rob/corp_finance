import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { ucdpFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const UcdpConflictsSchema = z.object({
  country: z.string().describe('ISO 3166-1 alpha-3 country code (e.g., UKR, SYR)'),
  start_date: z.string().optional().describe('Start date YYYY-MM-DD (optional)'),
  end_date: z.string().optional().describe('End date YYYY-MM-DD (optional)'),
  page: z.number().min(0).default(0).describe('Page number (0-indexed)'),
  pagesize: z.number().min(1).max(1000).default(100).describe('Results per page'),
});

const UcdpBattleDeathsSchema = z.object({
  conflict_id: z.string().optional().describe('UCDP conflict ID'),
  country: z.string().optional().describe('ISO 3166-1 alpha-3 country code'),
  page: z.number().min(0).default(0).describe('Page number (0-indexed)'),
  pagesize: z.number().min(1).max(1000).default(100).describe('Results per page'),
}).refine(
  (d) => d.conflict_id || d.country,
  { message: 'Either conflict_id or country must be provided' },
);

const UcdpCountryProfileSchema = z.object({
  country: z.string().describe('ISO 3166-1 alpha-3 country code (e.g., UKR, SYR, AFG)'),
});

// ---------- UCDP response typings ----------

interface UcdpGedEvent {
  id: number;
  conflict_new_id: number;
  type_of_violence: number;
  best: number;
  low: number;
  high: number;
  region: string;
  country: string;
  country_id: number;
  date_start: string;
  date_end: string;
  [key: string]: unknown;
}

interface UcdpBattleDeathEntry {
  conflict_id: number;
  year: number;
  best: number;
  low: number;
  high: number;
  [key: string]: unknown;
}

interface UcdpPagedResponse<T> {
  TotalCount: number;
  TotalPages: number;
  PreviousPageUrl: string;
  NextPageUrl: string;
  Result: T[];
}

// ---------- Helpers ----------

function violenceTypeLabel(typeCode: number): string {
  switch (typeCode) {
    case 1: return 'state-based';
    case 2: return 'non-state';
    case 3: return 'one-sided';
    default: return `unknown(${typeCode})`;
  }
}

function classifyIntensity(annualDeaths: number): string {
  if (annualDeaths >= 1000) return 'war';
  if (annualDeaths >= 25) return 'minor';
  return 'none';
}

// ---------- Tool registration ----------

export function registerUcdpTools(server: McpServer) {
  server.tool(
    'ucdp_conflicts',
    'Query UCDP geo-referenced events for armed conflicts by country and optional date range. Returns conflict ID, violence type (state-based/non-state/one-sided), death estimates, and region.',
    UcdpConflictsSchema.shape,
    async (params) => {
      const { country, start_date, end_date, page, pagesize } = UcdpConflictsSchema.parse(params);

      const queryParams: Record<string, string | number> = {
        Country: country,
        page,
        pagesize,
      };
      if (start_date) queryParams.StartDate = start_date;
      if (end_date) queryParams.EndDate = end_date;

      const raw = await ucdpFetch<UcdpPagedResponse<UcdpGedEvent>>(
        'gedevents/24.1',
        queryParams,
        { cacheTtl: CacheTTL.MEDIUM },
      );

      const events = (raw.Result ?? []).map((e) => ({
        conflict_id: e.conflict_new_id,
        type_of_violence: e.type_of_violence,
        type_label: violenceTypeLabel(e.type_of_violence),
        best_est: e.best,
        low_est: e.low,
        high_est: e.high,
        region: e.region,
        country: e.country,
        date_start: e.date_start,
        date_end: e.date_end,
      }));

      return wrapResponse({
        total_count: raw.TotalCount,
        total_pages: raw.TotalPages,
        page,
        country,
        events,
      });
    },
  );

  server.tool(
    'ucdp_battle_deaths',
    'Time series of battle-related deaths from UCDP. Query by conflict ID or country. Returns yearly best/low/high death estimates.',
    // UcdpBattleDeathsSchema uses .refine() so we extract the inner shape
    UcdpBattleDeathsSchema._def.schema.shape,
    async (params) => {
      const parsed = UcdpBattleDeathsSchema.parse(params);

      const queryParams: Record<string, string | number> = {
        page: parsed.page,
        pagesize: parsed.pagesize,
      };
      if (parsed.conflict_id) queryParams.ConflictId = parsed.conflict_id;
      if (parsed.country) queryParams.Country = parsed.country;

      const raw = await ucdpFetch<UcdpPagedResponse<UcdpBattleDeathEntry>>(
        'battledeaths/24.1',
        queryParams,
        { cacheTtl: CacheTTL.LONG },
      );

      const series = (raw.Result ?? []).map((r) => ({
        conflict_id: r.conflict_id,
        year: r.year,
        best_estimate: r.best,
        low_estimate: r.low,
        high_estimate: r.high,
      }));

      return wrapResponse({
        total_count: raw.TotalCount,
        total_pages: raw.TotalPages,
        page: parsed.page,
        series,
      });
    },
  );

  server.tool(
    'ucdp_country_profile',
    'Country conflict history from UCDP. Returns years with conflict, total battle deaths, active conflict types, and intensity classification (war/minor/none based on annual deaths).',
    UcdpCountryProfileSchema.shape,
    async (params) => {
      const { country } = UcdpCountryProfileSchema.parse(params);

      // Fetch geo-referenced events (all pages up to a reasonable limit)
      const eventsRaw = await ucdpFetch<UcdpPagedResponse<UcdpGedEvent>>(
        'gedevents/24.1',
        { Country: country, pagesize: 1000 },
        { cacheTtl: CacheTTL.LONG },
      );

      // Fetch battle deaths
      const deathsRaw = await ucdpFetch<UcdpPagedResponse<UcdpBattleDeathEntry>>(
        'battledeaths/24.1',
        { Country: country, pagesize: 1000 },
        { cacheTtl: CacheTTL.LONG },
      );

      const events = eventsRaw.Result ?? [];
      const deathEntries = deathsRaw.Result ?? [];

      // Years with conflict (from events)
      const yearsWithConflict = new Set<number>();
      const conflictTypes = new Set<string>();
      let totalDeathsFromEvents = 0;

      for (const e of events) {
        const year = new Date(e.date_start).getFullYear();
        if (!isNaN(year)) yearsWithConflict.add(year);
        conflictTypes.add(violenceTypeLabel(e.type_of_violence));
        totalDeathsFromEvents += e.best ?? 0;
      }

      // Total battle deaths from the deaths endpoint
      let totalBattleDeaths = 0;
      const deathsByYear: Record<number, number> = {};

      for (const d of deathEntries) {
        totalBattleDeaths += d.best ?? 0;
        deathsByYear[d.year] = (deathsByYear[d.year] ?? 0) + (d.best ?? 0);
      }

      // Most recent year's deaths for intensity classification
      const years = Object.keys(deathsByYear).map(Number).sort((a, b) => b - a);
      const latestYear = years[0] ?? 0;
      const latestYearDeaths = deathsByYear[latestYear] ?? 0;

      return wrapResponse({
        country,
        years_with_conflict: Array.from(yearsWithConflict).sort(),
        total_conflict_years: yearsWithConflict.size,
        total_battle_deaths: totalBattleDeaths,
        total_event_deaths: totalDeathsFromEvents,
        conflict_types: Array.from(conflictTypes),
        intensity_classification: classifyIntensity(latestYearDeaths),
        latest_year_assessed: latestYear,
        latest_year_deaths: latestYearDeaths,
        deaths_by_year: deathsByYear,
        total_events: events.length,
      });
    },
  );
}
