import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateCountryRiskPremium,
  assessPoliticalRisk,
  analyseCapitalControls,
  analyseEmBonds,
  calculateEmEquityPremium,
} from "corp-finance-bindings";
import {
  CountryRiskPremiumSchema,
  PoliticalRiskSchema,
  CapitalControlsSchema,
  EmBondAnalysisSchema,
  EmEquityPremiumSchema,
} from "../schemas/emerging_markets.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerEmergingMarketsTools(server: McpServer) {
  server.tool(
    "country_risk_premium",
    "Country risk premium: Damodaran sovereign spread, relative volatility, composite risk premium with governance and macro adjustments",
    CountryRiskPremiumSchema.shape,
    async (params) => {
      const validated = CountryRiskPremiumSchema.parse(coerceNumbers(params));
      const result = calculateCountryRiskPremium(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "political_risk",
    "Political risk assessment: WGI composite scoring, MIGA insurance valuation, expropriation/sanctions/conflict risk quantification",
    PoliticalRiskSchema.shape,
    async (params) => {
      const validated = PoliticalRiskSchema.parse(coerceNumbers(params));
      const result = assessPoliticalRisk(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "capital_controls",
    "Capital controls analysis: repatriation delay cost, withholding tax drag, FX conversion cost, effective yield impact, total cost of controls",
    CapitalControlsSchema.shape,
    async (params) => {
      const validated = CapitalControlsSchema.parse(coerceNumbers(params));
      const result = analyseCapitalControls(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "em_bond_analysis",
    "EM bond analysis: local vs hard currency comparison, FX-adjusted yield, carry trade decomposition, hedged/unhedged return scenarios",
    EmBondAnalysisSchema.shape,
    async (params) => {
      const validated = EmBondAnalysisSchema.parse(coerceNumbers(params));
      const result = analyseEmBonds(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "em_equity_premium",
    "EM equity risk premium: sovereign spread method, relative volatility method, composite ERP with valuation and growth adjustments",
    EmEquityPremiumSchema.shape,
    async (params) => {
      const validated = EmEquityPremiumSchema.parse(coerceNumbers(params));
      const result = calculateEmEquityPremium(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
