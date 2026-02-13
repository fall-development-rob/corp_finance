import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  calculateCloWaterfall,
  calculateCoverageTests,
  calculateReinvestment,
  calculateTrancheAnalytics,
  calculateCloScenario,
} from "../bindings.js";
import {
  CloWaterfallSchema,
  CloCoverageTestsSchema,
  CloReinvestmentSchema,
  CloTrancheAnalyticsSchema,
  CloScenarioSchema,
} from "../schemas/clo_analytics.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCloAnalyticsTools(server: McpServer) {
  server.tool(
    "clo_waterfall",
    "CLO waterfall engine: payment priority cascades, interest/principal distribution, sequential paydown, equity cash flows",
    CloWaterfallSchema.shape,
    async (params) => {
      const validated = CloWaterfallSchema.parse(coerceNumbers(params));
      const result = calculateCloWaterfall(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "clo_coverage_tests",
    "CLO coverage tests: OC/IC ratios, trigger breach detection, cure mechanics, diversion amounts",
    CloCoverageTestsSchema.shape,
    async (params) => {
      const validated = CloCoverageTestsSchema.parse(coerceNumbers(params));
      const result = calculateCoverageTests(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "clo_reinvestment",
    "CLO reinvestment period: WARF, WAL, WALS, diversity score, par build test, criteria compliance",
    CloReinvestmentSchema.shape,
    async (params) => {
      const validated = CloReinvestmentSchema.parse(coerceNumbers(params));
      const result = calculateReinvestment(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "clo_tranche_analytics",
    "CLO tranche analytics: yield-to-worst, WAL, spread duration, breakeven CDR, equity IRR, cash-on-cash",
    CloTrancheAnalyticsSchema.shape,
    async (params) => {
      const validated = CloTrancheAnalyticsSchema.parse(coerceNumbers(params));
      const result = calculateTrancheAnalytics(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "clo_scenario",
    "CLO scenario analysis: multi-scenario stress testing, tranche loss allocation, attachment/detachment points",
    CloScenarioSchema.shape,
    async (params) => {
      const validated = CloScenarioSchema.parse(coerceNumbers(params));
      const result = calculateCloScenario(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
