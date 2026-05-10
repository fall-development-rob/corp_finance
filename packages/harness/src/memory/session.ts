/**
 * File-based SessionStore implementation — Phase 31 Wave 4.
 *
 * Each session is written as pretty-printed JSON at `<dir>/<session_id>.json`.
 * Writes are atomic: data goes to a `.tmp` file then renamed into place.
 */
import { mkdir, readdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { SessionState, SessionStore } from "../types.js";

export interface FileSessionStoreOptions {
  dir: string;
}

function tmpPath(filePath: string): string {
  return `${filePath}.tmp`;
}

async function writeAtomic(filePath: string, data: string): Promise<void> {
  const tmp = tmpPath(filePath);
  await writeFile(tmp, data, "utf8");
  await rename(tmp, filePath);
}

function sessionPath(dir: string, sessionId: string): string {
  return join(dir, `${sessionId}.json`);
}

function nowIso(): string {
  return new Date().toISOString();
}

export function createFileSessionStore(opts: FileSessionStoreOptions): SessionStore {
  const { dir } = opts;
  const ready = mkdir(dir, { recursive: true });

  async function save(state: SessionState): Promise<void> {
    await ready;
    const updated: SessionState = { ...state, updated_at: nowIso() };
    await writeAtomic(sessionPath(dir, state.session_id), JSON.stringify(updated, null, 2));
  }

  async function load(sessionId: string): Promise<SessionState | null> {
    await ready;
    try {
      const raw = await readFile(sessionPath(dir, sessionId), "utf8");
      return JSON.parse(raw) as SessionState;
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw err;
    }
  }

  async function list(
    filter?: { agentId?: string; status?: SessionState["status"] },
  ): Promise<SessionState[]> {
    await ready;
    const entries = await readdir(dir);
    const jsonFiles = entries.filter((f) => f.endsWith(".json"));

    const states = await Promise.all(
      jsonFiles.map(async (f) => {
        try {
          const raw = await readFile(join(dir, f), "utf8");
          return JSON.parse(raw) as SessionState;
        } catch {
          return null;
        }
      }),
    );

    return states
      .filter((s): s is SessionState => s !== null)
      .filter((s) => !filter?.agentId || s.agent_id === filter.agentId)
      .filter((s) => !filter?.status || s.status === filter.status)
      .sort((a, b) => b.updated_at.localeCompare(a.updated_at));
  }

  async function del(sessionId: string): Promise<void> {
    await ready;
    try {
      await rm(sessionPath(dir, sessionId));
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
    }
  }

  return { save, load, list, delete: del };
}
