import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzePrepayment,
  analyzeMbs,
} from "@fall-development-rob/corp-finance-bindings";
import {
  PrepaymentSchema,
  MbsAnalyticsSchema,
} from "../schemas/mortgage_analytics.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMortgageAnalyticsTools(server: McpServer) {
  server.tool(
    "prepayment_analysis",
    "Mortgage prepayment modeling: PSA (Public Securities Association ramp), constant CPR, refinancing incentive with burnout. Outputs CPR/SMM schedules, projected balances, prepayment amounts, weighted average life (WAL), expected maturity",
    PrepaymentSchema.shape,
    async (params) => {
      const validated = PrepaymentSchema.parse(coerceNumbers(params));
      const result = analyzePrepayment(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "mbs_analytics",
    "MBS pass-through analytics: cash flow projection with PSA prepayment, servicing fees, OAS/Z-spread analysis (bisection method), effective duration and convexity (parallel shift), Macaulay/modified duration, DV01, negative convexity detection, WAL, WAC",
    MbsAnalyticsSchema.shape,
    async (params) => {
      const validated = MbsAnalyticsSchema.parse(coerceNumbers(params));
      const result = analyzeMbs(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
