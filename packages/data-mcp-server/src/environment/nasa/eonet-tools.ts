import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { eonetFetch, CacheTTL } from './eonet-client.js';
import { wrapResponse } from '../../shared/types.js';

// --- EONET response types ---

interface EonetSource {
  id: string;
  url: string;
}

interface EonetGeometry {
  magnitudeValue?: number;
  magnitudeUnit?: string;
  date: string;
  type: string;
  coordinates: number[];
}

interface EonetCategory {
  id: string;
  title: string;
  description?: string;
}

interface EonetEvent {
  id: string;
  title: string;
  description?: string;
  link?: string;
  closed?: string | null;
  categories: EonetCategory[];
  sources: EonetSource[];
  geometry: EonetGeometry[];
}

interface EonetEventsResponse {
  title: string;
  description: string;
  link: string;
  events: EonetEvent[];
}

interface EonetCategoriesResponse {
  title: string;
  description: string;
  link: string;
  categories: EonetCategory[];
}

// --- helpers ---

function toEventRecord(event: EonetEvent) {
  const latestGeom = event.geometry.length > 0
    ? event.geometry[event.geometry.length - 1]
    : null;

  return {
    id: event.id,
    title: event.title,
    category: event.categories.length > 0
      ? event.categories[0].title
      : 'Unknown',
    sources: event.sources.map(s => ({
      id: s.id,
      url: s.url,
    })),
    geometries: event.geometry.map(g => ({
      date: g.date,
      latitude: g.coordinates.length >= 2 ? g.coordinates[1] : null,
      longitude: g.coordinates.length >= 1 ? g.coordinates[0] : null,
      magnitude_value: g.magnitudeValue ?? null,
      magnitude_unit: g.magnitudeUnit ?? null,
    })),
    date: latestGeom?.date ?? null,
    closed: event.closed ?? null,
  };
}

// --- Zod schemas ---

const EonetEventsSchema = z.object({
  days: z
    .number()
    .int()
    .min(1)
    .max(365)
    .default(30)
    .describe('Number of days to look back (default 30)'),
  status: z
    .enum(['open', 'closed'])
    .default('open')
    .describe('Event status filter (default open)'),
});

// --- tool registration ---

export function registerEonetTools(server: McpServer) {
  // 1. eonet_events — Current EONET events
  server.tool(
    'eonet_events',
    'Current NASA EONET natural events. Optional filters: days (default 30), status (open/closed). Returns id, title, category, sources, geometries (lat/lon), date.',
    EonetEventsSchema.shape,
    async (params) => {
      const { days, status } = EonetEventsSchema.parse(params);
      const data = await eonetFetch<EonetEventsResponse>(
        'events',
        { days, status },
        { cacheTtl: CacheTTL.MEDIUM },
      );

      const events = (data.events ?? []).map(toEventRecord);

      return wrapResponse({
        count: events.length,
        events,
      });
    },
  );

  // 2. eonet_categories — List EONET categories with event counts
  server.tool(
    'eonet_categories',
    'List NASA EONET event categories with descriptions and active event counts. Returns category_id, title, description, count.',
    {},
    async () => {
      // Fetch categories
      const catData = await eonetFetch<EonetCategoriesResponse>(
        'categories',
        {},
        { cacheTtl: CacheTTL.LONG },
      );

      // Fetch current open events to count per category
      const eventsData = await eonetFetch<EonetEventsResponse>(
        'events',
        { status: 'open', days: 365 },
        { cacheTtl: CacheTTL.MEDIUM },
      );

      // Count events per category
      const countMap: Record<string, number> = {};
      for (const event of (eventsData.events ?? [])) {
        for (const cat of event.categories) {
          countMap[cat.id] = (countMap[cat.id] ?? 0) + 1;
        }
      }

      const categories = (catData.categories ?? []).map(cat => ({
        category_id: cat.id,
        title: cat.title,
        description: cat.description ?? null,
        count: countMap[cat.id] ?? 0,
      }));

      return wrapResponse({
        total_categories: categories.length,
        categories,
      });
    },
  );
}
