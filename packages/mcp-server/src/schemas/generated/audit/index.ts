/**
 * Phase 29 Wave 13 — Generated zod schemas for the audit domain.
 *
 * AUTO-GENERATED — do not edit by hand. Re-run `npm run schemas:gen:audit` to refresh.
 * Source of truth: crates/corp-finance-core/src/audit/ (surface_audit.rs, audit_manifest.rs)
 * Pipeline: schemars 1.2.1 (JsonSchema derive) → JSON Schema → json-schema-to-zod 2.8.1 → zod 3
 *
 * =============================================================================
 * Semantic diff: generated vs hand-maintained (schemas/audit.ts) — gaps observed
 * =============================================================================
 *
 * 1. Surface string-enum rewrite (NEW gap — audit-specific):
 *    The discriminated-union post-processor fires on `Surface` and on the
 *    `surface` field inside `SurfaceManifest` and `AuditManifest`, emitting
 *    `z.discriminatedUnion("kind", [z.literal("cli"), ...])`. This is invalid
 *    zod — `discriminatedUnion` requires object schemas, not bare literals.
 *    The hand-maintained `schemas/audit.ts` correctly uses
 *    `z.enum(["cli", "mcp", "skill", "plugin"])`. Call sites must use the
 *    hand-maintained `SurfaceAuditComputeSchema` for `surface` field validation
 *    rather than `SurfaceManifestSchema` or `AuditManifestSchema` directly.
 *    Resolution path: extend the gen-audit-schemas.mjs post-processor with a
 *    second pass that detects bare-literal arrays inside discriminatedUnion and
 *    rewrites them back to z.enum([...]).
 *
 * 2. Optional vs nullable (same as office/attest gap):
 *    Generated emits `z.union([z.string(), z.null()]).optional()` for
 *    `Option<String>` fields (`tenant_id`); hand-maintained uses `.optional()`
 *    only. Zod parse accepts both; the difference surfaces only when the caller
 *    passes explicit `null`.
 *
 * 3. DateTime as z.string().datetime() (same as known chrono gap):
 *    `DateTime<Utc>` fields (`ts` on `ToolCallRecord` and `AuditManifest`) are
 *    emitted as `z.string().datetime({ offset: true })`. The Rust type is a
 *    UTC timestamp; callers constructing manifests must supply ISO-8601 strings.
 *    Already covered by Wave 12 gap inventory.
 *
 * 4. PathBuf as z.string() (new surface for this domain):
 *    `output_path: PathBuf` emits as `z.string()` with no path-format
 *    constraint. The hand-maintained schema does not expose `output_path`
 *    directly (the MCP tool input is `manifest`, not a raw path). No call-site
 *    regression; documented for completeness.
 *
 * 5. serde_json::Value as z.any() (known gap — Wave 11 baseline):
 *    `command_args: serde_json::Value` emits as `z.any()`. No tighter
 *    constraint possible without domain knowledge.
 *
 * The known Wave 11 gaps (5 total) + Wave 12 chrono/uuid gap = 6 baseline gaps.
 * New gaps introduced by the audit domain: gaps 1 (string-enum rewrite misfire)
 * and 4 (PathBuf) are net-new. Gaps 2, 3, 5 are pre-existing.
 * =============================================================================
 */

import { z } from "zod";

// Re-export all generated schemas with stable, qualified names.
// NOTE: SurfaceSchema, SurfaceManifestSchema, and AuditManifestSchema contain
// an invalid z.discriminatedUnion("kind", [z.literal(...)]) pattern for the
// Surface string-enum (gap 1 above). Import them for informational/tooling use
// only; for runtime parse validation use the hand-maintained SurfaceAuditComputeSchema
// from schemas/audit.js.

export { SurfaceSchema } from "./Surface.js";
export { SurfaceManifestSchema } from "./SurfaceManifest.js";
export { ToolCallRecordSchema } from "./ToolCallRecord.js";
export { AuditManifestSchema } from "./AuditManifest.js";

// Suppress unused-import warning — z is needed by generated files that omit their own import.
export { z };
