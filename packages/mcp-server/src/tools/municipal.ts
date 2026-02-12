import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  priceMuniBond,
  analyzeMunicipal,
} from "@robotixai/corp-finance-bindings";
import {
  MuniBondSchema,
  MuniAnalysisSchema,
} from "../schemas/municipal.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMunicipalTools(server: McpServer) {
  server.tool(
    "muni_bond_pricing",
    "Price a municipal bond with tax-equivalent yield analysis. Computes clean/dirty price, accrued interest, current yield, tax-equivalent yield (TEY), after-tax comparison vs taxable bonds, de minimis rule analysis, yield-to-call, muni/Treasury and muni/corporate spread ratios. Handles GO, revenue, assessment, TIF, and COP bond types.",
    MuniBondSchema.shape,
    async (params) => {
      const validated = MuniBondSchema.parse(coerceNumbers(params));
      const result = priceMuniBond(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "municipal_analysis",
    "Comprehensive municipal credit analysis: GO bond analysis (debt ratios, debt burden, fund balance, pension liability), revenue bond analysis (DSCR, rate covenant compliance, additional bonds test, reserve fund adequacy), composite credit scoring (10-factor model with letter-grade rating), and refunding/advance refunding savings analysis with PV of savings and efficiency ratio.",
    MuniAnalysisSchema.shape,
    async (params) => {
      const validated = MuniAnalysisSchema.parse(coerceNumbers(params));
      const result = analyzeMunicipal(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
