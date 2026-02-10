import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateRegulatoryCapital,
  calculateLcr,
  calculateNsfr,
  analyzeAlm,
} from "corp-finance-bindings";
import {
  RegulatoryCapitalSchema,
  LcrSchema,
  NsfrSchema,
  AlmSchema,
} from "../schemas/regulatory.js";
import { wrapResponse } from "../formatters/response.js";

export function registerRegulatoryTools(server: McpServer) {
  server.tool(
    "regulatory_capital",
    "Calculate Basel III/IV regulatory capital and risk-weighted assets (RWA). Computes credit RWA under the Standardised Approach with collateral CRM, operational risk (BIA or SA), combines with market risk charge. Derives CET1, Tier 1, and total capital ratios, buffer requirements, and surplus/deficit analysis.",
    RegulatoryCapitalSchema.shape,
    async (params) => {
      const validated = RegulatoryCapitalSchema.parse(params);
      const result = calculateRegulatoryCapital(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "lcr",
    "Calculate the Basel III Liquidity Coverage Ratio (LCR). Applies standard haircuts to HQLA (Level 1/2A/2B with composition caps), run-off factors to outflows, inflow caps at 75% of outflows. Returns LCR ratio, HQLA detail, outflow detail, and whether the 100% minimum is met.",
    LcrSchema.shape,
    async (params) => {
      const validated = LcrSchema.parse(params);
      const result = calculateLcr(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "nsfr",
    "Calculate the Basel III Net Stable Funding Ratio (NSFR). Applies Available Stable Funding (ASF) and Required Stable Funding (RSF) factors per Basel III framework. Returns NSFR ratio, ASF/RSF breakdown by category, and whether the 100% minimum is met.",
    NsfrSchema.shape,
    async (params) => {
      const validated = NsfrSchema.parse(params);
      const result = calculateNsfr(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "alm_analysis",
    "Perform Asset-Liability Management (ALM) and Interest Rate Risk in the Banking Book (IRRBB) analysis. Computes duration gap, EVE sensitivity (change in equity value for a rate shock), NII sensitivity, maturity gap analysis across time buckets, and repricing gap ratio.",
    AlmSchema.shape,
    async (params) => {
      const validated = AlmSchema.parse(params);
      const result = analyzeAlm(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
