import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeSovereignBond,
  assessCountryRisk,
} from "@rob-otixai/corp-finance-bindings";
import {
  SovereignBondSchema,
  CountryRiskSchema,
} from "../schemas/sovereign.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerSovereignTools(server: McpServer) {
  server.tool(
    "sovereign_bond_analysis",
    "Analyze sovereign bonds: pricing, YTM, duration, convexity, spread decomposition, and local currency risk",
    SovereignBondSchema.shape,
    async (params) => {
      const validated = SovereignBondSchema.parse(coerceNumbers(params));
      const result = analyzeSovereignBond(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "country_risk_assessment",
    "Assess country/sovereign risk: multi-factor scoring, rating equivalent, CRP, implied default probability",
    CountryRiskSchema.shape,
    async (params) => {
      const validated = CountryRiskSchema.parse(coerceNumbers(params));
      const result = assessCountryRisk(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
