import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  DividendsSchema, DividendsCalendarSchema,
  SplitsSchema, SplitsCalendarSchema,
  IpoCalendarSchema,
  EarningsTranscriptLatestSchema, EarningsTranscriptDatesSchema,
} from '../schemas/dividends.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

const EmptySchema = z.object({});

export function registerDividendTools(server: McpServer) {
  server.tool(
    'fmp_dividends',
    'Get historical dividend data for a stock: payment dates, ex-dates, amounts, and yield. Use for income analysis and dividend growth tracking.',
    DividendsSchema.shape,
    async (params) => {
      const { symbol } = DividendsSchema.parse(params);
      const data = await fmpFetch('dividends', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_dividends_calendar',
    'Get upcoming dividend calendar across all companies. Filter by date range to find ex-dates and payment dates. Use for income event planning.',
    DividendsCalendarSchema.shape,
    async (params) => {
      const { from, to } = DividendsCalendarSchema.parse(params);
      const data = await fmpFetch('dividends-calendar', { from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_splits',
    'Get historical stock split data for a company: split dates, ratios (e.g., 4:1), and types. Use for adjusting historical price analysis.',
    SplitsSchema.shape,
    async (params) => {
      const { symbol } = SplitsSchema.parse(params);
      const data = await fmpFetch('splits', { symbol }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_splits_calendar',
    'Get upcoming stock splits calendar across all companies. Filter by date range. Use for corporate action monitoring.',
    SplitsCalendarSchema.shape,
    async (params) => {
      const { from, to } = SplitsCalendarSchema.parse(params);
      const data = await fmpFetch('splits-calendar', { from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_ipo_calendar',
    'Get upcoming IPO calendar with expected pricing dates, price ranges, and share counts. Use for new listing event tracking.',
    IpoCalendarSchema.shape,
    async (params) => {
      const { from, to } = IpoCalendarSchema.parse(params);
      const data = await fmpFetch('ipos-calendar', { from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_ipo_disclosure',
    'Get IPO disclosure filings including S-1 and related SEC filings. Filter by date range for recent IPO documentation.',
    IpoCalendarSchema.shape,
    async (params) => {
      const { from, to } = IpoCalendarSchema.parse(params);
      const data = await fmpFetch('ipos-disclosure', { from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_ipo_prospectus',
    'Get IPO prospectus filings with offering details, underwriters, and use of proceeds. Filter by date range.',
    IpoCalendarSchema.shape,
    async (params) => {
      const { from, to } = IpoCalendarSchema.parse(params);
      const data = await fmpFetch('ipos-prospectus', { from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_earnings_transcript_latest',
    'Get the latest earnings call transcripts across all companies. Paginated results with full transcript text, management commentary, and Q&A.',
    EarningsTranscriptLatestSchema.shape,
    async (params) => {
      const { page, limit } = EarningsTranscriptLatestSchema.parse(params);
      const data = await fmpFetch('earning-call-transcript-latest', { page, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_earnings_transcript_dates',
    'Get available earnings transcript dates for a specific stock. Returns list of quarters and years with transcripts on file.',
    EarningsTranscriptDatesSchema.shape,
    async (params) => {
      const { symbol } = EarningsTranscriptDatesSchema.parse(params);
      const data = await fmpFetch('earning-call-transcript-dates', { symbol }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_earnings_transcript_list',
    'Get all stock symbols that have earnings call transcripts available. Use to check transcript coverage before requesting specific transcripts.',
    EmptySchema.shape,
    async () => {
      const data = await fmpFetch('earnings-transcript-list', {}, { cacheTtl: CacheTTL.STATIC });
      return wrapResponse(data);
    },
  );
}
