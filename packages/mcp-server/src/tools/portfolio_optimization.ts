import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  optimizeMeanVariance,
  optimizeBlackLittermanPortfolio,
} from "@robotixai/corp-finance-bindings";
import {
  MeanVarianceSchema,
  BlackLittermanPortfolioSchema,
} from "../schemas/portfolio_optimization.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerPortfolioOptimizationTools(server: McpServer) {
  server.tool(
    "mean_variance_optimization",
    "Markowitz mean-variance portfolio optimization: efficient frontier generation, tangency (max Sharpe) portfolio, global minimum variance portfolio, optimal weights with constraints (long-only, sector limits, min/max weights), diversification ratio, HHI concentration",
    MeanVarianceSchema.shape,
    async (params) => {
      const validated = MeanVarianceSchema.parse(coerceNumbers(params));
      const result = optimizeMeanVariance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "black_litterman_portfolio",
    "Black-Litterman portfolio optimization: implied equilibrium returns, investor views (absolute/relative), posterior return estimation, posterior covariance, optimal tilted weights, view contribution analysis, tracking error vs market, information ratio",
    BlackLittermanPortfolioSchema.shape,
    async (params) => {
      const validated = BlackLittermanPortfolioSchema.parse(coerceNumbers(params));
      const result = optimizeBlackLittermanPortfolio(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
