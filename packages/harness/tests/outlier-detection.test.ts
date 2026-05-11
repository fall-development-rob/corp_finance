/**
 * Phase 41 Wave 2 — unit tests for outlier detection query functions.
 *
 * Uses a MockReasoningBank populated with synthetic ReasoningEntry records.
 * No real RuVector or file I/O (except watermark tests which use tmp files).
 *
 * Test plan (~14 cases):
 *  1. Empty bank → all detectors return [] and detectAllOutliers → clusters: []
 *  2. validation-failure: 5 entries with flag → cluster of 5
 *  3. validation-failure: cluster < minClusterSize → no cluster
 *  4. tool-thrashing: percentile computation sane threshold
 *  5. tool-thrashing: cluster grouping by agent_id
 *  6. delegation-mismatch: chief→specialist, specialist has validation_failed → cluster
 *  7. delegation-mismatch: child passed → no cluster
 *  8. novel-clusters: size ≥ minClusterSize → emits
 *  9. novel-clusters without embed fn → []
 * 10. cluster_id stability: same audit_id set → same cluster_id on two runs
 * 11. detectAllOutliers: runs all 4 in parallel, dedupes
 * 12. Watermark store: read returns undefined on missing file
 * 13. Watermark store: write + read round-trip
 * 14. confidence_score is in [0, 1]
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  computeClusterId,
  createFileWatermarkStore,
  detectAllOutliers,
  detectDelegationMismatch,
  detectNovelClusters,
  detectToolThrashing,
  detectValidationFailures,
  type OutlierDetectorOptions,
} from "../src/reasoning/outliers.js";
import type { ReasoningBank, ReasoningEntry } from "../src/reasoning/bank.js";
import { createDeterministicEmbedder } from "../src/reasoning/embeddings.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

type EntryPartial = Partial<Omit<ReasoningEntry, "embedding" | "audit_id">>;

function makeEntry(
  audit_id: string,
  partial: EntryPartial = {},
): ReasoningEntry {
  return {
    audit_id,
    agent_id: "chief-analyst",
    prompt_hash: "h-" + audit_id,
    prompt_summary: "Test dispatch summary for " + audit_id,
    embedding: [],
    tool_calls: [],
    delegations: [],
    result_excerpt: "ok",
    metadata: {},
    timestamp: new Date().toISOString(),
    ...partial,
  };
}

/**
 * MockReasoningBank: stores entries in memory, supports recallByGraph with
 * basic filtering (metadata equality, since/until, agent_id, hasDelegations).
 */
function makeMockBank(entries: ReasoningEntry[] = []): ReasoningBank {
  return {
    async index() {},

    async recallSimilar() {
      return [];
    },

    async recallByGraph(query) {
      const limit = query.limit ?? 500;
      let results = [...entries];

      if (query.agent_id !== undefined) {
        results = results.filter((e) => e.agent_id === query.agent_id);
      }
      if (query.since !== undefined) {
        results = results.filter(
          (e) => new Date(e.timestamp) >= query.since!,
        );
      }
      if (query.until !== undefined) {
        results = results.filter(
          (e) => new Date(e.timestamp) <= query.until!,
        );
      }
      if (query.metadata !== undefined) {
        for (const [k, v] of Object.entries(query.metadata)) {
          results = results.filter((e) => e.metadata[k] === v);
        }
      }
      if (query.hasDelegations !== undefined && query.hasDelegations.length > 0) {
        const want = new Set(query.hasDelegations);
        results = results.filter((e) =>
          e.delegations.some((d) => want.has(d)),
        );
      }
      if (query.hasTools !== undefined && query.hasTools.length > 0) {
        const want = new Set(query.hasTools);
        results = results.filter((e) =>
          e.tool_calls.some((tc) => want.has(tc.name)),
        );
      }

      return results.slice(0, limit);
    },

    async close() {},
  };
}

function makeOpts(
  entries: ReasoningEntry[],
  overrides: Partial<OutlierDetectorOptions> = {},
): OutlierDetectorOptions {
  const now = new Date();
  const from = new Date(now.getTime() - 14 * 24 * 60 * 60 * 1000);
  return {
    bank: makeMockBank(entries),
    window: { from, to: now },
    minClusterSize: 3,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Test 1: Empty bank → all detectors return [] and detectAllOutliers → clusters: []
// ---------------------------------------------------------------------------

describe("detectValidationFailures — empty bank", () => {
  it("returns [] when no entries exist", async () => {
    const result = await detectValidationFailures(makeOpts([]));
    expect(result).toEqual([]);
  });
});

describe("detectToolThrashing — empty bank", () => {
  it("returns [] when no entries exist", async () => {
    const result = await detectToolThrashing(makeOpts([]));
    expect(result).toEqual([]);
  });
});

describe("detectDelegationMismatch — empty bank", () => {
  it("returns [] when no entries exist", async () => {
    const result = await detectDelegationMismatch(makeOpts([]));
    expect(result).toEqual([]);
  });
});

describe("detectNovelClusters — empty bank", () => {
  it("returns [] when no entries exist", async () => {
    const embed = createDeterministicEmbedder();
    const result = await detectNovelClusters(makeOpts([], { embed }));
    expect(result).toEqual([]);
  });
});

describe("detectAllOutliers — empty bank", () => {
  it("returns clusters: [] when bank is empty", async () => {
    const report = await detectAllOutliers(makeOpts([]));
    expect(report.clusters).toEqual([]);
    expect(report.report_id).toBeTruthy();
    expect(report.run_window.from).toBeInstanceOf(Date);
    expect(report.run_window.to).toBeInstanceOf(Date);
  });
});

// ---------------------------------------------------------------------------
// Test 2: validation-failure — 5 entries with flag → cluster of 5
// ---------------------------------------------------------------------------

describe("detectValidationFailures — cluster of 5", () => {
  it("emits one cluster when 5 entries share agent_id and validation_failed", async () => {
    const entries = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`vf-${i}`, {
        agent_id: "equity-analyst",
        metadata: { validation_failed: true },
      }),
    );
    const opts = makeOpts(entries, { minClusterSize: 3 });
    const clusters = await detectValidationFailures(opts);
    expect(clusters).toHaveLength(1);
    expect(clusters[0]!.cluster_type).toBe("validation-failure");
    expect(clusters[0]!.motivating_audit_ids).toHaveLength(5);
    expect(clusters[0]!.affected_skill).toBe("equity-analyst");
  });
});

// ---------------------------------------------------------------------------
// Test 3: validation-failure — cluster < minClusterSize → no cluster
// ---------------------------------------------------------------------------

describe("detectValidationFailures — below minClusterSize", () => {
  it("emits nothing when fewer than minClusterSize share an agent", async () => {
    const entries = [
      makeEntry("vf-a", { agent_id: "equity-analyst", metadata: { validation_failed: true } }),
      makeEntry("vf-b", { agent_id: "equity-analyst", metadata: { validation_failed: true } }),
    ];
    const opts = makeOpts(entries, { minClusterSize: 3 });
    const clusters = await detectValidationFailures(opts);
    expect(clusters).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Test 4: tool-thrashing — percentile computation produces a sane threshold
// ---------------------------------------------------------------------------

describe("detectToolThrashing — percentile sanity", () => {
  it("correctly identifies p95 entries as thrashing", async () => {
    // 20 entries: 19 with 1 tool call, 1 with 100 tool calls.
    // p95 of [1, 1, ..., 1, 100] = 100 (the one outlier).
    // Agent has only 1 thrashing entry, which is below minClusterSize=3 → no cluster.
    const normal = Array.from({ length: 19 }, (_, i) =>
      makeEntry(`tc-normal-${i}`, {
        agent_id: "credit-analyst",
        tool_calls: [{ name: "tool-a", count: 1 }],
      }),
    );
    const heavy = makeEntry("tc-heavy", {
      agent_id: "credit-analyst",
      tool_calls: [{ name: "tool-a", count: 100 }],
    });
    const opts = makeOpts([...normal, heavy], { minClusterSize: 1 });
    const clusters = await detectToolThrashing(opts);
    // The one heavy entry qualifies; minClusterSize=1 so cluster is emitted.
    expect(clusters).toHaveLength(1);
    expect(clusters[0]!.cluster_type).toBe("tool-thrashing");
    expect(clusters[0]!.motivating_audit_ids).toContain("tc-heavy");
    expect(clusters[0]!.motivating_audit_ids).not.toContain("tc-normal-0");
  });
});

// ---------------------------------------------------------------------------
// Test 5: tool-thrashing — cluster grouping by agent_id
// ---------------------------------------------------------------------------

describe("detectToolThrashing — grouping by agent_id", () => {
  it("emits separate clusters for different agents above threshold", async () => {
    // Two agents each with 3 high-tool-use entries, plus many low entries.
    // Use toolThrashingPercentile: 50 (median) so the threshold is set by the
    // low half and the high half clearly exceeds it.
    // Entry counts: 7 low (count=1) + 3 high (count=100) per agent = 20 total.
    // Sorted: [1x14, 100x6]. p50: rank=ceil(10)=10, sorted[9]=1, threshold=1.
    // filter: > 1 → the 6 high entries qualify (3 per agent), satisfying minClusterSize=3.
    const lowA = Array.from({ length: 7 }, (_, i) =>
      makeEntry(`low-a-${i}`, { agent_id: "agent-a", tool_calls: [{ name: "t", count: 1 }] }),
    );
    const highA = Array.from({ length: 3 }, (_, i) =>
      makeEntry(`high-a-${i}`, { agent_id: "agent-a", tool_calls: [{ name: "t", count: 100 }] }),
    );
    const lowB = Array.from({ length: 7 }, (_, i) =>
      makeEntry(`low-b-${i}`, { agent_id: "agent-b", tool_calls: [{ name: "t", count: 1 }] }),
    );
    const highB = Array.from({ length: 3 }, (_, i) =>
      makeEntry(`high-b-${i}`, { agent_id: "agent-b", tool_calls: [{ name: "t", count: 100 }] }),
    );

    const entries = [...lowA, ...lowB, ...highA, ...highB];
    const opts = makeOpts(entries, { minClusterSize: 3, toolThrashingPercentile: 50 });
    const clusters = await detectToolThrashing(opts);

    const agentIds = clusters.map((c) => c.affected_skill).sort();
    expect(agentIds).toContain("agent-a");
    expect(agentIds).toContain("agent-b");
  });
});

// ---------------------------------------------------------------------------
// Test 6: delegation-mismatch — chief delegates, specialist has validation_failed
// ---------------------------------------------------------------------------

describe("detectDelegationMismatch — mismatch detected", () => {
  it("emits a cluster when chief delegates to a specialist with failures", async () => {
    // Specialist entries: 5 entries with validation_failed: true.
    const specialists = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`spec-${i}`, {
        agent_id: "equity-analyst",
        metadata: { validation_failed: true },
      }),
    );
    // Chief entries that delegate to equity-analyst.
    const chiefs = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`chief-${i}`, {
        agent_id: "chief-analyst",
        delegations: ["equity-analyst"],
      }),
    );

    const opts = makeOpts([...specialists, ...chiefs], { minClusterSize: 3 });
    const clusters = await detectDelegationMismatch(opts);

    expect(clusters).toHaveLength(1);
    expect(clusters[0]!.cluster_type).toBe("delegation-mismatch");
    expect(clusters[0]!.affected_skill).toContain("equity-analyst");
  });
});

// ---------------------------------------------------------------------------
// Test 7: delegation-mismatch — child validation passed → no cluster
// ---------------------------------------------------------------------------

describe("detectDelegationMismatch — no mismatch when child passed", () => {
  it("returns [] when specialist outputs all pass validation", async () => {
    const specialists = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`spec-ok-${i}`, {
        agent_id: "equity-analyst",
        metadata: { validation_failed: false },
      }),
    );
    const chiefs = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`chief-ok-${i}`, {
        agent_id: "chief-analyst",
        delegations: ["equity-analyst"],
      }),
    );

    const opts = makeOpts([...specialists, ...chiefs], { minClusterSize: 3 });
    const clusters = await detectDelegationMismatch(opts);
    expect(clusters).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Test 8: novel-clusters — size ≥ minClusterSize → emits
// ---------------------------------------------------------------------------

describe("detectNovelClusters — emits when sufficient novel entries", () => {
  it("emits a cluster when multiple novel dispatches share an agent_id", async () => {
    const embed = createDeterministicEmbedder({ dim: 32 });

    // Create entries with distinctly different prompt summaries → different embeddings.
    // The deterministic embedder produces different unit vectors for different texts.
    // We need multiple entries from the same agent that are all mutually dissimilar.
    const uniqueTexts = [
      "zygote cellular division embryology research quantum",
      "thermodynamic entropy black hole singularity spacetime",
      "macroeconomic fiscal policy quantitative easing inflation",
    ];

    const entries: ReasoningEntry[] = await Promise.all(
      uniqueTexts.map(async (text, i) => {
        const embedding = await embed(text);
        return makeEntry(`novel-${i}`, {
          agent_id: "chief-analyst",
          prompt_summary: text,
          embedding,
        });
      }),
    );

    // All entries have unique embeddings; we set a low novelThreshold so that
    // all qualify as novel relative to each other.
    // With threshold=0.0, cosine distance > 0 = novel for anything not identical.
    const opts = makeOpts(entries, {
      embed,
      novelThreshold: 0.0,
      minClusterSize: 3,
    });
    const clusters = await detectNovelClusters(opts);
    expect(clusters).toHaveLength(1);
    expect(clusters[0]!.cluster_type).toBe("novel");
    expect(clusters[0]!.motivating_audit_ids).toHaveLength(3);
  });
});

// ---------------------------------------------------------------------------
// Test 9: novel-clusters without embed fn → returns []
// ---------------------------------------------------------------------------

describe("detectNovelClusters — no embed fn", () => {
  it("returns [] quietly when opts.embed is absent", async () => {
    const entries = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`no-embed-${i}`, { agent_id: "chief-analyst" }),
    );
    // No embed fn in opts.
    const opts = makeOpts(entries, { embed: undefined });
    const clusters = await detectNovelClusters(opts);
    expect(clusters).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// Test 10: cluster_id stability — same audit_id set → same cluster_id
// ---------------------------------------------------------------------------

describe("computeClusterId — determinism", () => {
  it("produces identical cluster_id for the same audit_id set on two calls", () => {
    const auditIds = ["audit-zzz", "audit-aaa", "audit-mmm"];
    const id1 = computeClusterId(auditIds);
    const id2 = computeClusterId(auditIds);
    expect(id1).toBe(id2);
  });

  it("is order-independent (sorted internally)", () => {
    const forward = ["audit-a", "audit-b", "audit-c"];
    const reversed = ["audit-c", "audit-b", "audit-a"];
    expect(computeClusterId(forward)).toBe(computeClusterId(reversed));
  });

  it("starts with 'cluster-' prefix and has 8 hex chars after", () => {
    const id = computeClusterId(["audit-x", "audit-y", "audit-z"]);
    expect(id).toMatch(/^cluster-[0-9a-f]{8}$/);
  });
});

// ---------------------------------------------------------------------------
// Test 11: detectAllOutliers — runs all 4 in parallel, dedupes
// ---------------------------------------------------------------------------

describe("detectAllOutliers — deduplication", () => {
  it("combines results from all detectors and dedupes by cluster_id", async () => {
    // 5 validation-failure entries — this produces a validation-failure cluster.
    const entries = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`all-${i}`, {
        agent_id: "macro-analyst",
        metadata: { validation_failed: true },
      }),
    );
    const opts = makeOpts(entries, { minClusterSize: 3 });
    const report = await detectAllOutliers(opts);

    expect(report.clusters.length).toBeGreaterThanOrEqual(1);
    // All cluster_ids must be unique.
    const ids = report.clusters.map((c) => c.cluster_id);
    const unique = new Set(ids);
    expect(ids.length).toBe(unique.size);
  });

  it("returns a valid report shape", async () => {
    const report = await detectAllOutliers(makeOpts([]));
    expect(typeof report.report_id).toBe("string");
    expect(report.run_window).toHaveProperty("from");
    expect(report.run_window).toHaveProperty("to");
    expect(Array.isArray(report.clusters)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 12: Watermark store — read returns undefined on missing file
// ---------------------------------------------------------------------------

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "watermark-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

describe("createFileWatermarkStore — read on missing file", () => {
  it("returns undefined when the watermark file does not exist", async () => {
    const store = createFileWatermarkStore(join(tmpDir, "watermark.json"));
    const result = await store.read();
    expect(result).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Test 13: Watermark store — write + read round-trip
// ---------------------------------------------------------------------------

describe("createFileWatermarkStore — write/read round-trip", () => {
  it("reads back the same Date that was written", async () => {
    const path = join(tmpDir, "watermark.json");
    const store = createFileWatermarkStore(path);
    const now = new Date("2026-05-11T10:00:00.000Z");
    await store.write(now);
    const back = await store.read();
    expect(back).toBeInstanceOf(Date);
    expect(back!.toISOString()).toBe("2026-05-11T10:00:00.000Z");
  });

  it("overwrites previous watermark on second write", async () => {
    const path = join(tmpDir, "watermark.json");
    const store = createFileWatermarkStore(path);
    await store.write(new Date("2026-01-01T00:00:00.000Z"));
    const later = new Date("2026-05-11T12:00:00.000Z");
    await store.write(later);
    const back = await store.read();
    expect(back!.toISOString()).toBe("2026-05-11T12:00:00.000Z");
  });
});

// ---------------------------------------------------------------------------
// Test 14: confidence_score is in [0, 1] for all cluster types
// ---------------------------------------------------------------------------

describe("confidence_score — bounded to [0, 1]", () => {
  it("validation-failure confidence is in [0, 1]", async () => {
    const entries = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`conf-vf-${i}`, {
        agent_id: "agent-conf",
        metadata: { validation_failed: true },
      }),
    );
    const clusters = await detectValidationFailures(
      makeOpts(entries, { minClusterSize: 3 }),
    );
    for (const c of clusters) {
      expect(c.confidence_score).toBeGreaterThanOrEqual(0);
      expect(c.confidence_score).toBeLessThanOrEqual(1);
    }
  });

  it("tool-thrashing confidence is 0.6 (fixed heuristic)", async () => {
    const high = Array.from({ length: 3 }, (_, i) =>
      makeEntry(`conf-tt-${i}`, {
        agent_id: "agent-conf",
        tool_calls: [{ name: "t", count: 100 }],
      }),
    );
    const low = Array.from({ length: 10 }, (_, i) =>
      makeEntry(`conf-low-${i}`, {
        agent_id: "agent-conf",
        tool_calls: [{ name: "t", count: 1 }],
      }),
    );
    const clusters = await detectToolThrashing(
      makeOpts([...high, ...low], { minClusterSize: 3 }),
    );
    for (const c of clusters) {
      expect(c.confidence_score).toBe(0.6);
    }
  });

  it("delegation-mismatch confidence is 0.8 (fixed signal)", async () => {
    const specialists = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`conf-spec-${i}`, {
        agent_id: "equity-analyst",
        metadata: { validation_failed: true },
      }),
    );
    const chiefs = Array.from({ length: 5 }, (_, i) =>
      makeEntry(`conf-chief-${i}`, {
        agent_id: "chief-analyst",
        delegations: ["equity-analyst"],
      }),
    );
    const clusters = await detectDelegationMismatch(
      makeOpts([...specialists, ...chiefs], { minClusterSize: 3 }),
    );
    for (const c of clusters) {
      expect(c.confidence_score).toBe(0.8);
    }
  });
});
