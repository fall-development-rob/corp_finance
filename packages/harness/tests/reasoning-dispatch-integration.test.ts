/**
 * End-to-end integration test for Phase 34 Wave 2.
 *
 * Asserts the dispatch hot path indexes the audit record into the reasoning
 * bank fire-and-forget when both `audit` and `reasoning` are configured, and
 * that index failures emit a `reasoning_index_failed` event without
 * propagating to the caller.
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { dispatch } from "../src/core/agent-loop.js";
import { createFileAuditSink } from "../src/audit/index.js";
import {
  createDeterministicEmbedder,
  createRuVectorBank,
  type ReasoningBank,
  type ReasoningEntry,
} from "../src/reasoning/index.js";
import type {
  AgentDef,
  CanonicalTool,
  DispatchEvent,
  MCPClient,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../src/types.js";

const DIM = 384;

function makeMCPClient(tools: CanonicalTool[]): MCPClient {
  return {
    initialize: async () => {},
    listTools: async () => tools,
    callTool: async (name) => ({ content: [{ type: "text", text: `result-${name}` }] }),
    close: async () => {},
  };
}

function makeOneToolProvider(toolName: string): Provider {
  let turn = 0;
  return {
    name: "anthropic" as const,
    async turn(_req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      turn += 1;
      if (turn === 1) {
        return {
          message: {
            role: "assistant",
            content: [
              { type: "tool_use", id: "call-1", name: toolName, input: { x: 1 } },
            ],
          },
          stopReason: "tool_use",
          usage: { inputTokens: 10, outputTokens: 5 },
        };
      }
      return {
        message: {
          role: "assistant",
          content: [{ type: "text", text: "Final reasoning bank acceptance." }],
        },
        stopReason: "end_turn",
        usage: { inputTokens: 5, outputTokens: 3 },
      };
    },
  };
}

const tinyTool: CanonicalTool = {
  name: "tiny_tool",
  description: "Tiny test tool.",
  input_schema: {
    type: "object",
    properties: { x: { type: "number" } },
    required: ["x"],
  },
};

const testAgent: AgentDef = {
  id: "chief-analyst",
  description: "Test chief.",
  systemPrompt: "You are the test chief.",
  tools: ["tiny_tool"],
  maxRecursionDepth: 0,
  model: "claude-test",
  maxTokens: 1024,
};

describe("reasoning bank dispatch integration — Wave 2", () => {
  let auditDir: string;
  let bankDir: string;
  let bank: ReasoningBank | null = null;

  beforeEach(() => {
    auditDir = mkdtempSync(join(tmpdir(), "harness-audit-rb-"));
    bankDir = mkdtempSync(join(tmpdir(), "harness-rbank-"));
    bank = null;
  });

  afterEach(async () => {
    if (bank) {
      try {
        await bank.close();
      } catch {
        // best-effort
      }
    }
    rmSync(auditDir, { recursive: true, force: true });
    rmSync(bankDir, { recursive: true, force: true });
  });

  it("indexes the audit record into the reasoning bank when both are configured", async () => {
    const audit = createFileAuditSink({ dir: auditDir });
    bank = await createRuVectorBank({
      dir: bankDir,
      embed: createDeterministicEmbedder({ dim: DIM }),
      dimensions: DIM,
    });
    const mcp = makeMCPClient([tinyTool]);
    const provider = makeOneToolProvider("tiny_tool");
    const prompt = "Run the tiny tool and summarize.";

    const result = await dispatch({
      agent: testAgent,
      prompt,
      provider,
      mcp,
      audit,
      reasoning: bank,
    });

    expect(result.auditId).toBeDefined();
    // Audit record should have been written.
    const auditRecord = await audit.read(result.auditId!);
    expect(auditRecord).not.toBeNull();
    expect(auditRecord?.agent_id).toBe("chief-analyst");

    // Wait briefly for the fire-and-forget index to flush. The promise is
    // not awaited inside dispatch; we poll the bank up to ~500 ms.
    let hits: ReasoningEntry[] = [];
    for (let i = 0; i < 25; i++) {
      hits = await bank.recallSimilar(prompt);
      if (hits.length > 0) break;
      await new Promise((r) => setTimeout(r, 20));
    }
    expect(hits.length).toBe(1);
    const entry = hits[0]!;
    expect(entry.audit_id).toBe(result.auditId);
    expect(entry.agent_id).toBe("chief-analyst");
    expect(entry.prompt_hash).toBe(auditRecord!.prompt_hash);
    expect(entry.tool_calls).toEqual([{ name: "tiny_tool", count: 1 }]);
    expect(entry.delegations).toEqual([]);
  });

  it("does NOT fail dispatch and emits reasoning_index_failed when the bank throws", async () => {
    const audit = createFileAuditSink({ dir: auditDir });
    const failingBank: ReasoningBank = {
      async index() {
        throw new Error("boom: bank is offline");
      },
      async recallSimilar() {
        return [];
      },
      async recallByGraph() {
        throw new Error("not implemented");
      },
      async close() {},
    };
    const mcp = makeMCPClient([tinyTool]);
    const provider = makeOneToolProvider("tiny_tool");
    const prompt = "Index failure path.";

    const events: DispatchEvent[] = [];
    const result = await dispatch({
      agent: testAgent,
      prompt,
      provider,
      mcp,
      audit,
      reasoning: failingBank,
      onEvent: (e) => events.push(e),
    });

    // Dispatch must succeed.
    expect(result.finalText).toBe("Final reasoning bank acceptance.");
    expect(result.auditId).toBeDefined();

    // Audit must still be written.
    const auditRecord = await audit.read(result.auditId!);
    expect(auditRecord).not.toBeNull();

    // Wait for the fire-and-forget to settle and emit the failure event.
    let failure: Extract<DispatchEvent, { type: "reasoning_index_failed" }> | undefined;
    for (let i = 0; i < 25; i++) {
      failure = events.find(
        (e): e is Extract<DispatchEvent, { type: "reasoning_index_failed" }> =>
          e.type === "reasoning_index_failed",
      );
      if (failure) break;
      await new Promise((r) => setTimeout(r, 20));
    }
    expect(failure).toBeDefined();
    expect(failure!.audit_id).toBe(result.auditId);
    expect(failure!.reason).toContain("boom");
  });

  it("does not index when audit is omitted (reasoning alone is a no-op)", async () => {
    bank = await createRuVectorBank({
      dir: bankDir,
      embed: createDeterministicEmbedder({ dim: DIM }),
      dimensions: DIM,
    });
    const mcp = makeMCPClient([tinyTool]);
    const provider = makeOneToolProvider("tiny_tool");

    const result = await dispatch({
      agent: testAgent,
      prompt: "no audit configured",
      provider,
      mcp,
      reasoning: bank,
    });

    expect(result.finalText).toBe("Final reasoning bank acceptance.");
    // Wait briefly to ensure no async indexing leaked.
    await new Promise((r) => setTimeout(r, 50));
    const hits = await bank.recallSimilar("no audit configured");
    expect(hits.length).toBe(0);
  });
});
