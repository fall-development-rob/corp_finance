/**
 * Tests for the Phase 34 Wave 1 Reasoning Bank.
 *
 * All vector operations use the deterministic embedder; the OpenAI embedder
 * is exercised via mock fetch only (no live network, no API key required).
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  createDeterministicEmbedder,
  createOpenAIEmbedder,
  createRuVectorBank,
  type ReasoningBank,
  type ReasoningEntry,
} from "../src/reasoning/index.js";

const DIM = 384;

function makeEntry(
  partial: Partial<Omit<ReasoningEntry, "embedding">> & { audit_id: string },
): Omit<ReasoningEntry, "embedding"> {
  return {
    agent_id: "chief-analyst",
    prompt_hash: "hash-" + partial.audit_id,
    prompt_summary: partial.prompt_summary ?? "default summary",
    tool_calls: [],
    delegations: [],
    result_excerpt: "ok",
    metadata: {},
    timestamp: new Date().toISOString(),
    ...partial,
  };
}

let tmpDir: string;
let bank: ReasoningBank | null = null;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "reasoning-bank-test-"));
  bank = null;
});

afterEach(async () => {
  if (bank) {
    try {
      await bank.close();
    } catch {
      // tolerated — close-after-close behaviour is documented as no-op
    }
    bank = null;
  }
  rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// Test 1 — deterministic embedder is deterministic
// ---------------------------------------------------------------------------

describe("createDeterministicEmbedder", () => {
  it("Test 1: same input → identical output, different inputs → different outputs", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    const a1 = await embed("hello world");
    const a2 = await embed("hello world");
    const b = await embed("goodbye world");

    expect(a1).toHaveLength(DIM);
    expect(a2).toHaveLength(DIM);
    expect(b).toHaveLength(DIM);

    // Identical for identical input.
    expect(a1).toEqual(a2);

    // Different for different input — at least one component differs.
    let differs = false;
    for (let i = 0; i < DIM; i++) {
      if (Math.abs((a1[i] ?? 0) - (b[i] ?? 0)) > 1e-9) {
        differs = true;
        break;
      }
    }
    expect(differs).toBe(true);

    // Unit-norm property.
    let sumSq = 0;
    for (const v of a1) sumSq += v * v;
    expect(Math.abs(Math.sqrt(sumSq) - 1)).toBeLessThan(1e-6);
  });
});

// ---------------------------------------------------------------------------
// Test 2 — bank index + recall round-trip
// ---------------------------------------------------------------------------

describe("createRuVectorBank — index + recall", () => {
  it("Test 2: round-trip; first prompt's recall returns its own entry as #1", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });

    const entries = [
      makeEntry({
        audit_id: "rt-1",
        prompt_summary: "Argentine convertible debenture pricing",
      }),
      makeEntry({
        audit_id: "rt-2",
        prompt_summary: "Brazilian sovereign credit spread analysis",
      }),
      makeEntry({
        audit_id: "rt-3",
        prompt_summary: "Chilean equity sector rotation strategy",
      }),
    ];
    for (const e of entries) {
      await bank.index(e);
    }

    const top = await bank.recallSimilar(
      "Argentine convertible debenture pricing",
    );
    expect(top.length).toBeGreaterThanOrEqual(1);
    expect(top[0]?.audit_id).toBe("rt-1");
    expect(top[0]?.prompt_summary).toBe(
      "Argentine convertible debenture pricing",
    );
    // Embedding round-trip preserves length.
    expect(top[0]?.embedding).toHaveLength(DIM);
  });

  // -------------------------------------------------------------------------
  // Test 3 — ordering follows similarity ranking
  // -------------------------------------------------------------------------

  it("Test 3: recall ordering follows embedding distance", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });

    const summaries = [
      "alpha beta gamma",
      "alpha beta delta",
      "alpha beta epsilon",
      "totally unrelated phrase about pizza",
      "another random sentence about clouds",
    ];
    for (let i = 0; i < summaries.length; i++) {
      await bank.index(
        makeEntry({
          audit_id: `ord-${i}`,
          prompt_summary: summaries[i] as string,
        }),
      );
    }

    // Query exactly equals summary 0 → should be returned first.
    const queryText = summaries[0] as string;
    const results = await bank.recallSimilar(queryText, { k: 5 });
    expect(results.length).toBe(5);
    expect(results[0]?.prompt_summary).toBe(queryText);

    // The two pizza/clouds entries should rank below at least one of the
    // alpha-beta-* entries — different inputs are far apart in cosine space.
    const positionsOfAlphaBeta = results
      .map((r, idx) => ({ idx, summary: r.prompt_summary }))
      .filter((r) => r.summary.startsWith("alpha beta"))
      .map((r) => r.idx);
    const positionsOfOther = results
      .map((r, idx) => ({ idx, summary: r.prompt_summary }))
      .filter((r) => !r.summary.startsWith("alpha beta"))
      .map((r) => r.idx);
    // Best-ranked alpha-beta entry comes before best-ranked other entry.
    expect(Math.min(...positionsOfAlphaBeta)).toBeLessThan(
      Math.min(...positionsOfOther),
    );
  });

  // -------------------------------------------------------------------------
  // Test 4 — agent_id filter
  // -------------------------------------------------------------------------

  it("Test 4: filter by agent_id returns only matching entries", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });

    await bank.index(
      makeEntry({
        audit_id: "f-x-1",
        agent_id: "X",
        prompt_summary: "shared topic phrase",
      }),
    );
    await bank.index(
      makeEntry({
        audit_id: "f-y-1",
        agent_id: "Y",
        prompt_summary: "shared topic phrase",
      }),
    );
    await bank.index(
      makeEntry({
        audit_id: "f-x-2",
        agent_id: "X",
        prompt_summary: "shared topic phrase variation",
      }),
    );

    const xs = await bank.recallSimilar("shared topic phrase", {
      k: 10,
      filter: { agent_id: "X" },
    });
    expect(xs.length).toBe(2);
    expect(xs.every((e) => e.agent_id === "X")).toBe(true);
    expect(xs.map((e) => e.audit_id).sort()).toEqual(["f-x-1", "f-x-2"]);
  });

  // -------------------------------------------------------------------------
  // Test 5 — empty index returns empty array
  // -------------------------------------------------------------------------

  it("Test 5: recallSimilar on empty index returns []", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    const results = await bank.recallSimilar("anything");
    expect(results).toEqual([]);
  });

  // -------------------------------------------------------------------------
  // Test 6 — recallByGraph (Wave 4 implementation)
  // -------------------------------------------------------------------------

  it("Test 6: recallByGraph returns [] on empty bank, structured query on populated bank", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });

    // Empty bank → empty result.
    const empty = await bank.recallByGraph({});
    expect(empty).toEqual([]);

    // Populate two entries with distinct metadata.
    await bank.index(
      makeEntry({
        audit_id: "g-1",
        prompt_summary: "Argentine convertible debenture",
        metadata: { jurisdiction: "AR", instrument: "convertible" },
      }),
    );
    await bank.index(
      makeEntry({
        audit_id: "g-2",
        prompt_summary: "Brazilian sovereign credit spread",
        metadata: { jurisdiction: "BR", instrument: "sovereign" },
      }),
    );

    // Filter by metadata returns only the matching entry.
    const ar = await bank.recallByGraph({
      metadata: { jurisdiction: "AR" },
    });
    expect(ar.length).toBe(1);
    expect(ar[0]?.audit_id).toBe("g-1");
  });

  // -------------------------------------------------------------------------
  // Test 7 — close prevents subsequent operations
  // -------------------------------------------------------------------------

  it("Test 7: close() prevents subsequent insert/search", async () => {
    const embed = createDeterministicEmbedder({ dim: DIM });
    bank = await createRuVectorBank({ dir: tmpDir, embed, dimensions: DIM });
    await bank.index(makeEntry({ audit_id: "pre-close" }));
    await bank.close();
    // Subsequent operations on the same bank reference must throw — RuVector
    // itself has no close() so we enforce a closed-flag at the wrapper layer.
    await expect(bank.recallSimilar("pre-close")).rejects.toThrow(/close/i);
    await expect(bank.index(makeEntry({ audit_id: "post-close" }))).rejects.toThrow(
      /close/i,
    );
    bank = null; // already closed; afterEach must not double-close
  });
});

// ---------------------------------------------------------------------------
// Test 8 — OpenAI embedder cache hits don't make HTTP calls
// ---------------------------------------------------------------------------

describe("createOpenAIEmbedder", () => {
  it("Test 8: cache hits do not make HTTP calls", async () => {
    let calls = 0;
    const fakeVec = new Array(8).fill(0).map((_, i) => i / 7);
    const mockFetch: typeof fetch = async () => {
      calls += 1;
      return new Response(
        JSON.stringify({ data: [{ embedding: fakeVec }] }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      );
    };
    const embed = createOpenAIEmbedder({
      apiKey: "sk-test-not-real",
      model: "text-embedding-3-small",
      fetchImpl: mockFetch,
    });

    const v1 = await embed("hello");
    const v2 = await embed("hello"); // cache hit
    const v3 = await embed("different"); // miss

    expect(v1).toEqual(fakeVec);
    expect(v2).toEqual(fakeVec);
    expect(v3).toEqual(fakeVec);
    expect(calls).toBe(2); // hello once, different once; second hello is cached
  });
});
