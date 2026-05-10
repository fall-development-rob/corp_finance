/**
 * Generic file-backed JSON store — Phase 32 Wave 2.
 *
 * Each record is written to <dir>/<id>.<ext> as pretty JSON.
 * Atomic writes via .tmp + rename. Dir created lazily on first use.
 */
import { mkdir, readdir, readFile, rename, unlink, writeFile } from "node:fs/promises";
import { join } from "node:path";

export interface FileJsonStoreOptions<T> {
  /** Directory under which records are stored. Created if missing. */
  dir: string;
  /** File extension without leading dot. Default: "json". */
  ext?: string;
  /** Returns the record's file id (e.g., r => r.audit_id). */
  idOf: (record: T) => string;
  /** Optional comparator for list() ordering. */
  compare?: (a: T, b: T) => number;
}

export interface FileJsonStore<T> {
  /** Persist a record. Atomic via tmp + rename. */
  save(record: T): Promise<void>;
  /** Load a record by id. Returns null when the file does not exist. */
  load(id: string): Promise<T | null>;
  /** List all records, optionally filtered, sorted by compare if supplied. */
  list(filter?: (record: T) => boolean): Promise<T[]>;
  /** Remove a record by id. No-op when the file does not exist. */
  delete(id: string): Promise<void>;
}

export function createFileJsonStore<T>(opts: FileJsonStoreOptions<T>): FileJsonStore<T> {
  const { dir, idOf, compare } = opts;
  const ext = opts.ext ?? "json";
  const ready = mkdir(dir, { recursive: true });

  function filePath(id: string): string {
    return join(dir, `${id}.${ext}`);
  }

  async function save(record: T): Promise<void> {
    await ready;
    const id = idOf(record);
    const dest = filePath(id);
    const tmp = join(dir, `.${id}.${ext}.tmp`);
    await writeFile(tmp, JSON.stringify(record, null, 2), "utf-8");
    await rename(tmp, dest);
  }

  async function load(id: string): Promise<T | null> {
    await ready;
    try {
      const raw = await readFile(filePath(id), "utf-8");
      return JSON.parse(raw) as T;
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw err;
    }
  }

  async function list(filter?: (record: T) => boolean): Promise<T[]> {
    await ready;
    const entries = await readdir(dir);
    const jsonFiles = entries.filter((f) => f.endsWith(`.${ext}`) && !f.startsWith("."));

    const parsed: Array<T | null> = await Promise.all(
      jsonFiles.map(async (f): Promise<T | null> => {
        try {
          const raw = await readFile(join(dir, f), "utf-8");
          return JSON.parse(raw) as T;
        } catch {
          return null;
        }
      }),
    );

    let records: T[] = parsed.filter((r): r is T => r !== null);
    if (filter) records = records.filter(filter);
    if (compare) records = records.sort(compare);
    return records;
  }

  async function del(id: string): Promise<void> {
    await ready;
    try {
      await unlink(filePath(id));
    } catch (err) {
      if ((err as NodeJS.ErrnoException).code !== "ENOENT") throw err;
    }
  }

  return { save, load, list, delete: del };
}
