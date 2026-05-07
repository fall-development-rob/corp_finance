import { z } from "zod";

/**
 * Trust attestation schemas for Phase 29 Wave 4 (ADR-019 v2, RUF-FED-006..009).
 *
 * Mirrors `crates/corp-finance-core/src/federation/types.rs`:
 *   - TrustAttestation aggregate
 *   - AttestationStatus enum (snake_case serde)
 *   - Input schemas for five binding surfaces
 */

export const AttestationStatusEnum = z.enum([
  "valid",
  "expired",
  "revoked",
  "bad_signature",
  "issuer_mismatch",
]);

export const TrustAttestationSchema = z.object({
  attestation_id: z.string().uuid(),
  issuer_tenant: z.string().min(1),
  subject_tenant: z.string().min(1),
  capabilities: z.array(z.string()).min(1),
  issued_at: z.string(),
  expires_at: z.string(),
  signature: z.string().regex(/^[0-9a-f]{128}$/),
  public_key: z.string().regex(/^[0-9a-f]{64}$/),
});

export const AttestIssueInputSchema = z.object({
  issuer_tenant: z.string().min(1),
  subject_tenant: z.string().min(1),
  capabilities: z.array(z.string().min(1)).min(1),
  valid_for_hours: z.number().positive().default(24),
  db_path: z.string().min(1),
  keys_dir: z.string().min(1),
});

export const AttestVerifyInputSchema = z.object({
  attestation_id: z.string().uuid(),
  expected_issuer: z.string().min(1),
  db_path: z.string().min(1),
});

export const AttestVerifyOutputSchema = z.object({
  attestation_id: z.string().uuid(),
  expected_issuer: z.string().min(1),
  status: AttestationStatusEnum,
});

export const AttestListInputSchema = z.object({
  subject_tenant: z.string().min(1),
  db_path: z.string().min(1),
});

export const AttestListOutputSchema = z.object({
  subject_tenant: z.string().min(1),
  count: z.number().int().nonnegative(),
  attestations: z.array(TrustAttestationSchema),
});

export const AttestRevokeInputSchema = z.object({
  attestation_id: z.string().uuid(),
  revoked_by: z.string().min(1),
  reason: z.string().min(1),
  db_path: z.string().min(1),
});

export const AttestRevokeOutputSchema = z.object({
  attestation_id: z.string().uuid(),
  revoked_by: z.string().min(1),
  reason: z.string().min(1),
  revoked_at: z.string(),
});

export const AttestCheckInputSchema = z.object({
  subject_tenant: z.string().min(1),
  capability: z.string().min(1),
  db_path: z.string().min(1),
});

export const AttestCheckOutputSchema = z.object({
  subject_tenant: z.string().min(1),
  capability: z.string().min(1),
  granted: z.boolean(),
});
