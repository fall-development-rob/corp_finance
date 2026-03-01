import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { spFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  CompanySearchSchema,
  TearsheetSchema,
  CapitalStructureSchema,
  OwnershipSchema,
} from '../schemas/company.js';

export function registerCompanyTools(server: McpServer) {
  server.tool(
    'sp_company_search',
    'Search S&P Capital IQ for companies by name or identifier. Returns matching companies with IDs, names, tickers, and basic info. Use to discover company identifiers for other tools.',
    CompanySearchSchema.shape,
    async (params) => {
      const { query, limit, offset } = CompanySearchSchema.parse(params);
      const data = await spFetch('companies/search', {
        query,
        limit,
        offset,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_company_tearsheet',
    'Get comprehensive company tearsheet with key metrics including market cap, revenue, EBITDA, margins, multiples, sector, and description. Use as a starting point for company analysis.',
    TearsheetSchema.shape,
    async (params) => {
      const { company_id, ticker, name } = TearsheetSchema.parse(params);
      const data = await spFetch('companies/tearsheet', {
        company_id,
        ticker,
        name,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_capital_structure',
    'Get debt and equity capital structure breakdown including outstanding debt instruments, maturities, interest rates, and equity composition. Essential for credit and leverage analysis.',
    CapitalStructureSchema.shape,
    async (params) => {
      const { company_id, ticker, name, start_date, end_date } = CapitalStructureSchema.parse(params);
      const data = await spFetch('companies/capital-structure', {
        company_id,
        ticker,
        name,
        start_date,
        end_date,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'sp_ownership',
    'Get shareholder and institutional ownership data including top holders, ownership percentages, and recent changes. Use for governance analysis and investor base assessment.',
    OwnershipSchema.shape,
    async (params) => {
      const { company_id, ticker, name, limit, offset } = OwnershipSchema.parse(params);
      const data = await spFetch('companies/ownership', {
        company_id,
        ticker,
        name,
        limit,
        offset,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
