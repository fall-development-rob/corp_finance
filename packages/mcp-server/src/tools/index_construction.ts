import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateWeighting,
  calculateRebalancing,
  calculateTrackingError,
  calculateSmartBeta,
  calculateReconstitution,
} from "@rob-otixai/corp-finance-bindings";
import {
  WeightingSchema,
  RebalancingSchema,
  TrackingErrorSchema,
  SmartBetaSchema,
  ReconstitutionSchema,
} from "../schemas/index_construction.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerIndexConstructionTools(server: McpServer) {
  server.tool(
    "index_weighting",
    "Index weighting: market-cap, equal, fundamental, free-float weighting with cap constraints, HHI concentration, sector breakdown",
    WeightingSchema.shape,
    async (params) => {
      const validated = WeightingSchema.parse(coerceNumbers(params));
      const result = calculateWeighting(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "index_rebalancing",
    "Index rebalancing: drift analysis, optimal trade list, transaction cost estimation, turnover metrics, liquidity-adjusted scheduling",
    RebalancingSchema.shape,
    async (params) => {
      const validated = RebalancingSchema.parse(coerceNumbers(params));
      const result = calculateRebalancing(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "tracking_error",
    "Tracking error analysis: ex-post TE from returns, ex-ante TE from weights/covariance, active share, information ratio decomposition",
    TrackingErrorSchema.shape,
    async (params) => {
      const validated = TrackingErrorSchema.parse(coerceNumbers(params));
      const result = calculateTrackingError(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "smart_beta",
    "Smart beta construction: multi-factor tilted weights (value, momentum, quality, low-vol, dividend) with factor exposure and risk analysis",
    SmartBetaSchema.shape,
    async (params) => {
      const validated = SmartBetaSchema.parse(coerceNumbers(params));
      const result = calculateSmartBeta(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "index_reconstitution",
    "Index reconstitution: member eligibility screening, additions/deletions, buffer zone management, turnover estimation, impact analysis",
    ReconstitutionSchema.shape,
    async (params) => {
      const validated = ReconstitutionSchema.parse(coerceNumbers(params));
      const result = calculateReconstitution(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
