import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { modelAbsCashflows, analyzeTranching } from "@rob-otixai/corp-finance-bindings";
import { AbsMbsSchema, TranchingSchema } from "../schemas/securitization.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerSecuritizationTools(server: McpServer) {
  server.tool(
    "abs_cashflow_model",
    "Model ABS/MBS cash flows with prepayment and default projections. Supports CPR/PSA/SMM prepayment models and CDR/SDA default models. Projects monthly cash flows including scheduled principal & interest, prepayments, defaults, losses, recoveries, and servicing fees. Returns period-by-period detail plus summary statistics (WAL, cumulative loss rate, pool factor).",
    AbsMbsSchema.shape,
    async (params) => {
      const validated = AbsMbsSchema.parse(coerceNumbers(params));
      const result = modelAbsCashflows(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "tranching_analysis",
    "Analyse a CDO/CLO tranching structure with waterfall distribution. Runs the full sequential/turbo waterfall for each period of collateral cash flows, applying OC/IC tests, loss allocation bottom-up, and reinvestment logic. Returns per-tranche results (IRR, WAL, credit enhancement), waterfall period detail, and deal summary metrics.",
    TranchingSchema.shape,
    async (params) => {
      const validated = TranchingSchema.parse(coerceNumbers(params));
      const result = analyzeTranching(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
