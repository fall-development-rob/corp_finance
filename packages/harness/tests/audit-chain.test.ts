/**
 * Tests for the file-based audit chain sink and chain verifier.
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { createFileAuditSink, verifyAuditChain } from "../src/audit/index.js";
import type { AuditRecord } from "../src/types.js";

function makeRecord(overrides: Partial<AuditRecord> & { audit_id: string }): AuditRecord {
  return {
    agent_id: "test-agent",
    prompt_hash: "abc123",
    tool_calls: [],
    result_hash: "def456",
    duration_ms: 100,
    total_tool_uses: 0,
    child_audit_ids: [],
    timestamp: new Date().toISOString(),
    ...overrides,
  };
}

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "audit-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

describe("createFileAuditSink", () => {
  it("Test 1: write + read round-trip", async () => {
    const sink = createFileAuditSink({ dir: tmpDir });
    const record = makeRecord({ audit_id: "round-trip-id" });
    await sink.write(record);
    const result = await sink.read("round-trip-id");
    expect(result).toEqual(record);
  });

  it("Test 2: list filtering by agentId", async () => {
    const sink = createFileAuditSink({ dir: tmpDir });
    const r1 = makeRecord({ audit_id: "id-a1", agent_id: "agent-A" });
    const r2 = makeRecord({ audit_id: "id-a2", agent_id: "agent-A" });
    const r3 = makeRecord({ audit_id: "id-b1", agent_id: "agent-B" });
    await sink.write(r1);
    await sink.write(r2);
    await sink.write(r3);
    const results = await sink.list({ agentId: "agent-A" });
    expect(results).toHaveLength(2);
    expect(results.every((r) => r.agent_id === "agent-A")).toBe(true);
  });

  it("Test 3: list filtering by since (Date)", async () => {
    const sink = createFileAuditSink({ dir: tmpDir });
    const base = new Date("2024-01-01T00:00:00.000Z");
    const r1 = makeRecord({ audit_id: "old-id", timestamp: new Date("2023-12-31T00:00:00.000Z").toISOString() });
    const r2 = makeRecord({ audit_id: "new-id-1", timestamp: new Date("2024-01-02T00:00:00.000Z").toISOString() });
    const r3 = makeRecord({ audit_id: "new-id-2", timestamp: new Date("2024-01-03T00:00:00.000Z").toISOString() });
    await sink.write(r1);
    await sink.write(r2);
    await sink.write(r3);
    const results = await sink.list({ since: base });
    expect(results).toHaveLength(2);
    expect(results.map((r) => r.audit_id).sort()).toEqual(["new-id-1", "new-id-2"]);
  });

  it("Test 4: read returns null for unknown id", async () => {
    const sink = createFileAuditSink({ dir: tmpDir });
    const result = await sink.read("nonexistent-id");
    expect(result).toBeNull();
  });

  it("Test 5: list with no filter returns all records sorted descending by timestamp", async () => {
    const sink = createFileAuditSink({ dir: tmpDir });
    const r1 = makeRecord({ audit_id: "ts-1", timestamp: new Date("2024-01-01T00:00:00.000Z").toISOString() });
    const r2 = makeRecord({ audit_id: "ts-2", timestamp: new Date("2024-01-03T00:00:00.000Z").toISOString() });
    const r3 = makeRecord({ audit_id: "ts-3", timestamp: new Date("2024-01-02T00:00:00.000Z").toISOString() });
    await sink.write(r1);
    await sink.write(r2);
    await sink.write(r3);
    const results = await sink.list();
    expect(results).toHaveLength(3);
    expect(results[0]?.audit_id).toBe("ts-2");
    expect(results[1]?.audit_id).toBe("ts-3");
    expect(results[2]?.audit_id).toBe("ts-1");
  });
});

describe("verifyAuditChain", () => {
  it("Test 6: happy path — 3 well-formed records (parent → 2 children)", () => {
    const parent = makeRecord({ audit_id: "parent-id", child_audit_ids: ["child-1", "child-2"] });
    const child1 = makeRecord({ audit_id: "child-1", parent_audit_id: "parent-id" });
    const child2 = makeRecord({ audit_id: "child-2", parent_audit_id: "parent-id" });
    const result = verifyAuditChain([parent, child1, child2]);
    expect(result.ok).toBe(true);
    expect(result.issues).toHaveLength(0);
  });

  it("Test 7: detects duplicate audit_id", () => {
    const r1 = makeRecord({ audit_id: "dup-id" });
    const r2 = makeRecord({ audit_id: "dup-id" });
    const result = verifyAuditChain([r1, r2]);
    expect(result.ok).toBe(false);
    expect(result.issues.some((i) => i.includes("duplicate audit_id"))).toBe(true);
  });

  it("Test 8: detects missing parent_audit_id", () => {
    const child = makeRecord({ audit_id: "orphan-id", parent_audit_id: "ghost-parent" });
    const result = verifyAuditChain([child]);
    expect(result.ok).toBe(false);
    expect(result.issues.some((i) => i.includes("ghost-parent"))).toBe(true);
  });

  it("Test 9: detects missing child_audit_ids entry", () => {
    const parent = makeRecord({ audit_id: "parent-id", child_audit_ids: ["missing-child"] });
    const result = verifyAuditChain([parent]);
    expect(result.ok).toBe(false);
    expect(result.issues.some((i) => i.includes("missing-child"))).toBe(true);
  });
});
