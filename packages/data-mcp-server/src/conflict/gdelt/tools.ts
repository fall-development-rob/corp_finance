import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { z } from 'zod';
import { gdeltFetch, CacheTTL } from './client.js';
import { wrapResponse } from '../../shared/types.js';

// ---------- Schemas ----------

const GdeltEventsSchema = z.object({
  query: z.string().describe('Search query — country name, topic, or keyword (e.g., "Ukraine conflict")'),
  timespan: z.string().default('7d').describe('Lookback window: e.g., "24h", "7d", "30d" (default "7d")'),
  maxrecords: z.number().min(1).max(250).default(50).describe('Max articles to return (default 50)'),
});

const GdeltToneSchema = z.object({
  query: z.string().describe('Search query — country name or topic'),
  timespan: z.string().default('7d').describe('Lookback window: e.g., "24h", "7d", "30d"'),
});

const GdeltTensionSchema = z.object({
  country_a: z.string().describe('First country name (e.g., "Russia")'),
  country_b: z.string().describe('Second country name (e.g., "Ukraine")'),
  timespan: z.string().default('30d').describe('Lookback window (default "30d")'),
});

// ---------- GDELT response typings ----------

interface GdeltArticle {
  url: string;
  title: string;
  seendate: string;
  socialimage: string;
  domain: string;
  language: string;
  sourcecountry: string;
  tone: number;
  [key: string]: unknown;
}

interface GdeltArtListResponse {
  articles: GdeltArticle[];
}

interface GdeltTimelineEntry {
  date: string;
  value: number;
}

interface GdeltTimelineResponse {
  timeline: Array<{
    series: string;
    data: GdeltTimelineEntry[];
  }>;
}

// ---------- Tool registration ----------

export function registerGdeltTools(server: McpServer) {
  server.tool(
    'gdelt_events',
    'Search GDELT for recent news articles by query and timespan. Returns article title, URL, source, date, language, domain, and tone score (negative=hostile, positive=cooperative).',
    GdeltEventsSchema.shape,
    async (params) => {
      const { query, timespan, maxrecords } = GdeltEventsSchema.parse(params);

      const raw = await gdeltFetch<GdeltArtListResponse>(
        {
          query,
          mode: 'ArtList',
          maxrecords,
          timespan,
        },
        { cacheTtl: CacheTTL.SHORT },
      );

      const articles = (raw.articles ?? []).map((a) => ({
        title: a.title,
        url: a.url,
        source: a.domain,
        seendate: a.seendate,
        language: a.language,
        domain: a.domain,
        tone: a.tone,
      }));

      return wrapResponse({
        query,
        timespan,
        count: articles.length,
        articles,
      });
    },
  );

  server.tool(
    'gdelt_tone',
    'Average tone analysis from GDELT. Returns daily tone scores for a query over a timespan. Negative tone = hostile/negative coverage, positive tone = cooperative/positive. Useful for tracking sentiment trends.',
    GdeltToneSchema.shape,
    async (params) => {
      const { query, timespan } = GdeltToneSchema.parse(params);

      const raw = await gdeltFetch<GdeltTimelineResponse>(
        {
          query,
          mode: 'TimelineTone',
          timespan,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );

      const series = raw.timeline?.[0]?.data ?? [];
      const tonePoints = series.map((p) => ({
        date: p.date,
        tone: p.value,
      }));

      // Compute summary statistics
      const tones = tonePoints.map((p) => p.tone);
      const avgTone = tones.length > 0
        ? tones.reduce((s, t) => s + t, 0) / tones.length
        : 0;
      const minTone = tones.length > 0 ? Math.min(...tones) : 0;
      const maxTone = tones.length > 0 ? Math.max(...tones) : 0;

      // Trend: compare first half vs second half
      let trendDirection: 'improving' | 'worsening' | 'stable' = 'stable';
      if (tones.length >= 4) {
        const mid = Math.floor(tones.length / 2);
        const firstHalf = tones.slice(0, mid).reduce((s, t) => s + t, 0) / mid;
        const secondHalf = tones.slice(mid).reduce((s, t) => s + t, 0) / (tones.length - mid);
        const diff = secondHalf - firstHalf;
        if (diff > 0.5) trendDirection = 'improving';
        else if (diff < -0.5) trendDirection = 'worsening';
      }

      return wrapResponse({
        query,
        timespan,
        data_points: tonePoints.length,
        average_tone: Math.round(avgTone * 1000) / 1000,
        min_tone: Math.round(minTone * 1000) / 1000,
        max_tone: Math.round(maxTone * 1000) / 1000,
        trend_direction: trendDirection,
        timeline: tonePoints,
      });
    },
  );

  server.tool(
    'gdelt_country_tension',
    'Bilateral tension analysis between two countries. Queries GDELT for combined mentions, returns tone trend, volume trend, and a combined tension score. Higher tension score = more negative/hostile coverage with high volume.',
    GdeltTensionSchema.shape,
    async (params) => {
      const { country_a, country_b, timespan } = GdeltTensionSchema.parse(params);

      const combinedQuery = `"${country_a}" "${country_b}"`;

      // Fetch both tone and volume timelines
      const [toneRaw, volRaw] = await Promise.all([
        gdeltFetch<GdeltTimelineResponse>(
          { query: combinedQuery, mode: 'TimelineTone', timespan },
          { cacheTtl: CacheTTL.MEDIUM },
        ),
        gdeltFetch<GdeltTimelineResponse>(
          { query: combinedQuery, mode: 'TimelineVol', timespan },
          { cacheTtl: CacheTTL.MEDIUM },
        ),
      ]);

      const toneSeries = toneRaw.timeline?.[0]?.data ?? [];
      const volSeries = volRaw.timeline?.[0]?.data ?? [];

      const tonePoints = toneSeries.map((p) => ({ date: p.date, tone: p.value }));
      const volPoints = volSeries.map((p) => ({ date: p.date, volume: p.value }));

      // Compute tone statistics
      const tones = tonePoints.map((p) => p.tone);
      const avgTone = tones.length > 0
        ? tones.reduce((s, t) => s + t, 0) / tones.length
        : 0;

      // Compute volume statistics
      const volumes = volPoints.map((p) => p.volume);
      const avgVolume = volumes.length > 0
        ? volumes.reduce((s, v) => s + v, 0) / volumes.length
        : 0;
      const maxVolume = volumes.length > 0 ? Math.max(...volumes) : 0;

      // Tone trend (first half vs second half)
      let toneTrend: 'improving' | 'worsening' | 'stable' = 'stable';
      if (tones.length >= 4) {
        const mid = Math.floor(tones.length / 2);
        const first = tones.slice(0, mid).reduce((s, t) => s + t, 0) / mid;
        const second = tones.slice(mid).reduce((s, t) => s + t, 0) / (tones.length - mid);
        const diff = second - first;
        if (diff > 0.5) toneTrend = 'improving';
        else if (diff < -0.5) toneTrend = 'worsening';
      }

      // Volume trend
      let volumeTrend: 'increasing' | 'decreasing' | 'stable' = 'stable';
      if (volumes.length >= 4) {
        const mid = Math.floor(volumes.length / 2);
        const first = volumes.slice(0, mid).reduce((s, v) => s + v, 0) / mid;
        const second = volumes.slice(mid).reduce((s, v) => s + v, 0) / (volumes.length - mid);
        const ratio = first > 0 ? second / first : 1;
        if (ratio > 1.2) volumeTrend = 'increasing';
        else if (ratio < 0.8) volumeTrend = 'decreasing';
      }

      // Combined tension score: higher volume * more negative tone = higher tension
      // Normalize tone to [0, 1] range where 1 = most hostile (tone is typically -10 to +10)
      // Volume normalized by max volume in period
      const normalizedTone = Math.max(0, Math.min(1, ((-avgTone) + 10) / 20));
      const normalizedVolume = maxVolume > 0 ? avgVolume / maxVolume : 0;
      const tensionScore = Math.round(normalizedTone * normalizedVolume * 100 * 100) / 100;

      let tensionLevel: 'low' | 'moderate' | 'elevated' | 'high' | 'critical';
      if (tensionScore >= 80) tensionLevel = 'critical';
      else if (tensionScore >= 60) tensionLevel = 'high';
      else if (tensionScore >= 40) tensionLevel = 'elevated';
      else if (tensionScore >= 20) tensionLevel = 'moderate';
      else tensionLevel = 'low';

      return wrapResponse({
        country_a,
        country_b,
        timespan,
        average_tone: Math.round(avgTone * 1000) / 1000,
        average_volume: Math.round(avgVolume * 100) / 100,
        tone_trend: toneTrend,
        volume_trend: volumeTrend,
        tension_score: tensionScore,
        tension_level: tensionLevel,
        tone_timeline: tonePoints,
        volume_timeline: volPoints,
      });
    },
  );
}
