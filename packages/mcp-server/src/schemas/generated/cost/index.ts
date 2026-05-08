/**
 * Phase 29 Wave 13 — Generated zod schemas for the cost domain.
 *
 * AUTO-GENERATED — do not edit by hand. Re-run `npm run schemas:gen:cost` to refresh.
 * Source of truth: crates/corp-finance-core/src/cost/{types,reporter,budget}.rs + surface/mod.rs
 * Pipeline: schemars 1.2.1 (JsonSchema derive) → JSON Schema → json-schema-to-zod 2.8.1 → zod 3
 *
 * =============================================================================
 * Decimal note
 * =============================================================================
 * No rust_decimal::Decimal fields exist in the cost domain. All monetary values
 * are i64 integer cents (cost_cents, limit_cents, used_cents). These render as
 * z.number().int() in the generated schemas — correct and exact.
 *
 * =============================================================================
 * Semantic diff: generated vs hand-maintained (schemas/cost.ts) — remaining gaps
 * =============================================================================
 *
 * 1. CostFilter / BudgetFilter are not exported types (Rust structs without serde
 *    Deserialize on the input side). The hand-maintained cost.ts inlines their
 *    structure directly into SurfaceCostSummarySchema and SurfaceCostBudgetGetSchema.
 *    Generated schemas cover only the output/response types (CostSummary,
 *    BudgetStatus) and the value-object primitives. Import tool input schemas
 *    from schemas/cost.js (unchanged).
 *
 * 2. Optional vs nullable: generated emits z.union([z.string(), z.null()]).optional()
 *    for Option<String> fields (tenant_id on CostEvent). Hand-maintained schemas use
 *    .optional() only. Both parse correctly; the difference surfaces when the caller
 *    passes explicit null — generated allows it, hand-maintained rejects it.
 *
 * 3. BudgetStatus discriminator: hand-maintained cost.ts does not define BudgetStatus
 *    as a zod schema. Generated BudgetStatusSchema uses z.discriminatedUnion("status",
 *    [...]) with inline "status" literal fields — correct for the serde(tag = "status")
 *    wire format.
 *
 * 4. String-const enums (Surface, TierTag, BudgetPeriod, GroupBy): generated emits
 *    z.union([z.literal(...), ...]) because json-schema-to-zod emits superRefine for
 *    oneOf patterns. The post-processor rewrites these to z.union([...]) (not
 *    z.enum([...])). Functionally equivalent at parse time.
 * =============================================================================
 */

import { z } from "zod";

// Value-object primitives
export { SurfaceSchema } from "./Surface.js";
export { TierTagSchema } from "./TierTag.js";
export { BudgetPeriodSchema } from "./BudgetPeriod.js";
export { GroupBySchema } from "./GroupBy.js";

// Core domain types
export { CostEventSchema } from "./CostEvent.js";
export { CostBudgetSchema } from "./CostBudget.js";

// Response / output types
export { BudgetStatusSchema } from "./BudgetStatus.js";
export { CostSummaryRowSchema } from "./CostSummaryRow.js";
export { CostSummarySchema } from "./CostSummary.js";

// Suppress unused-import warning — z is needed by generated files that omit their own import.
export { z };
