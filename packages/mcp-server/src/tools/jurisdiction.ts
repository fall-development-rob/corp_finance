import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { calculateFundFees } from "corp-finance-bindings";
import { FundFeeSchema } from "../schemas/jurisdiction.js";
import { wrapResponse } from "../formatters/response.js";

export function registerJurisdictionTools(server: McpServer) {
  server.tool(
    "fund_fee_calculator",
    "Model fund economics including management fees (committed/invested/NAV basis), performance fees with hurdle rates and catch-up, European and American waterfall structures, GP co-investment returns, and LP net return analysis. Calculates fee drag, DPI, RVPI, TVPI projections across fund life.",
    FundFeeSchema.shape,
    async (params) => {
      const validated = FundFeeSchema.parse(params);
      const result = calculateFundFees(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
