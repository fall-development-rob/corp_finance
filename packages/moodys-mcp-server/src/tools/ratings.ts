import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { moodysFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  CreditRatingSchema,
  RatingHistorySchema,
  IssuerProfileSchema,
} from '../schemas/ratings.js';

function resolveIssuerParams(params: { issuer_id?: string; ticker?: string; name?: string }): Record<string, string | undefined> {
  return {
    issuer_id: params.issuer_id,
    ticker: params.ticker,
    name: params.name,
  };
}

export function registerRatingsTools(server: McpServer) {
  // 1. Credit rating
  server.tool(
    'moodys_credit_rating',
    'Get Moody\'s credit rating and outlook for an issuer. Returns the current long-term rating, short-term rating, outlook (stable/positive/negative), and review status. Use for credit analysis and counterparty risk assessment.',
    CreditRatingSchema.shape,
    async (params) => {
      const parsed = CreditRatingSchema.parse(params);
      const data = await moodysFetch(
        'ratings/v1/credit-ratings',
        resolveIssuerParams(parsed),
        { cacheTtl: CacheTTL.SHORT },
      );
      return wrapResponse(data);
    },
  );

  // 2. Rating history
  server.tool(
    'moodys_rating_history',
    'Get historical rating changes with action dates for an issuer. Returns chronological list of upgrades, downgrades, and outlook changes with effective dates and rationale. Use for credit trend analysis.',
    RatingHistorySchema.shape,
    async (params) => {
      const parsed = RatingHistorySchema.parse(params);
      const data = await moodysFetch(
        'ratings/v1/rating-history',
        {
          ...resolveIssuerParams(parsed),
          start_date: parsed.start_date,
          end_date: parsed.end_date,
        },
        { cacheTtl: CacheTTL.MEDIUM },
      );
      return wrapResponse(data);
    },
  );

  // 3. Issuer profile
  server.tool(
    'moodys_issuer_profile',
    'Get issuer profile with key credit metrics from Moody\'s. Returns company overview, sector classification, financial summary, peer group, and key credit factors. Use for fundamental credit research.',
    IssuerProfileSchema.shape,
    async (params) => {
      const parsed = IssuerProfileSchema.parse(params);
      const data = await moodysFetch(
        'ratings/v1/issuer-profile',
        resolveIssuerParams(parsed),
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );
}
