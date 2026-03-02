import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { pbFetch, CacheTTL } from '../client.js';
import {
  CompanySearchSchema,
  CompanyProfileSchema,
} from '../schemas/company.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerCompanyTools(server: McpServer) {
  server.tool(
    'pb_company_search',
    'Search PitchBook for private and public companies. Returns matching companies with name, industry, location, funding stage, total raised, latest valuation, and key investors.',
    CompanySearchSchema.shape,
    async (params) => {
      const { query, industry, page, page_size } = CompanySearchSchema.parse(params);
      const data = await pbFetch('companies/search', {
        query, industry, page, page_size,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_company_profile',
    'Get detailed company profile with financials and ownership. Returns company description, founding date, employees, revenue, EBITDA, valuation history, ownership breakdown, board members, and key investors.',
    CompanyProfileSchema.shape,
    async (params) => {
      const { entity_id, name } = CompanyProfileSchema.parse(params);
      const data = await pbFetch('companies/profile', {
        entity_id, name,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
