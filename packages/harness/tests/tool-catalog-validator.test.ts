/**
 * Unit + integration tests for the ToolCatalogValidator — Phase 32 Wave 1.
 *
 * Verifies that validateAllowlists / assertAllowlistsValid convert the
 * silent tool_uses=0 failure class into a loud startup error.
 */

import { describe, expect, it, vi } from "vitest";
import type {
  AgentDef,
  CanonicalTool,
  MCPClient,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../src/types.js";
import { dispatch } from "../src/core/agent-loop.js";
import {
  assertAllowlistsValid,
  validateAllowlists,
} from "../src/core/tool-schema.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeAgent(id: string, tools: string[] | "*"): AgentDef {
  return {
    id,
    description: `${id} agent`,
    systemPrompt: `You are ${id}.`,
    tools,
    maxRecursionDepth: 0,
  };
}

function makeCatalog(...names: string[]): Set<string> {
  return new Set(names);
}

function makeMCPClient(tools: CanonicalTool[]): MCPClient {
  return {
    initialize: vi.fn().mockResolvedValue(undefined),
    listTools: vi.fn().mockResolvedValue(tools),
    callTool: vi.fn().mockResolvedValue({ result: "ok" }),
    close: vi.fn().mockResolvedValue(undefined),
  };
}

function makeEndTurnProvider(): Provider {
  return {
    name: "anthropic" as const,
    turn: vi.fn().mockImplementation(
      async (_req: ProviderTurnRequest): Promise<ProviderTurnResponse> => ({
        message: {
          role: "assistant",
          content: [{ type: "text", text: "Done." }],
        },
        stopReason: "end_turn",
      }),
    ),
  };
}

// ---------------------------------------------------------------------------
// Test 1: happy path — all allowlist names in catalog → ok: true, issues: []
// ---------------------------------------------------------------------------

describe("validateAllowlists", () => {
  it("returns ok=true and empty issues when all tools are in the catalog", () => {
    const agents = [
      makeAgent("agent-a", ["tool_alpha", "tool_beta"]),
      makeAgent("agent-b", ["tool_alpha"]),
    ];
    const catalog = makeCatalog("tool_alpha", "tool_beta", "tool_gamma");

    const result = validateAllowlists(agents, catalog);

    expect(result.ok).toBe(true);
    expect(result.issues).toHaveLength(0);
  });

  // -------------------------------------------------------------------------
  // Test 2: single missing tool → ok: false, issues.length === 1
  // -------------------------------------------------------------------------

  it("returns ok=false with one issue when a single tool is missing", () => {
    const agents = [makeAgent("agent-x", ["real_tool", "ghost_tool"])];
    const catalog = makeCatalog("real_tool");

    const result = validateAllowlists(agents, catalog);

    expect(result.ok).toBe(false);
    expect(result.issues).toHaveLength(1);
    expect(result.issues[0]).toEqual({
      agent_id: "agent-x",
      unknown_tool: "ghost_tool",
    });
  });

  // -------------------------------------------------------------------------
  // Test 3: multiple agents with issues → sorted by agent_id then unknown_tool
  // -------------------------------------------------------------------------

  it("reports issues for multiple agents in stable sorted order", () => {
    const agents = [
      makeAgent("zebra-agent", ["missing_z1", "missing_z2"]),
      makeAgent("alpha-agent", ["missing_a1", "real_tool"]),
    ];
    const catalog = makeCatalog("real_tool");

    const result = validateAllowlists(agents, catalog);

    expect(result.ok).toBe(false);
    expect(result.issues).toHaveLength(3);

    // Must be sorted: alpha-agent first, then zebra-agent; within each agent
    // alphabetically by unknown_tool.
    expect(result.issues[0]).toEqual({ agent_id: "alpha-agent", unknown_tool: "missing_a1" });
    expect(result.issues[1]).toEqual({ agent_id: "zebra-agent", unknown_tool: "missing_z1" });
    expect(result.issues[2]).toEqual({ agent_id: "zebra-agent", unknown_tool: "missing_z2" });
  });

  // -------------------------------------------------------------------------
  // Test 4: agents with tools: "*" are skipped entirely
  // -------------------------------------------------------------------------

  it('skips agents with tools: "*" even when catalog is empty', () => {
    const agents = [
      makeAgent("wildcard-agent", "*"),
      makeAgent("explicit-agent", ["present_tool"]),
    ];
    const catalog = makeCatalog("present_tool");

    const result = validateAllowlists(agents, catalog);

    expect(result.ok).toBe(true);
    expect(result.issues).toHaveLength(0);
  });

  it('returns ok=true for a wildcard-only agent list against an empty catalog', () => {
    const agents = [makeAgent("all-tools", "*")];
    const result = validateAllowlists(agents, new Set());
    expect(result.ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 5: assertAllowlistsValid throws with documented format
// ---------------------------------------------------------------------------

describe("assertAllowlistsValid", () => {
  it("throws an Error with the documented message format on violation", () => {
    const agents = [makeAgent("my-agent", ["nonexistent_tool", "also_missing"])];
    const catalog = makeCatalog("some_other_tool");

    let caught: Error | undefined;
    try {
      assertAllowlistsValid(agents, catalog);
    } catch (err) {
      caught = err as Error;
    }

    expect(caught).toBeInstanceOf(Error);
    expect(caught!.message).toMatch(/^ToolCatalog validation failed \(2 issues\):/);
    expect(caught!.message).toContain('my-agent: unknown tool "also_missing"');
    expect(caught!.message).toContain('my-agent: unknown tool "nonexistent_tool"');
    expect(caught!.message).toContain("Hint:");
    expect(caught!.message).toContain("4 plugin MCP servers");
  });

  // -------------------------------------------------------------------------
  // Test 6: assertAllowlistsValid does NOT throw on a clean validation
  // -------------------------------------------------------------------------

  it("does not throw when all allowlists are valid", () => {
    const agents = [makeAgent("clean-agent", ["tool_a", "tool_b"])];
    const catalog = makeCatalog("tool_a", "tool_b", "tool_c");

    expect(() => assertAllowlistsValid(agents, catalog)).not.toThrow();
  });
});

// ---------------------------------------------------------------------------
// Test 7: integration — dispatch throws BEFORE provider.turn() on unknown tool
// ---------------------------------------------------------------------------

describe("dispatch integration — ToolCatalogValidator", () => {
  it("throws before provider.turn() when agent allowlist references an unknown tool", async () => {
    const agentWithBadTool: AgentDef = {
      id: "bad-agent",
      description: "Agent with invalid tool ref.",
      systemPrompt: "You are a bad agent.",
      tools: ["nonexistent_tool"],
      maxRecursionDepth: 0,
    };

    const realTool: CanonicalTool = {
      name: "real_tool",
      description: "A real tool.",
      input_schema: { type: "object", properties: {}, required: [] },
    };

    const mcp = makeMCPClient([realTool]);
    const provider = makeEndTurnProvider();

    let caught: Error | undefined;
    try {
      await dispatch({ agent: agentWithBadTool, prompt: "Hi", mcp, provider });
    } catch (err) {
      caught = err as Error;
    }

    // Dispatch must have thrown before any provider turn.
    expect(caught).toBeInstanceOf(Error);
    expect(caught!.message).toContain("ToolCatalog validation failed");
    expect(caught!.message).toContain('bad-agent: unknown tool "nonexistent_tool"');

    // Provider must NOT have been called.
    expect(provider.turn).not.toHaveBeenCalled();
  });
});
