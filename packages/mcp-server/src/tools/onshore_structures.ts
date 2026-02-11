import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeUsFundStructure,
  analyzeUkEuFund,
} from "corp-finance-bindings";
import {
  UsFundSchema,
  UkEuFundSchema,
} from "../schemas/onshore_structures.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerOnshoreStructuresTools(server: McpServer) {
  server.tool(
    "us_fund_structure",
    "US onshore fund structure analysis: Delaware LP, LLC, REIT, MLP, BDC, QOZ with tax analysis, ERISA compliance, investor suitability",
    UsFundSchema.shape,
    async (params) => {
      const validated = UsFundSchema.parse(coerceNumbers(params));
      const result = analyzeUsFundStructure(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "uk_eu_fund_structure",
    "UK/EU onshore fund structure analysis: UK LP/LLP, OEIC, ACS, SICAV, FCP, KG with AIFMD passport, VAT analysis, cross-border marketing",
    UkEuFundSchema.shape,
    async (params) => {
      const validated = UkEuFundSchema.parse(coerceNumbers(params));
      const result = analyzeUkEuFund(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
