import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { wbFetch, CacheTTL } from '../client.js';
import {
  CountryIndicatorSchema,
  IndicatorSearchSchema,
  IndicatorInfoSchema,
  PaginationSchema,
} from '../schemas/common.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerIndicatorTools(server: McpServer) {
  server.tool(
    'wb_indicator',
    'Get World Bank indicator data for a country. Returns time series of values for indicators like GDP (NY.GDP.MKTP.CD), inflation (FP.CPI.TOTL.ZG), population (SP.POP.TOTL), etc.',
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
    'wb_indicator_search',
    'Search World Bank indicators by keyword. Returns matching indicator codes, names, and descriptions. Use to discover available indicators before querying data.',
    IndicatorSearchSchema.shape,
    async (params) => {
      const { query, per_page, page } = IndicatorSearchSchema.parse(params);
      const data = await wbFetch(
        'indicator',
        { searchvalue: query, per_page, page },
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'wb_indicator_info',
    'Get metadata for a specific World Bank indicator: name, description, source, unit, and topic classification.',
    IndicatorInfoSchema.shape,
    async (params) => {
      const { indicator } = IndicatorInfoSchema.parse(params);
      const data = await wbFetch(
        `indicator/${encodeURIComponent(indicator)}`,
        {},
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  server.tool(
    'wb_indicator_sources',
    'List all World Bank data sources (e.g., WDI, IDS, Doing Business). Returns source IDs, names, and descriptions.',
    PaginationSchema.shape,
    async (params) => {
      const { per_page, page } = PaginationSchema.parse(params);
      const data = await wbFetch('source', { per_page, page }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'wb_topics',
    'List all World Bank topic categories (e.g., Agriculture, Education, Health, Trade). Returns topic IDs, names, and notes.',
    PaginationSchema.shape,
    async (params) => {
      const { per_page, page } = PaginationSchema.parse(params);
      const data = await wbFetch('topic', { per_page, page }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
