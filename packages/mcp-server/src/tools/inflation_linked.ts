import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeTips,
  analyzeInflationDerivatives,
} from "corp-finance-bindings";
import {
  TipsAnalyticsSchema,
  InflationDerivativeSchema,
} from "../schemas/inflation_linked.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerInflationLinkedTools(server: McpServer) {
  server.tool(
    "tips_analytics",
    "TIPS/inflation-linked bond analytics: CPI-adjusted pricing (real/nominal clean and dirty), breakeven inflation analysis (Fisher equation, term structure, forward breakeven), real yield curve analysis, deflation floor valuation, projected inflation-adjusted cashflows",
    TipsAnalyticsSchema.shape,
    async (params) => {
      const validated = TipsAnalyticsSchema.parse(coerceNumbers(params));
      const result = analyzeTips(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "inflation_derivatives",
    "Inflation derivative pricing: zero-coupon inflation swap (ZCIS), year-on-year inflation swap (YYIS), inflation cap/floor (Black model). Outputs fair swap rates, leg PVs, NPV, caplet/floorlet decomposition, Greeks (delta/vega), put-call parity consistency",
    InflationDerivativeSchema.shape,
    async (params) => {
      const validated = InflationDerivativeSchema.parse(coerceNumbers(params));
      const result = analyzeInflationDerivatives(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
