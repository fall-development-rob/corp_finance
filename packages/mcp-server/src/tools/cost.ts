import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  surfaceCostSummary,
  surfaceCostBudgetSet,
  surfaceCostBudgetGet,
} from "../bindings.js";
import {
  SurfaceCostSummarySchema,
  SurfaceCostBudgetSetSchema,
  SurfaceCostBudgetGetSchema,
} from "../schemas/cost.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerCostTools(server: McpServer) {
  server.tool(
    "surface_cost_summary",
    "Aggregate cost-ledger rows by surface, tier, tenant, or surface_and_tier. Input: ledger path, optional CostFilter, group_by enum. Output: total cents, total tokens, and per-bucket breakdown ordered by spend desc. RUF-COST-004.",
    SurfaceCostSummarySchema.shape,
    async (params) => {
      const validated = SurfaceCostSummarySchema.parse(coerceNumbers(params));
      const result = surfaceCostSummary(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "surface_cost_budget_set",
    "Define or replace a CostBudget keyed on (surface_filter, tier_filter, period). Input: ledger path and budget definition with limit_cents and threshold_pct. Output: persisted budget id. RUF-COST-005.",
    SurfaceCostBudgetSetSchema.shape,
    async (params) => {
      const validated = SurfaceCostBudgetSetSchema.parse(coerceNumbers(params));
      const result = surfaceCostBudgetSet(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "surface_cost_budget_get",
    "Read budget consumption against limits, scoped by an optional CostFilter. Input: ledger path and filter. Output: per-budget current spend, limit, percentage consumed, and threshold-crossed flag. RUF-COST-006.",
    SurfaceCostBudgetGetSchema.shape,
    async (params) => {
      const validated = SurfaceCostBudgetGetSchema.parse(coerceNumbers(params));
      const result = surfaceCostBudgetGet(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
