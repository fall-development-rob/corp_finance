import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculatePortfolioCreditRisk,
  calculateMigration,
} from "corp-finance-bindings";
import {
  PortfolioRiskSchema,
  MigrationSchema,
} from "../schemas/credit_portfolio.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCreditPortfolioTools(server: McpServer) {
  server.tool(
    "portfolio_credit_risk",
    "Portfolio credit risk analysis: Gaussian copula VaR, HHI concentration, granularity adjustment, marginal risk contribution",
    PortfolioRiskSchema.shape,
    async (params) => {
      const validated = PortfolioRiskSchema.parse(coerceNumbers(params));
      const result = calculatePortfolioCreditRisk(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "credit_migration",
    "Rating migration analysis: transition matrix exponentiation, multi-year default probabilities, mark-to-market migration VaR",
    MigrationSchema.shape,
    async (params) => {
      const validated = MigrationSchema.parse(coerceNumbers(params));
      const result = calculateMigration(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
