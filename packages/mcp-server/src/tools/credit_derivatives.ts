import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  priceCds,
  calculateCva,
} from "@robotixai/corp-finance-bindings";
import {
  CdsPricingSchema,
  CvaCalculationSchema,
} from "../schemas/credit_derivatives.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCreditDerivativesTools(server: McpServer) {
  server.tool(
    "cds_pricing",
    "Price a single-name credit default swap. Calculates premium and protection leg PVs using a discrete hazard-rate model, producing survival curves, risky PV01, mark-to-market, DV01, jump-to-default exposure, breakeven spread, and credit triangle metrics.",
    CdsPricingSchema.shape,
    async (params) => {
      const validated = CdsPricingSchema.parse(coerceNumbers(params));
      const result = priceCds(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "cva_calculation",
    "Calculate Credit Valuation Adjustment (CVA) and Debit Valuation Adjustment (DVA). Computes unilateral and bilateral CVA using a discrete marginal-default-probability framework, with optional netting and collateral adjustments. Produces exposure-at-default, expected loss, and CVA as running spread.",
    CvaCalculationSchema.shape,
    async (params) => {
      const validated = CvaCalculationSchema.parse(coerceNumbers(params));
      const result = calculateCva(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
