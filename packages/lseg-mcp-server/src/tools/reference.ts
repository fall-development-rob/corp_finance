import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { lsegFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  ReferenceDataSchema,
  CorporateActionsSchema,
  OwnershipSchema,
} from '../schemas/reference.js';

function resolveIdentifier(params: { ric?: string; isin?: string; sedol?: string }): string {
  if (params.ric) return params.ric;
  if (params.isin) return params.isin;
  if (params.sedol) return params.sedol;
  throw new Error('At least one identifier (ric, isin, or sedol) is required');
}

export function registerReferenceTools(server: McpServer) {
  // 1. Reference data
  server.tool(
    'lseg_reference_data',
    'Get reference/static data for a security including issuer, currency, exchange, sector, industry, country of domicile, and classification codes. Use to enrich instrument metadata.',
    ReferenceDataSchema.shape,
    async (params) => {
      const parsed = ReferenceDataSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `data-store/v1/instruments/${encodeURIComponent(identifier)}`,
        {},
        { cacheTtl: CacheTTL.STATIC },
      );
      return wrapResponse(data);
    },
  );

  // 2. Corporate actions
  server.tool(
    'lseg_corporate_actions',
    'Get corporate actions including dividends, splits, mergers, acquisitions, spinoffs, and rights issues. Returns event dates, ratios, and amounts. Use for total return and event analysis.',
    CorporateActionsSchema.shape,
    async (params) => {
      const parsed = CorporateActionsSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `data-store/v1/corporate-actions/${encodeURIComponent(identifier)}`,
        {
          start: parsed.start_date,
          end: parsed.end_date,
          type: parsed.type,
        },
        { cacheTtl: CacheTTL.STATIC },
      );
      return wrapResponse(data);
    },
  );

  // 3. Ownership
  server.tool(
    'lseg_ownership',
    'Get institutional ownership data including top holders, ownership percentage, shares held, and change in holdings. Use for shareholder analysis and activism monitoring.',
    OwnershipSchema.shape,
    async (params) => {
      const parsed = OwnershipSchema.parse(params);
      const identifier = resolveIdentifier(parsed);
      const data = await lsegFetch(
        `data-store/v1/ownership/${encodeURIComponent(identifier)}`,
        {
          top: parsed.limit,
          skip: parsed.offset,
        },
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );
}
