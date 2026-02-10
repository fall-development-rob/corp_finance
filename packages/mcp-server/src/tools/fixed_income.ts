import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { priceBond, calculateBondYield, bootstrapSpotCurve, fitNelsonSiegel, calculateDuration, calculateCreditSpreads } from "corp-finance-bindings";
import { BondPricingSchema, BondYieldSchema, BootstrapSchema, NelsonSiegelSchema, DurationSchema, CreditSpreadSchema } from "../schemas/fixed_income.js";
import { wrapResponse } from "../formatters/response.js";

export function registerFixedIncomeTools(server: McpServer) {
  server.tool(
    "bond_pricer",
    "Price a bond — clean/dirty prices, accrued interest, current yield, cashflow schedule, YTC/YTW for callable bonds",
    BondPricingSchema.shape,
    async (params) => {
      const validated = BondPricingSchema.parse(params);
      const result = priceBond(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "bond_yield",
    "Calculate bond yield metrics — YTM (Newton-Raphson), current yield, BEY, effective annual yield",
    BondYieldSchema.shape,
    async (params) => {
      const validated = BondYieldSchema.parse(params);
      const result = calculateBondYield(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "bootstrap_spot_curve",
    "Bootstrap a zero-coupon spot rate curve from par instruments — spot rates, forward rates, discount factors",
    BootstrapSchema.shape,
    async (params) => {
      const validated = BootstrapSchema.parse(params);
      const result = bootstrapSpotCurve(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "nelson_siegel_fit",
    "Fit a Nelson-Siegel yield curve model to observed market yields — beta parameters, fitted rates, RMSE",
    NelsonSiegelSchema.shape,
    async (params) => {
      const validated = NelsonSiegelSchema.parse(params);
      const result = fitNelsonSiegel(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "bond_duration",
    "Calculate bond duration, convexity, DV01, and key rate durations — Macaulay, modified, effective duration",
    DurationSchema.shape,
    async (params) => {
      const validated = DurationSchema.parse(params);
      const result = calculateDuration(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "credit_spreads",
    "Calculate credit spreads — I-spread, G-spread, Z-spread, spread duration, CDS spread estimate, credit quality indicator",
    CreditSpreadSchema.shape,
    async (params) => {
      const validated = CreditSpreadSchema.parse(params);
      const result = calculateCreditSpreads(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
