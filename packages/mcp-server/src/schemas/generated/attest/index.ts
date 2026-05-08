/**
 * Phase 29 Wave 12b — Generated zod schemas for the attest domain.
 *
 * AUTO-GENERATED — do not edit by hand. Re-run `npm run schemas:gen:attest` to refresh.
 * Source of truth: crates/corp-finance-core/src/federation/types.rs
 * Pipeline: schemars 1.2.1 (JsonSchema derive) → JSON Schema → json-schema-to-zod 2.8.1 → zod 3
 *
 * =============================================================================
 * Semantic diff: generated vs hand-maintained (schemas/attest.ts)
 * =============================================================================
 *
 * 1. signature / public_key regex constraints: hand-maintained adds
 *    `.regex(/^[0-9a-f]{128}$/)` and `.regex(/^[0-9a-f]{64}$/)` for strict
 *    format validation; generated emits plain `z.string()` with only a
 *    `.describe(...)` annotation (the regex is in the doc comment, not the JSON
 *    Schema type). Call sites that must validate hex format must keep importing
 *    TrustAttestationSchema from the hand-maintained schemas/attest.ts.
 *
 * 2. capabilities min(1): hand-maintained enforces `.array(z.string()).min(1)`;
 *    generated emits `z.array(z.string())` with no minimum.
 *
 * 3. issuer_tenant / subject_tenant min(1): hand-maintained enforces
 *    `.string().min(1)`; generated emits plain `z.string()`.
 *
 * 4. AttestationStatus tagged-union encoding: generated emits
 *    `z.any().superRefine(oneOf[...])` (same Wave 11 gap #3 pattern) rather
 *    than `z.enum([...])`. Both accept the same wire shape; hand-maintained
 *    AttestationStatusEnum uses `z.enum` for cleaner TypeScript inference.
 *
 * 5. Input / output types not auto-genned (no public Rust struct exists):
 *    AttestIssueInputSchema, AttestVerifyInputSchema, AttestVerifyOutputSchema,
 *    AttestListInputSchema, AttestListOutputSchema, AttestRevokeInputSchema,
 *    AttestRevokeOutputSchema, AttestCheckInputSchema, AttestCheckOutputSchema.
 *    These remain hand-maintained in schemas/attest.ts.
 *
 * 6. datetime format: generated emits `z.string().datetime({ offset: true })`
 *    for DateTime<Utc> fields (chrono RFC3339 with timezone offset); hand-
 *    maintained uses plain `z.string()`. Generated is more restrictive and
 *    correct; no adaptation needed at call sites.
 * =============================================================================
 */

import { z } from "zod";

// Re-export all generated schemas with stable, qualified names.

export { TrustAttestationSchema } from "./TrustAttestation.js";
export { AttestationStatusSchema } from "./AttestationStatus.js";
export { AttestationRevocationSchema } from "./AttestationRevocation.js";

// Suppress unused-import warning — z is needed by generated files that omit their own import.
export { z };
