import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  valueRealOption,
  analyzeDecisionTree,
} from "@robotixai/corp-finance-bindings";
import {
  RealOptionSchema,
  DecisionTreeSchema,
} from "../schemas/real_options.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerRealOptionsTools(server: McpServer) {
  server.tool(
    "real_option_valuation",
    "Value real options (expand, abandon, defer, switch, contract, compound) using CRR binomial tree with Greeks",
    RealOptionSchema.shape,
    async (params) => {
      const validated = RealOptionSchema.parse(coerceNumbers(params));
      const result = valueRealOption(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "decision_tree_analysis",
    "Decision tree analysis with EMV rollback, EVPI, sensitivity analysis, and optimal path identification",
    DecisionTreeSchema.shape,
    async (params) => {
      const validated = DecisionTreeSchema.parse(coerceNumbers(params));
      const result = analyzeDecisionTree(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
