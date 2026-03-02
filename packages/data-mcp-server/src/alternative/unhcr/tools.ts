import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { unhcrFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const UnhcrDisplacementSchema = z.object({
  year: z
    .number()
    .min(1951)
    .max(2100)
    .optional()
    .describe('Data year (default: current year minus 1)'),
  country_of_origin: z
    .string()
    .length(3)
    .optional()
    .describe('ISO 3166-1 alpha-3 country of origin code (e.g., SYR, AFG)'),
  country_of_asylum: z
    .string()
    .length(3)
    .optional()
    .describe('ISO 3166-1 alpha-3 country of asylum code (e.g., TUR, DEU)'),
  limit: z
    .number()
    .min(1)
    .max(1000)
    .default(50)
    .describe('Max rows to return (default 50)'),
});

const UnhcrCountrySchema = z.object({
  country: z
    .string()
    .length(3)
    .describe('ISO 3166-1 alpha-3 country code (e.g., SYR, UKR, AFG)'),
  role: z
    .enum(['origin', 'asylum', 'both'])
    .default('both')
    .describe('Query as country of origin, asylum, or both (default both)'),
  year: z
    .number()
    .min(1951)
    .max(2100)
    .optional()
    .describe('Data year (default: current year minus 1)'),
});

// ---------- Response typings ----------

interface UnhcrPopulationItem {
  year: number;
  coo_name: string;
  coo_iso: string;
  coa_name: string;
  coa_iso: string;
  refugees: number | null;
  asylum_seekers: number | null;
  idps: number | null;
  stateless: number | null;
  ooc: number | null; // others of concern
  [key: string]: unknown;
}

interface UnhcrApiResponse {
  items: UnhcrPopulationItem[];
  maxPages: number;
  page: number;
  total: number;
}

// ---------- Helpers ----------

function defaultYear(): number {
  return new Date().getFullYear() - 1;
}

function safeNum(val: unknown): number {
  if (val === null || val === undefined) return 0;
  const n = Number(val);
  return Number.isFinite(n) ? n : 0;
}

// ---------- Tool registration ----------

export function registerUnhcrTools(server: McpServer) {
  server.tool(
    'unhcr_displacement',
    'Global displacement statistics from UNHCR. Returns refugee, asylum seeker, IDP, and stateless populations by country of origin and asylum.',
    UnhcrDisplacementSchema.shape,
    async (params) => {
      const parsed = UnhcrDisplacementSchema.parse(params);
      const year = parsed.year ?? defaultYear();

      const queryParams: Record<string, string | number | boolean> = {
        year,
        limit: parsed.limit,
        coo_all: true,
      };

      if (parsed.country_of_origin) {
        queryParams.coo = parsed.country_of_origin;
      }
      if (parsed.country_of_asylum) {
        queryParams.coa = parsed.country_of_asylum;
      }

      const data = await unhcrFetch<UnhcrApiResponse>(queryParams, {
        cacheTtl: CacheTTL.MEDIUM,
      });

      const items = (data.items ?? []).map((item) => {
        const refugees = safeNum(item.refugees);
        const asylumSeekers = safeNum(item.asylum_seekers);
        const idps = safeNum(item.idps);
        const stateless = safeNum(item.stateless);
        const ooc = safeNum(item.ooc);

        return {
          year: item.year,
          coo_name: item.coo_name,
          coo_iso: item.coo_iso,
          coa_name: item.coa_name,
          coa_iso: item.coa_iso,
          refugees,
          asylum_seekers: asylumSeekers,
          idps,
          stateless,
          total_population: refugees + asylumSeekers + idps + stateless + ooc,
        };
      });

      return wrapResponse({
        year,
        total_rows: data.total ?? items.length,
        returned: items.length,
        data: items,
      });
    },
  );

  server.tool(
    'unhcr_country',
    'Country displacement profile from UNHCR. Returns total displaced, refugees from/hosted, IDPs, asylum seekers, and trend comparison vs prior year.',
    UnhcrCountrySchema.shape,
    async (params) => {
      const { country, role, year: requestedYear } = UnhcrCountrySchema.parse(params);
      const year = requestedYear ?? defaultYear();
      const priorYear = year - 1;

      // Build queries based on role
      const queries: Array<{
        label: string;
        params: Record<string, string | number | boolean>;
      }> = [];

      if (role === 'origin' || role === 'both') {
        queries.push({
          label: 'origin',
          params: { year, coo: country, coo_all: true, limit: 1000 },
        });
        queries.push({
          label: 'origin_prior',
          params: { year: priorYear, coo: country, coo_all: true, limit: 1000 },
        });
      }

      if (role === 'asylum' || role === 'both') {
        queries.push({
          label: 'asylum',
          params: { year, coa: country, coo_all: true, limit: 1000 },
        });
        queries.push({
          label: 'asylum_prior',
          params: { year: priorYear, coa: country, coo_all: true, limit: 1000 },
        });
      }

      // Fetch all in parallel
      const results = await Promise.all(
        queries.map(async (q) => {
          const data = await unhcrFetch<UnhcrApiResponse>(q.params, {
            cacheTtl: CacheTTL.MEDIUM,
          }).catch(() => ({ items: [], maxPages: 0, page: 0, total: 0 }));
          return { label: q.label, data };
        }),
      );

      const resultMap = new Map(results.map((r) => [r.label, r.data]));

      // Aggregate function
      function aggregate(items: UnhcrPopulationItem[]) {
        let refugees = 0;
        let asylumSeekers = 0;
        let idps = 0;
        let stateless = 0;

        for (const item of items) {
          refugees += safeNum(item.refugees);
          asylumSeekers += safeNum(item.asylum_seekers);
          idps += safeNum(item.idps);
          stateless += safeNum(item.stateless);
        }

        return {
          refugees,
          asylum_seekers: asylumSeekers,
          idps,
          stateless,
          total: refugees + asylumSeekers + idps + stateless,
        };
      }

      const originData = resultMap.get('origin');
      const originPriorData = resultMap.get('origin_prior');
      const asylumData = resultMap.get('asylum');
      const asylumPriorData = resultMap.get('asylum_prior');

      const originAgg = originData ? aggregate(originData.items) : null;
      const originPriorAgg = originPriorData ? aggregate(originPriorData.items) : null;
      const asylumAgg = asylumData ? aggregate(asylumData.items) : null;
      const asylumPriorAgg = asylumPriorData ? aggregate(asylumPriorData.items) : null;

      // Compute total displaced and trend
      const totalCurrent =
        (originAgg?.total ?? 0) + (asylumAgg?.total ?? 0);
      const totalPrior =
        (originPriorAgg?.total ?? 0) + (asylumPriorAgg?.total ?? 0);
      const trendVsPriorYear =
        totalPrior > 0
          ? Math.round(((totalCurrent - totalPrior) / totalPrior) * 10000) / 100
          : null;

      return wrapResponse({
        country,
        year,
        role,
        total_displaced: totalCurrent,
        refugees_from: originAgg?.refugees ?? null,
        refugees_hosted: asylumAgg?.refugees ?? null,
        idps: originAgg?.idps ?? asylumAgg?.idps ?? null,
        asylum_seekers:
          (originAgg?.asylum_seekers ?? 0) +
          (asylumAgg?.asylum_seekers ?? 0),
        trend_vs_prior_year_pct: trendVsPriorYear,
        breakdown: {
          as_origin: originAgg,
          as_asylum: asylumAgg,
        },
      });
    },
  );
}
