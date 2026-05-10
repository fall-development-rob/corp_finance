/**
 * Audit module barrel export.
 *
 * Exports the file-based AuditSink factory and the chain verifier.
 */
export { createFileAuditSink } from "./chain.js";

import type { AuditRecord } from "../types.js";

export interface VerifyResult {
  ok: boolean;
  issues: string[];
}

/**
 * Verify the integrity of a set of AuditRecord objects.
 *
 * Checks:
 *   - Every audit_id is unique.
 *   - Every parent_audit_id references a record in the input list.
 *   - Every child_audit_ids[] entry exists in the input list.
 */
export function verifyAuditChain(records: AuditRecord[]): VerifyResult {
  const issues: string[] = [];
  const idSet = new Set<string>();

  for (const r of records) {
    if (idSet.has(r.audit_id)) {
      issues.push(`duplicate audit_id: ${r.audit_id}`);
    }
    idSet.add(r.audit_id);
  }

  for (const r of records) {
    if (r.parent_audit_id !== undefined && !idSet.has(r.parent_audit_id)) {
      issues.push(
        `record ${r.audit_id} references missing parent_audit_id: ${r.parent_audit_id}`,
      );
    }
    for (const childId of r.child_audit_ids) {
      if (!idSet.has(childId)) {
        issues.push(
          `record ${r.audit_id} references missing child_audit_id: ${childId}`,
        );
      }
    }
  }

  return { ok: issues.length === 0, issues };
}
