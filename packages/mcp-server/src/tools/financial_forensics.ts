import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeBenfordsLaw,
  calculateDupont,
  calculateZscoreModels,
  calculatePeerBenchmarking,
  calculateRedFlagScoring,
} from "../bindings.js";
import {
  BenfordsLawSchema,
  DupontSchema,
  ZScoreModelsSchema,
  PeerBenchmarkingSchema,
  RedFlagScoringSchema,
} from "../schemas/financial_forensics.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerFinancialForensicsTools(server: McpServer) {
  server.tool(
    "benfords_law",
    "Benford's Law analysis: first/second/first-two digit distribution test, chi-squared and MAD statistics, conformity assessment",
    BenfordsLawSchema.shape,
    async (params) => {
      const validated = BenfordsLawSchema.parse(coerceNumbers(params));
      const result = analyzeBenfordsLaw(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "dupont_analysis",
    "DuPont decomposition: 3-step and 5-step ROE breakdown (profit margin, asset turnover, leverage, tax burden, interest burden) with trend",
    DupontSchema.shape,
    async (params) => {
      const validated = DupontSchema.parse(coerceNumbers(params));
      const result = calculateDupont(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "zscore_models",
    "Z-Score models: Altman original/revised/private, Ohlson O-Score, Zmijewski, Springate with distress classification and composite score",
    ZScoreModelsSchema.shape,
    async (params) => {
      const validated = ZScoreModelsSchema.parse(coerceNumbers(params));
      const result = calculateZscoreModels(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "peer_benchmarking",
    "Peer benchmarking: percentile ranking, z-score normalization, composite scoring with direction-aware comparison across multiple metrics",
    PeerBenchmarkingSchema.shape,
    async (params) => {
      const validated = PeerBenchmarkingSchema.parse(coerceNumbers(params));
      const result = calculatePeerBenchmarking(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "red_flag_scoring",
    "Red flag scoring: composite fraud/distress risk from Beneish, Altman, Piotroski, financial ratios, and qualitative audit indicators",
    RedFlagScoringSchema.shape,
    async (params) => {
      const validated = RedFlagScoringSchema.parse(coerceNumbers(params));
      const result = calculateRedFlagScoring(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
