import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { avFetch, CacheTTL } from '../client.js';
import { CommodityIntervalSchema, EconIntervalSchema, TreasuryMaturitySchema } from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

const TreasurySchema = TreasuryMaturitySchema.merge(CommodityIntervalSchema);

export function registerEconomicsTools(server: McpServer) {
  // ── Economic indicators ──────────────────────────────────────────────

  server.tool(
    'av_real_gdp',
    'Get US real GDP time series (quarterly or annual). Core macroeconomic indicator for growth analysis.',
    EconIntervalSchema.shape,
    async (params) => {
      const { interval } = EconIntervalSchema.parse(params);
      const data = await avFetch({ function: 'REAL_GDP', interval }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_cpi',
    'Get US Consumer Price Index (CPI) time series. Monthly or semiannual. Essential for inflation analysis.',
    z.object({
      interval: z.enum(['monthly', 'semiannual']).default('monthly').describe('Reporting interval'),
    }).shape,
    async (params) => {
      const { interval } = z.object({ interval: z.enum(['monthly', 'semiannual']).default('monthly') }).parse(params);
      const data = await avFetch({ function: 'CPI', interval }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_inflation',
    'Get US annual inflation rate time series. Derived from CPI. Use for real return calculations and macro analysis.',
    {},
    async () => {
      const data = await avFetch({ function: 'INFLATION' }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_federal_funds_rate',
    'Get effective federal funds rate time series (daily, weekly, or monthly). The key benchmark interest rate set by the Fed.',
    CommodityIntervalSchema.shape,
    async (params) => {
      const { interval } = CommodityIntervalSchema.parse(params);
      const data = await avFetch({ function: 'FEDERAL_FUNDS_RATE', interval }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_treasury_yield',
    'Get US Treasury yield time series for a specific maturity (3mo, 2y, 5y, 7y, 10y, 30y). Essential for yield curve analysis and risk-free rate selection.',
    TreasurySchema.shape,
    async (params) => {
      const { maturity, interval } = TreasurySchema.parse(params);
      const data = await avFetch({ function: 'TREASURY_YIELD', maturity, interval }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_unemployment',
    'Get US unemployment rate time series (monthly). Key labor market indicator.',
    {},
    async () => {
      const data = await avFetch({ function: 'UNEMPLOYMENT' }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_nonfarm_payroll',
    'Get US nonfarm payroll time series (monthly). Total number of paid US workers excluding farm, government, and non-profit employees.',
    {},
    async () => {
      const data = await avFetch({ function: 'NONFARM_PAYROLL' }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_retail_sales',
    'Get US advance retail sales time series (monthly). Measures consumer spending patterns.',
    {},
    async () => {
      const data = await avFetch({ function: 'RETAIL_SALES' }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  // ── Commodities ──────────────────────────────────────────────────────

  server.tool(
    'av_wti_oil',
    'Get WTI crude oil price time series (daily, weekly, or monthly). West Texas Intermediate benchmark.',
    CommodityIntervalSchema.shape,
    async (params) => {
      const { interval } = CommodityIntervalSchema.parse(params);
      const data = await avFetch({ function: 'WTI', interval }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_brent_oil',
    'Get Brent crude oil price time series (daily, weekly, or monthly). International oil price benchmark.',
    CommodityIntervalSchema.shape,
    async (params) => {
      const { interval } = CommodityIntervalSchema.parse(params);
      const data = await avFetch({ function: 'BRENT', interval }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_natural_gas',
    'Get Henry Hub natural gas price time series (daily, weekly, or monthly). US natural gas benchmark.',
    CommodityIntervalSchema.shape,
    async (params) => {
      const { interval } = CommodityIntervalSchema.parse(params);
      const data = await avFetch({ function: 'NATURAL_GAS', interval }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'av_copper',
    'Get global copper price time series (daily, weekly, or monthly). Key industrial metal and economic bellwether.',
    CommodityIntervalSchema.shape,
    async (params) => {
      const { interval } = CommodityIntervalSchema.parse(params);
      const data = await avFetch({ function: 'COPPER', interval }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
