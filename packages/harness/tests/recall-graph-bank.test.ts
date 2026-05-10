/**
 * Phase 34 Wave 4 — integration tests for `ReasoningBank.recallByGraph` and
 * the underlying `RVIndex.scan` JSONL sidecar.
 *
 * Index a varied corpus, then assert each filter primitive — metadata,
 * hasTools, hasDelegations, agent_id, since/until, combined filters,
 * limit clamping, sort-by-timestamp-desc.
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  createDeterministicEmbedder,
  createRuVectorBank,
  GRAPH_RECALL_DEFAULT_LIMIT,
  GRAPH_RECALL_MAX_LIMIT,
  openRVIndex,
  type ReasoningBank,
  type ReasoningEntry,
} from "../src/reasoning/index.js";

const DIM = 384;

function entry(
  audit_id: string,
  partial: Partial<Omit<ReasoningEntry, "embedding" | "audit_id">> = {},
): Omit<ReasoningEntry, "embedding"> {
  return {
    audit_id,
    agent_id: "chief-analyst",
    prompt_hash: "h-" + audit_id,
    prompt_summary: "summary " + audit_id,
    tool_calls: [],
    delegations: [],
    result_excerpt: "ok",
    metadata: {},
    timestamp: "2026-01-01T00:00:00.000Z",
    ...partial,
  };
}

let tmpDir: string;
let bank: ReasoningBank | null = null;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "recall-graph-bank-test-"));
  bank = null;
});

afterEach(async () => {
  if (bank) {
    try {
      await bank.close();
    } catch {
      // tolerated
    }
    bank = null;
  }
  rmSync(tmpDir, { recursive: true, force: true });
});

async function populate(b: ReasoningBank): Promise<void> {
  // 10 entries spanning jurisdictions, instruments, agents, tools,
  // delegations, and timestamps. Numbers chosen so each assertion below
  // narrows to a known subset.
  const fixtures: Omit<ReasoningEntry, "embedding">[] = [
    entry("e-01", {
      metadata: { jurisdiction: "AR", instrument: "convertible" },
      timestamp: "2026-01-05T00:00:00.000Z",
      tool_calls: [{ name: "option_pricer", count: 2 }],
      delegations: ["derivatives-analyst"],
    }),
    entry("e-02", {
      metadata: { jurisdiction: "AR", instrument: "sovereign" },
      timestamp: "2026-02-01T00:00:00.000Z",
      tool_calls: [{ name: "bond_pricer", count: 3 }],
      delegations: ["fixed-income-analyst"],
    }),
    entry("e-03", {
      metadata: { jurisdiction: "BR", instrument: "convertible" },
      timestamp: "2026-02-15T00:00:00.000Z",
      tool_calls: [
        { name: "option_pricer", count: 1 },
        { name: "wacc_calculator", count: 1 },
      ],
      delegations: ["derivatives-analyst", "equity-analyst"],
    }),
    entry("e-04", {
      agent_id: "equity-analyst",
      metadata: { jurisdiction: "US", instrument: "equity" },
      timestamp: "2026-03-01T00:00:00.000Z",
      tool_calls: [{ name: "wacc_calculator", count: 1 }],
      delegations: [],
    }),
    entry("e-05", {
      metadata: { jurisdiction: "US", instrument: "equity" },
      timestamp: "2026-03-10T00:00:00.000Z",
      tool_calls: [{ name: "dcf_model", count: 1 }],
      delegations: ["equity-analyst"],
    }),
    entry("e-06", {
      metadata: { jurisdiction: "AR", instrument: "convertible" },
      timestamp: "2026-04-01T00:00:00.000Z",
      tool_calls: [{ name: "option_pricer", count: 5 }],
      delegations: ["derivatives-analyst"],
    }),
    entry("e-07", {
      agent_id: "macro-analyst",
      metadata: { jurisdiction: "AR", instrument: "macro" },
      timestamp: "2026-04-20T00:00:00.000Z",
    }),
    entry("e-08", {
      metadata: { jurisdiction: "BR", instrument: "sovereign" },
      timestamp: "2026-05-01T00:00:00.000Z",
      tool_calls: [{ name: "bond_pricer", count: 2 }],
      delegations: ["fixed-income-analyst", "macro-analyst"],
    }),
    entry("e-09", {
      metadata: { jurisdiction: "AR", instrument: "convertible" },
      timestamp: "2026-05-05T00:00:00.000Z",
      tool_calls: [{ name: "option_pricer", count: 1 }],
      delegations: ["derivatives-analyst"],
    }),
    entry("e-10", {
      metadata: { jurisdiction: "EU", instrument: "muni" },
      timestamp: "2026-05-10T00:00:00.000Z",
      tool_calls: [{ name: "muni_bond_pricing", count: 1 }],
      delegations: [],
    }),
  ];
  for (const e of fixtures) {
    await b.index(e);
  }
}

// ---------------------------------------------------------------------------
// RVIndex.scan unit checks
// ---------------------------------------------------------------------------

describe("RVIndex.scan", () => {
  it("returns [] when nothing has been inserted", async () => {
    const idx = await openRVIndex({ dir: tmpDir, dimensions: DIM });
    try {
      const out = await idx.scan();
      expect(out).toEqual([]);
    } finally {
      await idx.close();
    }
  });

  it("returns inserted entries newest-first up to limit", async () => {
    const idx = await openRVIndex({ dir: tmpDir, dimensions: DIM });
    try {
      const e1: ReasoningEntry = {
        ...entry("s-1", { timestamp: "2026-01-01T00:00:00.000Z" }),
        embedding: new Array(DIM).fill(0.1),
      };
      const e2: ReasoningEntry = {
        ...entry("s-2", { timestamp: "2026-02-01T00:00:00.000Z" }),
        embedding: new Array(DIM).fill(0.2),
      };
      const e3: ReasoningEntry = {
        ...entry("s-3", { timestamp: "2026-03-01T00:00:00.000Z" }),
        embedding: new Array(DIM).fill(0.3),
      };
      await idx.insert(e1);
      await idx.insert(e2);
      await idx.insert(e3);

      const all = await idx.scan(undefined, 100);
      expect(all.map((e) => e.audit_id)).toEqual(["s-3", "s-2", "s-1"]);

      const justOne = await idx.scan(undefined, 1);
      expect(justOne).toHaveLength(1);
      expect(justOne[0]?.audit_id).toBe("s-3");
    } finally {
      await idx.close();
    }
  });

  it("filters by agent_id and time range", async () => {
    const idx = await openRVIndex({ dir: tmpDir, dimensions: DIM });
    try {
      await idx.insert({
        ...entry("a", { agent_id: "X", timestamp: "2026-01-01T00:00:00Z" }),
        embedding: new Array(DIM).fill(0.1),
      });
      await idx.insert({
        ...entry("b", { agent_id: "Y", timestamp: "2026-02-01T00:00:00Z" }),
        embedding: new Array(DIM).fill(0.2),
      });
      await idx.insert({
        ...entry("c", { agent_id: "X", timestamp: "2026-03-01T00:00:00Z" }),
        embedding: new Array(DIM).fill(0.3),
      });

      const xs = await idx.scan({ agent_id: "X" }, 100);
      expect(xs.map((e) => e.audit_id).sort()).toEqual(["a", "c"]);

      const since = await idx.scan(
        { since: new Date("2026-02-15T00:00:00Z") },
        100,
      );
      expect(since.map((e) => e.audit_id)).toEqual(["c"]);

      const until = await idx.scan(
        { until: new Date("2026-01-15T00:00:00Z") },
        100,
      );
      expect(until.map((e) => e.audit_id)).toEqual(["a"]);
    } finally {
      await idx.close();
    }
  });
});

// ---------------------------------------------------------------------------
// recallByGraph end-to-end
// ---------------------------------------------------------------------------

describe("ReasoningBank.recallByGraph", () => {
  it("metadata equality returns only Argentine entries", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const ar = await bank.recallByGraph({
      metadata: { jurisdiction: "AR" },
    });
    const ids = ar.map((e) => e.audit_id);
    // Five AR entries: e-01, e-02, e-06, e-07, e-09.
    expect(ids.sort()).toEqual(["e-01", "e-02", "e-06", "e-07", "e-09"]);
  });

  it("hasTools returns only entries that called the tool", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const opt = await bank.recallByGraph({ hasTools: ["option_pricer"] });
    const ids = opt.map((e) => e.audit_id).sort();
    // Four option_pricer callers: e-01, e-03, e-06, e-09.
    expect(ids).toEqual(["e-01", "e-03", "e-06", "e-09"]);
  });

  it("hasDelegations returns only entries that delegated there", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const drv = await bank.recallByGraph({
      hasDelegations: ["derivatives-analyst"],
    });
    const ids = drv.map((e) => e.audit_id).sort();
    // Four derivatives delegations: e-01, e-03, e-06, e-09.
    expect(ids).toEqual(["e-01", "e-03", "e-06", "e-09"]);
  });

  it("since returns only entries on or after the cutoff", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const recent = await bank.recallByGraph({
      since: new Date("2026-04-01T00:00:00.000Z"),
    });
    const ids = recent.map((e) => e.audit_id).sort();
    // April-onwards: e-06, e-07, e-08, e-09, e-10.
    expect(ids).toEqual(["e-06", "e-07", "e-08", "e-09", "e-10"]);
  });

  it("until returns only entries on or before the cutoff", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const old = await bank.recallByGraph({
      until: new Date("2026-02-15T00:00:00.000Z"),
    });
    const ids = old.map((e) => e.audit_id).sort();
    // Up to and including 2026-02-15: e-01, e-02, e-03.
    expect(ids).toEqual(["e-01", "e-02", "e-03"]);
  });

  it("combined filters return the intersection", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const ar_drv = await bank.recallByGraph({
      metadata: { jurisdiction: "AR" },
      hasDelegations: ["derivatives-analyst"],
    });
    const ids = ar_drv.map((e) => e.audit_id).sort();
    // AR ∩ derivatives-delegated: e-01, e-06, e-09.
    expect(ids).toEqual(["e-01", "e-06", "e-09"]);
  });

  it("filters by agent_id correctly", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const equity = await bank.recallByGraph({ agent_id: "equity-analyst" });
    expect(equity.map((e) => e.audit_id)).toEqual(["e-04"]);

    const macro = await bank.recallByGraph({ agent_id: "macro-analyst" });
    expect(macro.map((e) => e.audit_id)).toEqual(["e-07"]);
  });

  it("empty filter returns up to default limit, sorted by timestamp desc", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const all = await bank.recallByGraph({});
    // We have 10 entries; default limit is 50, so all return.
    expect(all).toHaveLength(10);
    // Timestamp descending: e-10 first.
    expect(all[0]?.audit_id).toBe("e-10");
    // Verify monotonically non-increasing timestamp.
    for (let i = 1; i < all.length; i++) {
      expect(all[i]!.timestamp <= all[i - 1]!.timestamp).toBe(true);
    }
  });

  it("respects an explicit limit", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const top3 = await bank.recallByGraph({ limit: 3 });
    expect(top3).toHaveLength(3);
    // Top 3 by timestamp desc: e-10, e-09, e-08.
    expect(top3.map((e) => e.audit_id)).toEqual(["e-10", "e-09", "e-08"]);
  });

  it("clamps limits to the hard cap", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const huge = await bank.recallByGraph({ limit: 99999 });
    // Caller asked for a million; corpus is 10. We get all 10, no error.
    expect(huge).toHaveLength(10);
    expect(GRAPH_RECALL_MAX_LIMIT).toBe(500);
    expect(GRAPH_RECALL_DEFAULT_LIMIT).toBe(50);
  });

  it("returns [] when no entry matches", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await populate(bank);

    const none = await bank.recallByGraph({
      metadata: { jurisdiction: "ZZ-fictional" },
    });
    expect(none).toEqual([]);
  });
});
