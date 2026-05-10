/**
 * Phase 34 Wave 3 — integration test for the `recall_similar` virtual tool.
 *
 * Pre-populates a deterministic-embedded reasoning bank with three entries,
 * then runs a dispatch where the stub provider issues a `recall_similar`
 * tool call on turn 0. Asserts the harness intercepts the call, queries the
 * bank, and returns a tool_result block populated with the matching prior.
 *
 * Also asserts that when `reasoning` is omitted from DispatchOptions, the
 * `recall_similar` tool is NOT injected into the tool list given to the
 * provider — defensive: the model can never call a tool that doesn't exist
 * in the surface presented to it.
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { dispatch } from "../src/core/agent-loop.js";
import {
  createDeterministicEmbedder,
  createRuVectorBank,
  type ReasoningBank,
} from "../src/reasoning/index.js";
import type {
  AgentDef,
  CanonicalTool,
  MCPClient,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
  ToolResultBlock,
} from "../src/types.js";

const DIM = 384;

function makeMCPClient(tools: CanonicalTool[]): MCPClient {
  return {
    initialize: async () => {},
    listTools: async () => tools,
    callTool: async () => ({ content: [{ type: "text", text: "ok" }] }),
    close: async () => {},
  };
}

/**
 * Provider that issues a `recall_similar` tool call on turn 1 and ends on
 * turn 2. The recall query is hard-coded so the test can pre-seed the bank
 * with a matching prior summary.
 */
function makeRecallingProvider(query: string): {
  provider: Provider;
  toolsSeen: CanonicalTool[][];
} {
  const toolsSeen: CanonicalTool[][] = [];
  let turn = 0;
  const provider: Provider = {
    name: "anthropic" as const,
    async turn(req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      toolsSeen.push(req.tools);
      turn += 1;
      if (turn === 1) {
        return {
          message: {
            role: "assistant",
            content: [
              {
                type: "tool_use",
                id: "call-recall-1",
                name: "recall_similar",
                input: { query, k: 3 },
              },
            ],
          },
          stopReason: "tool_use",
          usage: { inputTokens: 10, outputTokens: 5 },
        };
      }
      return {
        message: {
          role: "assistant",
          content: [{ type: "text", text: "Done. Used recall results." }],
        },
        stopReason: "end_turn",
        usage: { inputTokens: 5, outputTokens: 3 },
      };
    },
  };
  return { provider, toolsSeen };
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

describe("recall_similar dispatch integration — Wave 3", () => {
  let bankDir: string;
  let bank: ReasoningBank | null = null;

  beforeEach(() => {
    bankDir = mkdtempSync(join(tmpdir(), "harness-recall-"));
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
    rmSync(bankDir, { recursive: true, force: true });
  });

  it("intercepts recall_similar, queries the bank, and returns prior entries to the chief", async () => {
    bank = await createRuVectorBank({
      dir: bankDir,
      embed: createDeterministicEmbedder({ dim: DIM }),
      dimensions: DIM,
    });

    // Pre-populate three entries directly via bank.index.
    await bank.index({
      audit_id: "prior-arg-1",
      agent_id: "chief-analyst",
      prompt_hash: "h-arg-1",
      prompt_summary: "Argentine convertible debenture pricing",
      tool_calls: [{ name: "convertible_bond_pricing", count: 1 }],
      delegations: ["derivatives-analyst"],
      result_excerpt: "Priced at $98.4 with 12% implied vol.",
      metadata: { jurisdiction: "AR" },
      timestamp: "2026-04-01T00:00:00.000Z",
    });
    await bank.index({
      audit_id: "prior-br-1",
      agent_id: "chief-analyst",
      prompt_hash: "h-br-1",
      prompt_summary: "Brazilian sovereign credit spread analysis",
      tool_calls: [{ name: "sovereign_bond_analysis", count: 1 }],
      delegations: ["macro-analyst"],
      result_excerpt: "Spread tightening 35bps over the quarter.",
      metadata: { jurisdiction: "BR" },
      timestamp: "2026-04-15T00:00:00.000Z",
    });
    await bank.index({
      audit_id: "prior-cl-1",
      agent_id: "chief-analyst",
      prompt_hash: "h-cl-1",
      prompt_summary: "Chilean equity sector rotation strategy",
      tool_calls: [],
      delegations: ["equity-analyst"],
      result_excerpt: "Rotated into materials.",
      metadata: { jurisdiction: "CL" },
      timestamp: "2026-04-22T00:00:00.000Z",
    });

    const mcp = makeMCPClient([tinyTool]);
    const { provider } = makeRecallingProvider(
      "Argentine convertible debenture pricing",
    );

    const result = await dispatch({
      agent: testAgent,
      prompt: "What priors do I have on Argentine converts?",
      provider,
      mcp,
      reasoning: bank,
    });

    expect(result.toolUses).toBe(1);
    expect(result.finalText).toBe("Done. Used recall results.");

    // Find the tool_result block in the conversation; assert it contains the
    // matching prior's audit_id and prompt_summary.
    const toolResultBlocks: ToolResultBlock[] = [];
    for (const m of result.messages) {
      for (const b of m.content) {
        if (b.type === "tool_result") toolResultBlocks.push(b);
      }
    }
    expect(toolResultBlocks).toHaveLength(1);
    const block = toolResultBlocks[0]!;
    expect(block.tool_use_id).toBe("call-recall-1");
    expect(block.is_error).toBe(false);
    expect(block.content).toContain("prior-arg-1");
    expect(block.content).toContain(
      "Argentine convertible debenture pricing",
    );
    expect(block.content).toContain("derivatives-analyst");
  });

  it("does NOT inject recall_similar into the tool list when reasoning is omitted", async () => {
    const mcp = makeMCPClient([tinyTool]);
    // Provider that simply ends the turn — we only care about the tools it sees.
    const toolsSeen: CanonicalTool[][] = [];
    const provider: Provider = {
      name: "anthropic" as const,
      async turn(req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
        toolsSeen.push(req.tools);
        return {
          message: {
            role: "assistant",
            content: [{ type: "text", text: "no-op" }],
          },
          stopReason: "end_turn",
          usage: { inputTokens: 1, outputTokens: 1 },
        };
      },
    };

    await dispatch({
      agent: testAgent,
      prompt: "no reasoning configured",
      provider,
      mcp,
      // reasoning omitted on purpose
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === "recall_similar")).toBe(false);
    // Real tool must still be present.
    expect(firstTurnTools.some((t) => t.name === "tiny_tool")).toBe(true);
  });

  it("DOES inject recall_similar into the tool list when reasoning is configured", async () => {
    bank = await createRuVectorBank({
      dir: bankDir,
      embed: createDeterministicEmbedder({ dim: DIM }),
      dimensions: DIM,
    });
    const mcp = makeMCPClient([tinyTool]);
    const toolsSeen: CanonicalTool[][] = [];
    const provider: Provider = {
      name: "anthropic" as const,
      async turn(req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
        toolsSeen.push(req.tools);
        return {
          message: {
            role: "assistant",
            content: [{ type: "text", text: "no-op" }],
          },
          stopReason: "end_turn",
          usage: { inputTokens: 1, outputTokens: 1 },
        };
      },
    };

    await dispatch({
      agent: testAgent,
      prompt: "reasoning on",
      provider,
      mcp,
      reasoning: bank,
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    const recall = firstTurnTools.find((t) => t.name === "recall_similar");
    expect(recall).toBeDefined();
    expect(recall!.input_schema.required).toEqual(["query"]);
  });
});
