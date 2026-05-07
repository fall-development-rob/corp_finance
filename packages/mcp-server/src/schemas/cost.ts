import { z } from "zod";

/**
 * Cost bounded-context schemas. Mirrors
 * `crates/corp-finance-core/src/cost/{types,ledger,budget,reporter}.rs`.
 * Per ADR-017 §3-4 / RUF-COST-001..006.
 */

const SurfaceEnum = z.enum(["cli", "mcp", "skill", "plugin"]);

const TierTagEnum = z.enum([
  "cookbook_core_only",
  "cookbook_freemium",
  "cookbook_paid_vendor",
  "mcp_free_native",
  "mcp_free_public_with_api_key",
  "mcp_freemium",
  "mcp_paid_vendor",
  "unknown",
]);

const BudgetPeriodEnum = z.enum(["daily", "weekly", "monthly", "total"]);

const GroupByEnum = z.enum(["surface", "tier", "tenant", "surface_and_tier"]);

const CostFilterSchema = z
  .object({
    surface: SurfaceEnum.optional().describe("Filter to a single surface."),
    tier: TierTagEnum.optional().describe("Filter to a single tier tag."),
    tenant_id: z
      .string()
      .optional()
      .describe("Filter to a single tenant id."),
    since: z
      .string()
      .optional()
      .describe("RFC3339 inclusive lower bound on event timestamp."),
    until: z
      .string()
      .optional()
      .describe("RFC3339 exclusive upper bound on event timestamp."),
  })
  .describe("Optional WHERE clause for ledger queries.");

export const SurfaceCostSummarySchema = z.object({
  ledger_path: z
    .string()
    .describe("Absolute path to the SQLite cost-ledger file."),
  filter: CostFilterSchema,
  group_by: GroupByEnum.describe("Bucket dimension for the aggregation."),
});

export const SurfaceCostBudgetSetSchema = z.object({
  ledger_path: z
    .string()
    .describe("Absolute path to the SQLite cost-ledger file."),
  budget: z
    .object({
      surface_filter: SurfaceEnum.optional().describe(
        "Optional surface filter; null covers all surfaces."
      ),
      tier_filter: TierTagEnum.optional().describe(
        "Optional tier filter; null covers all tiers."
      ),
      period: BudgetPeriodEnum.describe(
        "Period the budget covers (daily/weekly/monthly/total)."
      ),
      limit_cents: z
        .number()
        .int()
        .describe("Hard cap in integer cents."),
      threshold_pct: z
        .number()
        .int()
        .min(0)
        .max(100)
        .describe(
          "Warning threshold (0..=100). Crossing fires the budget_threshold_crossed event."
        ),
    })
    .describe("CostBudget definition keyed on (surface_filter, tier_filter, period)."),
});

export const SurfaceCostBudgetGetSchema = z.object({
  ledger_path: z
    .string()
    .describe("Absolute path to the SQLite cost-ledger file."),
  filter: CostFilterSchema,
});
