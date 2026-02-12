import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  riskAdjustedReturns,
  riskMetrics,
  kellySizing,
} from "@robotixai/corp-finance-bindings";
import {
  RiskAdjustedSchema,
  RiskMetricsSchema,
  KellySchema,
} from "../schemas/portfolio.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerPortfolioTools(server: McpServer) {
  server.tool(
    "risk_adjusted_returns",
    "Calculate risk-adjusted return metrics: Sharpe ratio, Sortino ratio, Calmar ratio, Information ratio, and Treynor ratio. Accepts a return series with optional benchmark and risk-free rate. Annualises results based on observation frequency.",
    RiskAdjustedSchema.shape,
    async (params) => {
      const validated = RiskAdjustedSchema.parse(coerceNumbers(params));
      const result = riskAdjustedReturns(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "risk_metrics",
    "Calculate portfolio risk metrics: parametric and historical VaR, CVaR (Expected Shortfall), maximum drawdown, downside deviation, skewness, kurtosis. Optionally computes relative metrics (tracking error, beta, alpha, capture ratios) against a benchmark.",
    RiskMetricsSchema.shape,
    async (params) => {
      const validated = RiskMetricsSchema.parse(coerceNumbers(params));
      const result = riskMetrics(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "kelly_sizing",
    "Calculate optimal position size using the Kelly criterion. Returns full Kelly percentage, fractional Kelly recommendation, expected edge, and growth rate. Supports a hard cap on maximum position size.",
    KellySchema.shape,
    async (params) => {
      const validated = KellySchema.parse(coerceNumbers(params));
      const result = kellySizing(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
