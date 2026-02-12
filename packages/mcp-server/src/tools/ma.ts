import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { analyzeMerger } from "@robotixai/corp-finance-bindings";
import { MergerSchema } from "../schemas/ma.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerMATools(server: McpServer) {
  server.tool(
    "merger_model",
    "Analyze a merger for EPS accretion/dilution. Supports all-cash, all-stock, and mixed consideration. Calculates pro-forma EPS, premium analysis, exchange ratios, synergy impact, and breakeven synergies needed for EPS neutrality.",
    MergerSchema.shape,
    async (params) => {
      const validated = MergerSchema.parse(coerceNumbers(params));
      const result = analyzeMerger(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
