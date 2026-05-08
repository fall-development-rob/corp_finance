/**
 * Phase 29 Wave 13 — Generated zod schemas for the observability domain.
 *
 * AUTO-GENERATED — do not edit by hand. Re-run `npm run schemas:gen:observability` to refresh.
 * Source of truth: crates/corp-finance-core/src/observability/types.rs
 *                  crates/corp-finance-core/src/surface/mod.rs
 * Pipeline: schemars 1.2.1 (JsonSchema derive) → JSON Schema → json-schema-to-zod 2.8.1 → zod 3
 *
 * =============================================================================
 * Semantic diff: generated vs hand-maintained (schemas/audit.ts SurfaceEnum)
 * =============================================================================
 *
 * 1. Surface enum encoding: generated emits
 *    `z.discriminatedUnion("kind", [z.literal("cli"), ...])` rather than
 *    `z.enum(["cli", "mcp", "skill", "plugin"])`. Both accept the same wire
 *    values; however TypeScript inference differs — `z.enum` produces a union
 *    string type whereas the generated form produces a literal union. Call
 *    sites that need `z.enum` for `.options` access must keep using the
 *    hand-maintained SurfaceEnum in schemas/audit.ts.
 *
 * 2. SpanContext.surface is inlined: with inline_subschemas=true the Surface
 *    discriminated union is copied into SpanContextSchema rather than
 *    referenced by $ref. This is intentional and correct for the MCP input
 *    validation boundary.
 *
 * 3. No input/output tool schemas exist for the observability domain.
 *    The observability module is pure Rust tracing infrastructure with no
 *    NAPI bindings and no MCP tool handlers. SpanContext and Surface are
 *    value types consumed internally (audit manifest, cost ledger, memory
 *    context); they have no corresponding tools/observability.ts. The
 *    generated schemas are available for cross-domain consumers that embed
 *    SpanContext in their own MCP input shapes.
 *
 * Known gaps beyond the standard 6:
 *   None. The domain exports exactly the types available (SpanContext,
 *   Surface). No Uuid or &str leaf-type gaps apply here beyond the standard
 *   set shared with all other domains.
 * =============================================================================
 */

import { z } from "zod";

// Re-export all generated schemas with stable, qualified names.

export { SurfaceSchema } from "./Surface.js";
export { SpanContextSchema } from "./SpanContext.js";

// Suppress unused-import warning — z is needed by generated files that omit their own import.
export { z };
