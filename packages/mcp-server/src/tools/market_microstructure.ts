import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeSpreads,
  optimizeExecution,
} from "../bindings.js";
import {
  SpreadAnalysisSchema,
  OptimalExecutionSchema,
} from "../schemas/market_microstructure.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMarketMicrostructureTools(server: McpServer) {
  server.tool(
    "spread_analysis",
    "Bid-ask spread decomposition and market quality analysis: quoted/effective/realized spreads, adverse selection component (Kyle lambda), inventory risk, information share, market quality score, price impact estimation",
    SpreadAnalysisSchema.shape,
    async (params) => {
      const validated = SpreadAnalysisSchema.parse(coerceNumbers(params));
      const result = analyzeSpreads(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "optimal_execution",
    "Optimal trade execution: Almgren-Chriss framework, TWAP/VWAP/IS strategies, execution cost estimation (market impact, timing risk, opportunity cost), optimal trajectory, adaptive scheduling, implementation shortfall decomposition",
    OptimalExecutionSchema.shape,
    async (params) => {
      const validated = OptimalExecutionSchema.parse(coerceNumbers(params));
      const result = optimizeExecution(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
