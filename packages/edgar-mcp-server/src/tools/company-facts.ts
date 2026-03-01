import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { edgarFetch, CacheTTL, padCik } from '../client.js';
import { CikSchema, TaxonomySchema, ConceptSchema, YearSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

// Composite schemas for multi-field tools
const CompanyConceptSchema = CikSchema.merge(TaxonomySchema).merge(ConceptSchema);

const FramesSchema = TaxonomySchema.merge(ConceptSchema).merge(YearSchema).extend({
  unit: z.string().default('USD').describe('Unit of measure (USD, shares, pure, etc.)'),
  quarter: z.number().int().min(1).max(4).optional().describe('Quarter (1-4). Omit for annual.'),
});

const XbrlTagsSchema = CikSchema.merge(TaxonomySchema);

export function registerCompanyFactsTools(server: McpServer) {
  server.tool(
    'edgar_company_facts',
    'Get all XBRL facts for a company by CIK. Returns every financial concept (revenue, assets, liabilities, etc.) across all filings. Comprehensive structured financial data.',
    CikSchema.shape,
    async (params) => {
      const { cik } = CikSchema.parse(params);
      const data = await edgarFetch(
        `api/xbrl/companyfacts/CIK${padCik(cik)}.json`,
        {},
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_company_concept',
    'Get a single XBRL concept for one company over time. Returns all reported values for that concept across filings (e.g., Revenue history for Apple). Ideal for time-series analysis of a specific metric.',
    CompanyConceptSchema.shape,
    async (params) => {
      const { cik, taxonomy, concept } = CompanyConceptSchema.parse(params);
      const data = await edgarFetch(
        `api/xbrl/companyconcept/CIK${padCik(cik)}/${taxonomy}/${concept}.json`,
        {},
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_frames',
    'Get cross-sectional XBRL frame data: one concept across ALL companies for a single period. Returns every company that reported the concept in that year/quarter. Great for peer comparison and screening.',
    FramesSchema.shape,
    async (params) => {
      const { taxonomy, concept, unit, year, quarter } = FramesSchema.parse(params);
      const period = quarter ? `CY${year}Q${quarter}I` : `CY${year}`;
      const data = await edgarFetch(
        `api/xbrl/frames/${taxonomy}/${concept}/${unit}/${period}.json`,
        {},
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_concept',
    'Get aggregated data for a single XBRL concept across all companies for a year. Alias for frames with USD unit. Use for broad market-level financial data.',
    TaxonomySchema.merge(ConceptSchema).merge(YearSchema).shape,
    async (params) => {
      const parsed = TaxonomySchema.merge(ConceptSchema).merge(YearSchema).parse(params);
      const { taxonomy, concept, year } = parsed;
      const data = await edgarFetch(
        `api/xbrl/frames/${taxonomy}/${concept}/USD/CY${year}.json`,
        {},
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'edgar_xbrl_tags',
    'List available XBRL tags/concepts for a company within a taxonomy. Useful for discovering what financial concepts a company reports before querying specific data.',
    XbrlTagsSchema.shape,
    async (params) => {
      const { cik, taxonomy } = XbrlTagsSchema.parse(params);
      // Fetch all facts and extract the tag names for the requested taxonomy
      const data = await edgarFetch<Record<string, unknown>>(
        `api/xbrl/companyfacts/CIK${padCik(cik)}.json`,
        {},
        { cacheTtl: CacheTTL.MEDIUM },
      );
      const facts = (data as { facts?: Record<string, Record<string, unknown>> }).facts;
      if (!facts || !facts[taxonomy]) {
        return wrapResponse({ taxonomy, tags: [], message: `No ${taxonomy} facts found for CIK ${cik}` });
      }
      const tags = Object.keys(facts[taxonomy]).sort();
      return wrapResponse({ cik, taxonomy, tag_count: tags.length, tags });
    },
  );
}
