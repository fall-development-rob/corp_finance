import { z } from "zod";

/**
 * Federation bounded-context schemas. Mirrors
 * `crates/corp-finance-core/src/federation/types.rs`:
 *   - Tenant aggregate + TenantContext value object
 *   - PIIRedactionPolicy aggregate
 *   - TrustScore aggregate
 *
 * Per ADR-019 / RUF-FED-001..008 / FED-INV-001..008.
 */

const TrustTierEnum = z.enum(["open", "verified", "trusted"]);

const RedactionActionEnum = z.enum(["block", "redact", "hash", "pass"]);

const ResourceKindEnum = z.enum([
  "output",
  "memory",
  "cost-ledger",
  "session",
  "audit",
]);

const TenantSchema = z.object({
  tenant_id: z
    .string()
    .describe(
      "Lowercase kebab-case tenant id, unique within the registry (FED-INV-001)."
    ),
  display_name: z
    .string()
    .describe("Human-readable tenant name for surfaces and reports."),
  output_root: z
    .string()
    .describe(
      "Absolute path to the per-tenant root under <base>/<tenant_id>/ (FED-INV-008)."
    ),
  env_namespace: z
    .string()
    .describe(
      "Per-tenant environment-variable prefix (e.g. TENANT_ACME_) for plugin/CLI scoping."
    ),
  created_at: z
    .string()
    .optional()
    .describe("RFC3339 creation timestamp. Defaults to now() when omitted."),
  trust_tier: TrustTierEnum.describe(
    "Federation trust posture: open / verified / trusted (FED-INV-005)."
  ),
});

const TenantContextSchema = z.object({
  tenant_id: z.string().describe("Tenant id threaded through the surface."),
  output_root: z
    .string()
    .describe("Absolute path to the tenant's output_root directory."),
  env_namespace: z
    .string()
    .describe("Per-tenant env-var prefix carried through the dispatch."),
});

export const TenantProvisionSchema = z.object({
  tenant: TenantSchema.describe("Tenant aggregate to provision on disk."),
  base_dir: z
    .string()
    .describe(
      "Absolute base directory under which <tenant_id>/{out,memory,cost-ledger,session,audit}/ is created."
    ),
});

export const TenantResolveCliSchema = z.object({
  args: z
    .array(z.string())
    .describe(
      "Raw CLI argv slice. The resolver looks for --tenant <id> / --tenant=<id>, then falls back to CFA_TENANT_ID."
    ),
});

export const TenantScopedPathSchema = z.object({
  context: TenantContextSchema.describe(
    "TenantContext threaded from the surface boundary."
  ),
  kind: ResourceKindEnum.describe(
    "Resource subtree: output / memory / cost-ledger / session / audit."
  ),
  name: z
    .string()
    .describe(
      "Filename appended verbatim under the resource subtree; not normalised here (caller validates)."
    ),
});

const PIIRedactionPolicySchema = z.object({
  tier: TrustTierEnum.describe(
    "Trust tier this policy binds to (open/verified/trusted)."
  ),
  action: RedactionActionEnum.describe(
    "Action applied to each covered finding: block / redact / hash / pass."
  ),
  categories: z
    .array(z.string())
    .describe(
      "PII categories the action applies to (subset of the 14-category set; categories not listed receive implicit pass)."
    ),
});

export const PiiRedactApplySchema = z.object({
  text: z
    .string()
    .describe("Outbound text scanned for PII before policy application."),
  policy: PIIRedactionPolicySchema.describe(
    "PIIRedactionPolicy aggregate to apply (per-tenant)."
  ),
});

export const TrustScoreComputeSchema = z.object({
  success_rate: z
    .number()
    .min(0)
    .max(1)
    .describe("Peer success rate in [0, 1] (40% weight in composite)."),
  uptime: z
    .number()
    .min(0)
    .max(1)
    .describe("Peer uptime in [0, 1] (20% weight in composite)."),
  threat_score: z
    .number()
    .min(0)
    .max(1)
    .describe(
      "Peer threat indicator in [0, 1]; contributes inversely (20% weight via 1 - threat_score)."
    ),
  integrity_score: z
    .number()
    .min(0)
    .max(1)
    .describe("Peer integrity score in [0, 1] (20% weight in composite)."),
});
