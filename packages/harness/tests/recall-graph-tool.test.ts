/**
 * Phase 34 Wave 4 — unit tests for the `recall_by_graph` virtual tool.
 *
 * Covers tool spec shape, name predicate, formatter, and
 * `executeRecallGraphCall` happy path / malformed input / bank-throws /
 * since/until coercion / limit clamping.
 */
import { describe, expect, it, vi } from "vitest";
import {
  RECALL_GRAPH_TOOL_NAME,
  createRecallGraphTool,
  executeRecallGraphCall,
  formatRecallGraphResult,
  isRecallGraphToolName,
} from "../src/reasoning/recall-graph-tool.js";
import {
  GRAPH_RECALL_DEFAULT_LIMIT,
  GRAPH_RECALL_MAX_LIMIT,
  type GraphRecallQuery,
  type ReasoningBank,
  type ReasoningEntry,
} from "../src/reasoning/bank.js";
import type { ToolCall } from "../src/types.js";

function makeEntry(
  partial: Partial<ReasoningEntry> & { audit_id: string },
): ReasoningEntry {
  return {
    agent_id: "chief-analyst",
    prompt_hash: "h-" + partial.audit_id,
    prompt_summary: "summary " + partial.audit_id,
    embedding: [],
    tool_calls: [],
    delegations: [],
    result_excerpt: "ok",
    metadata: {},
    timestamp: "2026-05-10T00:00:00.000Z",
    ...partial,
  };
}

function makeBank(over: Partial<ReasoningBank>): ReasoningBank {
  return {
    async index() {},
    async recallSimilar() {
      return [];
    },
    async recallByGraph() {
      return [];
    },
    async close() {},
    ...over,
  };
}

// ---------------------------------------------------------------------------
// 1. Tool spec shape
// ---------------------------------------------------------------------------

describe("createRecallGraphTool", () => {
  it("returns a CanonicalTool with the expected name and schema", () => {
    const tool = createRecallGraphTool();
    expect(tool.name).toBe(RECALL_GRAPH_TOOL_NAME);
    expect(tool.name).toBe("recall_by_graph");
    expect(tool.input_schema.type).toBe("object");
    const props = tool.input_schema.properties;
    expect(props).toHaveProperty("agent_id");
    expect(props).toHaveProperty("metadata");
    expect(props).toHaveProperty("hasTools");
    expect(props).toHaveProperty("hasDelegations");
    expect(props).toHaveProperty("since");
    expect(props).toHaveProperty("until");
    expect(props).toHaveProperty("limit");
    // No required fields — every filter is optional by design.
    expect(tool.input_schema.required).toBeUndefined();
    // Description must mention the differentiator vs recall_similar.
    expect(tool.description.toLowerCase()).toContain("structured");
    expect(tool.description.toLowerCase()).toContain("recall_similar");
  });
});

// ---------------------------------------------------------------------------
// 2. isRecallGraphToolName predicate
// ---------------------------------------------------------------------------

describe("isRecallGraphToolName", () => {
  it("matches only the exact `recall_by_graph` name", () => {
    expect(isRecallGraphToolName("recall_by_graph")).toBe(true);
    expect(isRecallGraphToolName("recall_similar")).toBe(false);
    expect(isRecallGraphToolName("recall_by_graph_v2")).toBe(false);
    expect(isRecallGraphToolName("RECALL_BY_GRAPH")).toBe(false);
    expect(isRecallGraphToolName("")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// 3. formatRecallGraphResult
// ---------------------------------------------------------------------------

describe("formatRecallGraphResult", () => {
  it("returns the explicit no-results sentinel for empty input", () => {
    expect(formatRecallGraphResult([])).toBe(
      "No matching prior dispatches found.",
    );
  });

  it("renders audit_id, summary, tool_calls, delegations, metadata", () => {
    const a = makeEntry({
      audit_id: "g-1",
      prompt_summary: "Argentine convertible",
      metadata: { jurisdiction: "AR", instrument: "convertible" },
    });
    const b = makeEntry({
      audit_id: "g-2",
      prompt_summary: "Brazilian sovereign",
      tool_calls: [{ name: "bond_pricer", count: 2 }],
      delegations: ["fixed-income-analyst"],
    });
    const out = formatRecallGraphResult([a, b]);
    expect(out).toContain("Matched 2 prior dispatches");
    expect(out).toContain("g-1");
    expect(out).toContain("g-2");
    expect(out).toContain("Argentine convertible");
    expect(out).toContain("Brazilian sovereign");
    expect(out).toContain("bond_pricer×2");
    expect(out).toContain("fixed-income-analyst");
    expect(out).toContain("jurisdiction");
  });

  it("uses singular 'dispatch' for exactly one match", () => {
    const out = formatRecallGraphResult([makeEntry({ audit_id: "solo" })]);
    expect(out).toContain("Matched 1 prior dispatch:");
    expect(out).not.toContain("dispatches");
  });
});

// ---------------------------------------------------------------------------
// 4. executeRecallGraphCall — happy path
// ---------------------------------------------------------------------------

describe("executeRecallGraphCall — happy path", () => {
  it("calls bank.recallByGraph with the parsed query and formats results", async () => {
    const entries = [
      makeEntry({
        audit_id: "h-1",
        prompt_summary: "AAPL DCF",
        metadata: { issuer: "AAPL" },
      }),
    ];
    const captured: { q?: GraphRecallQuery } = {};
    const bank = makeBank({
      recallByGraph: async (q) => {
        captured.q = q;
        return entries;
      },
    });
    const call: ToolCall = {
      id: "g-call-1",
      name: "recall_by_graph",
      input: {
        agent_id: "chief-analyst",
        metadata: { issuer: "AAPL" },
        limit: 10,
      },
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(false);
    expect(result.call_id).toBe("g-call-1");
    expect(captured.q?.agent_id).toBe("chief-analyst");
    expect(captured.q?.metadata).toEqual({ issuer: "AAPL" });
    expect(captured.q?.limit).toBe(10);
    expect(String(result.content)).toContain("h-1");
    expect(String(result.content)).toContain("AAPL DCF");
  });

  it("accepts an empty input and forwards an empty filter", async () => {
    const captured: { q?: GraphRecallQuery } = {};
    const bank = makeBank({
      recallByGraph: async (q) => {
        captured.q = q;
        return [];
      },
    });
    const call: ToolCall = {
      id: "g-empty",
      name: "recall_by_graph",
      input: {},
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(false);
    expect(captured.q).toBeDefined();
    expect(captured.q?.agent_id).toBeUndefined();
    expect(captured.q?.metadata).toBeUndefined();
    expect(captured.q?.hasTools).toBeUndefined();
    expect(captured.q?.hasDelegations).toBeUndefined();
  });

  it("forwards hasTools and hasDelegations arrays unchanged", async () => {
    const spy = vi.fn(async () => []);
    const bank = makeBank({ recallByGraph: spy });
    const call: ToolCall = {
      id: "g-arrays",
      name: "recall_by_graph",
      input: {
        hasTools: ["option_pricer", "bond_pricer"],
        hasDelegations: ["derivatives-analyst"],
      },
    };
    await executeRecallGraphCall(call, bank);
    const q = spy.mock.calls[0]![0] as GraphRecallQuery;
    expect(q.hasTools).toEqual(["option_pricer", "bond_pricer"]);
    expect(q.hasDelegations).toEqual(["derivatives-analyst"]);
  });
});

// ---------------------------------------------------------------------------
// 5. executeRecallGraphCall — malformed input
// ---------------------------------------------------------------------------

describe("executeRecallGraphCall — malformed input", () => {
  it("rejects non-string agent_id", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-1",
      name: "recall_by_graph",
      input: { agent_id: 123 },
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/agent_id/i);
  });

  it("rejects non-object metadata", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-2",
      name: "recall_by_graph",
      input: { metadata: "not-an-object" },
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/metadata/i);
  });

  it("rejects array-not-of-strings hasTools", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-3",
      name: "recall_by_graph",
      input: { hasTools: ["ok", 123] },
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/hasTools/);
  });

  it("rejects malformed since string", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-4",
      name: "recall_by_graph",
      input: { since: "not-a-date" },
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/since/);
  });

  it("rejects negative limit", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-5",
      name: "recall_by_graph",
      input: { limit: -3 },
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/limit/);
  });
});

// ---------------------------------------------------------------------------
// 6. executeRecallGraphCall — bank throws
// ---------------------------------------------------------------------------

describe("executeRecallGraphCall — bank throws", () => {
  it("captures the error into is_error rather than throwing", async () => {
    const bank = makeBank({
      recallByGraph: async () => {
        throw new Error("bank-offline");
      },
    });
    const call: ToolCall = {
      id: "throw-1",
      name: "recall_by_graph",
      input: { agent_id: "chief-analyst" },
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/recall_by_graph failed/);
    expect(String(result.content)).toMatch(/bank-offline/);
  });
});

// ---------------------------------------------------------------------------
// 7. executeRecallGraphCall — since/until coercion
// ---------------------------------------------------------------------------

describe("executeRecallGraphCall — coercion", () => {
  it("converts since/until ISO strings into Date instances", async () => {
    const captured: { since?: unknown; until?: unknown } = {};
    const bank = makeBank({
      recallByGraph: async (q) => {
        captured.since = q.since;
        captured.until = q.until;
        return [];
      },
    });
    const call: ToolCall = {
      id: "since-until",
      name: "recall_by_graph",
      input: {
        since: "2026-01-01T00:00:00Z",
        until: "2026-12-31T23:59:59Z",
      },
    };
    const result = await executeRecallGraphCall(call, bank);
    expect(result.is_error).toBe(false);
    expect(captured.since).toBeInstanceOf(Date);
    expect(captured.until).toBeInstanceOf(Date);
    expect((captured.since as Date).toISOString()).toBe(
      "2026-01-01T00:00:00.000Z",
    );
  });
});

// ---------------------------------------------------------------------------
// 8. executeRecallGraphCall — limit clamping
// ---------------------------------------------------------------------------

describe("executeRecallGraphCall — limit clamping", () => {
  it("uses default limit when omitted", async () => {
    const captured: { limit?: number } = {};
    const bank = makeBank({
      recallByGraph: async (q) => {
        captured.limit = q.limit;
        return [];
      },
    });
    const call: ToolCall = {
      id: "lim-default",
      name: "recall_by_graph",
      input: {},
    };
    await executeRecallGraphCall(call, bank);
    // The bank receives `limit: undefined` when omitted; the bank applies
    // `?? GRAPH_RECALL_DEFAULT_LIMIT`. The tool layer leaves it undefined
    // unless the user provides one — that contract is intentional.
    expect(captured.limit).toBeUndefined();
  });

  it("clamps limit above the hard cap", async () => {
    const captured: { limit?: number } = {};
    const bank = makeBank({
      recallByGraph: async (q) => {
        captured.limit = q.limit;
        return [];
      },
    });
    const call: ToolCall = {
      id: "lim-huge",
      name: "recall_by_graph",
      input: { limit: 99999 },
    };
    await executeRecallGraphCall(call, bank);
    expect(captured.limit).toBe(GRAPH_RECALL_MAX_LIMIT);
  });

  it("uses limit=1 when caller passes a small positive value", async () => {
    const captured: { limit?: number } = {};
    const bank = makeBank({
      recallByGraph: async (q) => {
        captured.limit = q.limit;
        return [];
      },
    });
    const call: ToolCall = {
      id: "lim-1",
      name: "recall_by_graph",
      input: { limit: 1 },
    };
    await executeRecallGraphCall(call, bank);
    expect(captured.limit).toBe(1);
  });

  it("default limit constant is exposed", () => {
    expect(GRAPH_RECALL_DEFAULT_LIMIT).toBe(50);
    expect(GRAPH_RECALL_MAX_LIMIT).toBe(500);
  });
});
