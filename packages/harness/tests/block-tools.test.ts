/**
 * Phase 33: Hermes-inspired block-list tests.
 *
 * Verifies that agent.blockTools[] is enforced at dispatch time — blocked
 * tools are stripped from the assembled tool list (real + virtual) before
 * the first model turn, so the model never sees them. Blocklist takes
 * precedence over allowlist on conflicts.
 *
 * Tests:
 *   1.  No blockTools → tool list unchanged (smoke)
 *   2.  blockTools includes a virtual delegation tool → stripped; others remain
 *   3.  blockTools includes recall tools → both stripped; other virtual tools remain
 *   4.  blockTools includes initiate_handoff → handoff tool stripped
 *   5.  blockTools includes a real MCP tool name → stripped from realTools
 *   6.  blockTools overrides allowlist — tool present in both is stripped
 *   7.  tool_blocked events fire once per unique stripped tool
 *   8.  specialist's own blockTools applies during nested dispatch (not chief's)
 *   9.  yaml-loader: block_tools in manifest → AgentDef.blockTools
 *   10. integration via yaml-loader: loaded specialist with block_tools cannot
 *       see the blocked tool even when injected by orchestration
 */
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dispatch } from "../src/core/agent-loop.js";
import {
  createHandoffOrchestrator,
  HANDOFF_TOOL_NAME,
} from "../src/handoff/index.js";
import { createDirectYamlManifestLoader } from "../src/manifests/yaml-loader.js";
import type {
  AgentDef,
  CanonicalTool,
  DispatchEvent,
  MCPClient,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../src/types.js";
import type { SkillLoader } from "../src/skills/types.js";
import type { ParsedSkill } from "../src/skills/types.js";

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

const tinyTool: CanonicalTool = {
  name: "tiny_tool",
  description: "A tiny test tool.",
  input_schema: { type: "object", properties: { x: { type: "number" } }, required: ["x"] },
};

const waccTool: CanonicalTool = {
  name: "wacc_calculator",
  description: "Computes WACC.",
  input_schema: { type: "object", properties: {}, required: [] },
};

const xlsxWriteTool: CanonicalTool = {
  name: "office_xlsx_write",
  description: "Writes an XLSX workbook.",
  input_schema: { type: "object", properties: {}, required: [] },
};

function makeMCPClient(tools: CanonicalTool[] = [tinyTool]): MCPClient {
  return {
    initialize: async () => {},
    listTools: async () => tools,
    callTool: async () => ({ content: [{ type: "text", text: "ok" }] }),
    close: async () => {},
  };
}

/** Provider that captures each turn's tool list and ends immediately. */
function makeToolCapturingProvider(): { provider: Provider; toolsSeen: CanonicalTool[][] } {
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

/** Provider that issues a delegation call on turn 1 then ends on turn 2. */
function makeDelegatingProvider(delegateName: string): Provider {
  let turn = 0;
  return {
    name: "anthropic" as const,
    async turn(): Promise<ProviderTurnResponse> {
      turn += 1;
      if (turn === 1) {
        return {
          message: {
            role: "assistant",
            content: [
              {
                type: "tool_use",
                id: "call-delegate-1",
                name: delegateName,
                input: { sub_prompt: "do the work" },
              },
            ],
          },
          stopReason: "tool_use",
          usage: { inputTokens: 5, outputTokens: 3 },
        };
      }
      return {
        message: { role: "assistant", content: [{ type: "text", text: "done" }] },
        stopReason: "end_turn",
        usage: { inputTokens: 2, outputTokens: 1 },
      };
    },
  };
}

const baseChief: AgentDef = {
  id: "chief-analyst",
  description: "Chief test agent.",
  systemPrompt: "You are the test chief.",
  tools: ["tiny_tool"],
  maxRecursionDepth: 1,
  model: "claude-test",
  maxTokens: 1024,
};

const specialistA: AgentDef = {
  id: "specialist-a",
  description: "Specialist A.",
  systemPrompt: "You are specialist A.",
  tools: ["tiny_tool"],
  model: "claude-test",
  maxTokens: 1024,
};

// ---------------------------------------------------------------------------
// YAML loader helpers (for tests 9 and 10)
// ---------------------------------------------------------------------------

let tmpDir: string;

beforeEach(() => {
  tmpDir = mkdtempSync(join(tmpdir(), "block-tools-test-"));
});

afterEach(() => {
  rmSync(tmpDir, { recursive: true, force: true });
});

function writeFixture(relPath: string, content: string): string {
  const full = join(tmpDir, relPath);
  mkdirSync(full.replace(/\/[^/]+$/, ""), { recursive: true });
  writeFileSync(full, content, "utf-8");
  return full;
}

function makeSkillLoader(): SkillLoader {
  return {
    async loadSkill(id: string): Promise<ParsedSkill> {
      throw new Error(`Skill "${id}" not found`);
    },
    async loadAgent() {
      throw new Error("not used");
    },
    clearCache() {},
  };
}

// ---------------------------------------------------------------------------
// Test 1: No blockTools → tool list unchanged (smoke)
// ---------------------------------------------------------------------------

describe("Test 1: no blockTools — tool list unchanged", () => {
  it("agent without blockTools receives all assembled tools", async () => {
    const { provider, toolsSeen } = makeToolCapturingProvider();
    // tools: "*" — both MCP tools should pass through unfiltered
    const agent: AgentDef = { ...baseChief, tools: "*", blockTools: undefined };

    await dispatch({
      agent,
      prompt: "hello",
      provider,
      mcp: makeMCPClient([tinyTool, waccTool]),
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === "tiny_tool")).toBe(true);
    expect(firstTurnTools.some((t) => t.name === "wacc_calculator")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 2: blockTools blocks a virtual delegation tool; others remain
// ---------------------------------------------------------------------------

describe("Test 2: blockTools strips a virtual delegation tool", () => {
  it("delegate_to_specialist-a stripped; other delegates still present", async () => {
    const specialistB: AgentDef = {
      id: "specialist-b",
      description: "Specialist B.",
      systemPrompt: "You are specialist B.",
      // "*" so the ACL validator skips it — we're testing virtual tool blocking, not allowlists
      tools: "*",
    };
    const agent: AgentDef = {
      ...baseChief,
      blockTools: ["delegate_to_specialist_a"],
    };
    const { provider, toolsSeen } = makeToolCapturingProvider();

    await dispatch({
      agent,
      prompt: "delegate test",
      provider,
      mcp: makeMCPClient(),
      delegates: [specialistA, specialistB],
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === "delegate_to_specialist_a")).toBe(false);
    expect(firstTurnTools.some((t) => t.name === "delegate_to_specialist_b")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 3: blockTools strips recall tools; delegation + workflow + handoff remain
// ---------------------------------------------------------------------------

describe("Test 3: blockTools strips recall_similar and recall_by_graph", () => {
  it("both recall tools stripped; delegation tool remains when delegates configured", async () => {
    const agent: AgentDef = {
      ...baseChief,
      blockTools: ["recall_similar", "recall_by_graph"],
    };
    const { provider, toolsSeen } = makeToolCapturingProvider();

    const fakeBank = {
      index: vi.fn(),
      search: vi.fn().mockResolvedValue([]),
      searchByGraph: vi.fn().mockResolvedValue([]),
    };

    await dispatch({
      agent,
      prompt: "recall test",
      provider,
      mcp: makeMCPClient(),
      delegates: [specialistA],
      reasoning: fakeBank as never,
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === "recall_similar")).toBe(false);
    expect(firstTurnTools.some((t) => t.name === "recall_by_graph")).toBe(false);
    // Delegation tool should still be present
    expect(firstTurnTools.some((t) => t.name === "delegate_to_specialist_a")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 4: blockTools includes initiate_handoff → handoff tool stripped
// ---------------------------------------------------------------------------

describe("Test 4: blockTools strips initiate_handoff", () => {
  it("agent with blockTools:[initiate_handoff] does not see the handoff tool", async () => {
    const agent: AgentDef = {
      ...baseChief,
      maxRecursionDepth: 0,
      blockTools: [HANDOFF_TOOL_NAME],
    };
    const { provider, toolsSeen } = makeToolCapturingProvider();
    const orchestrator = createHandoffOrchestrator({
      allowlist: [{ source: agent.id, targets: [specialistA.id] }],
      resolveAgent: (slug) => (slug === specialistA.id ? specialistA : undefined),
      dispatchAgent: vi.fn().mockResolvedValue({ finalText: "ok" }),
    });

    await dispatch({
      agent,
      prompt: "handoff test",
      provider,
      mcp: makeMCPClient(),
      handoff: orchestrator,
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);
    // Real tool still present
    expect(firstTurnTools.some((t) => t.name === "tiny_tool")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 5: blockTools blocks a real MCP tool name → stripped from realTools
// ---------------------------------------------------------------------------

describe("Test 5: blockTools strips a real MCP tool", () => {
  it("office_xlsx_write is stripped from realTools when in blockTools", async () => {
    const agent: AgentDef = {
      ...baseChief,
      tools: "*",
      blockTools: ["office_xlsx_write"],
    };
    const { provider, toolsSeen } = makeToolCapturingProvider();

    await dispatch({
      agent,
      prompt: "xlsx block test",
      provider,
      mcp: makeMCPClient([tinyTool, xlsxWriteTool]),
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === "office_xlsx_write")).toBe(false);
    expect(firstTurnTools.some((t) => t.name === "tiny_tool")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test 6: blockTools overrides allowlist — tool in both is stripped
// ---------------------------------------------------------------------------

describe("Test 6: blockTools takes precedence over allowlist", () => {
  it("wacc_calculator is stripped even though it is in agent.tools allowlist", async () => {
    const agent: AgentDef = {
      ...baseChief,
      tools: ["wacc_calculator"],
      blockTools: ["wacc_calculator"],
    };
    const { provider, toolsSeen } = makeToolCapturingProvider();

    await dispatch({
      agent,
      prompt: "allowlist conflict test",
      provider,
      mcp: makeMCPClient([waccTool]),
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === "wacc_calculator")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Test 7: tool_blocked events fire once per unique stripped tool
// ---------------------------------------------------------------------------

describe("Test 7: tool_blocked events fire for each stripped tool", () => {
  it("one tool_blocked event emitted per unique tool stripped", async () => {
    const agent: AgentDef = {
      ...baseChief,
      tools: "*",
      blockTools: ["tiny_tool", "office_xlsx_write"],
    };
    const { provider } = makeToolCapturingProvider();
    const events: DispatchEvent[] = [];

    await dispatch({
      agent,
      prompt: "event test",
      provider,
      mcp: makeMCPClient([tinyTool, xlsxWriteTool, waccTool]),
      onEvent: (e) => events.push(e),
    });

    const blocked = events.filter(
      (e): e is Extract<DispatchEvent, { type: "tool_blocked" }> =>
        e.type === "tool_blocked",
    );
    expect(blocked).toHaveLength(2);
    const blockedNames = blocked.map((e) => e.toolName).sort();
    expect(blockedNames).toEqual(["office_xlsx_write", "tiny_tool"]);
    blocked.forEach((e) => expect(e.reason).toBe("blockTools"));
    // wacc_calculator is NOT blocked
    expect(blocked.some((e) => e.toolName === "wacc_calculator")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Test 8: specialist's own blockTools applies during its own dispatch
// ---------------------------------------------------------------------------
//
// Each dispatch call uses the blockTools of the agent passed as `options.agent`.
// A specialist dispatched directly uses ITS OWN blockTools — the chief's
// blockTools do not apply. This test verifies two things:
//   (a) specialist with blockTools: [office_xlsx_write] never sees that tool
//   (b) a separate chief dispatch with no blockTools sees both tools
//
describe("Test 8: specialist blockTools applies during nested dispatch", () => {
  it("specialist blockTools strips its blocked tool; chief with no blockTools sees all tools", async () => {
    // Specialist blocks xlsx_write
    const specialist: AgentDef = {
      ...specialistA,
      tools: "*",
      blockTools: ["office_xlsx_write"],
    };
    // Chief has no blockTools
    const chiefAgent: AgentDef = {
      ...baseChief,
      tools: "*",
      blockTools: undefined,
    };

    const specialistToolsSeen: CanonicalTool[][] = [];
    const chiefToolsSeen: CanonicalTool[][] = [];

    const makeCapturing = (sink: CanonicalTool[][]): Provider => ({
      name: "anthropic" as const,
      async turn(req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
        sink.push(req.tools);
        return {
          message: { role: "assistant", content: [{ type: "text", text: "done" }] },
          stopReason: "end_turn",
        };
      },
    });

    const mcp = makeMCPClient([tinyTool, xlsxWriteTool]);

    // Dispatch specialist directly
    const specialistEvents: DispatchEvent[] = [];
    await dispatch({
      agent: specialist,
      prompt: "specialist run",
      provider: makeCapturing(specialistToolsSeen),
      mcp,
      onEvent: (e) => specialistEvents.push(e),
    });

    // Dispatch chief directly
    await dispatch({
      agent: chiefAgent,
      prompt: "chief run",
      provider: makeCapturing(chiefToolsSeen),
      mcp,
    });

    // Specialist must NOT see office_xlsx_write
    const specialistFirst = specialistToolsSeen[0]!;
    expect(specialistFirst.some((t) => t.name === "office_xlsx_write")).toBe(false);
    expect(specialistFirst.some((t) => t.name === "tiny_tool")).toBe(true);

    // Chief must see both (no blockTools on chief)
    const chiefFirst = chiefToolsSeen[0]!;
    expect(chiefFirst.some((t) => t.name === "office_xlsx_write")).toBe(true);
    expect(chiefFirst.some((t) => t.name === "tiny_tool")).toBe(true);

    // tool_blocked event fired for specialist's blocked tool
    const blocked = specialistEvents.filter(
      (e): e is Extract<DispatchEvent, { type: "tool_blocked" }> =>
        e.type === "tool_blocked",
    );
    expect(blocked).toHaveLength(1);
    expect(blocked[0]!.toolName).toBe("office_xlsx_write");
    expect(blocked[0]!.reason).toBe("blockTools");
  });
});

// ---------------------------------------------------------------------------
// Test 9: yaml-loader maps block_tools manifest field → AgentDef.blockTools
// ---------------------------------------------------------------------------

describe("Test 9: yaml-loader maps block_tools → AgentDef.blockTools", () => {
  it("manifest with block_tools produces AgentDef with matching blockTools array", async () => {
    writeFixture(
      "agents/reader.yaml",
      [
        "name: reader-agent",
        "description: A read-only agent.",
        "system:",
        "  text: You are a read-only analyst.",
        "block_tools:",
        "  - office_xlsx_write",
        "  - office_pptx_write",
        "  - initiate_handoff",
      ].join("\n"),
    );

    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader(),
    });

    const def = await loader.loadAgent("reader");
    expect(def.blockTools).toBeDefined();
    expect(def.blockTools).toHaveLength(3);
    expect(def.blockTools).toContain("office_xlsx_write");
    expect(def.blockTools).toContain("office_pptx_write");
    expect(def.blockTools).toContain("initiate_handoff");
  });

  it("manifest without block_tools produces AgentDef with blockTools undefined", async () => {
    writeFixture(
      "agents/unrestricted.yaml",
      [
        "name: unrestricted-agent",
        "system:",
        "  text: You are unrestricted.",
      ].join("\n"),
    );

    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader(),
    });

    const def = await loader.loadAgent("unrestricted");
    expect(def.blockTools).toBeUndefined();
  });
});

// ---------------------------------------------------------------------------
// Test 10: integration — yaml-loaded agent with block_tools cannot see the
//           blocked tool even when the orchestration would normally inject it
// ---------------------------------------------------------------------------

describe("Test 10: integration — yaml-loaded specialist with block_tools", () => {
  it("loaded specialist with block_tools:[initiate_handoff] never sees initiate_handoff", async () => {
    writeFixture(
      "agents/safe-specialist.yaml",
      [
        "name: safe-specialist",
        "description: Specialist blocked from initiating further handoffs.",
        "system:",
        "  text: You are a safe specialist.",
        "block_tools:",
        "  - initiate_handoff",
      ].join("\n"),
    );

    const loader = createDirectYamlManifestLoader({
      agentsRoot: join(tmpDir, "agents"),
      skillLoader: makeSkillLoader(),
    });

    const specialistDef = await loader.loadAgent("safe-specialist");
    expect(specialistDef.blockTools).toContain(HANDOFF_TOOL_NAME);

    // Simulate dispatching the specialist directly with a handoff orchestrator configured
    // (which would normally inject initiate_handoff). The blockTools must prevent this.
    const orchestrator = createHandoffOrchestrator({
      allowlist: [{ source: specialistDef.id, targets: ["some-agent"] }],
      resolveAgent: () => undefined,
      dispatchAgent: vi.fn().mockResolvedValue({ finalText: "ok" }),
    });

    const { provider, toolsSeen } = makeToolCapturingProvider();

    await dispatch({
      agent: specialistDef,
      prompt: "safe specialist run",
      provider,
      mcp: makeMCPClient([tinyTool]),
      handoff: orchestrator,
    });

    expect(toolsSeen.length).toBeGreaterThan(0);
    const firstTurnTools = toolsSeen[0]!;
    expect(firstTurnTools.some((t) => t.name === HANDOFF_TOOL_NAME)).toBe(false);
  });
});
