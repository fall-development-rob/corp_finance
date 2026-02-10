import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  planRetirement,
  simulateTaxLossHarvesting,
  planEstate,
} from "corp-finance-bindings";
import {
  RetirementSchema,
  TlhSchema,
  EstatePlanSchema,
} from "../schemas/wealth.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerWealthTools(server: McpServer) {
  server.tool(
    "retirement_planning",
    "Project a full retirement plan including accumulation phase (contributions + investment growth), decumulation phase (withdrawals with ConstantDollar, ConstantPercentage, or Guardrails strategy), savings-gap analysis (income needed vs available, additional savings required), and year-by-year schedule from current age through life expectancy.",
    RetirementSchema.shape,
    async (params) => {
      const validated = RetirementSchema.parse(coerceNumbers(params));
      const result = planRetirement(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "tax_loss_harvesting",
    "Simulate tax-loss harvesting across a portfolio. Identifies harvest candidates (positions with unrealized losses exceeding threshold), calculates tax savings from offsetting capital gains (short-term and long-term), projects portfolio impact including deferred tax liability from lower cost basis, and provides actionable recommendations.",
    TlhSchema.shape,
    async (params) => {
      const validated = TlhSchema.parse(coerceNumbers(params));
      const result = simulateTaxLossHarvesting(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "estate_planning",
    "Comprehensive estate planning analysis including gifting strategy (annual exclusion, taxable gifts, GST), federal and state estate tax calculations, trust analysis (Revocable, Irrevocable, GRAT, ILIT, QPRT, Crummey, Charitable Remainder), charitable bequests, marital deduction, and planning strategy recommendations.",
    EstatePlanSchema.shape,
    async (params) => {
      const validated = EstatePlanSchema.parse(coerceNumbers(params));
      const result = planEstate(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
