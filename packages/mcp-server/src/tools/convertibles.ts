import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  priceConvertible,
  analyzeConvertible,
} from "@rob-otixai/corp-finance-bindings";
import {
  ConvertiblePricingSchema,
  ConvertibleAnalysisSchema,
} from "../schemas/convertibles.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerConvertibleTools(server: McpServer) {
  server.tool(
    "convertible_bond_pricing",
    "Price a convertible bond using CRR binomial tree model. Accounts for call/put provisions, dividend yield, and credit spread. Outputs model price, bond floor, conversion value, conversion/investment premiums, embedded option value, Greeks (delta, gamma, vega, theta), yield-to-maturity, current yield, breakeven years, and risk profile classification.",
    ConvertiblePricingSchema.shape,
    async (params) => {
      const validated = ConvertiblePricingSchema.parse(coerceNumbers(params));
      const result = priceConvertible(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "convertible_bond_analysis",
    "Analyze convertible bond scenarios including stock/vol/spread sensitivity, forced conversion analysis, income advantage (yield vs dividend with breakeven), and risk-return profile (upside participation, downside protection, asymmetry ratio). Ideal for relative value and event-driven analysis.",
    ConvertibleAnalysisSchema.shape,
    async (params) => {
      const validated = ConvertibleAnalysisSchema.parse(coerceNumbers(params));
      const result = analyzeConvertible(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
