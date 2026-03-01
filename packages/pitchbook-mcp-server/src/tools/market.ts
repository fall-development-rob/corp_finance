import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { pbFetch, CacheTTL } from '../client.js';
import {
  VcExitsSchema,
  FundraisingSchema,
  MarketStatsSchema,
  PeopleSearchSchema,
  ServiceProvidersSchema,
} from '../schemas/market.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerMarketTools(server: McpServer) {
  server.tool(
    'pb_vc_exits',
    'Get VC exit data (IPO, M&A, secondary) with valuations. Returns exit events with company, exit type, valuation, return multiple, holding period, acquirer/exchange, and investor returns.',
    VcExitsSchema.shape,
    async (params) => {
      const { exit_type, start_date, end_date, page, page_size } = VcExitsSchema.parse(params);
      const data = await pbFetch('market/exits', {
        exit_type, start_date, end_date, page, page_size,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_fundraising',
    'Get fundraising activity by strategy and vintage. Returns fund closes with manager, strategy, vintage, target/final size, number of LPs, time to close, and step-up from prior fund.',
    FundraisingSchema.shape,
    async (params) => {
      const { strategy, start_date, end_date, page, page_size } = FundraisingSchema.parse(params);
      const data = await pbFetch('market/fundraising', {
        strategy, start_date, end_date, page, page_size,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_market_stats',
    'Get private market statistics (deal flow, dry powder, valuations). Returns aggregate metrics including deal count, total value, median/mean multiples, dry powder levels, and year-over-year trends.',
    MarketStatsSchema.shape,
    async (params) => {
      const { sector, geography, start_date, end_date } = MarketStatsSchema.parse(params);
      const data = await pbFetch('market/stats', {
        sector, geography, start_date, end_date,
      }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_people_search',
    'Search for key people in private markets. Returns matching professionals with name, title, firm, board seats, deal involvement, education, and career history.',
    PeopleSearchSchema.shape,
    async (params) => {
      const { name, role, page, page_size } = PeopleSearchSchema.parse(params);
      const data = await pbFetch('people/search', {
        name, role, page, page_size,
      }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'pb_service_providers',
    'Find service providers (law firms, banks, consultants). Returns providers with name, type, deal count, notable clients, specializations, and geographic coverage.',
    ServiceProvidersSchema.shape,
    async (params) => {
      const { type, geography, page, page_size } = ServiceProvidersSchema.parse(params);
      const data = await pbFetch('providers/search', {
        type, geography, page, page_size,
      }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );
}
