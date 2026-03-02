import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { factsetPost, wrapResponse, CacheTTL } from '../client.js';
import { PricesSchema, BondPricingSchema } from '../schemas/pricing.js';

export function registerPricingTools(server: McpServer) {
  server.tool(
    'factset_prices',
    'Get historical equity pricing data from FactSet. Returns open, high, low, close, volume for specified date range and frequency. Supports daily, weekly, and monthly frequencies.',
    PricesSchema.shape,
    async (params) => {
      const { ids, start_date, end_date, frequency } = PricesSchema.parse(params);
      const body: Record<string, unknown> = { ids };
      if (start_date) body.startDate = start_date;
      if (end_date) body.endDate = end_date;
      if (frequency) body.frequency = frequency;
      const data = await factsetPost('factset-prices/v1/prices', body, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'factset_bond_pricing',
    'Get bond pricing, yield, spread, and duration data from FactSet. Returns price, yield-to-maturity, OAS, modified duration, and convexity for fixed income securities.',
    BondPricingSchema.shape,
    async (params) => {
      const { ids } = BondPricingSchema.parse(params);
      const body = { ids };
      const data = await factsetPost('factset-prices/v1/fixed-income', body, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
