import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { wbFetch, CacheTTL } from '../client.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

/** WGI dimension name to indicator code prefix mapping */
const DIMENSION_MAP: Record<string, { code: string; label: string }> = {
  corruption:    { code: 'CC', label: 'Control of Corruption' },
  effectiveness: { code: 'GE', label: 'Government Effectiveness' },
  stability:     { code: 'PV', label: 'Political Stability and Absence of Violence' },
  regulatory:    { code: 'RQ', label: 'Regulatory Quality' },
  rule_of_law:   { code: 'RL', label: 'Rule of Law' },
  voice:         { code: 'VA', label: 'Voice and Accountability' },
};

const ALL_ESTIMATE_CODES = Object.values(DIMENSION_MAP).map(d => `${d.code}.EST`);
const ALL_PERCENTILE_CODES = Object.values(DIMENSION_MAP).map(d => `${d.code}.PER`);

const GovernanceSchema = z.object({
  country: z.string().min(1).describe('Country code (ISO2 or ISO3, e.g. US, GBR, CN)'),
});

const GovernanceCompareSchema = z.object({
  countries: z.string().min(1).describe('Semicolon-separated ISO country codes (e.g. US;GB;CN;BR;IN)'),
  dimension: z.enum(['corruption', 'effectiveness', 'stability', 'regulatory', 'rule_of_law', 'voice'])
    .describe('WGI dimension to compare'),
});

const GovernanceTrendSchema = z.object({
  country: z.string().min(1).describe('Country code (ISO2 or ISO3, e.g. US, GBR)'),
  dimension: z.enum(['corruption', 'effectiveness', 'stability', 'regulatory', 'rule_of_law', 'voice'])
    .describe('WGI dimension'),
  date: z.string().default('2010:2023').describe('Date range (e.g. 2010:2023)'),
});

interface WbRecord {
  indicator?: { id?: string; value?: string };
  country?: { id?: string; value?: string };
  date?: string;
  value?: number | null;
}

function extractRecords(raw: unknown): WbRecord[] {
  if (Array.isArray(raw) && raw.length >= 2 && Array.isArray(raw[1])) {
    return raw[1] as WbRecord[];
  }
  return [];
}

export function registerGovernanceTools(server: McpServer) {
  server.tool(
    'wb_governance',
    'Get World Governance Indicators (WGI) for a country. Returns estimates (-2.5 to +2.5) and percentile ranks (0-100) across 6 dimensions: corruption, effectiveness, stability, regulatory quality, rule of law, and voice/accountability.',
    GovernanceSchema.shape,
    async (params) => {
      const { country } = GovernanceSchema.parse(params);
      const allCodes = [...ALL_ESTIMATE_CODES, ...ALL_PERCENTILE_CODES].join(';');
      const raw = await wbFetch(
        `country/${encodeURIComponent(country)}/indicator/${encodeURIComponent(allCodes)}`,
        { per_page: 500 },
        { cacheTtl: CacheTTL.STATIC },
      );
      const records = extractRecords(raw);

      // Group by dimension, taking the most recent year per indicator
      const latest: Record<string, { estimate?: number | null; percentile_rank?: number | null; year?: string }> = {};
      for (const rec of records) {
        const indicatorId = rec.indicator?.id;
        if (!indicatorId) continue;
        for (const [dimName, dim] of Object.entries(DIMENSION_MAP)) {
          if (indicatorId === `${dim.code}.EST`) {
            if (!latest[dimName] || (rec.date && rec.date > (latest[dimName].year ?? ''))) {
              latest[dimName] = { ...latest[dimName], estimate: rec.value, year: rec.date };
            }
          }
          if (indicatorId === `${dim.code}.PER`) {
            if (!latest[dimName]?.percentile_rank || (rec.date && rec.date > (latest[dimName].year ?? ''))) {
              latest[dimName] = { ...latest[dimName], percentile_rank: rec.value };
            }
          }
        }
      }

      const result = Object.entries(DIMENSION_MAP).map(([dimName, dim]) => ({
        dimension_name: dim.label,
        dimension_key: dimName,
        estimate: latest[dimName]?.estimate ?? null,
        percentile_rank: latest[dimName]?.percentile_rank ?? null,
        year: latest[dimName]?.year ?? null,
      }));

      return wrapResponse({ country, governance: result });
    },
  );

  server.tool(
    'wb_governance_compare',
    'Compare WGI scores across multiple countries for one governance dimension. Returns sorted comparison with estimates and percentile ranks.',
    GovernanceCompareSchema.shape,
    async (params) => {
      const { countries, dimension } = GovernanceCompareSchema.parse(params);
      const dim = DIMENSION_MAP[dimension];
      const indicators = `${dim.code}.EST;${dim.code}.PER`;
      const raw = await wbFetch(
        `country/${encodeURIComponent(countries)}/indicator/${encodeURIComponent(indicators)}`,
        { per_page: 500 },
        { cacheTtl: CacheTTL.STATIC },
      );
      const records = extractRecords(raw);

      // Group by country, taking most recent year
      const byCountry: Record<string, { country_name?: string; estimate?: number | null; percentile_rank?: number | null; year?: string }> = {};
      for (const rec of records) {
        const countryId = rec.country?.id;
        const indicatorId = rec.indicator?.id;
        if (!countryId || !indicatorId) continue;

        if (!byCountry[countryId]) {
          byCountry[countryId] = { country_name: rec.country?.value };
        }

        if (indicatorId === `${dim.code}.EST`) {
          if (!byCountry[countryId].year || (rec.date && rec.date > byCountry[countryId].year!)) {
            byCountry[countryId].estimate = rec.value;
            byCountry[countryId].year = rec.date;
          }
        }
        if (indicatorId === `${dim.code}.PER`) {
          if (!byCountry[countryId].year || (rec.date && rec.date > byCountry[countryId].year!)) {
            byCountry[countryId].percentile_rank = rec.value;
          }
        }
      }

      // Sort by estimate descending (best governance first)
      const comparison = Object.entries(byCountry)
        .map(([code, data]) => ({
          country_code: code,
          country_name: data.country_name ?? code,
          estimate: data.estimate ?? null,
          percentile_rank: data.percentile_rank ?? null,
          year: data.year ?? null,
        }))
        .sort((a, b) => (b.estimate ?? -999) - (a.estimate ?? -999));

      return wrapResponse({
        dimension: dim.label,
        dimension_key: dimension,
        comparison,
      });
    },
  );

  server.tool(
    'wb_governance_trend',
    'Get historical WGI trend for a country and governance dimension. Returns yearly estimates and percentile ranks over the specified date range.',
    GovernanceTrendSchema.shape,
    async (params) => {
      const { country, dimension, date } = GovernanceTrendSchema.parse(params);
      const dim = DIMENSION_MAP[dimension];
      const indicators = `${dim.code}.EST;${dim.code}.PER`;
      const raw = await wbFetch(
        `country/${encodeURIComponent(country)}/indicator/${encodeURIComponent(indicators)}`,
        { per_page: 500, date },
        { cacheTtl: CacheTTL.STATIC },
      );
      const records = extractRecords(raw);

      // Group by year
      const byYear: Record<string, { estimate?: number | null; percentile_rank?: number | null }> = {};
      for (const rec of records) {
        const indicatorId = rec.indicator?.id;
        const year = rec.date;
        if (!indicatorId || !year) continue;

        if (!byYear[year]) byYear[year] = {};

        if (indicatorId === `${dim.code}.EST`) {
          byYear[year].estimate = rec.value;
        }
        if (indicatorId === `${dim.code}.PER`) {
          byYear[year].percentile_rank = rec.value;
        }
      }

      // Sort chronologically
      const trend = Object.entries(byYear)
        .map(([year, data]) => ({
          year,
          estimate: data.estimate ?? null,
          percentile_rank: data.percentile_rank ?? null,
        }))
        .sort((a, b) => a.year.localeCompare(b.year));

      return wrapResponse({
        country,
        dimension: dim.label,
        dimension_key: dimension,
        trend,
      });
    },
  );
}
