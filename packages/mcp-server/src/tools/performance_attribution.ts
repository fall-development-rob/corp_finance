import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  brinsonAttribution,
  factorAttribution,
} from "@rob-otixai/corp-finance-bindings";
import {
  BrinsonSchema,
  FactorAttributionSchema,
} from "../schemas/performance_attribution.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerPerformanceAttributionTools(server: McpServer) {
  server.tool(
    "brinson_attribution",
    "Brinson-Fachler performance attribution: allocation, selection, interaction effects by sector with multi-period linking via Carino method",
    BrinsonSchema.shape,
    async (params) => {
      const validated = BrinsonSchema.parse(coerceNumbers(params));
      const result = brinsonAttribution(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "factor_attribution",
    "Factor-based return attribution: active exposure decomposition, R-squared, tracking error breakdown by systematic factors",
    FactorAttributionSchema.shape,
    async (params) => {
      const validated = FactorAttributionSchema.parse(coerceNumbers(params));
      const result = factorAttribution(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
