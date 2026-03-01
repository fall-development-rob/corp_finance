import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { factsetPost, wrapResponse, CacheTTL } from '../client.js';
import { OwnershipSchema, InstitutionalSchema } from '../schemas/ownership.js';

export function registerOwnershipTools(server: McpServer) {
  server.tool(
    'factset_ownership',
    'Get ownership breakdown (institutional, insider, retail) for securities from FactSet. Shows percentage held by each category with historical changes. Use for understanding shareholder structure.',
    OwnershipSchema.shape,
    async (params) => {
      const { ids, limit, offset } = OwnershipSchema.parse(params);
      const body = { ids, limit, offset };
      const data = await factsetPost('ownership/v1/fund-holdings', body, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_institutional',
    'Get institutional holder details and 13F filings from FactSet. Returns institution name, shares held, market value, percent of portfolio, and filing date. Use for institutional investor analysis.',
    InstitutionalSchema.shape,
    async (params) => {
      const { ids, holder_type, limit, offset } = InstitutionalSchema.parse(params);
      const body: Record<string, unknown> = { ids, limit, offset };
      if (holder_type) body.holderType = holder_type;
      const data = await factsetPost('ownership/v1/institutional-holdings', body, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
