import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  InsiderLatestSchema, InsiderSearchSchema, InsiderByNameSchema, InsiderStatsSchema,
} from '../schemas/insider.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerInsiderTools(server: McpServer) {
  server.tool(
    'fmp_insider_latest',
    'Get latest insider trades across all companies. Returns recent SEC Form 4 filings with transaction details, shares traded, and prices.',
    InsiderLatestSchema.shape,
    async (params) => {
      const { page, limit } = InsiderLatestSchema.parse(params);
      const data = await fmpFetch('insider-trading/latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_insider_search',
    'Search insider trades with optional ticker filter. Returns SEC Form 4 filings including insider name, title, transaction type, shares, and price.',
    InsiderSearchSchema.shape,
    async (params) => {
      const { symbol, page, limit } = InsiderSearchSchema.parse(params);
      const data = await fmpFetch('insider-trading/search', { symbol, page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_insider_by_name',
    'Get insider trades by person name. Returns all SEC Form 4 filings for a specific insider (e.g., Zuckerberg) across all companies.',
    InsiderByNameSchema.shape,
    async (params) => {
      const { name } = InsiderByNameSchema.parse(params);
      const data = await fmpFetch('insider-trading/reporting-name', { name }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  const NoParamsSchema = z.object({});

  server.tool(
    'fmp_insider_transaction_types',
    'Get all insider transaction type codes and their descriptions. Use to decode transaction types in insider trading data.',
    NoParamsSchema.shape,
    async () => {
      const data = await fmpFetch('insider-trading-transaction-type', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_insider_stats',
    'Get insider trading statistics for a stock. Returns aggregate buy/sell volumes, net activity, and insider sentiment indicators.',
    InsiderStatsSchema.shape,
    async (params) => {
      const { symbol } = InsiderStatsSchema.parse(params);
      const data = await fmpFetch('insider-trading/statistics', { symbol }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_beneficial_ownership',
    'Get beneficial ownership filings (Schedule 13D/13G). Returns acquisition of beneficial ownership data for a stock.',
    InsiderStatsSchema.shape,
    async (params) => {
      const { symbol } = InsiderStatsSchema.parse(params);
      const data = await fmpFetch('acquisition-of-beneficial-ownership', { symbol }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
