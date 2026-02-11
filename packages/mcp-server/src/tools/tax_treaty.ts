import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeTreatyNetwork,
  optimizeTreatyStructure,
} from "corp-finance-bindings";
import {
  TreatyNetworkSchema,
  TreatyOptSchema,
} from "../schemas/tax_treaty.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerTaxTreatyTools(server: McpServer) {
  server.tool(
    "treaty_network",
    "Tax treaty network analysis: WHT optimization, treaty conduit routing, LOB/PPT anti-avoidance scoring, entity-specific exemptions",
    TreatyNetworkSchema.shape,
    async (params) => {
      const validated = TreatyNetworkSchema.parse(coerceNumbers(params));
      const result = analyzeTreatyNetwork(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "treaty_structure_optimization",
    "Multi-jurisdiction holding structure optimization: participation exemption, IP box, interest deduction limits, PE risk assessment, substance cost-benefit",
    TreatyOptSchema.shape,
    async (params) => {
      const validated = TreatyOptSchema.parse(coerceNumbers(params));
      const result = optimizeTreatyStructure(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
