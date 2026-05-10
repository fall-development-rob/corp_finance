/**
 * File-based SessionStore — Phase 32 Wave 2.
 *
 * Thin wrapper over FileJsonStore<SessionState>.
 */
import type { SessionState, SessionStore } from "../types.js";
import { createFileJsonStore } from "../persistence/index.js";

export interface FileSessionStoreOptions {
  dir: string;
}

function nowIso(): string {
  return new Date().toISOString();
}

export function createFileSessionStore(opts: FileSessionStoreOptions): SessionStore {
  const store = createFileJsonStore<SessionState>({
    dir: opts.dir,
    idOf: (s) => s.session_id,
    compare: (a, b) => b.updated_at.localeCompare(a.updated_at),
  });

  return {
    async save(state) {
      await store.save({ ...state, updated_at: nowIso() });
    },
    load: (sessionId) => store.load(sessionId),
    async list(filter?: { agentId?: string; status?: SessionState["status"] }) {
      return store.list((s) => {
        if (filter?.agentId !== undefined && s.agent_id !== filter.agentId) return false;
        if (filter?.status !== undefined && s.status !== filter.status) return false;
        return true;
      });
    },
    delete: (sessionId) => store.delete(sessionId),
  };
}
