import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  priceCarbonCredit,
  analyzeEtsCompliance,
  analyzeCbam,
  valueCarbonOffset,
  calculateShadowCarbonPrice,
} from "@fall-development-rob/corp-finance-bindings";
import {
  CarbonCreditPricingSchema,
  EtsComplianceSchema,
  CbamAnalysisSchema,
  OffsetValuationSchema,
  ShadowCarbonPriceSchema,
} from "../schemas/carbon_markets.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCarbonMarketsTools(server: McpServer) {
  server.tool(
    "carbon_credit_pricing",
    "Carbon credit pricing: forward price via cost-of-carry, vintage discount, registry premium, credit type adjustment",
    CarbonCreditPricingSchema.shape,
    async (params) => {
      const validated = CarbonCreditPricingSchema.parse(coerceNumbers(params));
      const result = priceCarbonCredit(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "ets_compliance",
    "ETS compliance analysis: allowance surplus/deficit, compliance cost, price volatility, carbon intensity vs benchmark",
    EtsComplianceSchema.shape,
    async (params) => {
      const validated = EtsComplianceSchema.parse(coerceNumbers(params));
      const result = analyzeEtsCompliance(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "cbam_analysis",
    "EU CBAM analysis: certificate cost per good, net CBAM liability after origin carbon price credit, total exposure",
    CbamAnalysisSchema.shape,
    async (params) => {
      const validated = CbamAnalysisSchema.parse(coerceNumbers(params));
      const result = analyzeCbam(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "offset_valuation",
    "Carbon offset valuation: quality-adjusted price, permanence/additionality/vintage/certification adjustments, co-benefit premium",
    OffsetValuationSchema.shape,
    async (params) => {
      const validated = OffsetValuationSchema.parse(coerceNumbers(params));
      const result = valueCarbonOffset(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "shadow_carbon_price",
    "Shadow carbon price analysis: carbon-adjusted NPV, abatement cost, project ranking with/without carbon pricing, breakeven carbon price",
    ShadowCarbonPriceSchema.shape,
    async (params) => {
      const validated = ShadowCarbonPriceSchema.parse(coerceNumbers(params));
      const result = calculateShadowCarbonPrice(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
