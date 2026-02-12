import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeConcentratedStock,
  comparePhilanthropicVehicles,
  analyzeWealthTransfer,
  analyzeDirectIndexing,
  evaluateFamilyGovernance,
} from "@fall-development-rob/corp-finance-bindings";
import {
  ConcentratedStockSchema,
  PhilanthropicVehiclesSchema,
  WealthTransferSchema,
  DirectIndexingSchema,
  FamilyGovernanceSchema,
} from "../schemas/private_wealth.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerPrivateWealthTools(server: McpServer) {
  server.tool(
    "concentrated_stock",
    "Concentrated stock analysis: collar, exchange fund, prepaid forward, charitable strategies with tax-adjusted after-tax comparison",
    ConcentratedStockSchema.shape,
    async (params) => {
      const validated = ConcentratedStockSchema.parse(coerceNumbers(params));
      const result = analyzeConcentratedStock(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "philanthropic_vehicles",
    "Philanthropic vehicle comparison: CRT, CLT, DAF, private foundation with tax deduction, income stream, and remainder analysis",
    PhilanthropicVehiclesSchema.shape,
    async (params) => {
      const validated = PhilanthropicVehiclesSchema.parse(coerceNumbers(params));
      const result = comparePhilanthropicVehicles(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "wealth_transfer",
    "Wealth transfer planning: estate tax, GST, annual exclusion, GRAT, grantor trust, dynasty trust, ILIT analysis with tax savings",
    WealthTransferSchema.shape,
    async (params) => {
      const validated = WealthTransferSchema.parse(coerceNumbers(params));
      const result = analyzeWealthTransfer(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "direct_indexing",
    "Direct indexing analysis: tax-loss harvesting opportunities, wash sale compliance, tracking error, after-tax alpha estimation",
    DirectIndexingSchema.shape,
    async (params) => {
      const validated = DirectIndexingSchema.parse(coerceNumbers(params));
      const result = analyzeDirectIndexing(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "family_governance",
    "Family governance evaluation: governance score, complexity assessment, structure recommendations, risk identification",
    FamilyGovernanceSchema.shape,
    async (params) => {
      const validated = FamilyGovernanceSchema.parse(coerceNumbers(params));
      const result = evaluateFamilyGovernance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
