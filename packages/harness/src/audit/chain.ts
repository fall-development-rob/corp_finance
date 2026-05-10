/**
 * File-based audit sink — Phase 32 Wave 2.
 *
 * Thin wrapper over FileJsonStore<AuditRecord>.
 */
import type { AuditRecord, AuditSink } from "../types.js";
import { createFileJsonStore } from "../persistence/index.js";

export function createFileAuditSink(opts: { dir: string }): AuditSink {
  const store = createFileJsonStore<AuditRecord>({
    dir: opts.dir,
    idOf: (r) => r.audit_id,
    compare: (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime(),
  });

  return {
    write: (record) => store.save(record),
    read: (auditId) => store.load(auditId),
    async list(filter?: { agentId?: string; since?: Date }) {
      return store.list((r) => {
        if (filter?.agentId !== undefined && r.agent_id !== filter.agentId) return false;
        if (filter?.since !== undefined && new Date(r.timestamp).getTime() < filter.since.getTime()) return false;
        return true;
      });
    },
  };
}
