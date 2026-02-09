import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  creditMetrics,
  debtCapacity,
  covenantCompliance,
} from "corp-finance-bindings";
import {
  CreditMetricsSchema,
  DebtCapacitySchema,
  CovenantTestSchema,
} from "../schemas/credit.js";
import { wrapResponse } from "../formatters/response.js";

export function registerCreditTools(server: McpServer) {
  server.tool(
    "credit_metrics",
    "Calculate comprehensive credit metrics from financial data. Returns leverage ratios (Net Debt/EBITDA, Debt/Equity, Debt/Assets), coverage ratios (interest coverage, DSCR, fixed charge), cash flow metrics (FCF/Debt, OCF/Debt, FFO/Debt), liquidity ratios (current, quick), and a synthetic credit rating with rationale.",
    CreditMetricsSchema.shape,
    async (params) => {
      const validated = CreditMetricsSchema.parse(params);
      const result = creditMetrics(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "debt_capacity",
    "Size maximum debt capacity from EBITDA and constraint-based analysis. Tests leverage ceiling, minimum interest coverage, minimum DSCR, and minimum FFO/Debt constraints to find the binding constraint and maximum incremental debt.",
    DebtCapacitySchema.shape,
    async (params) => {
      const validated = DebtCapacitySchema.parse(params);
      const result = debtCapacity(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "covenant_compliance",
    "Test financial covenant compliance. Compares actual financial metrics against covenant thresholds (MaxOf for leverage ceilings, MinOf for coverage floors). Returns pass/fail status and headroom for each covenant.",
    CovenantTestSchema.shape,
    async (params) => {
      const validated = CovenantTestSchema.parse(params);
      const result = covenantCompliance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
