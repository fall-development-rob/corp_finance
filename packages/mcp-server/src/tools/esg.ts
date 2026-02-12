import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateEsgScore,
  analyzeCarbonFootprint,
  analyzeGreenBond,
  testSllCovenants,
} from "@robotixai/corp-finance-bindings";
import {
  EsgScoreSchema,
  CarbonFootprintSchema,
  GreenBondSchema,
  SllSchema,
} from "../schemas/esg.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerEsgTools(server: McpServer) {
  server.tool(
    "esg_score",
    "Calculate a comprehensive ESG score with pillar weighting (E/S/G), materiality mapping, peer benchmarking, and red/amber/green flag analysis. Returns overall and pillar scores (0-100), letter rating (AAA-CCC), materiality issues, and optional peer comparison.",
    EsgScoreSchema.shape,
    async (params) => {
      const validated = EsgScoreSchema.parse(coerceNumbers(params));
      const result = calculateEsgScore(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "carbon_footprint",
    "Analyse an organisation's carbon footprint across Scope 1, 2, and 3 emissions. Computes carbon intensity, carbon cost exposure, target gap analysis vs SBTi targets, and implied temperature alignment. Identifies the largest Scope 3 category.",
    CarbonFootprintSchema.shape,
    async (params) => {
      const validated = CarbonFootprintSchema.parse(coerceNumbers(params));
      const result = analyzeCarbonFootprint(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "green_bond",
    "Analyse a green bond's premium (greenium) versus a comparable conventional bond. Calculates greenium in basis points, PV of coupon savings, total CO2 impact, cost per tonne avoided, allocation by project category, and framework alignment score.",
    GreenBondSchema.shape,
    async (params) => {
      const validated = GreenBondSchema.parse(coerceNumbers(params));
      const result = analyzeGreenBond(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "sll_covenants",
    "Test sustainability-linked loan (SLL) covenants against sustainability performance targets (SPTs). For each KPI, evaluates progress toward target, determines if target is met, and calculates the margin adjustment. Returns adjusted margin, annual savings, and per-target results.",
    SllSchema.shape,
    async (params) => {
      const validated = SllSchema.parse(coerceNumbers(params));
      const result = testSllCovenants(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
