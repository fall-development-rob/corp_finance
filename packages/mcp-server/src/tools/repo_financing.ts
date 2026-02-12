import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeRepo,
  analyzeCollateral,
} from "@robotixai/corp-finance-bindings";
import {
  RepoAnalyticsSchema,
  CollateralSchema,
} from "../schemas/repo_financing.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerRepoFinancingTools(server: McpServer) {
  server.tool(
    "repo_analytics",
    "Repo rate and securities lending analytics: repo rate calculation (haircut, margin, forward price), implied repo rate (carry analysis, basis), term structure (interpolated curve, forward repo rates, specialness premium), securities lending economics (fee income, reinvestment, intrinsic value)",
    RepoAnalyticsSchema.shape,
    async (params) => {
      const validated = RepoAnalyticsSchema.parse(coerceNumbers(params));
      const result = analyzeRepo(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "collateral_analytics",
    "Collateral management analytics: risk-based haircut calculation (credit/maturity/volatility/liquidity/FX adjustments), margin call analysis (trigger detection, call amount, LTV, coverage ratio), rehypothecation analysis (funding benefit, collateral velocity, counterparty exposure, regulatory compliance)",
    CollateralSchema.shape,
    async (params) => {
      const validated = CollateralSchema.parse(coerceNumbers(params));
      const result = analyzeCollateral(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
