import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { valueProperty, modelProjectFinance } from "corp-finance-bindings";
import { PropertyValuationSchema, ProjectFinanceSchema } from "../schemas/real_assets.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerRealAssetsTools(server: McpServer) {
  server.tool(
    "property_valuation",
    "Value a property using direct capitalisation (NOI / cap rate), discounted cash flow (projected NOI over holding period), and/or gross rent multiplier (from comparable sales). Calculates NOI, effective gross income, operating expense ratio, leveraged returns (LTV, DSCR, cash-on-cash, equity multiple), and recommended value range across methods used.",
    PropertyValuationSchema.shape,
    async (params) => {
      const validated = PropertyValuationSchema.parse(coerceNumbers(params));
      const result = valueProperty(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "project_finance_model",
    "Build a full project finance model for infrastructure, PPP, and energy projects. Models construction and operating phases, debt sculpting (level repayment, sculpted to target DSCR, or bullet maturity), DSRA contributions, and distribution waterfall. Computes project IRR, equity IRR, NPV, equity multiple, payback period, DSCR (min/avg), LLCR, PLCR, and year-by-year projections.",
    ProjectFinanceSchema.shape,
    async (params) => {
      const validated = ProjectFinanceSchema.parse(coerceNumbers(params));
      const result = modelProjectFinance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
