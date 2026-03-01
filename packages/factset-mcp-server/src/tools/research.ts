import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { factsetFetch, factsetPost, wrapResponse, CacheTTL } from '../client.js';
import {
  SupplyChainSchema,
  GeoRevenueSchema,
  EventsSchema,
  PeopleSchema,
  MaDealsSchema,
} from '../schemas/research.js';

export function registerResearchTools(server: McpServer) {
  server.tool(
    'factset_supply_chain',
    'Get supply chain relationships (suppliers and customers) for a company from FactSet. Returns related companies with revenue exposure, relationship type, and confidence score. Use for supply chain risk and opportunity analysis.',
    SupplyChainSchema.shape,
    async (params) => {
      const { id, direction } = SupplyChainSchema.parse(params);
      const queryParams: Record<string, string> = {};
      if (direction) queryParams.direction = direction;
      const data = await factsetFetch(`factset-supply-chain/v1/supply-chain/${encodeURIComponent(id)}`, queryParams, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_geo_revenue',
    'Get geographic revenue exposure breakdown for a company from FactSet. Returns revenue by region/country with percentage allocation. Use for geographic diversification and country risk analysis.',
    GeoRevenueSchema.shape,
    async (params) => {
      const { id } = GeoRevenueSchema.parse(params);
      const data = await factsetFetch(`factset-geo-revenue/v1/revenue/${encodeURIComponent(id)}`, {}, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_events',
    'Get corporate events (earnings, conferences, filings) from FactSet. Returns event date, type, description, and related documents. Use for event-driven analysis and corporate calendar monitoring.',
    EventsSchema.shape,
    async (params) => {
      const { ids, start_date, end_date, type } = EventsSchema.parse(params);
      const body: Record<string, unknown> = { ids };
      if (start_date) body.startDate = start_date;
      if (end_date) body.endDate = end_date;
      if (type) body.types = [type];
      const data = await factsetPost('events/v1/corporate-events', body, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_people',
    'Get key people and management for a company from FactSet. Returns executives and board members with name, title, age, tenure, and compensation. Use for management quality analysis and corporate governance review.',
    PeopleSchema.shape,
    async (params) => {
      const { id } = PeopleSchema.parse(params);
      const data = await factsetFetch(`factset-people/v1/profiles`, { ids: id }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_ma_deals',
    'Search M&A deal activity from FactSet. Returns deal details including target, acquirer, deal value, premium, status, and announcement date. Use for M&A market analysis and deal screening.',
    MaDealsSchema.shape,
    async (params) => {
      const { target, acquirer, start_date, end_date, limit, offset } = MaDealsSchema.parse(params);
      const body: Record<string, unknown> = { limit, offset };
      if (target) body.target = target;
      if (acquirer) body.acquirer = acquirer;
      if (start_date) body.startDate = start_date;
      if (end_date) body.endDate = end_date;
      const data = await factsetPost('mergers-acquisitions/v1/deals', body, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );
}
