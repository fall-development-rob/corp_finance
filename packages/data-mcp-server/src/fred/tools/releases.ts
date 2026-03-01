import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fredFetch, CacheTTL } from '../client.js';
import {
  ReleasesSchema,
  ReleaseSchema,
  ReleaseDatesSchema,
  ReleaseSeriesSchema,
} from '../schemas/releases.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerReleaseTools(server: McpServer) {
  server.tool(
    'fred_releases',
    'Get all FRED data releases. Returns list of releases with ID, name, link, and press release flag. Use to discover available economic data releases.',
    ReleasesSchema.shape,
    async (params) => {
      const { limit, offset } = ReleasesSchema.parse(params);
      const data = await fredFetch('releases', { limit, offset }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_release',
    'Get details for a single FRED release by ID. Returns release name, link, notes, and press release info.',
    ReleaseSchema.shape,
    async (params) => {
      const { release_id } = ReleaseSchema.parse(params);
      const data = await fredFetch('release', { release_id }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_release_dates',
    'Get release dates for a specific FRED release. Shows historical and upcoming publication dates. Use for economic calendar and event-driven analysis.',
    ReleaseDatesSchema.shape,
    async (params) => {
      const { release_id, limit } = ReleaseDatesSchema.parse(params);
      const data = await fredFetch('release/dates', { release_id, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fred_release_series',
    'Get all data series associated with a FRED release. Use to find all series published as part of a specific economic report.',
    ReleaseSeriesSchema.shape,
    async (params) => {
      const { release_id, limit } = ReleaseSeriesSchema.parse(params);
      const data = await fredFetch('release/series', { release_id, limit }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );
}
