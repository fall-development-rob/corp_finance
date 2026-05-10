/**
 * File-based audit sink implementation.
 *
 * Each AuditRecord is written as pretty-printed JSON to <dir>/<audit_id>.json.
 * Writes are atomic: temp file written then renamed.
 */
import { mkdir, readdir, readFile, rename, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { AuditRecord, AuditSink } from "../types.js";

function tmpPath(dir: string, auditId: string): string {
  return join(dir, `.${auditId}.json.tmp`);
}

function finalPath(dir: string, auditId: string): string {
  return join(dir, `${auditId}.json`);
}

async function ensureDir(dir: string): Promise<void> {
  await mkdir(dir, { recursive: true });
}

async function parseRecord(dir: string, filename: string): Promise<AuditRecord | null> {
  try {
    const raw = await readFile(join(dir, filename), "utf-8");
    return JSON.parse(raw) as AuditRecord;
  } catch {
    return null;
  }
}

export function createFileAuditSink(opts: { dir: string }): AuditSink {
  const { dir } = opts;

  return {
    async write(record: AuditRecord): Promise<void> {
      await ensureDir(dir);
      const tmp = tmpPath(dir, record.audit_id);
      const dest = finalPath(dir, record.audit_id);
      await writeFile(tmp, JSON.stringify(record, null, 2), "utf-8");
      await rename(tmp, dest);
    },

    async read(auditId: string): Promise<AuditRecord | null> {
      const record = await parseRecord(dir, `${auditId}.json`);
      return record;
    },

    async list(filter?: { agentId?: string; since?: Date }): Promise<AuditRecord[]> {
      await ensureDir(dir);
      let entries: string[];
      try {
        entries = await readdir(dir);
      } catch {
        return [];
      }

      const jsonFiles = entries.filter((f) => f.endsWith(".json") && !f.startsWith("."));
      const records = await Promise.all(jsonFiles.map((f) => parseRecord(dir, f)));
      let valid = records.filter((r): r is AuditRecord => r !== null);

      if (filter?.agentId !== undefined) {
        const id = filter.agentId;
        valid = valid.filter((r) => r.agent_id === id);
      }
      if (filter?.since !== undefined) {
        const since = filter.since.getTime();
        valid = valid.filter((r) => new Date(r.timestamp).getTime() >= since);
      }

      return valid.sort(
        (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime(),
      );
    },
  };
}
