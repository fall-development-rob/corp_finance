import { z } from "zod";

/**
 * Multi-agent coordination bounded-context schemas. Mirrors
 * `crates/corp-finance-core/src/multi_agent/types.rs`:
 *   - GoapPlan / PlanStep / PlanAction (A* GOAP planner)
 *   - AgentInvocation aggregate (chief-analyst -> specialist delegation)
 *
 * Per ADR-018 / RUF-ORC-001..007 / MAC-INV-001..009.
 */

const InvocationStatusEnum = z.enum([
  "pending",
  "running",
  "completed",
  "failed",
]);

export const ChiefPlanEmitSchema = z.object({
  goal: z
    .string()
    .describe(
      "Free-text goal handed to the chief-analyst (e.g. 'initiate coverage on AAPL', 'IC memo for Acme')."
    ),
});

export const ChiefPlanReplanSchema = z.object({
  plan: z
    .record(z.unknown())
    .describe(
      "Existing GoapPlan envelope (plan_id, goal, steps, plan_hash, replan_count, max_replans)."
    ),
  failed_step: z
    .string()
    .uuid()
    .describe("UUID of the PlanStep that failed and triggered the replan."),
  reason: z
    .string()
    .describe(
      "Short human-readable cause; folded into the replanned step's result_summary."
    ),
});

export const ChiefPatternDetectSchema = z.object({
  graph: z
    .record(z.unknown())
    .describe(
      "Serialised entity graph (nodes: EntityRef list; edges: EntityRelation list) from the active multi-agent session."
    ),
  threshold: z
    .number()
    .int()
    .positive()
    .describe(
      "Minimum co-occurrence / centrality threshold for emitting a pattern (positive integer)."
    ),
});

const AgentInvocationSchema = z.object({
  invocation_id: z
    .string()
    .uuid()
    .optional()
    .describe("UUID v7 invocation id. If omitted the core allocates one."),
  target_agent: z
    .string()
    .describe(
      "Specialist slug being invoked; must be in the registered CFA specialist set (MAC-INV-002)."
    ),
  parent_invocation_id: z
    .string()
    .uuid()
    .optional()
    .describe(
      "Parent invocation id for sub-calls; None for the chief-analyst's root."
    ),
  input_summary: z
    .string()
    .describe("Short summary of the input handed to the specialist."),
  tenant_id: z
    .string()
    .optional()
    .describe("Federation tenant scope (MAC-INV-001)."),
  ts: z
    .string()
    .optional()
    .describe("RFC3339 open timestamp. Defaults to now() when omitted."),
  status: InvocationStatusEnum.optional().describe(
    "Lifecycle status; defaults to pending."
  ),
});

export const AgentInvokeRecordSchema = z.object({
  invocation: AgentInvocationSchema.describe(
    "AgentInvocation aggregate to record at the chief -> specialist boundary."
  ),
});

export const AgentTraceGetSchema = z.object({
  invocation_id: z
    .string()
    .uuid()
    .describe(
      "UUID of the AgentInvocation whose ancestor / descendant trace is requested."
    ),
});
