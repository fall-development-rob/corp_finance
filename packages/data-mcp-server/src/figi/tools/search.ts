import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { figiFetch, figiPost, CacheTTL } from '../client.js';
import {
  SearchSchema,
  FilterSchema,
  EnumerationsSchema,
  FigiLookupSchema,
} from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerSearchTools(server: McpServer) {
  server.tool(
    'figi_search',
    'Search OpenFIGI for securities by keyword (company name, partial ticker). Returns matching FIGI records with instrument details. Optionally filter by exchange or MIC code.',
    SearchSchema.shape,
    async (params) => {
      const { query, exchCode, micCode } = SearchSchema.parse(params);
      const queryParams: Record<string, string> = { query };
      if (exchCode) queryParams.exchCode = exchCode;
      if (micCode) queryParams.micCode = micCode;
      const data = await figiFetch('search', queryParams, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'figi_filter',
    'Filter OpenFIGI securities by attributes: exchange code, MIC code, currency, security type, market sector, or ticker. POST request returning matching instruments.',
    FilterSchema.shape,
    async (params) => {
      const parsed = FilterSchema.parse(params);
      const body: Record<string, string> = {};
      if (parsed.exchCode) body.exchCode = parsed.exchCode;
      if (parsed.micCode) body.micCode = parsed.micCode;
      if (parsed.currency) body.currency = parsed.currency;
      if (parsed.securityType) body.securityType = parsed.securityType;
      if (parsed.securityType2) body.securityType2 = parsed.securityType2;
      if (parsed.marketSector) body.marketSector = parsed.marketSector;
      if (parsed.ticker) body.ticker = parsed.ticker;
      const data = await figiPost('filter', body, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'figi_enumerations',
    'Get valid enumeration values for a mapping attribute. Supported keys: exchCode, micCode, securityType, securityType2, marketSector, currency. Use to discover valid filter values.',
    EnumerationsSchema.shape,
    async (params) => {
      const { key } = EnumerationsSchema.parse(params);
      const data = await figiFetch(`mapping/values/${encodeURIComponent(key)}`, {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'figi_security_info',
    'Look up a specific FIGI identifier to get full instrument details including name, ticker, exchange, security type, and market sector.',
    FigiLookupSchema.shape,
    async (params) => {
      const { figi } = FigiLookupSchema.parse(params);
      // Use mapping endpoint with COMPOSITE_FIGI to look up by FIGI
      const data = await figiPost('mapping', [{ idType: 'ID_BB_GLOBAL', idValue: figi }], { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
