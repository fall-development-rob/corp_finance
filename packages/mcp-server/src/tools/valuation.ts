import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { calculateWacc, buildDcf, compsAnalysis } from "@rob-otixai/corp-finance-bindings";
import { WaccSchema, DcfSchema, CompsSchema } from "../schemas/valuation.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerValuationTools(server: McpServer) {
  server.tool(
    "wacc_calculator",
    "Calculate weighted average cost of capital (WACC) using CAPM. Returns cost of equity, after-tax cost of debt, and blended WACC. Supports size premium, country risk premium, and beta re-levering.",
    WaccSchema.shape,
    async (params) => {
      const validated = WaccSchema.parse(coerceNumbers(params));
      const result = calculateWacc(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "dcf_model",
    "Build a discounted cash flow (DCF) model using FCFF methodology. Projects revenue, EBITDA, and free cash flow, then discounts to present value. Supports Gordon Growth and/or exit multiple terminal value. Returns enterprise value, equity value, and per-share value with full year-by-year projections.",
    DcfSchema.shape,
    async (params) => {
      const validated = DcfSchema.parse(coerceNumbers(params));
      const result = buildDcf(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "comps_analysis",
    "Perform trading comparables (comps) analysis. Calculates valuation multiples (EV/EBITDA, EV/Revenue, P/E, P/B, PEG) across a peer set, computes mean/median/high/low statistics, and derives implied valuations for the target company.",
    CompsSchema.shape,
    async (params) => {
      const validated = CompsSchema.parse(coerceNumbers(params));
      const result = compsAnalysis(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
