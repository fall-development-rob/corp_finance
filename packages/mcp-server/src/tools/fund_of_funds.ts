import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateJCurve,
  calculateCommitmentPacing,
  analyzeManagerSelection,
  calculateSecondariesPricing,
  analyzeFofPortfolio,
} from "corp-finance-bindings";
import {
  JCurveSchema,
  CommitmentPacingSchema,
  ManagerSelectionSchema,
  SecondariesPricingSchema,
  FofPortfolioSchema,
} from "../schemas/fund_of_funds.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerFundOfFundsTools(server: McpServer) {
  server.tool(
    "j_curve_model",
    "J-curve fund lifecycle: cash flow projection, TVPI/DPI/RVPI, PME (Kaplan-Schoar), net/gross IRR, trough analysis",
    JCurveSchema.shape,
    async (params) => {
      const validated = JCurveSchema.parse(coerceNumbers(params));
      const result = calculateJCurve(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "commitment_pacing",
    "Commitment pacing: vintage year allocation, drawdown modeling, NAV projection, over-commitment ratio",
    CommitmentPacingSchema.shape,
    async (params) => {
      const validated = CommitmentPacingSchema.parse(coerceNumbers(params));
      const result = calculateCommitmentPacing(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "manager_selection",
    "Manager due diligence: performance scoring, persistence analysis, alpha estimation, qualitative rating",
    ManagerSelectionSchema.shape,
    async (params) => {
      const validated = ManagerSelectionSchema.parse(coerceNumbers(params));
      const result = analyzeManagerSelection(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "secondaries_pricing",
    "Secondaries pricing: NAV discount, unfunded PV, IRR sensitivity at multiple exit multiples, breakeven",
    SecondariesPricingSchema.shape,
    async (params) => {
      const validated = SecondariesPricingSchema.parse(coerceNumbers(params));
      const result = calculateSecondariesPricing(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "fof_portfolio",
    "Fund of funds portfolio: diversification by strategy/vintage/geography, HHI, constraint monitoring",
    FofPortfolioSchema.shape,
    async (params) => {
      const validated = FofPortfolioSchema.parse(coerceNumbers(params));
      const result = analyzeFofPortfolio(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
