/**
 * Phase 34 Wave 3 — unit tests for the `recall_similar` virtual tool.
 *
 * Covers tool spec shape, name predicate, formatter, and `executeRecallCall`
 * happy path / malformed input / bank-throws / since-string-to-Date.
 */
import { describe, expect, it, vi } from "vitest";
import {
  RECALL_TOOL_NAME,
  createRecallTool,
  executeRecallCall,
  formatRecallResult,
  isRecallToolName,
} from "../src/reasoning/recall-tool.js";
import type { ReasoningBank, ReasoningEntry } from "../src/reasoning/bank.js";
import type { ToolCall } from "../src/types.js";

function makeEntry(partial: Partial<ReasoningEntry> & { audit_id: string }): ReasoningEntry {
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

describe("createRecallTool", () => {
  it("returns a CanonicalTool with the right name, schema, and description", () => {
    const tool = createRecallTool();
    expect(tool.name).toBe(RECALL_TOOL_NAME);
    expect(tool.name).toBe("recall_similar");

    expect(tool.input_schema.type).toBe("object");
    expect(tool.input_schema.properties).toHaveProperty("query");
    expect(tool.input_schema.properties).toHaveProperty("k");
    expect(tool.input_schema.properties).toHaveProperty("filter");
    expect(tool.input_schema.required).toEqual(["query"]);

    // Description must mention the key concepts so the model knows what to do.
    expect(tool.description.toLowerCase()).toContain("similar");
    expect(tool.description.toLowerCase()).toContain("reasoning bank");
  });
});

// ---------------------------------------------------------------------------
// 2. isRecallToolName predicate
// ---------------------------------------------------------------------------

describe("isRecallToolName", () => {
  it("returns true only for the exact `recall_similar` name", () => {
    expect(isRecallToolName("recall_similar")).toBe(true);
    expect(isRecallToolName("recall_other")).toBe(false);
    expect(isRecallToolName("delegate_to_recall_similar")).toBe(false);
    expect(isRecallToolName("RECALL_SIMILAR")).toBe(false);
    expect(isRecallToolName("")).toBe(false);
    expect(isRecallToolName("recall_similar_v2")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// 3. formatRecallResult
// ---------------------------------------------------------------------------

describe("formatRecallResult", () => {
  it("returns the explicit no-results sentinel for empty input", () => {
    expect(formatRecallResult([])).toBe("No similar prior dispatches found.");
  });

  it("includes audit_id and prompt_summary for every entry", () => {
    const a = makeEntry({
      audit_id: "abc-1",
      prompt_summary: "Argentine convertible debenture pricing",
    });
    const b = makeEntry({
      audit_id: "def-2",
      prompt_summary: "Brazilian sovereign credit spread analysis",
      tool_calls: [{ name: "bond_pricer", count: 3 }],
      delegations: ["fixed-income-analyst"],
    });
    const out = formatRecallResult([a, b]);
    expect(out).toContain("abc-1");
    expect(out).toContain("def-2");
    expect(out).toContain("Argentine convertible debenture pricing");
    expect(out).toContain("Brazilian sovereign credit spread analysis");
    expect(out).toContain("bond_pricer×3");
    expect(out).toContain("fixed-income-analyst");
  });
});

// ---------------------------------------------------------------------------
// 4. executeRecallCall — happy path
// ---------------------------------------------------------------------------

describe("executeRecallCall — happy path", () => {
  it("calls bank.recallSimilar and returns formatted entries with is_error=false", async () => {
    const entries = [
      makeEntry({ audit_id: "h-1", prompt_summary: "WACC for AAPL" }),
      makeEntry({ audit_id: "h-2", prompt_summary: "DCF for AAPL" }),
    ];
    const bank = makeBank({
      recallSimilar: async (q, opts) => {
        expect(q).toBe("AAPL valuation");
        expect(opts?.k).toBe(3);
        return entries;
      },
    });
    const call: ToolCall = {
      id: "call-recall-1",
      name: "recall_similar",
      input: { query: "AAPL valuation", k: 3 },
    };
    const result = await executeRecallCall(call, bank);
    expect(result.call_id).toBe("call-recall-1");
    expect(result.is_error).toBe(false);
    expect(typeof result.content).toBe("string");
    const content = result.content as string;
    expect(content).toContain("h-1");
    expect(content).toContain("WACC for AAPL");
    expect(content).toContain("h-2");
  });

  it("uses k=5 default when k is omitted", async () => {
    const spy = vi.fn(async () => []);
    const bank = makeBank({ recallSimilar: spy });
    const call: ToolCall = {
      id: "call-recall-default-k",
      name: "recall_similar",
      input: { query: "anything" },
    };
    await executeRecallCall(call, bank);
    expect(spy).toHaveBeenCalledTimes(1);
    const opts = spy.mock.calls[0]![1] as { k: number };
    expect(opts.k).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// 5. executeRecallCall — malformed input
// ---------------------------------------------------------------------------

describe("executeRecallCall — malformed input", () => {
  it("returns is_error=true with a descriptive message when query is missing", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-1",
      name: "recall_similar",
      input: {},
    };
    const result = await executeRecallCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(result.call_id).toBe("bad-1");
    expect(String(result.content)).toMatch(/recall_similar failed/i);
    expect(String(result.content)).toMatch(/query/i);
  });

  it("returns is_error=true when query is non-string", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-2",
      name: "recall_similar",
      input: { query: 123 },
    };
    const result = await executeRecallCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/non-empty string/i);
  });

  it("returns is_error=true when query is empty string", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-3",
      name: "recall_similar",
      input: { query: "   " },
    };
    const result = await executeRecallCall(call, bank);
    expect(result.is_error).toBe(true);
  });

  it("returns is_error=true when filter.since is malformed", async () => {
    const bank = makeBank({});
    const call: ToolCall = {
      id: "bad-4",
      name: "recall_similar",
      input: { query: "x", filter: { since: "not-a-date" } },
    };
    const result = await executeRecallCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/since/i);
  });
});

// ---------------------------------------------------------------------------
// 6. executeRecallCall — bank throws
// ---------------------------------------------------------------------------

describe("executeRecallCall — bank throws", () => {
  it("captures the error into is_error=true rather than throwing", async () => {
    const bank = makeBank({
      recallSimilar: async () => {
        throw new Error("bank-offline");
      },
    });
    const call: ToolCall = {
      id: "throw-1",
      name: "recall_similar",
      input: { query: "anything" },
    };
    const result = await executeRecallCall(call, bank);
    expect(result.is_error).toBe(true);
    expect(String(result.content)).toMatch(/recall_similar failed/i);
    expect(String(result.content)).toMatch(/bank-offline/);
  });
});

// ---------------------------------------------------------------------------
// 7. executeRecallCall — converts since string to Date
// ---------------------------------------------------------------------------

describe("executeRecallCall — filter coercion", () => {
  it("converts filter.since from ISO string to Date instance", async () => {
    const captured: { since?: unknown } = {};
    const bank = makeBank({
      recallSimilar: async (_q, opts) => {
        captured.since = opts?.filter?.since;
        return [];
      },
    });
    const call: ToolCall = {
      id: "since-1",
      name: "recall_similar",
      input: {
        query: "x",
        filter: { since: "2026-01-15T08:00:00Z" },
      },
    };
    const result = await executeRecallCall(call, bank);
    expect(result.is_error).toBe(false);
    expect(captured.since).toBeInstanceOf(Date);
    expect((captured.since as Date).toISOString()).toBe(
      "2026-01-15T08:00:00.000Z",
    );
  });

  it("passes agent_id and metadata filters through unchanged", async () => {
    const captured: {
      agent_id?: unknown;
      metadata?: unknown;
    } = {};
    const bank = makeBank({
      recallSimilar: async (_q, opts) => {
        captured.agent_id = opts?.filter?.agent_id;
        captured.metadata = opts?.filter?.metadata;
        return [];
      },
    });
    const call: ToolCall = {
      id: "filt-1",
      name: "recall_similar",
      input: {
        query: "x",
        filter: { agent_id: "chief-analyst", metadata: { issuer: "AAPL" } },
      },
    };
    const result = await executeRecallCall(call, bank);
    expect(result.is_error).toBe(false);
    expect(captured.agent_id).toBe("chief-analyst");
    expect(captured.metadata).toEqual({ issuer: "AAPL" });
  });

  it("clamps k to [1, 20]", async () => {
    const captured: { k?: number } = {};
    const bank = makeBank({
      recallSimilar: async (_q, opts) => {
        captured.k = opts?.k;
        return [];
      },
    });
    const call: ToolCall = {
      id: "k-1",
      name: "recall_similar",
      input: { query: "x", k: 999 },
    };
    await executeRecallCall(call, bank);
    expect(captured.k).toBe(20);
  });
});
