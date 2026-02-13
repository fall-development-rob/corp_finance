import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeCashManagement,
  analyzeHedging,
} from "../bindings.js";
import {
  CashManagementSchema,
  HedgingSchema,
} from "../schemas/treasury.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerTreasuryTools(server: McpServer) {
  server.tool(
    "cash_management",
    "Corporate cash management analysis: liquidity forecasting, investment policy, cash pooling, working capital optimization",
    CashManagementSchema.shape,
    async (params) => {
      const validated = CashManagementSchema.parse(coerceNumbers(params));
      const result = analyzeCashManagement(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "hedge_effectiveness",
    "Hedge effectiveness testing: dollar offset, regression analysis, hypothetical derivative method, prospective/retrospective",
    HedgingSchema.shape,
    async (params) => {
      const validated = HedgingSchema.parse(coerceNumbers(params));
      const result = analyzeHedging(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
