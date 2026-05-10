/**
 * Tests for the generic FileJsonStore primitive — Phase 32 Wave 2.
 */
import { mkdtempSync, rmSync, existsSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createFileJsonStore } from "../src/persistence/index.js";

interface TestRecord {
  id: string;
  value: string;
  ts: string;
}

function makeRecord(id: string, value = "v", ts = new Date().toISOString()): TestRecord {
  return { id, value, ts };
}

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "fjstore-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

describe("FileJsonStore", () => {
  it("save + load round-trip", async () => {
    const store = createFileJsonStore<TestRecord>({ dir: join(tmpDir, "data"), idOf: (r) => r.id });
    const rec = makeRecord("r1", "hello");
    await store.save(rec);
    const loaded = await store.load("r1");
    expect(loaded).toEqual(rec);
  });

  it("load on missing id returns null", async () => {
    const store = createFileJsonStore<TestRecord>({ dir: join(tmpDir, "data"), idOf: (r) => r.id });
    expect(await store.load("nonexistent")).toBeNull();
  });

  it("list returns all records", async () => {
    const store = createFileJsonStore<TestRecord>({ dir: tmpDir, idOf: (r) => r.id });
    await store.save(makeRecord("a"));
    await store.save(makeRecord("b"));
    await store.save(makeRecord("c"));
    const all = await store.list();
    expect(all).toHaveLength(3);
  });

  it("list with filter returns subset", async () => {
    const store = createFileJsonStore<TestRecord>({ dir: tmpDir, idOf: (r) => r.id });
    await store.save(makeRecord("x1", "keep"));
    await store.save(makeRecord("x2", "drop"));
    await store.save(makeRecord("x3", "keep"));
    const kept = await store.list((r) => r.value === "keep");
    expect(kept).toHaveLength(2);
    expect(kept.map((r) => r.id).sort()).toEqual(["x1", "x3"]);
  });

  it("list with compare returns sorted order", async () => {
    const store = createFileJsonStore<TestRecord>({
      dir: tmpDir,
      idOf: (r) => r.id,
      compare: (a, b) => b.ts.localeCompare(a.ts),
    });
    await store.save(makeRecord("a", "v", "2024-01-01T00:00:00.000Z"));
    await store.save(makeRecord("b", "v", "2024-03-01T00:00:00.000Z"));
    await store.save(makeRecord("c", "v", "2024-02-01T00:00:00.000Z"));
    const sorted = await store.list();
    expect(sorted.map((r) => r.id)).toEqual(["b", "c", "a"]);
  });

  it("delete removes the file", async () => {
    const store = createFileJsonStore<TestRecord>({ dir: tmpDir, idOf: (r) => r.id });
    await store.save(makeRecord("del-me"));
    await store.delete("del-me");
    expect(await store.load("del-me")).toBeNull();
  });

  it("delete on missing id is a no-op (does not throw)", async () => {
    const store = createFileJsonStore<TestRecord>({ dir: tmpDir, idOf: (r) => r.id });
    await expect(store.delete("ghost")).resolves.toBeUndefined();
  });

  it("mkdir is created automatically when dir is missing", async () => {
    const newDir = join(tmpDir, "nested", "deep", "dir");
    const store = createFileJsonStore<TestRecord>({ dir: newDir, idOf: (r) => r.id });
    await store.save(makeRecord("auto-mkdir"));
    expect(existsSync(newDir)).toBe(true);
    expect(await store.load("auto-mkdir")).not.toBeNull();
  });

  it("atomic write — .tmp file does not appear in list()", async () => {
    const store = createFileJsonStore<TestRecord>({ dir: tmpDir, idOf: (r) => r.id });
    // Introduce a stray .tmp file to simulate a crashed write
    const { writeFile } = await import("node:fs/promises");
    await writeFile(join(tmpDir, ".stray.json.tmp"), "{}", "utf-8");
    await store.save(makeRecord("real"));
    const all = await store.list();
    expect(all).toHaveLength(1);
    expect(all[0].id).toBe("real");
  });

  it("list with filter and compare composes correctly", async () => {
    const store = createFileJsonStore<TestRecord>({
      dir: tmpDir,
      idOf: (r) => r.id,
      compare: (a, b) => a.id.localeCompare(b.id),
    });
    await store.save(makeRecord("z", "keep", "2024-01-01T00:00:00.000Z"));
    await store.save(makeRecord("a", "keep", "2024-03-01T00:00:00.000Z"));
    await store.save(makeRecord("m", "drop", "2024-02-01T00:00:00.000Z"));
    const result = await store.list((r) => r.value === "keep");
    expect(result.map((r) => r.id)).toEqual(["a", "z"]);
  });
});
