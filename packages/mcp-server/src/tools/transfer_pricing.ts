import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeBepsCompliance,
  analyzeIntercompany,
} from "@rob-otixai/corp-finance-bindings";
import {
  BepsSchema,
  IntercompanySchema,
} from "../schemas/transfer_pricing.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerTransferPricingTools(server: McpServer) {
  server.tool(
    "beps_compliance",
    "OECD BEPS compliance analysis: CbCR reporting, Pillar Two GloBE 15% minimum tax, functional analysis, profit/substance alignment, risk scoring",
    BepsSchema.shape,
    async (params) => {
      const validated = BepsSchema.parse(coerceNumbers(params));
      const result = analyzeBepsCompliance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "intercompany_pricing",
    "Transfer pricing analysis: CUP, RPM, CPLM, TNMM, Profit Split methods with arm's length range, CFC analysis (Subpart F/GILTI/ATAD), GAAR assessment",
    IntercompanySchema.shape,
    async (params) => {
      const validated = IntercompanySchema.parse(coerceNumbers(params));
      const result = analyzeIntercompany(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
