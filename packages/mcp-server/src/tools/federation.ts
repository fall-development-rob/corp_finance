import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  tenantProvision,
  tenantResolveCli,
  tenantScopedPath,
  piiRedactApply,
  trustScoreCompute,
} from "../bindings.js";
import {
  TenantProvisionSchema,
  TenantResolveCliSchema,
  TenantScopedPathSchema,
  PiiRedactApplySchema,
  TrustScoreComputeSchema,
} from "../schemas/federation.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerFederationTools(server: McpServer) {
  server.tool(
    "tenant_provision",
    "Provision a Tenant on disk under <base_dir>/<tenant_id>/{out,memory,cost-ledger,session,audit}/ with POSIX 0700 isolation. Input: Tenant aggregate and absolute base_dir. Output: materialised TenantContext threaded through downstream surfaces. RUF-FED-001 / FED-INV-001 / FED-INV-003.",
    TenantProvisionSchema.shape,
    async (params) => {
      const validated = TenantProvisionSchema.parse(coerceNumbers(params));
      const result = tenantProvision(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "tenant_resolve_cli",
    "Resolve the active TenantContext from CLI argv (--tenant <id> / --tenant=<id>) with CFA_TENANT_ID fallback. Input: argv string array. Output: TenantContext or null when neither flag nor env var is present. RUF-FED-002 / FED-INV-007.",
    TenantResolveCliSchema.shape,
    async (params) => {
      const validated = TenantResolveCliSchema.parse(coerceNumbers(params));
      const result = tenantResolveCli(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "tenant_scoped_path",
    "Compose a tenant-scoped path under one of the five resource subtrees (output / memory / cost-ledger / session / audit). Input: TenantContext, resource kind enum, filename. Output: absolute joined path; callers must still call enforce_isolation. RUF-FED-003 / FED-INV-008.",
    TenantScopedPathSchema.shape,
    async (params) => {
      const validated = TenantScopedPathSchema.parse(coerceNumbers(params));
      const result = tenantScopedPath(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "pii_redact_apply",
    "Apply a PIIRedactionPolicy to outbound text using the security module's 14-category scanner, walking findings right-to-left so byte offsets remain valid. Input: text and PIIRedactionPolicy. Output: redacted text, findings_count, action taken, and per-category counts. RUF-FED-004 / FED-INV-005.",
    PiiRedactApplySchema.shape,
    async (params) => {
      const validated = PiiRedactApplySchema.parse(coerceNumbers(params));
      const result = piiRedactApply(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );

  server.tool(
    "trust_score_compute",
    "Compute a peer's composite TrustScore as 0.4*success + 0.2*uptime + 0.2*(1-threat) + 0.2*integrity, clamping each input to [0, 1]. Input: success_rate, uptime, threat_score, integrity_score. Output: TrustScore with composite in [0, 1]. RUF-FED-005 / FED-INV-004.",
    TrustScoreComputeSchema.shape,
    async (params) => {
      const validated = TrustScoreComputeSchema.parse(coerceNumbers(params));
      const result = trustScoreCompute(JSON.stringify(validated));
      return wrapResponse(result);
    }
  );
}
