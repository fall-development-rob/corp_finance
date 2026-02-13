import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { fmpFetch, CacheTTL } from '../client.js';
import {
  EarningsSchema, EarningsCalendarSchema, EarningsTranscriptSchema,
  AnalystEstimatesSchema, PriceTargetSchema, GradesSchema,
} from '../schemas/earnings.js';

function wrapResponse(data: unknown) {
  return { content: [{ type: 'text' as const, text: JSON.stringify(data, null, 2) }] };
}

export function registerEarningsTools(server: McpServer) {
  server.tool(
    'fmp_earnings',
    'Get historical earnings data: actual EPS, estimated EPS, surprise, and revenue for each quarter. Use for earnings trend analysis.',
    EarningsSchema.shape,
    async (params) => {
      const { symbol, limit } = EarningsSchema.parse(params);
      const data = await fmpFetch('earnings', { symbol, limit }, { cacheTtl: CacheTTL.MEDIUM });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_earnings_calendar',
    'Get upcoming earnings announcement dates across all companies. Filter by date range. Use for event-driven analysis.',
    EarningsCalendarSchema.shape,
    async (params) => {
      const { from, to } = EarningsCalendarSchema.parse(params);
      const data = await fmpFetch('earnings-calendar', { from, to }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_earnings_transcript',
    'Get full earnings call transcript for a specific company, year, and quarter. Contains management commentary, guidance, and Q&A.',
    EarningsTranscriptSchema.shape,
    async (params) => {
      const { symbol, year, quarter } = EarningsTranscriptSchema.parse(params);
      const data = await fmpFetch('earning-call-transcript', { symbol, year, quarter }, { cacheTtl: CacheTTL.LONG });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_analyst_estimates',
    'Get analyst consensus estimates: revenue, EBITDA, EPS, net income estimates for future periods. Annual or quarterly.',
    AnalystEstimatesSchema.shape,
    async (params) => {
      const { symbol, period, limit } = AnalystEstimatesSchema.parse(params);
      const data = await fmpFetch('analyst-estimates', { symbol, period, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_price_target',
    'Get analyst price target summary: average, median, high, low targets and number of analysts. Use for consensus valuation.',
    PriceTargetSchema.shape,
    async (params) => {
      const { symbol } = PriceTargetSchema.parse(params);
      const data = await fmpFetch('price-target-summary', { symbol }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );

  server.tool(
    'fmp_grades',
    'Get analyst grades and recommendations history: buy/sell/hold ratings from major brokerages with date and previous grade.',
    GradesSchema.shape,
    async (params) => {
      const { symbol, limit } = GradesSchema.parse(params);
      const data = await fmpFetch('grades', { symbol, limit }, { cacheTtl: CacheTTL.SHORT });
      return wrapResponse(data);
    },
  );
}
