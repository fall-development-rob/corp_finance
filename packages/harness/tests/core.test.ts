/**
 * Unit tests for the Phase 31 Wave 1 core dispatch loop.
 *
 * All Anthropic API calls are mocked. No network I/O in this suite.
 */
import { describe, it, expect, vi } from "vitest";
import type {
  AgentDef,
  CanonicalTool,
  DispatchOptions,
  MCPClient,
  Message,
  Provider,
  ProviderTurnResponse,
  ToolCall,
  ToolResult,
} from "../src/types.js";
import { dispatch } from "../src/core/agent-loop.js";
import { filterToolsForAgent } from "../src/core/tool-schema.js";
import { routeToolCalls } from "../src/core/tool-router.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const TOOL_A: CanonicalTool = {
  name: "tool_a",
  description: "Tool A",
  input_schema: { type: "object", properties: {}, required: [] },
};

const TOOL_B: CanonicalTool = {
  name: "tool_b",
  description: "Tool B",
  input_schema: { type: "object", properties: {}, required: [] },
};

const AGENT: AgentDef = {
  id: "test-agent",
  description: "Test agent",
  systemPrompt: "You are a test assistant.",
  tools: "*",
  maxRecursionDepth: 0,
};

function makeMCPClient(tools: CanonicalTool[] = [TOOL_A, TOOL_B]): MCPClient {
  return {
    initialize: vi.fn().mockResolvedValue(undefined),
    listTools: vi.fn().mockResolvedValue(tools),
    callTool: vi.fn().mockResolvedValue({ result: "ok" }),
    close: vi.fn().mockResolvedValue(undefined),
  };
}

function makeEndTurnResponse(text = "Done."): ProviderTurnResponse {
  return {
    message: { role: "assistant", content: [{ type: "text", text }] },
    stopReason: "end_turn",
    usage: { inputTokens: 10, outputTokens: 5 },
  };
}

function makeToolUseResponse(toolName: string, toolId = "call_1"): ProviderTurnResponse {
  return {
    message: {
      role: "assistant",
      content: [
        { type: "tool_use", id: toolId, name: toolName, input: { x: 1 } },
      ],
    },
    stopReason: "tool_use",
    usage: { inputTokens: 10, outputTokens: 5 },
  };
}

function makeProvider(responses: ProviderTurnResponse[]): Provider {
  let idx = 0;
  return {
    name: "anthropic",
    turn: vi.fn().mockImplementation(async () => {
      const resp = responses[idx];
      idx++;
      return resp;
    }),
  };
}

function makeOptions(
  overrides: Partial<DispatchOptions> & { provider: Provider; mcp: MCPClient },
): DispatchOptions {
  return {
    agent: AGENT,
    prompt: "Hello",
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Test 1: provider returns end_turn immediately → toolUses=0
// ---------------------------------------------------------------------------

describe("dispatch", () => {
  it("completes immediately on end_turn with toolUses=0", async () => {
    const provider = makeProvider([makeEndTurnResponse("All done.")]);
    const mcp = makeMCPClient();

    const result = await dispatch(makeOptions({ provider, mcp }));

    expect(result.toolUses).toBe(0);
    expect(result.finalText).toBe("All done.");
    expect(result.childDispatches).toEqual([]);
    // messages: user + assistant
    expect(result.messages).toHaveLength(2);
    expect(result.messages[1]!.role).toBe("assistant");
  });

  // -------------------------------------------------------------------------
  // Test 2: tool_use then end_turn → toolUses=1
  // -------------------------------------------------------------------------

  it("routes one tool call and completes on second turn with toolUses=1", async () => {
    const provider = makeProvider([
      makeToolUseResponse("tool_a", "call_1"),
      makeEndTurnResponse("Result processed."),
    ]);
    const mcp = makeMCPClient();

    const result = await dispatch(makeOptions({ provider, mcp }));

    expect(result.toolUses).toBe(1);
    expect(result.finalText).toBe("Result processed.");
    // messages: user, assistant(tool_use), user(tool_result), assistant(text)
    expect(result.messages).toHaveLength(4);

    // Verify MCP was called with the bare tool name
    expect(mcp.callTool).toHaveBeenCalledWith("tool_a", { x: 1 });
  });

  // -------------------------------------------------------------------------
  // Test 3: allowlist filtering
  // -------------------------------------------------------------------------

  it('allowlist "*" returns all tools', () => {
    const filtered = filterToolsForAgent([TOOL_A, TOOL_B], "*");
    expect(filtered).toHaveLength(2);
  });

  it("explicit allowlist returns only matching tools", () => {
    const filtered = filterToolsForAgent([TOOL_A, TOOL_B], ["tool_a"]);
    expect(filtered).toHaveLength(1);
    expect(filtered[0]!.name).toBe("tool_a");
  });

  it("dispatch with explicit tool allowlist only exposes allowed tools to provider", async () => {
    const restrictedAgent: AgentDef = { ...AGENT, tools: ["tool_a"] };
    const provider = makeProvider([makeEndTurnResponse()]);
    const mcp = makeMCPClient();

    await dispatch({ agent: restrictedAgent, prompt: "Hi", provider, mcp });

    const turnArgs = (provider.turn as ReturnType<typeof vi.fn>).mock.calls[0]![0];
    expect(turnArgs.tools).toHaveLength(1);
    expect(turnArgs.tools[0].name).toBe("tool_a");
  });

  // -------------------------------------------------------------------------
  // Test 4: parallel tool execution
  // -------------------------------------------------------------------------

  it("routeToolCalls executes multiple calls in parallel (Promise.all)", async () => {
    const order: string[] = [];

    // Tool A takes 20ms, Tool B takes 5ms. Sequential would finish A then B.
    // Parallel: B resolves first. We record completion order.
    const mcp: MCPClient = {
      initialize: vi.fn(),
      listTools: vi.fn().mockResolvedValue([TOOL_A, TOOL_B]),
      callTool: vi.fn().mockImplementation(async (name: string) => {
        const delay = name === "tool_a" ? 20 : 5;
        await new Promise((r) => setTimeout(r, delay));
        order.push(name);
        return `result_${name}`;
      }),
      close: vi.fn(),
    };

    const calls: ToolCall[] = [
      { id: "c1", name: "tool_a", input: {} },
      { id: "c2", name: "tool_b", input: {} },
    ];

    const results: ToolResult[] = await routeToolCalls(calls, mcp);

    // Both completed
    expect(results).toHaveLength(2);
    expect(results[0]!.call_id).toBe("c1");
    expect(results[1]!.call_id).toBe("c2");

    // Parallel: B (5ms) finishes before A (20ms)
    expect(order).toEqual(["tool_b", "tool_a"]);
  });

  it("routeToolCalls wraps MCP errors in is_error result", async () => {
    const mcp: MCPClient = {
      initialize: vi.fn(),
      listTools: vi.fn(),
      callTool: vi.fn().mockRejectedValue(new Error("MCP failure")),
      close: vi.fn(),
    };

    const results = await routeToolCalls(
      [{ id: "e1", name: "bad_tool", input: {} }],
      mcp,
    );

    expect(results[0]!.is_error).toBe(true);
    expect(results[0]!.content).toContain("MCP failure");
  });
});
