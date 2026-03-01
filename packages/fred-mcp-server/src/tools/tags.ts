import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fredFetch, CacheTTL } from '../client.js';
import {
  TagsSchema,
  RelatedTagsSchema,
  TagsSeriesSchema,
} from '../schemas/tags.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerTagTools(server: McpServer) {
  server.tool(
    'fred_tags',
    'Get all FRED tags with names, group IDs, and series counts. Tags describe attributes like geography, source, frequency. Use to discover data dimensions.',
    TagsSchema.shape,
    async (params) => {
      const { limit, offset, search_text } = TagsSchema.parse(params);
      const data = await fredFetch('tags', { limit, offset, search_text }, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_related_tags',
    'Get tags related to given tag names. Use to discover what other attributes co-occur with specific tags for narrowing data searches.',
    RelatedTagsSchema.shape,
    async (params) => {
      const { tag_names, limit } = RelatedTagsSchema.parse(params);
      const data = await fredFetch('related_tags', { tag_names, limit }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_series_match_tags',
    'Get FRED series matching specific tag names. Use to find all series with particular attributes (e.g., all monthly CPI series or all quarterly GDP series).',
    TagsSeriesSchema.shape,
    async (params) => {
      const { tag_names, limit } = TagsSeriesSchema.parse(params);
      const data = await fredFetch('tags/series', { tag_names, limit }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
