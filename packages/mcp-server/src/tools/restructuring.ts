import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { analyzeRecovery, analyzeDistressedDebt } from "@rob-otixai/corp-finance-bindings";
import { RecoverySchema, DistressedDebtSchema } from "../schemas/restructuring.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerRestructuringTools(server: McpServer) {
  server.tool(
    "recovery_analysis",
    "Run a restructuring recovery analysis using the Absolute Priority Rule (APR). Distributes enterprise value or liquidation value through the capital structure, paying each priority class in full before moving to the next. Returns per-claim recoveries (cents on the dollar), fulcrum security identification, going-concern vs liquidation comparison, DIP facility analysis, and shortfall calculation.",
    RecoverySchema.shape,
    async (params) => {
      const validated = RecoverySchema.parse(coerceNumbers(params));
      const result = analyzeRecovery(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "distressed_debt_analysis",
    "Analyze a distressed debt situation and restructuring plan. Performs a claims waterfall against exit enterprise value, identifies the fulcrum security, calculates per-tranche recoveries and implied IRRs at market prices, evaluates credit bid opportunity, equity value creation, and DIP facility economics. Supports Reinstate, Amend, Exchange, Equity Conversion, Cash Paydown, and Combination treatments.",
    DistressedDebtSchema.shape,
    async (params) => {
      const validated = DistressedDebtSchema.parse(coerceNumbers(params));
      const result = analyzeDistressedDebt(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
