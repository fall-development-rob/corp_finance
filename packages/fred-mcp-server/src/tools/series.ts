import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fredFetch, CacheTTL } from '../client.js';
import {
  SeriesObservationsSchema,
  SeriesInfoSchema,
  SeriesSearchSchema,
  SeriesCategoriesSchema,
  SeriesTagsSchema,
  SeriesVintageSchema,
} from '../schemas/series.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerSeriesTools(server: McpServer) {
  server.tool(
    'fred_series',
    'Get time-series observations for a FRED series. Returns date/value pairs for economic indicators like GDP, CPI, unemployment, interest rates, etc. Use for macroeconomic data analysis.',
    SeriesObservationsSchema.shape,
    async (params) => {
      const { series_id, observation_start, observation_end, frequency, limit } = SeriesObservationsSchema.parse(params);
      const data = await fredFetch('series/observations', {
        series_id,
        observation_start,
        observation_end,
        frequency,
        limit,
      }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_series_info',
    'Get metadata for a FRED series including title, units, frequency, seasonal adjustment, and date range. Use to understand what a series measures before pulling observations.',
    SeriesInfoSchema.shape,
    async (params) => {
      const { series_id } = SeriesInfoSchema.parse(params);
      const data = await fredFetch('series', { series_id }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_series_search',
    'Search FRED for economic data series by keyword. Returns matching series with ID, title, frequency, units, and popularity. Use to discover available data series.',
    SeriesSearchSchema.shape,
    async (params) => {
      const { search_text, limit } = SeriesSearchSchema.parse(params);
      const data = await fredFetch('series/search', { search_text, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_series_categories',
    'Get the categories that a FRED series belongs to. Useful for understanding classification and finding related series in the same category.',
    SeriesCategoriesSchema.shape,
    async (params) => {
      const { series_id } = SeriesCategoriesSchema.parse(params);
      const data = await fredFetch('series/categories', { series_id }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_series_tags',
    'Get the tags assigned to a FRED series. Tags describe attributes like geography, source, frequency, and topic. Use to find similar series.',
    SeriesTagsSchema.shape,
    async (params) => {
      const { series_id } = SeriesTagsSchema.parse(params);
      const data = await fredFetch('series/tags', { series_id }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_series_vintage',
    'Get vintage dates for a FRED series showing when data revisions were published. Essential for real-time analysis and understanding data revision history.',
    SeriesVintageSchema.shape,
    async (params) => {
      const { series_id, limit } = SeriesVintageSchema.parse(params);
      const data = await fredFetch('series/vintagedates', { series_id, limit }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
