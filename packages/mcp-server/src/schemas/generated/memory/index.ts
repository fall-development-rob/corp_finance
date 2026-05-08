/**
 * Phase 29 Wave 13 — Generated zod schemas for the memory domain.
 *
 * AUTO-GENERATED — do not edit by hand. Re-run `npm run schemas:gen:memory` to refresh.
 * Source of truth: crates/corp-finance-core/src/memory/types.rs
 * Pipeline: schemars 1.2.1 (JsonSchema derive) → JSON Schema → json-schema-to-zod 2.8.1 → zod 3
 *
 * =============================================================================
 * Semantic diff: generated vs hand-maintained (schemas/memory.ts)
 * =============================================================================
 *
 * 1. Surface shape: hand-maintained uses `z.enum(["cli","mcp","skill","plugin"])`.
 *    Generated emits `z.discriminatedUnion("kind", [z.literal(...), ...])` after
 *    the post-processing transform. The discriminatedUnion form is more precise
 *    but subtly different in TypeScript type inference. Call sites using Surface
 *    as a plain enum string should keep importing SurfaceEnum from schemas/memory.js.
 *
 * 2. RunSummary.embedding: hand-maintained allows omitting embedding (`optional()`);
 *    generated enforces it as required (mirrors the Rust struct which has no
 *    `#[serde(default)]`). Ingest call sites must always supply the embedding vector.
 *
 * 3. RunSummary.run_id: hand-maintained marks as optional (core allocates one);
 *    generated emits required UUID. The NAPI boundary accepts Option<Uuid> via
 *    serde skip_serializing_if, so both wire shapes work — but hand-maintained
 *    schema better captures the MCP tool contract.
 *
 * 4. RunSummary.ts: hand-maintained marks optional; generated emits required
 *    `z.string().datetime({ offset: true })`. More restrictive and correct for
 *    RFC3339 validation; the Rust struct has no default for `ts`.
 *
 * 5. MemoryQuery.limit: hand-maintained uses `.int().positive()` (min 1);
 *    generated emits `.int().gte(0)` (min 0, from usize). Hand-maintained
 *    is semantically stricter (limit=0 is useless). No adaptation needed at
 *    call sites; Rust clamps anyway.
 *
 * 6. Optional nullable fields: generated wraps optional fields in
 *    `z.union([T, z.null()])` (schemars Option<T> → oneOf null pattern) before
 *    the `.optional()`. Hand-maintained uses `.optional()` alone. Both accept
 *    the same JSON; the union form is more accurate for serde's null vs absent.
 *
 * 7. Input / output types not auto-genned (no public Rust struct exists):
 *    SurfaceMemoryIngestSchema, SurfaceMemoryFindSchema,
 *    SurfaceSessionSaveSchema, SurfaceSessionRestoreSchema.
 *    These remain hand-maintained in schemas/memory.ts.
 *
 * 8. CfaSession: not in the hand-maintained file. Fully new type exported only
 *    from here. Generated correctly; no gaps vs the Rust struct.
 *
 * 9. EntityRef / EntityKind: not in the hand-maintained file. Phase 27 types
 *    used by the multi-agent coordination context. Exported here for the first
 *    time. No gaps observed.
 *
 * Total new gaps vs prior documented set (6 known): 0 new gaps found.
 * =============================================================================
 */

import { z } from "zod";

// Re-export all generated schemas with stable, qualified names.

export { SurfaceSchema } from "./Surface.js";
export { EntityKindSchema } from "./EntityKind.js";
export { EntityRefSchema } from "./EntityRef.js";
export { RunSummarySchema } from "./RunSummary.js";
export { CfaSessionSchema } from "./CfaSession.js";
export { MemoryQuerySchema } from "./MemoryQuery.js";

// Suppress unused-import warning — z is needed by generated files that omit their own import.
export { z };
