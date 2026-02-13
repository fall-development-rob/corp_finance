import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  assessKycRisk,
  screenSanctions,
} from "../bindings.js";
import {
  KycRiskSchema,
  SanctionsScreeningSchema,
} from "../schemas/aml_compliance.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerAmlComplianceTools(server: McpServer) {
  server.tool(
    "kyc_risk_assessment",
    "KYC/AML risk scoring: customer risk assessment with jurisdiction/PEP/product/channel/structure factors, due diligence level determination (SDD/CDD/EDD), monitoring frequency, risk breakdown, PEP assessment, enhanced due diligence requirements",
    KycRiskSchema.shape,
    async (params) => {
      const validated = KycRiskSchema.parse(coerceNumbers(params));
      const result = assessKycRisk(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "sanctions_screening",
    "Sanctions screening: multi-list screening (OFAC SDN, EU Consolidated, HMT UK, UN UNSC, FATF Grey/Black lists), fuzzy name matching with configurable threshold, PEP screening, adverse media checks, transaction screening, batch rescreening",
    SanctionsScreeningSchema.shape,
    async (params) => {
      const validated = SanctionsScreeningSchema.parse(coerceNumbers(params));
      const result = screenSanctions(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
