/**
 * Audit + session integration test — Phase 31 Wave 4.
 *
 * Verifies the agent loop's audit and session hooks fire end-to-end:
 *   1. dispatch() with an audit sink writes an AuditRecord on completion
 *   2. The record's prompt_hash and result_hash match sha256(prompt) /
 *      sha256(finalText) recomputed independently
 *   3. tool_calls[] entries have non-empty input_hash / result_hash
 *   4. dispatch() with a session store persists SessionState (in_progress
 *      then completed) — load returns the latest with messages array
 *      populated and status="completed"
 *   5. delegation produces a child AuditRecord with parent_audit_id linking
 *      to the chief's audit_id; verifyAuditChain returns ok=true on the
 *      complete chain
 *
 * Uses mocked Provider + MCPClient so the test runs offline (no API keys
 * needed). Validates wiring, not live model behavior.
 */

import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createHash } from "node:crypto";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { dispatch } from "../src/core/agent-loop.js";
import { createFileAuditSink, verifyAuditChain } from "../src/audit/index.js";
import { createFileSessionStore } from "../src/memory/index.js";
import type {
  AgentDef,
  CanonicalTool,
  MCPClient,
  Message,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../src/types.js";

function sha256(s: string): string {
  return createHash("sha256").update(s).digest("hex");
}

function makeMCPClient(tools: CanonicalTool[]): MCPClient {
  return {
    initialize: async () => {},
    listTools: async () => tools,
    callTool: async (name) => ({ content: [{ type: "text", text: `result-${name}` }] }),
    close: async () => {},
  };
}

/** Provider that emits a single tool call then ends the turn. */
function makeOneToolProvider(toolName: string): Provider {
  let turnCount = 0;
  return {
    name: "anthropic" as const,
    async turn(_req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      turnCount += 1;
      if (turnCount === 1) {
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
          content: [{ type: "text", text: `Computed using ${toolName}.` }],
        },
        stopReason: "end_turn",
        usage: { inputTokens: 12, outputTokens: 6 },
      };
    },
  };
}

const tinyTool: CanonicalTool = {
  name: "tiny_tool",
  description: "A tiny tool for tests.",
  input_schema: {
    type: "object",
    properties: { x: { type: "number" } },
    required: ["x"],
  },
};

const testAgent: AgentDef = {
  id: "test-agent",
  description: "Test agent.",
  systemPrompt: "You are a test agent.",
  tools: ["tiny_tool"],
  maxRecursionDepth: 0,
  model: "claude-test",
  maxTokens: 1024,
};

describe("audit + session integration — Wave 4", () => {
  let auditDir: string;
  let sessionDir: string;

  beforeEach(() => {
    auditDir = mkdtempSync(join(tmpdir(), "harness-audit-"));
    sessionDir = mkdtempSync(join(tmpdir(), "harness-session-"));
  });

  afterEach(() => {
    rmSync(auditDir, { recursive: true, force: true });
    rmSync(sessionDir, { recursive: true, force: true });
  });

  it("audit sink writes a record with correct prompt/result hashes after dispatch", async () => {
    const audit = createFileAuditSink({ dir: auditDir });
    const mcp = makeMCPClient([tinyTool]);
    const provider = makeOneToolProvider("tiny_tool");
    const prompt = "Run the tiny tool.";

    const result = await dispatch({
      agent: testAgent,
      prompt,
      provider,
      mcp,
      audit,
    });

    expect(result.auditId).toBeDefined();
    const record = await audit.read(result.auditId!);
    expect(record).not.toBeNull();
    expect(record?.agent_id).toBe("test-agent");
    expect(record?.prompt_hash).toBe(sha256(prompt));
    expect(record?.result_hash).toBe(sha256(result.finalText));
    expect(record?.tool_calls).toHaveLength(1);
    const tc = record?.tool_calls[0];
    expect(tc?.name).toBe("tiny_tool");
    expect(tc?.input_hash.length).toBe(64);
    expect(tc?.result_hash.length).toBe(64);
    expect(tc?.is_error).toBe(false);
    expect(record?.total_tool_uses).toBe(1);
    expect(record?.usage).toEqual({ inputTokens: 22, outputTokens: 11 });
  });

  it("session store persists state with messages + status=completed", async () => {
    const session = createFileSessionStore({ dir: sessionDir });
    const mcp = makeMCPClient([tinyTool]);
    const provider = makeOneToolProvider("tiny_tool");

    const result = await dispatch({
      agent: testAgent,
      prompt: "run tiny tool",
      provider,
      mcp,
      session,
    });

    expect(result.sessionId).toBeDefined();
    const loaded = await session.load(result.sessionId!);
    expect(loaded).not.toBeNull();
    expect(loaded?.agent_id).toBe("test-agent");
    expect(loaded?.status).toBe("completed");
    expect(loaded?.tool_uses).toBe(1);
    // Initial user prompt + assistant tool_use + user tool_result + assistant final = 4 messages
    expect(loaded?.messages.length).toBeGreaterThanOrEqual(4);
    expect(loaded?.final_text).toBe(result.finalText);
  });

  it("delegation produces a child AuditRecord linked via parent_audit_id; chain verifies ok", async () => {
    const audit = createFileAuditSink({ dir: auditDir });
    const tools: CanonicalTool[] = [tinyTool];
    const mcp = makeMCPClient(tools);

    const specialist: AgentDef = {
      id: "specialist",
      description: "Specialist that just calls tiny_tool.",
      systemPrompt: "You are a specialist.",
      tools: ["tiny_tool"],
      maxRecursionDepth: 0,
      model: "claude-test",
      maxTokens: 1024,
    };

    const chief: AgentDef = {
      id: "chief",
      description: "Chief that delegates to specialist.",
      systemPrompt: "You are a chief.",
      tools: ["tiny_tool"],
      maxRecursionDepth: 1,
      model: "claude-test",
      maxTokens: 1024,
    };

    // Provider that delegates on turn 1, then ends on turn 2
    let turn = 0;
    const provider: Provider = {
      name: "anthropic",
      async turn(_req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
        turn += 1;
        if (turn === 1) {
          return {
            message: {
              role: "assistant",
              content: [
                {
                  type: "tool_use",
                  id: "del-1",
                  name: "delegate_to_specialist",
                  input: { sub_prompt: "do tiny work" },
                },
              ],
            },
            stopReason: "tool_use",
          };
        }
        if (turn === 2) {
          // Specialist's first turn — invoke tiny_tool
          return {
            message: {
              role: "assistant",
              content: [
                { type: "tool_use", id: "call-x", name: "tiny_tool", input: { x: 2 } },
              ],
            },
            stopReason: "tool_use",
          };
        }
        if (turn === 3) {
          // Specialist's second turn — end
          return {
            message: {
              role: "assistant",
              content: [{ type: "text", text: "specialist done" }],
            },
            stopReason: "end_turn",
          };
        }
        // Chief's final turn after delegation result
        return {
          message: {
            role: "assistant",
            content: [{ type: "text", text: "chief done" }],
          },
          stopReason: "end_turn",
        };
      },
    };

    const result = await dispatch({
      agent: chief,
      prompt: "delegate then summarize",
      provider,
      mcp,
      audit,
      delegates: [specialist],
    });

    expect(result.childDispatches?.length).toBe(1);
    const childResult = result.childDispatches![0]!;
    expect(childResult.auditId).toBeDefined();

    const all = await audit.list();
    expect(all.length).toBe(2);

    const chiefRecord = await audit.read(result.auditId!);
    const childRecord = await audit.read(childResult.auditId!);
    expect(chiefRecord?.child_audit_ids).toContain(childRecord?.audit_id);
    expect(childRecord?.parent_audit_id).toBe(chiefRecord?.audit_id);

    const verification = verifyAuditChain(all);
    expect(verification.ok).toBe(true);
    expect(verification.issues).toHaveLength(0);
  });
});
