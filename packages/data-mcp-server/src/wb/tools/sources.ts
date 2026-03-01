import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { wbFetch, CacheTTL } from '../client.js';
import { PaginationSchema, SourceSchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerSourceTools(server: McpServer) {
  server.tool(
    'wb_sources',
    'List all World Bank data sources with IDs, names, descriptions, and URLs. Sources include WDI, IDS, Doing Business, Health Nutrition Population, etc.',
    PaginationSchema.shape,
    async (params) => {
      const { per_page, page } = PaginationSchema.parse(params);
      const data = await wbFetch('source', { per_page, page }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'wb_source_indicators',
    'List indicators available in a specific data source. Provide the source ID number. Returns indicator codes, names, and descriptions within that source.',
    SourceSchema.shape,
    async (params) => {
      const { source, per_page, page } = SourceSchema.parse(params);
      const data = await wbFetch(
        `source/${encodeURIComponent(source)}/indicator`,
        { per_page, page },
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );
}
