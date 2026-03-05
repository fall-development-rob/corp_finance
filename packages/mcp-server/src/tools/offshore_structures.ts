import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeCaymanStructure,
  analyzeLuxStructure,
  analyzeJerseyFund,
  analyzeVccStructure,
  analyzeOfcStructure,
  analyzeDifcFund,
  compareJurisdictions,
  migrationFeasibility,
} from "../bindings.js";
import {
  CaymanFundSchema,
  LuxFundSchema,
  JerseyFundSchema,
  VccFundSchema,
  HkOfcSchema,
  DifcFundSchema,
  JurisdictionComparisonSchema,
  MigrationFeasibilitySchema,
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

  server.tool(
    "channel_islands_fund_structure",
    "Channel Islands fund structures: Jersey JPF/Expert/QIF, Guernsey PIF/QIF/RQIF, Protected/Incorporated Cell Companies",
    JerseyFundSchema.shape,
    async (params) => {
      const validated = JerseyFundSchema.parse(coerceNumbers(params));
      const result = analyzeJerseyFund(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "singapore_vcc_structure",
    "Singapore Variable Capital Company (VCC): standalone/umbrella structures, sub-fund allocation, RFMC/LRFMC/A-LFMC licensing, S13O/S13U/S13D tax incentives",
    VccFundSchema.shape,
    async (params) => {
      const validated = VccFundSchema.parse(coerceNumbers(params));
      const result = analyzeVccStructure(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "hong_kong_fund_structure",
    "Hong Kong Open-ended Fund Company (OFC): public/private, umbrella sub-funds, SFC Type 9 licensing, OFC grant scheme eligibility",
    HkOfcSchema.shape,
    async (params) => {
      const validated = HkOfcSchema.parse(coerceNumbers(params));
      const result = analyzeOfcStructure(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "middle_east_fund_structure",
    "Middle East fund structures: DIFC QIF/Exempt/Domestic funds, Sharia compliance, DFSA regulatory framework",
    DifcFundSchema.shape,
    async (params) => {
      const validated = DifcFundSchema.parse(coerceNumbers(params));
      const result = analyzeDifcFund(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "jurisdiction_comparison",
    "Multi-jurisdiction comparison: weighted scoring across setup cost, annual cost, tax, regulatory speed, distribution reach, and substance for 10 offshore domiciles",
    JurisdictionComparisonSchema.shape,
    async (params) => {
      const validated = JurisdictionComparisonSchema.parse(coerceNumbers(params));
      const result = compareJurisdictions(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "fund_migration_analysis",
    "Fund migration/redomiciliation feasibility: statutory continuation, scheme of arrangement, investor consent, timeline, cost estimation across offshore corridors",
    MigrationFeasibilitySchema.shape,
    async (params) => {
      const validated = MigrationFeasibilitySchema.parse(coerceNumbers(params));
      const result = migrationFeasibility(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
