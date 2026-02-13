import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeEconomicSubstance,
  runJurisdictionSubstanceTest,
} from "../bindings.js";
import {
  EconomicSubstanceSchema,
  JurisdictionSubstanceTestSchema,
} from "../schemas/substance_requirements.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerSubstanceRequirementsTools(server: McpServer) {
  server.tool(
    "economic_substance",
    "Economic substance analysis: 0-100 substance score with personnel/premises/decision-making/expenditure/CIGA breakdown, compliance status, gap identification, remediation recommendations, penalty exposure, substance cost estimation, treaty denial risk",
    EconomicSubstanceSchema.shape,
    async (params) => {
      const validated = EconomicSubstanceSchema.parse(coerceNumbers(params));
      const result = analyzeEconomicSubstance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "jurisdiction_substance_test",
    "Jurisdiction-specific substance testing: Directed & Managed (Cayman/BVI), Central Management & Control (Ireland/UK), POEM (OECD), ATAD (Luxembourg/Netherlands), Tax Incentive (Singapore) with multi-jurisdiction comparison, cost-benefit analysis, optimal jurisdiction selection",
    JurisdictionSubstanceTestSchema.shape,
    async (params) => {
      const validated = JurisdictionSubstanceTestSchema.parse(coerceNumbers(params));
      const result = runJurisdictionSubstanceTest(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
