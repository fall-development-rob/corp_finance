import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { wbFetch, CacheTTL } from '../client.js';
import {
  CountryIndicatorSchema,
  MultiCountrySchema,
  MultiIndicatorSchema,
} from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerDataTools(server: McpServer) {
  server.tool(
    'wb_data_series',
    'Get a time series for one country and one indicator. Returns yearly values over the specified date range. Use for trend analysis.',
    CountryIndicatorSchema.shape,
    async (params) => {
      const { country, indicator, date, per_page, page } = CountryIndicatorSchema.parse(params);
      const queryParams: Record<string, string | number> = { per_page, page };
      if (date) queryParams.date = date;
      const data = await wbFetch(
        `country/${encodeURIComponent(country)}/indicator/${encodeURIComponent(indicator)}`,
        queryParams,
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'wb_multi_country',
    'Compare one indicator across multiple countries. Provide semicolon-separated country codes (e.g., US;GB;CN). Returns data for all countries.',
    MultiCountrySchema.shape,
    async (params) => {
      const { countries, indicator, date, per_page } = MultiCountrySchema.parse(params);
      const queryParams: Record<string, string | number> = { per_page };
      if (date) queryParams.date = date;
      const data = await wbFetch(
        `country/${encodeURIComponent(countries)}/indicator/${encodeURIComponent(indicator)}`,
        queryParams,
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'wb_time_series',
    'Get multiple indicators for one country. Provide semicolon-separated indicator codes. Returns data for all indicators combined.',
    MultiIndicatorSchema.shape,
    async (params) => {
      const { country, indicators, date, per_page } = MultiIndicatorSchema.parse(params);
      const queryParams: Record<string, string | number> = { per_page };
      if (date) queryParams.date = date;
      const data = await wbFetch(
        `country/${encodeURIComponent(country)}/indicator/${encodeURIComponent(indicators)}`,
        queryParams,
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );
}
