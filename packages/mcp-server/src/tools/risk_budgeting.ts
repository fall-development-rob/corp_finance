import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeFactorRiskBudget,
  analyzeTailRisk,
} from "../bindings.js";
import {
  FactorRiskBudgetSchema,
  TailRiskSchema,
} from "../schemas/risk_budgeting.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerRiskBudgetingTools(server: McpServer) {
  server.tool(
    "factor_risk_budget",
    "Factor-based risk budgeting: per-factor risk contribution decomposition, factor exposure analysis, marginal risk, systematic vs idiosyncratic risk breakdown, risk budget percentages, concentration analysis",
    FactorRiskBudgetSchema.shape,
    async (params) => {
      const validated = FactorRiskBudgetSchema.parse(coerceNumbers(params));
      const result = analyzeFactorRiskBudget(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "tail_risk_analysis",
    "Tail risk analysis: parametric/Cornish-Fisher/historical VaR and CVaR, marginal and component risk decomposition, stress testing across multiple scenarios, maximum drawdown estimation, tail dependence metrics",
    TailRiskSchema.shape,
    async (params) => {
      const validated = TailRiskSchema.parse(coerceNumbers(params));
      const result = analyzeTailRisk(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
