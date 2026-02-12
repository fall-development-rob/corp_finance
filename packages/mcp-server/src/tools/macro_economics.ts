import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  analyzeMonetaryPolicy,
  analyzeInternational,
} from "@robotixai/corp-finance-bindings";
import {
  MonetaryPolicySchema,
  InternationalSchema,
} from "../schemas/macro_economics.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMacroEconomicsTools(server: McpServer) {
  server.tool(
    "monetary_policy",
    "Monetary policy analysis: Taylor Rule rate prescription, Phillips Curve dynamics, Okun's Law output gap, recession risk scoring",
    MonetaryPolicySchema.shape,
    async (params) => {
      const validated = MonetaryPolicySchema.parse(coerceNumbers(params));
      const result = analyzeMonetaryPolicy(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "international_economics",
    "International economics analysis: purchasing power parity, covered/uncovered interest rate parity, balance of payments, REER",
    InternationalSchema.shape,
    async (params) => {
      const validated = InternationalSchema.parse(coerceNumbers(params));
      const result = analyzeInternational(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
