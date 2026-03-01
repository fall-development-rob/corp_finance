import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fredFetch, CacheTTL } from '../client.js';
import {
  CategorySchema,
  CategoryChildrenSchema,
  CategorySeriesSchema,
} from '../schemas/categories.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerCategoryTools(server: McpServer) {
  server.tool(
    'fred_category',
    'Get info for a FRED category by ID. Categories organize series hierarchically (0 = root). Returns category name and parent ID.',
    CategorySchema.shape,
    async (params) => {
      const { category_id } = CategorySchema.parse(params);
      const data = await fredFetch('category', { category_id }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_category_children',
    'Get child categories of a FRED category. Use to navigate the category tree and discover data organized by topic (e.g., Money/Banking, Production, Prices).',
    CategoryChildrenSchema.shape,
    async (params) => {
      const { category_id } = CategoryChildrenSchema.parse(params);
      const data = await fredFetch('category/children', { category_id }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_category_series',
    'Get all data series within a FRED category. Returns series with ID, title, frequency, units, and seasonal adjustment. Use to find available data in a topic area.',
    CategorySeriesSchema.shape,
    async (params) => {
      const { category_id, limit } = CategorySeriesSchema.parse(params);
      const data = await fredFetch('category/series', { category_id, limit }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
