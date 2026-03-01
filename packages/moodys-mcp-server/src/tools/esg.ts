import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { moodysFetch, wrapResponse, CacheTTL } from '../client.js';
import {
  EsgScoreSchema,
  ClimateRiskSchema,
} from '../schemas/esg.js';

function resolveIssuerParams(params: { issuer_id?: string; ticker?: string; name?: string }): Record<string, string | undefined> {
  return {
    issuer_id: params.issuer_id,
    ticker: params.ticker,
    name: params.name,
  };
}

export function registerEsgTools(server: McpServer) {
  // 1. ESG score
  server.tool(
    'moodys_esg_score',
    'Get ESG credit impact score for an issuer from Moody\'s. Returns environmental, social, and governance scores with credit impact assessment (CIS-1 to CIS-5). Includes issuer-level E, S, G exposure scores and management quality. Use for ESG-integrated credit analysis.',
    EsgScoreSchema.shape,
    async (params) => {
      const parsed = EsgScoreSchema.parse(params);
      const data = await moodysFetch(
        'esg/v1/scores',
        resolveIssuerParams(parsed),
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );

  // 2. Climate risk
  server.tool(
    'moodys_climate_risk',
    'Get climate risk assessment (physical and transition) for an issuer from Moody\'s. Returns physical risk exposure (floods, heat stress, hurricanes, sea level rise), transition risk scores, and carbon intensity metrics under various climate scenarios. Use for climate-adjusted credit analysis.',
    ClimateRiskSchema.shape,
    async (params) => {
      const parsed = ClimateRiskSchema.parse(params);
      const data = await moodysFetch(
        'esg/v1/climate-risk',
        {
          ...resolveIssuerParams(parsed),
          scenario: parsed.scenario,
        },
        { cacheTtl: CacheTTL.LONG },
      );
      return wrapResponse(data);
    },
  );
}
