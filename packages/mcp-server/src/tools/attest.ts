import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import {
  attestIssue,
  attestVerify,
  attestList,
  attestRevoke,
  attestCheck,
} from "../bindings.js";
// Generated schemas: types with a public Rust struct and no critical runtime constraints.
// AttestationRevocationSchema is imported from generated — it correctly emits uuid + datetime.
import { AttestationRevocationSchema } from "../schemas/generated/attest/index.js";

// Hand-maintained schemas: kept for two reasons:
// 1. TrustAttestationSchema — generated omits signature (/^[0-9a-f]{128}$/) and
//    public_key (/^[0-9a-f]{64}$/) regex constraints; these are security-critical
//    and must be validated at the boundary.
// 2. AttestationStatusEnum — generated emits z.any().superRefine(...) instead of
//    z.enum([...]); hand-maintained form gives correct TypeScript inference.
// 3. All Input/Output schemas — no public Rust struct exists for these types;
//    they are defined inline in the federation MCP handler and cannot be auto-genned.
import {
  AttestIssueInputSchema,
  AttestVerifyInputSchema,
  AttestVerifyOutputSchema,
  AttestListInputSchema,
  AttestListOutputSchema,
  AttestRevokeInputSchema,
  AttestCheckInputSchema,
  AttestCheckOutputSchema,
  TrustAttestationSchema,
} from "../schemas/attest.js";
import { wrapResponse, coerceNumbers } from "../formatters/response.js";

export function registerAttestTools(server: McpServer) {
  server.tool(
    "attest_issue",
    "Issue a signed ed25519 trust attestation granting a subject tenant a set of capabilities, persist it to the SQLite store at db_path, and return the full TrustAttestation aggregate. RUF-FED-006 / ADR-019 v2.",
    AttestIssueInputSchema.shape,
    async (params) => {
      const validated = AttestIssueInputSchema.parse(coerceNumbers(params));
      const resultJson = attestIssue(JSON.stringify(validated));
      const parsed = TrustAttestationSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );

  server.tool(
    "attest_verify",
    "Load an attestation by ID from db_path, crypto-verify the ed25519 signature, and check the revocation table; returns attestation_id, expected_issuer, and status (valid/expired/revoked/bad_signature/issuer_mismatch). RUF-FED-007 / ADR-019 v2.",
    AttestVerifyInputSchema.shape,
    async (params) => {
      const validated = AttestVerifyInputSchema.parse(coerceNumbers(params));
      const resultJson = attestVerify(JSON.stringify(validated));
      const parsed = AttestVerifyOutputSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );

  server.tool(
    "attest_list",
    "List all trust attestations stored in db_path for a given subject_tenant; returns subject_tenant, count, and the full TrustAttestation array. RUF-FED-008 / ADR-019 v2.",
    AttestListInputSchema.shape,
    async (params) => {
      const validated = AttestListInputSchema.parse(coerceNumbers(params));
      const resultJson = attestList(JSON.stringify(validated));
      const parsed = AttestListOutputSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );

  server.tool(
    "attest_revoke",
    "Record a revocation tombstone for an attestation in db_path; subsequent attest_verify and attest_check calls will return revoked/false. RUF-FED-009 / ADR-019 v2.",
    AttestRevokeInputSchema.shape,
    async (params) => {
      const validated = AttestRevokeInputSchema.parse(coerceNumbers(params));
      const resultJson = attestRevoke(JSON.stringify(validated));
      // AttestationRevocationSchema from generated/attest: covers the revoke output
      // shape (attestation_id uuid, revoked_by, reason, revoked_at datetime).
      const parsed = AttestationRevocationSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );

  server.tool(
    "attest_check",
    "Return whether subject_tenant currently holds a valid, non-revoked, non-expired attestation granting capability from any issuer; crypto re-verified at every call per RUF-FED-008 / ADR-019 v2.",
    AttestCheckInputSchema.shape,
    async (params) => {
      const validated = AttestCheckInputSchema.parse(coerceNumbers(params));
      const resultJson = attestCheck(JSON.stringify(validated));
      const parsed = AttestCheckOutputSchema.parse(JSON.parse(String(resultJson)));
      return wrapResponse(JSON.stringify(parsed));
    }
  );
}
