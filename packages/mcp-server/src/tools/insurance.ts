import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  estimateReserves,
  pricePremium,
  analyzeCombinedRatio,
  calculateScr,
} from "../bindings.js";
import {
  ReservingSchema,
  PremiumPricingSchema,
  CombinedRatioSchema,
  ScrSchema,
} from "../schemas/insurance.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerInsuranceTools(server: McpServer) {
  server.tool(
    "loss_reserving",
    "Estimate insurance loss reserves using Chain-Ladder and/or Bornhuetter-Ferguson methods. Computes age-to-age development factors, ultimate losses by accident year, IBNR reserves, tail factors, and optionally present-values reserves at a discount rate.",
    ReservingSchema.shape,
    async (params) => {
      const validated = ReservingSchema.parse(coerceNumbers(params));
      const result = estimateReserves(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "premium_pricing",
    "Calculate insurance premium using frequency x severity approach. Computes pure premium, trended pure premium (severity/frequency trends), gross premium with loading factors (expense ratio, profit margin, reinsurance, large loss, investment credit), rate component breakdown, and year-by-year projected experience.",
    PremiumPricingSchema.shape,
    async (params) => {
      const validated = PremiumPricingSchema.parse(coerceNumbers(params));
      const result = pricePremium(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "combined_ratio",
    "Analyse insurance combined ratio over multiple periods. Computes loss ratio, LAE ratio, expense ratio, dividend ratio, combined ratio, operating ratio, underwriting profit/loss, and net income per period. Provides summary statistics including averages, trend direction, best/worst years, and profitable year count.",
    CombinedRatioSchema.shape,
    async (params) => {
      const validated = CombinedRatioSchema.parse(coerceNumbers(params));
      const result = analyzeCombinedRatio(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "solvency_scr",
    "Calculate Solvency II Standard Formula Solvency Capital Requirement (SCR). Computes non-life underwriting SCR (premium + reserve risk with square-root correlation formula), catastrophe SCR, market/credit/operational SCR, diversification benefit, total SCR, MCR, solvency ratio, and surplus analysis.",
    ScrSchema.shape,
    async (params) => {
      const validated = ScrSchema.parse(coerceNumbers(params));
      const result = calculateScr(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
