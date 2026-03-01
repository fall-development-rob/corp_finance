import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { wbFetch, CacheTTL } from '../client.js';
import {
  CountrySchema,
  CountryListSchema,
  CountryPopularSchema,
  PaginationSchema,
} from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

/** Popular macroeconomic indicators for country overview */
const POPULAR_INDICATORS = [
  'NY.GDP.MKTP.CD',     // GDP (current USD)
  'NY.GDP.MKTP.KD.ZG',  // GDP growth (annual %)
  'FP.CPI.TOTL.ZG',     // Inflation (CPI, annual %)
  'SL.UEM.TOTL.ZS',     // Unemployment (% of labor force)
  'SP.POP.TOTL',         // Population
  'BN.CAB.XOKA.GD.ZS',  // Current account balance (% of GDP)
  'GC.DOD.TOTL.GD.ZS',  // Government debt (% of GDP)
  'NE.EXP.GNFS.ZS',     // Exports (% of GDP)
  'FR.INR.RINR',         // Real interest rate (%)
  'PA.NUS.FCRF',         // Exchange rate (LCU per USD)
];

export function registerCountryTools(server: McpServer) {
  server.tool(
    'wb_country',
    'Get detailed information about a specific country: name, region, income level, capital city, longitude/latitude, and lending type.',
    CountrySchema.shape,
    async (params) => {
      const { country } = CountrySchema.parse(params);
      const data = await wbFetch(
        `country/${encodeURIComponent(country)}`,
        {},
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'wb_countries',
    'List all countries in the World Bank database with their codes, regions, income levels, and capital cities. Paginated.',
    CountryListSchema.shape,
    async (params) => {
      const { per_page, page } = CountryListSchema.parse(params);
      const data = await wbFetch('country', { per_page, page }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'wb_country_indicators',
    'Get popular macroeconomic indicators for a country: GDP, GDP growth, inflation, unemployment, population, current account, government debt, exports, real interest rate, and exchange rate.',
    CountryPopularSchema.shape,
    async (params) => {
      const { country, date, per_page } = CountryPopularSchema.parse(params);
      const results: Record<string, unknown> = {};
      for (const indicator of POPULAR_INDICATORS) {
        const queryParams: Record<string, string | number> = { per_page };
        if (date) queryParams.date = date;
        try {
          const data = await wbFetch(
            `country/${encodeURIComponent(country)}/indicator/${encodeURIComponent(indicator)}`,
            queryParams,
            { cacheTtl: CacheTTL.SHORT },
          );
          results[indicator] = data;
        } catch {
          results[indicator] = { error: 'Failed to fetch' };
        }
      }
      return wrapResponse(results);
    },
  );

  server.tool(
    'wb_income_levels',
    'List World Bank income level classifications (e.g., High income, Upper middle income, Lower middle income, Low income). Returns codes and descriptions.',
    PaginationSchema.shape,
    async (params) => {
      const { per_page, page } = PaginationSchema.parse(params);
      const data = await wbFetch('incomelevel', { per_page, page }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );
}
