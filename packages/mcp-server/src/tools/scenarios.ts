import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  buildSensitivityGrid,
  scenarioAnalysis,
} from "@rob-otixai/corp-finance-bindings";
import {
  SensitivitySchema,
  ScenarioSchema,
} from "../schemas/scenarios.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerScenarioTools(server: McpServer) {
  server.tool(
    "sensitivity_matrix",
    "Generate a 2-way sensitivity matrix. Varies two input parameters across specified ranges and runs the selected financial model at each combination. Returns an output matrix showing the target metric at every (variable_1, variable_2) intersection, with the base case position highlighted.",
    SensitivitySchema.shape,
    async (params) => {
      const validated = SensitivitySchema.parse(coerceNumbers(params));
      const result = buildSensitivityGrid(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "scenario_analysis",
    "Run scenario analysis (e.g. Bear/Base/Bull) across a financial model. Each scenario overrides specific parameters with probability weights. Returns per-scenario outputs and a probability-weighted expected value for the target metric.",
    ScenarioSchema.shape,
    async (params) => {
      const validated = ScenarioSchema.parse(coerceNumbers(params));
      const result = scenarioAnalysis(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
