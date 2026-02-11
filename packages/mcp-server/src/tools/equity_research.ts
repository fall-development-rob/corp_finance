import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateSotp,
  calculateTargetPrice,
} from "corp-finance-bindings";
import {
  SotpSchema,
  TargetPriceSchema,
} from "../schemas/equity_research.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerEquityResearchTools(server: McpServer) {
  server.tool(
    "sotp_valuation",
    "Sum-of-the-parts valuation: segment-level multiples, conglomerate discount, football field analysis",
    SotpSchema.shape,
    async (params) => {
      const validated = SotpSchema.parse(coerceNumbers(params));
      const result = calculateSotp(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "target_price",
    "Multi-method target price: PE, PEG, PB, PS, DDM, analyst consensus with football field and recommendation",
    TargetPriceSchema.shape,
    async (params) => {
      const validated = TargetPriceSchema.parse(coerceNumbers(params));
      const result = calculateTargetPrice(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
