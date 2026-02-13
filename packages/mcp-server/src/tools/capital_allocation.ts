import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateEconomicCapital,
  calculateRaroc,
  calculateEulerAllocation,
  calculateShapleyAllocation,
  evaluateLimits,
} from "../bindings.js";
import {
  EconomicCapitalSchema,
  RarocSchema,
  EulerAllocationSchema,
  ShapleyAllocationSchema,
  LimitManagementSchema,
} from "../schemas/capital_allocation.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCapitalAllocationTools(server: McpServer) {
  server.tool(
    "economic_capital",
    "Economic capital: VaR/ES-based capital, IRB capital requirement (Basel), stress capital buffer, adequacy ratio",
    EconomicCapitalSchema.shape,
    async (params) => {
      const validated = EconomicCapitalSchema.parse(coerceNumbers(params));
      const result = calculateEconomicCapital(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "raroc_calculation",
    "RAROC: risk-adjusted return on capital, RORAC, EVA, SVA, spread to hurdle, risk-adjusted pricing",
    RarocSchema.shape,
    async (params) => {
      const validated = RarocSchema.parse(coerceNumbers(params));
      const result = calculateRaroc(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "euler_allocation",
    "Euler risk contribution: marginal capital allocation, diversification benefit, HHI concentration",
    EulerAllocationSchema.shape,
    async (params) => {
      const validated = EulerAllocationSchema.parse(coerceNumbers(params));
      const result = calculateEulerAllocation(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "shapley_allocation",
    "Shapley value capital allocation: game-theoretic fair allocation (exact N<=8, sampled N>8)",
    ShapleyAllocationSchema.shape,
    async (params) => {
      const validated = ShapleyAllocationSchema.parse(coerceNumbers(params));
      const result = calculateShapleyAllocation(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "limit_management",
    "Risk limit management: notional/VaR/concentration limits, utilization tracking, breach detection",
    LimitManagementSchema.shape,
    async (params) => {
      const validated = LimitManagementSchema.parse(coerceNumbers(params));
      const result = evaluateLimits(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
