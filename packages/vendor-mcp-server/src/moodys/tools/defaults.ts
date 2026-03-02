import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { moodysFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  DefaultRatesSchema,
  RecoveryRatesSchema,
  TransitionMatrixSchema,
} from '../schemas/defaults.js';

export function registerDefaultsTools(server: McpServer) {
  // 1. Default rates
  server.tool(
    'moodys_default_rates',
    'Get historical default rates by rating and sector from Moody\'s. Returns annualized default frequencies across rating categories and time horizons. Use for PD estimation, credit portfolio modeling, and regulatory capital.',
    DefaultRatesSchema.shape,
    async (params) => {
      const parsed = DefaultRatesSchema.parse(params);
      const data = await moodysFetch(
        'research/v1/default-rates',
        {
          rating: parsed.rating,
          sector: parsed.sector,
          horizon: parsed.horizon,
          start_date: parsed.start_date,
          end_date: parsed.end_date,
        },
        { cacheTtl: CacheTTL.STATIC },
      );
      return wrapResponse(data);
    },
  );

  // 2. Recovery rates
  server.tool(
    'moodys_recovery_rates',
    'Get recovery rate data by seniority and sector from Moody\'s. Returns post-default recovery statistics including mean, median, and distribution by debt class. Use for LGD estimation and credit loss modeling.',
    RecoveryRatesSchema.shape,
    async (params) => {
      const parsed = RecoveryRatesSchema.parse(params);
      const data = await moodysFetch(
        'research/v1/recovery-rates',
        {
          seniority: parsed.seniority,
          sector: parsed.sector,
          start_date: parsed.start_date,
          end_date: parsed.end_date,
        },
        { cacheTtl: CacheTTL.STATIC },
      );
      return wrapResponse(data);
    },
  );

  // 3. Transition matrix
  server.tool(
    'moodys_transition_matrix',
    'Get rating transition probability matrix from Moody\'s. Returns probabilities of migrating from one rating to another over a specified horizon. Use for credit migration analysis, portfolio risk, and expected loss calculations.',
    TransitionMatrixSchema.shape,
    async (params) => {
      const parsed = TransitionMatrixSchema.parse(params);
      const data = await moodysFetch(
        'research/v1/transition-matrix',
        {
          from_rating: parsed.from_rating,
          horizon: parsed.horizon,
        },
        { cacheTtl: CacheTTL.STATIC },
      );
      return wrapResponse(data);
    },
  );
}
