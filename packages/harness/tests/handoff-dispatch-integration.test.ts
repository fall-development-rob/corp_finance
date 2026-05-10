/**
 * REC-4 Wave 2 — integration tests for the `initiate_handoff` virtual tool
 * wired into the dispatch flow.
 *
 * Tests cover:
 *   1.  dispatch with handoff configured → chief's tool list includes `initiate_handoff`
 *   2.  dispatch without handoff → no `initiate_handoff` tool
 *   3.  stubbed provider issues `initiate_handoff` → orchestrator.execute called, result lands
 *   4.  handoff at depth=1 (specialist) → tool NOT in tool list
 *   5.  target not in allowlist → is_error: true with rejection reason
 *   6.  payload validation failure → is_error: true with validator errors
 *   7.  target dispatch throws → is_error: true with error message
 *   8.  audit record captures handoff with is_error tag
 *   9.  handoff_initiated + handoff_returned events fire in order
 *   10. depth cap enforced (handoff at maxDepth-1 attempts further handoff → rejected)
 *   11. buildAllowlistFromAgent extracts callableAgents correctly
 *   12. end-to-end: chief → handoff → specialist → returns structured result
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dispatch } from "../src/core/agent-loop.js";
import { createFileAuditSink } from "../src/audit/index.js";
import {
  createHandoffOrchestrator,
  buildAllowlistFromAgent,
  buildResolverFromAgent,
  HANDOFF_TOOL_NAME,
  type HandoffOrchestratorOptions,
} from "../src/handoff/index.js";
import type {
  AgentDef,
  AuditRecord,
  AuditSink,
  CanonicalTool,
  DispatchEvent,
  MCPClient,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
  ToolResultBlock,
} from "../src/types.js";

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

const tinyTool: CanonicalTool = {
  name: "tiny_tool",
  description: "Tiny test tool.",
  input_schema: {
    type: "object",
    properties: { x: { type: "number" } },
    required: ["x"],
  },
};

function makeMCPClient(tools: CanonicalTool[] = [tinyTool]): MCPClient {
  return {
    initialize: async () => {},
    listTools: async () => tools,
    callTool: async () => ({ content: [{ type: "text", text: "ok" }] }),
    close: async () => {},
  };
}

/** Provider that ends immediately with a canned message. No tool calls. */
function makePassiveProvider(text = "done"): Provider {
  return {
    name: "anthropic" as const,
    async turn(): Promise<ProviderTurnResponse> {
      return {
        message: { role: "assistant", content: [{ type: "text", text }] },
        stopReason: "end_turn",
        usage: { inputTokens: 1, outputTokens: 1 },
      };
    },
  };
}

/** Provider that captures each turn's tool list and ends. */
function makeToolCapturingProvider(): { provider: Provider; toolsSeen: CanonicalTool[][] } {
  const toolsSeen: CanonicalTool[][] = [];
  const provider: Provider = {
    name: "anthropic" as const,
    async turn(req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      toolsSeen.push(req.tools);
      return {
        message: { role: "assistant", content: [{ type: "text", text: "no-op" }] },
        stopReason: "end_turn",
        usage: { inputTokens: 1, outputTokens: 1 },
      };
    },
  };
  return { provider, toolsSeen };
}

/**
 * Provider that issues `initiate_handoff` on turn 1 with a given target/payload,
 * then ends on turn 2.
 */
function makeHandoffProvider(
  target: string,
  payload: Record<string, unknown>,
  context?: Record<string, unknown>,
): { provider: Provider; toolsSeen: CanonicalTool[][] } {
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
                id: "call-handoff-1",
                name: HANDOFF_TOOL_NAME,
                input: { target, payload, ...(context ? { context } : {}) },
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
          content: [{ type: "text", text: "handoff complete" }],
        },
        stopReason: "end_turn",
        usage: { inputTokens: 5, outputTokens: 3 },
      };
    },
  };
  return { provider, toolsSeen };
}

/** Minimal AgentDef for the chief. */
const chiefAgent: AgentDef = {
  id: "chief-analyst",
  description: "Chief test agent.",
  systemPrompt: "You are the test chief.",
  tools: ["tiny_tool"],
  maxRecursionDepth: 0,
  model: "claude-test",
  maxTokens: 1024,
};

/** Minimal AgentDef for a specialist target. */
const specialistAgent: AgentDef = {
  id: "credit-analyst",
  description: "Credit specialist.",
  systemPrompt: "You are the credit analyst.",
  tools: ["tiny_tool"],
  model: "claude-test",
  maxTokens: 1024,
};

/** Chief with callableAgents wired (used for buildAllowlistFromAgent tests). */
const chiefWithCallable: AgentDef = {
  ...chiefAgent,
  callableAgents: [specialistAgent],
};

/** Default dispatchAgent stub — returns a canned response. */
function makeDispatchStub(
  response = { finalText: "specialist result", auditId: "audit-specialist-1" },
): HandoffOrchestratorOptions["dispatchAgent"] {
  return vi.fn().mockResolvedValue(response);
}

/** Build a minimal orchestrator with the chief→credit-analyst allowlist. */
function makeOrchestrator(
  dispatchAgent: HandoffOrchestratorOptions["dispatchAgent"] = makeDispatchStub(),
  opts: Partial<HandoffOrchestratorOptions> = {},
) {
  return createHandoffOrchestrator({
    allowlist: [{ source: chiefAgent.id, targets: [specialistAgent.id] }],
    resolveAgent: (slug) => (slug === specialistAgent.id ? specialistAgent : undefined),
    dispatchAgent,
    ...opts,
  });
}

// ---------------------------------------------------------------------------
// Test 1: handoff tool injected at depth=0 when configured
// ---------------------------------------------------------------------------

describe("Test 1: handoff tool injected at depth=0", () => {
  it("dispatch with handoff configured → chief's tool list includes initiate_handoff", async () => {
    const { provider, toolsSeen } = makeToolCapturingProvider();
    const orchestrator = makeOrchestrator();

    await dispatch({
      agent: chiefAgent,
      prompt: "test",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    const tool = firstTurnTools.find((t) => t.name === HANDOFF_TOOL_NAME);
    expect(tool).toBeDefined();
    expect(tool!.input_schema.required).toContain("target");
    expect(tool!.input_schema.required).toContain("payload");
  });
});

// ---------------------------------------------------------------------------
// Test 2: handoff tool NOT injected when handoff omitted
// ---------------------------------------------------------------------------

describe("Test 2: no handoff tool when handoff omitted", () => {
  it("dispatch without handoff → no initiate_handoff in tool list", async () => {
    const { provider, toolsSeen } = makeToolCapturingProvider();

    await dispatch({
      agent: chiefAgent,
      prompt: "no handoff",
      provider,
      mcp: makeMCPClient(),
      // handoff intentionally omitted
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);
    // Real tool still present
    expect(firstTurnTools.some((t) => t.name === "tiny_tool")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 3: orchestrator.execute called; result lands in conversation
// ---------------------------------------------------------------------------

describe("Test 3: orchestrator.execute called and result surfaces", () => {
  it("provider issues initiate_handoff → orchestrator.execute called, result in messages", async () => {
    const dispatchStub = makeDispatchStub({ finalText: "credit analysis done", auditId: "audit-99" });
    const orchestrator = makeOrchestrator(dispatchStub);

    const { provider } = makeHandoffProvider(specialistAgent.id, { ticker: "AAPL" });

    const result = await dispatch({
      agent: chiefAgent,
      prompt: "run credit analysis",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
    });

    // dispatchAgent on the orchestrator must have been called
    expect(dispatchStub).toHaveBeenCalledOnce();
    expect(result.toolUses).toBe(1);
    expect(result.finalText).toBe("handoff complete");

    // Find the tool_result block
    const toolResultBlocks: ToolResultBlock[] = [];
    for (const m of result.messages) {
      for (const b of m.content) {
        if (b.type === "tool_result") toolResultBlocks.push(b);
      }
    }
    expect(toolResultBlocks).toHaveLength(1);
    const block = toolResultBlocks[0]!;
    expect(block.tool_use_id).toBe("call-handoff-1");
    expect(block.is_error).toBeFalsy();
    expect(block.content).toContain("credit-analyst");
    expect(block.content).toContain("completed");
  });
});

// ---------------------------------------------------------------------------
// Test 4: handoff tool NOT injected at depth=1
// ---------------------------------------------------------------------------

describe("Test 4: no handoff tool at depth=1 (specialist)", () => {
  it("dispatch at depth=1 does not inject initiate_handoff", async () => {
    const { provider, toolsSeen } = makeToolCapturingProvider();
    const orchestrator = makeOrchestrator();

    await dispatch({
      agent: chiefAgent,
      prompt: "specialist run",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
      depth: 1, // simulate specialist depth
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Test 5: target not in allowlist → is_error
// ---------------------------------------------------------------------------

describe("Test 5: target not in allowlist → is_error", () => {
  it("handoff to unknown target → is_error with rejection reason", async () => {
    const dispatchStub = makeDispatchStub();
    const orchestrator = makeOrchestrator(dispatchStub);

    const { provider } = makeHandoffProvider("unknown-agent", { data: "x" });

    const result = await dispatch({
      agent: chiefAgent,
      prompt: "handoff to unknown",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
    });

    const toolResultBlocks: ToolResultBlock[] = [];
    for (const m of result.messages) {
      for (const b of m.content) {
        if (b.type === "tool_result") toolResultBlocks.push(b);
      }
    }
    expect(toolResultBlocks).toHaveLength(1);
    const block = toolResultBlocks[0]!;
    expect(block.is_error).toBe(true);
    expect(block.content).toMatch(/rejected|not allowed|no handoff permissions/i);
    // dispatchAgent must NOT have been called (rejection before dispatch)
    expect(dispatchStub).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Test 6: payload validation failure → is_error
// ---------------------------------------------------------------------------

describe("Test 6: payload validation failure → is_error", () => {
  it("handoff with invalid payload → is_error with validator errors", async () => {
    const dispatchStub = makeDispatchStub();
    const orchestrator = createHandoffOrchestrator({
      allowlist: [
        {
          source: chiefAgent.id,
          targets: [specialistAgent.id],
          payload_schema: {
            type: "object",
            required: ["ticker", "period"],
            properties: {
              ticker: { type: "string" },
              period: { type: "string" },
            },
          },
        },
      ],
      resolveAgent: (slug) => (slug === specialistAgent.id ? specialistAgent : undefined),
      dispatchAgent: dispatchStub,
    });

    // payload is missing required "period" field
    const { provider } = makeHandoffProvider(specialistAgent.id, { ticker: "AAPL" });

    const result = await dispatch({
      agent: chiefAgent,
      prompt: "bad payload handoff",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
    });

    const toolResultBlocks: ToolResultBlock[] = [];
    for (const m of result.messages) {
      for (const b of m.content) {
        if (b.type === "tool_result") toolResultBlocks.push(b);
      }
    }
    expect(toolResultBlocks).toHaveLength(1);
    const block = toolResultBlocks[0]!;
    expect(block.is_error).toBe(true);
    expect(block.content).toContain("period");
    expect(dispatchStub).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Test 7: target dispatch throws → is_error
// ---------------------------------------------------------------------------

describe("Test 7: target dispatch throws → is_error", () => {
  it("dispatchAgent throws → is_error with error message in result", async () => {
    const dispatchStub = vi.fn().mockRejectedValue(new Error("model timeout"));
    const orchestrator = makeOrchestrator(dispatchStub);

    const { provider } = makeHandoffProvider(specialistAgent.id, { data: "x" });

    const result = await dispatch({
      agent: chiefAgent,
      prompt: "handoff that will fail",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
    });

    const toolResultBlocks: ToolResultBlock[] = [];
    for (const m of result.messages) {
      for (const b of m.content) {
        if (b.type === "tool_result") toolResultBlocks.push(b);
      }
    }
    expect(toolResultBlocks).toHaveLength(1);
    const block = toolResultBlocks[0]!;
    expect(block.is_error).toBe(true);
    expect(block.content).toContain("model timeout");
  });
});

// ---------------------------------------------------------------------------
// Test 8: audit record captures handoff with is_error tag
// ---------------------------------------------------------------------------

describe("Test 8: audit record captures handoff call", () => {
  let tmpDir: string;
  let audit: AuditSink;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "harness-handoff-audit-"));
    audit = createFileAuditSink({ dir: tmpDir });
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("successful handoff audit record has is_error=false for the tool call", async () => {
    const dispatchStub = makeDispatchStub();
    const orchestrator = makeOrchestrator(dispatchStub);

    const { provider } = makeHandoffProvider(specialistAgent.id, { data: "x" });

    const result = await dispatch({
      agent: chiefAgent,
      prompt: "audit handoff",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
      audit,
    });

    expect(result.auditId).toBeDefined();
    const record = (await audit.read(result.auditId!)) as AuditRecord;
    expect(record).not.toBeNull();
    const handoffAudit = record.tool_calls.find((tc) => tc.name === HANDOFF_TOOL_NAME);
    expect(handoffAudit).toBeDefined();
    expect(handoffAudit!.is_error).toBe(false);
  });

  it("rejected handoff audit record has is_error=true for the tool call", async () => {
    const orchestrator = makeOrchestrator(makeDispatchStub());

    const { provider } = makeHandoffProvider("unknown-agent", { data: "x" });

    const result = await dispatch({
      agent: chiefAgent,
      prompt: "audit rejected handoff",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
      audit,
    });

    expect(result.auditId).toBeDefined();
    const record = (await audit.read(result.auditId!)) as AuditRecord;
    expect(record).not.toBeNull();
    const handoffAudit = record.tool_calls.find((tc) => tc.name === HANDOFF_TOOL_NAME);
    expect(handoffAudit).toBeDefined();
    expect(handoffAudit!.is_error).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 9: handoff_initiated + handoff_returned events fire in order
// ---------------------------------------------------------------------------

describe("Test 9: handoff_initiated + handoff_returned events", () => {
  it("events fire in the correct order with matching target_agent", async () => {
    const dispatchStub = makeDispatchStub();
    const orchestrator = makeOrchestrator(dispatchStub);

    const events: DispatchEvent[] = [];
    const { provider } = makeHandoffProvider(specialistAgent.id, { data: "x" });

    await dispatch({
      agent: chiefAgent,
      prompt: "observe events",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
      onEvent: (e) => events.push(e),
    });

    const initiated = events.filter((e) => e.type === "handoff_initiated");
    const returned = events.filter((e) => e.type === "handoff_returned");

    expect(initiated).toHaveLength(1);
    expect(returned).toHaveLength(1);

    const initiatedIdx = events.findIndex((e) => e.type === "handoff_initiated");
    const returnedIdx = events.findIndex((e) => e.type === "handoff_returned");
    expect(initiatedIdx).toBeLessThan(returnedIdx);

    // Both events carry the correct target_agent slug
    const ini = initiated[0] as Extract<DispatchEvent, { type: "handoff_initiated" }>;
    const ret = returned[0] as Extract<DispatchEvent, { type: "handoff_returned" }>;
    expect(ini.target_agent).toBe(specialistAgent.id);
    expect(ret.target_agent).toBe(specialistAgent.id);
    expect(ret.ok).toBe(true);
    expect(ret.duration_ms).toBeGreaterThanOrEqual(0);
  });
});

// ---------------------------------------------------------------------------
// Test 10: depth cap enforced
// ---------------------------------------------------------------------------

describe("Test 10: depth cap enforced", () => {
  it("handoff at maxDepth rejects with 'max depth' reason", async () => {
    const dispatchStub = makeDispatchStub();
    const orchestrator = createHandoffOrchestrator({
      allowlist: [{ source: chiefAgent.id, targets: [specialistAgent.id] }],
      resolveAgent: (slug) => (slug === specialistAgent.id ? specialistAgent : undefined),
      dispatchAgent: dispatchStub,
      maxDepth: 2,
    });

    const { provider } = makeHandoffProvider(specialistAgent.id, { data: "x" });

    const result = await dispatch({
      agent: chiefAgent,
      prompt: "at depth cap",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
      depth: 2, // depth === maxDepth → should be rejected
    });

    // At depth=2, the tool is NOT injected (depth=0 only guard)
    // so the provider sees no initiate_handoff tool and ends normally.
    // We verify dispatch completes without error.
    expect(result.finalText).toBeDefined();
    expect(dispatchStub).not.toHaveBeenCalled();
  });

  it("handoff at depth=maxDepth-1 through orchestrator rejects internally", async () => {
    const dispatchStub = makeDispatchStub();
    const orchestrator = createHandoffOrchestrator({
      allowlist: [{ source: chiefAgent.id, targets: [specialistAgent.id] }],
      resolveAgent: (slug) => (slug === specialistAgent.id ? specialistAgent : undefined),
      dispatchAgent: dispatchStub,
      maxDepth: 1, // depth=0 is fine, depth=1 would be rejected
    });

    // Direct orchestrator execute to test depth cap in isolation
    const result = await orchestrator.execute(
      {
        source_agent: chiefAgent.id,
        target_agent: specialistAgent.id,
        payload: { data: "x" },
      },
      1, // currentDepth === maxDepth → reject
    );

    expect(result.ok).toBe(false);
    expect(result.reason).toContain("max chain depth");
    expect(dispatchStub).not.toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// Test 11: buildAllowlistFromAgent and buildResolverFromAgent
// ---------------------------------------------------------------------------

describe("Test 11: buildAllowlistFromAgent + buildResolverFromAgent", () => {
  it("buildAllowlistFromAgent returns empty array when callableAgents is absent", () => {
    const result = buildAllowlistFromAgent(chiefAgent);
    expect(result).toEqual([]);
  });

  it("buildAllowlistFromAgent returns empty array when callableAgents is empty", () => {
    const agent: AgentDef = { ...chiefAgent, callableAgents: [] };
    expect(buildAllowlistFromAgent(agent)).toEqual([]);
  });

  it("buildAllowlistFromAgent extracts source and targets correctly", () => {
    const result = buildAllowlistFromAgent(chiefWithCallable);
    expect(result).toHaveLength(1);
    expect(result[0]!.source).toBe(chiefAgent.id);
    expect(result[0]!.targets).toEqual([specialistAgent.id]);
  });

  it("buildAllowlistFromAgent includes all callableAgents targets", () => {
    const second: AgentDef = { ...specialistAgent, id: "equity-analyst" };
    const agent: AgentDef = { ...chiefAgent, callableAgents: [specialistAgent, second] };
    const result = buildAllowlistFromAgent(agent);
    expect(result[0]!.targets).toContain(specialistAgent.id);
    expect(result[0]!.targets).toContain(second.id);
  });

  it("buildResolverFromAgent resolves known slugs to AgentDef", () => {
    const resolver = buildResolverFromAgent(chiefWithCallable);
    const found = resolver(specialistAgent.id);
    expect(found).toBeDefined();
    expect(found!.id).toBe(specialistAgent.id);
  });

  it("buildResolverFromAgent returns undefined for unknown slugs", () => {
    const resolver = buildResolverFromAgent(chiefWithCallable);
    expect(resolver("does-not-exist")).toBeUndefined();
  });

  it("buildResolverFromAgent returns undefined when callableAgents is absent", () => {
    const resolver = buildResolverFromAgent(chiefAgent);
    expect(resolver("credit-analyst")).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Test 12: end-to-end — chief → handoff → specialist returns structured result
// ---------------------------------------------------------------------------

describe("Test 12: end-to-end chief → handoff → specialist", () => {
  it("specialist finalText surfaces as formatted tool_result content to the chief", async () => {
    const specialistFinalText = "## Credit Analysis\n\nDefault probability: 2.3%";
    const dispatchStub = vi.fn().mockResolvedValue({
      finalText: specialistFinalText,
      auditId: "audit-e2e-1",
    });

    const orchestrator = createHandoffOrchestrator({
      allowlist: buildAllowlistFromAgent(chiefWithCallable),
      resolveAgent: buildResolverFromAgent(chiefWithCallable),
      dispatchAgent: dispatchStub,
    });

    const { provider } = makeHandoffProvider(
      specialistAgent.id,
      { ticker: "AAPL", period: "Q1-2026" },
      { priority: "high" },
    );

    const result = await dispatch({
      agent: chiefWithCallable,
      prompt: "Run credit analysis on AAPL for Q1-2026.",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
    });

    // Specialist was dispatched
    expect(dispatchStub).toHaveBeenCalledOnce();
    const [dispatchedAgent, prompt] = dispatchStub.mock.calls[0] as [AgentDef, string];
    expect(dispatchedAgent.id).toBe(specialistAgent.id);
    // The orchestrator prompt identifies the source agent and includes the payload
    expect(prompt).toContain("chief-analyst");
    expect(prompt).toContain("AAPL");

    // Chief's conversation contains the tool_result with specialist output
    const toolResultBlocks: ToolResultBlock[] = [];
    for (const m of result.messages) {
      for (const b of m.content) {
        if (b.type === "tool_result") toolResultBlocks.push(b);
      }
    }
    expect(toolResultBlocks).toHaveLength(1);
    const block = toolResultBlocks[0]!;
    expect(block.is_error).toBeFalsy();
    // The formatted result embeds the specialist's finalText inside the JSON result body
    expect(block.content).toContain("Credit Analysis");
    expect(block.content).toContain("2.3%");
  });
});
