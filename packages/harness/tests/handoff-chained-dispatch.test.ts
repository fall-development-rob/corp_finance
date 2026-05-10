/**
 * REC-4 Wave 5 — chained-handoff dispatch tests.
 *
 * Covers the relaxed `maxHandoffDepth` guard introduced in W5:
 *   1.  maxHandoffDepth default (0): depth=0 sees tool
 *   2.  maxHandoffDepth default (0): depth=1 does NOT see tool
 *   3.  maxHandoffDepth=1: depth=0 sees tool
 *   4.  maxHandoffDepth=1: depth=1 sees tool
 *   5.  maxHandoffDepth=1: depth=2 does NOT see tool (depth > maxHandoffDepth)
 *   6.  block_tools precedence: blockTools:["initiate_handoff"] wins over maxHandoffDepth
 *   7.  End-to-end chained handoff: chief→A→B; 2 handoff_initiated / completed events
 *   8.  Chain depth cap: orchestrator rejects when currentDepth >= maxDepth
 *   9.  handoffMode "disabled": tool never injected at any depth
 *   10. Per-source allowlist scoping at depth=1: A→B allowed; chief→B blocked
 */
import { describe, expect, it, vi } from "vitest";
import { dispatch } from "../src/core/agent-loop.js";
import {
  createHandoffOrchestrator,
  buildAllowlistResolver,
  HANDOFF_TOOL_NAME,
  type HandoffOrchestratorOptions,
} from "../src/handoff/index.js";
import type {
  AgentDef,
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
  input_schema: { type: "object", properties: { x: { type: "number" } } },
};

function makeMCP(tools: CanonicalTool[] = [tinyTool]): MCPClient {
  return {
    initialize: async () => {},
    listTools: async () => tools,
    callTool: async () => ({ content: [{ type: "text", text: "ok" }] }),
    close: async () => {},
  };
}

/** Passive provider — captures tool list per turn and ends immediately. */
function makeCapturing(): { provider: Provider; toolsSeen: CanonicalTool[][] } {
  const toolsSeen: CanonicalTool[][] = [];
  const provider: Provider = {
    name: "anthropic" as const,
    async turn(req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      toolsSeen.push(req.tools);
      return {
        message: { role: "assistant", content: [{ type: "text", text: "done" }] },
        stopReason: "end_turn",
        usage: { inputTokens: 1, outputTokens: 1 },
      };
    },
  };
  return { provider, toolsSeen };
}

/**
 * Provider that fires `initiate_handoff` on turn 1 then ends.
 * Captures tool lists for all turns.
 */
function makeHandoffProvider(
  target: string,
  payload: Record<string, unknown>,
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
                id: `call-handoff-${target}`,
                name: HANDOFF_TOOL_NAME,
                input: { target, payload },
              },
            ],
          },
          stopReason: "tool_use",
          usage: { inputTokens: 10, outputTokens: 5 },
        };
      }
      return {
        message: { role: "assistant", content: [{ type: "text", text: "done" }] },
        stopReason: "end_turn",
        usage: { inputTokens: 5, outputTokens: 3 },
      };
    },
  };
  return { provider, toolsSeen };
}

/** Minimal agent defs */
const chiefAgent: AgentDef = {
  id: "chief-analyst",
  description: "Chief.",
  systemPrompt: "You are chief.",
  tools: ["tiny_tool"],
  model: "claude-test",
  maxTokens: 1024,
};

const specA: AgentDef = {
  id: "spec-a",
  description: "Specialist A.",
  systemPrompt: "You are spec-a.",
  tools: ["tiny_tool"],
  model: "claude-test",
  maxTokens: 1024,
};

const specB: AgentDef = {
  id: "spec-b",
  description: "Specialist B.",
  systemPrompt: "You are spec-b.",
  tools: ["tiny_tool"],
  model: "claude-test",
  maxTokens: 1024,
};

function makeDispatchStub(
  response = { finalText: "specialist result", auditId: "audit-1" },
): HandoffOrchestratorOptions["dispatchAgent"] {
  return vi.fn().mockResolvedValue(response);
}

/** Build a simple orchestrator with chief→specA allowlist. */
function makeOrchestrator(
  dispatchStub: HandoffOrchestratorOptions["dispatchAgent"] = makeDispatchStub(),
  opts: Partial<HandoffOrchestratorOptions> = {},
) {
  return createHandoffOrchestrator({
    allowlist: [{ source: chiefAgent.id, targets: [specA.id] }],
    resolveAgent: (slug) => {
      if (slug === specA.id) return specA;
      if (slug === specB.id) return specB;
      return undefined;
    },
    dispatchAgent: dispatchStub,
    ...opts,
  });
}

// ---------------------------------------------------------------------------
// Test 1: maxHandoffDepth default (0) — depth=0 sees tool
// ---------------------------------------------------------------------------

describe("Test 1: maxHandoffDepth default — depth=0 sees tool", () => {
  it("no maxHandoffDepth set, depth=0 → initiate_handoff injected", async () => {
    const { provider, toolsSeen } = makeCapturing();

    await dispatch({
      agent: chiefAgent,
      prompt: "test",
      provider,
      mcp: makeMCP(),
      handoff: makeOrchestrator(),
      // maxHandoffDepth intentionally omitted → defaults to 0
    });

    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 2: maxHandoffDepth default (0) — depth=1 does NOT see tool
// ---------------------------------------------------------------------------

describe("Test 2: maxHandoffDepth default — depth=1 excluded", () => {
  it("no maxHandoffDepth set, depth=1 → no initiate_handoff", async () => {
    const { provider, toolsSeen } = makeCapturing();

    await dispatch({
      agent: specA,
      prompt: "specialist run",
      provider,
      mcp: makeMCP(),
      handoff: makeOrchestrator(),
      depth: 1,
      // maxHandoffDepth defaults to 0 → depth 1 > 0 → excluded
    });

    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Test 3: maxHandoffDepth=1 at depth=0 → tool present
// ---------------------------------------------------------------------------

describe("Test 3: maxHandoffDepth=1 at depth=0 → tool present", () => {
  it("maxHandoffDepth=1, depth=0 → initiate_handoff injected", async () => {
    const { provider, toolsSeen } = makeCapturing();

    await dispatch({
      agent: chiefAgent,
      prompt: "test",
      provider,
      mcp: makeMCP(),
      handoff: makeOrchestrator(),
      maxHandoffDepth: 1,
    });

    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 4: maxHandoffDepth=1 at depth=1 → tool present
// ---------------------------------------------------------------------------

describe("Test 4: maxHandoffDepth=1 at depth=1 → tool present", () => {
  it("maxHandoffDepth=1, depth=1 → initiate_handoff injected", async () => {
    const { provider, toolsSeen } = makeCapturing();

    await dispatch({
      agent: specA,
      prompt: "specialist with handoff access",
      provider,
      mcp: makeMCP(),
      handoff: makeOrchestrator(),
      depth: 1,
      maxHandoffDepth: 1,
    });

    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 5: maxHandoffDepth=1 at depth=2 → tool absent
// ---------------------------------------------------------------------------

describe("Test 5: maxHandoffDepth=1 at depth=2 → tool absent", () => {
  it("maxHandoffDepth=1, depth=2 → no initiate_handoff (depth > max)", async () => {
    const { provider, toolsSeen } = makeCapturing();

    await dispatch({
      agent: specB,
      prompt: "deep specialist",
      provider,
      mcp: makeMCP(),
      handoff: makeOrchestrator(),
      depth: 2,
      maxHandoffDepth: 1,
    });

    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Test 6: block_tools precedence — blockTools wins over maxHandoffDepth
// ---------------------------------------------------------------------------

describe("Test 6: block_tools precedence over maxHandoffDepth", () => {
  it("specialist with blockTools:[initiate_handoff] never sees tool even at depth=1 with maxHandoffDepth=2", async () => {
    const { provider, toolsSeen } = makeCapturing();
    const events: DispatchEvent[] = [];

    const blockedSpecialist: AgentDef = {
      ...specA,
      blockTools: [HANDOFF_TOOL_NAME],
    };

    await dispatch({
      agent: blockedSpecialist,
      prompt: "blocked specialist",
      provider,
      mcp: makeMCP(),
      handoff: makeOrchestrator(),
      depth: 1,
      maxHandoffDepth: 2,
      onEvent: (e) => events.push(e),
    });

    // Tool must not be in the assembled list
    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);

    // tool_blocked event must have fired
    const blocked = events.filter((e) => e.type === "tool_blocked") as Array<
      Extract<DispatchEvent, { type: "tool_blocked" }>
    >;
    expect(blocked.some((e) => e.toolName === HANDOFF_TOOL_NAME)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 7: End-to-end chained handoff: chief→A→B
// ---------------------------------------------------------------------------

describe("Test 7: end-to-end chained handoff chief→A→B", () => {
  it("produces 2 handoff_initiated + 2 handoff_completed events and threads results", async () => {
    const handoffEvents: DispatchEvent[] = [];
    const allEvents: DispatchEvent[] = [];

    // Agents with callable_agents declarations for resolver
    const chiefWithCallable: AgentDef = {
      ...chiefAgent,
      callableAgents: [{ ...specA, callableAgents: [specB] }],
    };
    const specAWithCallable: AgentDef = {
      ...specA,
      callableAgents: [specB],
    };

    const allAgents: AgentDef[] = [chiefWithCallable, specAWithCallable, specB];
    const resolveAllowlist = buildAllowlistResolver(allAgents);

    const mcp = makeMCP();

    // The dispatch callback is what the orchestrator calls when it executes a handoff.
    // We need it to invoke the real `dispatch` function so that spec-a (at depth=1)
    // can itself call initiate_handoff and chain to spec-b.
    //
    // The orchestrator is created before the callback is wired; use a `let` reference
    // so the closure can refer to `orchestrator` after construction.
    let orchestrator: ReturnType<typeof createHandoffOrchestrator>;

    const dispatchCallback: HandoffOrchestratorOptions["dispatchAgent"] = async (
      agent,
      prompt,
      parentDepth,
    ) => {
      // spec-a will call initiate_handoff to chain to spec-b
      const agentProvider =
        agent.id === specA.id
          ? makeHandoffProvider(specB.id, { forwarded: true }).provider
          : makeCapturing().provider;

      const result = await dispatch({
        agent,
        prompt,
        provider: agentProvider,
        mcp,
        handoff: orchestrator,
        depth: parentDepth,
        maxHandoffDepth: 1, // allow depth=1 specialist to see the tool
        onEvent: (e) => allEvents.push(e),
      });
      return { finalText: result.finalText, auditId: result.auditId };
    };

    orchestrator = createHandoffOrchestrator({
      allowlist: [{ source: chiefWithCallable.id, targets: [specA.id] }],
      resolveAllowlist,
      resolveAgent: (slug) => allAgents.find((a) => a.id === slug),
      dispatchAgent: dispatchCallback,
    });

    // Chief provider fires handoff to spec-a
    const { provider: chiefProvider } = makeHandoffProvider(specA.id, { task: "chain" });

    await dispatch({
      agent: chiefWithCallable,
      prompt: "chain test",
      provider: chiefProvider,
      mcp,
      handoff: orchestrator,
      maxHandoffDepth: 0, // chief at depth=0: tool injected
      onEvent: (e) => {
        allEvents.push(e);
        handoffEvents.push(e);
      },
    });

    // Chief→A handoff fires one initiated + one returned at the dispatch level
    const initiated = allEvents.filter((e) => e.type === "handoff_initiated");
    const returned = allEvents.filter((e) => e.type === "handoff_returned");

    // There should be 2 handoff_initiated: chief→A (outer dispatch) + A→B (inner dispatch)
    expect(initiated.length).toBeGreaterThanOrEqual(2);
    expect(returned.length).toBeGreaterThanOrEqual(2);

    // Verify the agent targets appear in the events
    const targets = initiated.map(
      (e) => (e as Extract<DispatchEvent, { type: "handoff_initiated" }>).target_agent,
    );
    expect(targets).toContain(specA.id);
    expect(targets).toContain(specB.id);
  });
});

// ---------------------------------------------------------------------------
// Test 8: chain depth cap — orchestrator rejects when currentDepth >= maxDepth
// ---------------------------------------------------------------------------

describe("Test 8: chain depth cap — orchestrator rejects at maxDepth", () => {
  it("orchestrator with maxDepth=2 rejects execute at currentDepth=2", async () => {
    const dispatchStub = makeDispatchStub();
    const orch = createHandoffOrchestrator({
      allowlist: [{ source: chiefAgent.id, targets: [specA.id] }],
      resolveAgent: (slug) => (slug === specA.id ? specA : undefined),
      dispatchAgent: dispatchStub,
      maxDepth: 2,
    });

    // Direct orchestrator call at currentDepth=2 (equals maxDepth) → reject
    const result = await orch.execute(
      {
        source_agent: chiefAgent.id,
        target_agent: specA.id,
        payload: { data: "x" },
      },
      2,
    );

    expect(result.ok).toBe(false);
    expect(result.reason).toMatch(/max chain depth/i);
    expect(dispatchStub).not.toHaveBeenCalled();
  });

  it("orchestrator with maxDepth=2 allows execute at currentDepth=1", async () => {
    const dispatchStub = makeDispatchStub();
    const orch = createHandoffOrchestrator({
      allowlist: [{ source: chiefAgent.id, targets: [specA.id] }],
      resolveAgent: (slug) => (slug === specA.id ? specA : undefined),
      dispatchAgent: dispatchStub,
      maxDepth: 2,
    });

    const result = await orch.execute(
      {
        source_agent: chiefAgent.id,
        target_agent: specA.id,
        payload: { data: "x" },
      },
      1,
    );

    expect(result.ok).toBe(true);
    expect(dispatchStub).toHaveBeenCalledOnce();
  });
});

// ---------------------------------------------------------------------------
// Test 9: handoffMode "disabled" — tool never injected at any depth
// ---------------------------------------------------------------------------

describe("Test 9: handoffMode disabled — tool never injected", () => {
  it("handoffMode=disabled + maxHandoffDepth=5 → no initiate_handoff at depth=0", async () => {
    const { provider, toolsSeen } = makeCapturing();

    await dispatch({
      agent: chiefAgent,
      prompt: "disabled mode",
      provider,
      mcp: makeMCP(),
      handoff: makeOrchestrator(),
      handoffMode: "disabled",
      maxHandoffDepth: 5, // high value, but disabled wins
    });

    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);
  });

  it("handoffMode=disabled + maxHandoffDepth=5 → no initiate_handoff at depth=1", async () => {
    const { provider, toolsSeen } = makeCapturing();

    await dispatch({
      agent: specA,
      prompt: "disabled at depth 1",
      provider,
      mcp: makeMCP(),
      handoff: makeOrchestrator(),
      handoffMode: "disabled",
      maxHandoffDepth: 5,
      depth: 1,
    });

    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Test 10: per-source allowlist scoping at depth=1
// ---------------------------------------------------------------------------

describe("Test 10: per-source allowlist scoping at depth=1", () => {
  it("A→B allowed (A has callableAgents=[B]); chief→B blocked (not in chief allowlist)", async () => {
    const chiefWithCallable: AgentDef = {
      ...chiefAgent,
      callableAgents: [{ ...specA, callableAgents: [specB] }],
    };
    const specAWithCallable: AgentDef = {
      ...specA,
      callableAgents: [specB],
    };

    const allAgents = [chiefWithCallable, specAWithCallable, specB];
    const resolveAllowlist = buildAllowlistResolver(allAgents);

    const orch = createHandoffOrchestrator({
      allowlist: [{ source: chiefWithCallable.id, targets: [specA.id] }],
      resolveAllowlist,
      resolveAgent: (slug) => allAgents.find((a) => a.id === slug),
      dispatchAgent: makeDispatchStub(),
    });

    // A→B: should be allowed
    expect(orch.isAllowed(specA.id, specB.id)).toBe(true);

    // chief→B: not in chief's allowlist (chief can only reach spec-a directly)
    expect(orch.isAllowed(chiefAgent.id, specB.id)).toBe(false);

    // chief→A: allowed
    expect(orch.isAllowed(chiefAgent.id, specA.id)).toBe(true);
  });

  it("spec-a at depth=1 with maxHandoffDepth=1 receives tool and can call handoff to spec-b", async () => {
    // This verifies the injection + allowlist together work for spec-a at depth=1.
    const { provider: providerA, toolsSeen } = makeCapturing();

    const specAWithCallable: AgentDef = {
      ...specA,
      callableAgents: [specB],
    };
    const allAgents = [{ ...chiefAgent, callableAgents: [specAWithCallable] }, specAWithCallable, specB];
    const resolveAllowlist = buildAllowlistResolver(allAgents);

    const orch = createHandoffOrchestrator({
      allowlist: [{ source: chiefAgent.id, targets: [specA.id] }],
      resolveAllowlist,
      resolveAgent: (slug) => allAgents.find((a) => a.id === slug),
      dispatchAgent: makeDispatchStub(),
    });

    await dispatch({
      agent: specAWithCallable,
      prompt: "I am spec-a at depth 1",
      provider: providerA,
      mcp: makeMCP(),
      handoff: orch,
      depth: 1,
      maxHandoffDepth: 1,
    });

    // spec-a at depth=1 with maxHandoffDepth=1 → tool must be present
    expect(toolsSeen[0]!.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(true);

    // And the allowlist correctly scopes A→B
    expect(orch.isAllowed(specA.id, specB.id)).toBe(true);
  });
});
