import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateHModelDdm,
  calculateMultistageDdm,
  analyzeBuyback,
  analyzePayoutSustainability,
  calculateTotalShareholderReturn,
} from "corp-finance-bindings";
import {
  HModelDdmSchema,
  MultistageDdmSchema,
  BuybackAnalysisSchema,
  PayoutSustainabilitySchema,
  TotalShareholderReturnSchema,
} from "../schemas/dividend_policy.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerDividendPolicyTools(server: McpServer) {
  server.tool(
    "h_model_ddm",
    "H-Model DDM: Fuller & Hsia dividend valuation with linearly declining growth from short-term to long-term rate over half-life",
    HModelDdmSchema.shape,
    async (params) => {
      const validated = HModelDdmSchema.parse(coerceNumbers(params));
      const result = calculateHModelDdm(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "multistage_ddm",
    "Multi-stage DDM: N-stage dividend discount model with explicit growth periods and terminal Gordon Growth value",
    MultistageDdmSchema.shape,
    async (params) => {
      const validated = MultistageDdmSchema.parse(coerceNumbers(params));
      const result = calculateMultistageDdm(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "buyback_analysis",
    "Share buyback analysis: EPS accretion/dilution, P/E breakeven, tax efficiency vs dividends, debt-funded vs cash-funded comparison",
    BuybackAnalysisSchema.shape,
    async (params) => {
      const validated = BuybackAnalysisSchema.parse(coerceNumbers(params));
      const result = analyzeBuyback(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "payout_sustainability",
    "Payout sustainability analysis: payout ratio, FCF coverage, debt capacity, dividend safety score, Lintner smoothing model",
    PayoutSustainabilitySchema.shape,
    async (params) => {
      const validated = PayoutSustainabilitySchema.parse(coerceNumbers(params));
      const result = analyzePayoutSustainability(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "total_shareholder_return",
    "Total shareholder return: price appreciation, dividend yield, buyback yield, annualized TSR, component attribution",
    TotalShareholderReturnSchema.shape,
    async (params) => {
      const validated = TotalShareholderReturnSchema.parse(coerceNumbers(params));
      const result = calculateTotalShareholderReturn(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
