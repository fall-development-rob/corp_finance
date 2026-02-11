import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeCaymanStructure,
  analyzeLuxStructure,
} from "corp-finance-bindings";
import {
  CaymanFundSchema,
  LuxFundSchema,
} from "../schemas/offshore_structures.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerOffshoreStructuresTools(server: McpServer) {
  server.tool(
    "cayman_fund_structure",
    "Cayman/BVI offshore fund structure: Exempted LP, SPC, Unit Trust, BVI BCA with master-feeder economics, CIMA registration, economic substance",
    CaymanFundSchema.shape,
    async (params) => {
      const validated = CaymanFundSchema.parse(coerceNumbers(params));
      const result = analyzeCaymanStructure(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "lux_ireland_fund_structure",
    "Luxembourg/Ireland fund structure: SICAV-SIF, RAIF, SCSp, ICAV, QIAIF, Section 110 with subscription tax, AIFMD passport, UCITS analysis",
    LuxFundSchema.shape,
    async (params) => {
      const validated = LuxFundSchema.parse(coerceNumbers(params));
      const result = analyzeLuxStructure(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
