/**
 * Phase 35 — Hybrid dispatch integration tests.
 *
 * Tests cover:
 *   1. workflowMode="auto" + high-confidence match → bypasses LLM (provider.turn never called)
 *   2. workflowMode="advisory" → LLM runs, gets list_workflows + run_workflow in tools
 *   3. workflowMode="disabled" → router.match never called, LLM runs normally
 *   4. workflow_failed event + LLM fallback when router.run() throws
 *   5. AuditRecord.path="workflow" + workflow_slug + workflow_audit_hash when workflow path taken
 *   6. AuditRecord.path is absent ("llm" semantics) when no workflow configured
 */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dispatch } from "../src/core/agent-loop.js";
import { createFileAuditSink } from "../src/audit/index.js";
import {
  createMockWorkflowRouter,
  WorkflowRouterError,
  type WorkflowMatch,
  type WorkflowResult,
} from "../src/workflow/index.js";
import type {
  AgentDef,
  AuditSink,
  CanonicalTool,
  DispatchEvent,
  MCPClient,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../src/types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeMCPClient(tools: CanonicalTool[] = []): MCPClient {
  return {
    initialize: async () => {},
    listTools: async () => tools,
    callTool: async () => ({ content: [{ type: "text", text: "ok" }] }),
    close: async () => {},
  };
}

// Provider that tracks invocations via the outer closure.
// Returns a mutable ref object so callers read .calls/.toolsSeen after dispatch.
interface ProviderSpy {
  provider: Provider;
  calls: number;
  toolsSeen: CanonicalTool[][];
}

function makeSpy(): ProviderSpy {
  const state: ProviderSpy = {
    calls: 0,
    toolsSeen: [],
    provider: null as unknown as Provider,
  };
  state.provider = {
    name: "anthropic" as const,
    async turn(req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      state.calls += 1;
      state.toolsSeen.push(req.tools);
      return {
        message: {
          role: "assistant",
          content: [{ type: "text", text: "LLM fallback answer" }],
        },
        stopReason: "end_turn",
        usage: { inputTokens: 5, outputTokens: 3 },
      };
    },
  };
  return state;
}

const tinyTool: CanonicalTool = {
  name: "tiny_tool",
  description: "Tiny test tool.",
  input_schema: { type: "object", properties: {}, required: [] },
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

const HIGH_CONFIDENCE_MATCH: WorkflowMatch = {
  slug: "fa-dcf-model",
  confidence: 0.95,
  extracted_params: { ticker: "AAPL", wacc: 0.09 },
};

const LOW_CONFIDENCE_MATCH: WorkflowMatch = {
  slug: "fa-dcf-model",
  confidence: 0.60,
  extracted_params: {},
};

const SAMPLE_RESULT: WorkflowResult = {
  slug: "fa-dcf-model",
  audit_hash: "workflow-audit-hash-abc123",
  deliverable: "## DCF Analysis\n\nFair value: $200/share",
  tool_calls: [
    { name: "dcf_model", input_hash: "ih1", result_hash: "rh1", duration_ms: 100 },
  ],
  duration_ms: 800,
};

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------

describe("hybrid dispatch integration — Phase 35", () => {
  let tmpDir: string;
  let audit: AuditSink;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "wf-dispatch-test-"));
    audit = createFileAuditSink({ dir: tmpDir });
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  // -------------------------------------------------------------------------
  // Test 1: workflowMode="auto" + high-confidence match → bypasses LLM
  // -------------------------------------------------------------------------

  it("Test 1: workflowMode=auto + high confidence match → bypasses LLM entirely", async () => {
    const spy = makeSpy();
    const router = createMockWorkflowRouter({
      matchResult: HIGH_CONFIDENCE_MATCH,
      runResult: SAMPLE_RESULT,
    });

    const events: DispatchEvent[] = [];

    const result = await dispatch({
      agent: testAgent,
      prompt: "Build a DCF for AAPL",
      provider: spy.provider,
      mcp: makeMCPClient([tinyTool]),
      workflow: router,
      workflowMode: "auto",
      onEvent: (e) => events.push(e),
    });

    // LLM was never called
    expect(spy.calls).toBe(0);

    // Returned workflow deliverable
    expect(result.finalText).toBe(SAMPLE_RESULT.deliverable);
    expect(result.toolUses).toBe(1);
    expect(result.messages).toHaveLength(0);
    expect(result.notes).toContain("fa-dcf-model");

    // Events emitted in order
    const matchedEvt = events.find((e) => e.type === "workflow_matched");
    const executedEvt = events.find((e) => e.type === "workflow_executed");
    expect(matchedEvt).toBeDefined();
    expect(executedEvt).toBeDefined();
    if (matchedEvt?.type === "workflow_matched") {
      expect(matchedEvt.slug).toBe("fa-dcf-model");
      expect(matchedEvt.confidence).toBe(0.95);
    }
    if (executedEvt?.type === "workflow_executed") {
      expect(executedEvt.slug).toBe("fa-dcf-model");
      expect(executedEvt.audit_hash).toBe(SAMPLE_RESULT.audit_hash);
    }
    const matchedIdx = events.findIndex((e) => e.type === "workflow_matched");
    const executedIdx = events.findIndex((e) => e.type === "workflow_executed");
    expect(matchedIdx).toBeLessThan(executedIdx);
  });

  // -------------------------------------------------------------------------
  // Test 2: workflowMode="advisory" → LLM runs with workflow tools injected
  // -------------------------------------------------------------------------

  it("Test 2: workflowMode=advisory → LLM runs and receives list_workflows + run_workflow tools", async () => {
    const spy = makeSpy();
    const router = createMockWorkflowRouter({
      matchResult: HIGH_CONFIDENCE_MATCH,
      runResult: SAMPLE_RESULT,
    });

    const result = await dispatch({
      agent: testAgent,
      prompt: "Build a DCF for AAPL",
      provider: spy.provider,
      mcp: makeMCPClient([tinyTool]),
      workflow: router,
      workflowMode: "advisory",
    });

    // LLM WAS called
    expect(spy.calls).toBeGreaterThan(0);
    expect(result.finalText).toBe("LLM fallback answer");

    // Workflow tools were injected at depth=0
    const toolsOnFirstTurn = spy.toolsSeen[0]!;
    const toolNames = toolsOnFirstTurn.map((t) => t.name);
    expect(toolNames).toContain("list_workflows");
    expect(toolNames).toContain("run_workflow");
  });

  // -------------------------------------------------------------------------
  // Test 3: workflowMode="disabled" → router.match never called
  // -------------------------------------------------------------------------

  it("Test 3: workflowMode=disabled → router never called, LLM runs normally", async () => {
    const spy = makeSpy();
    const matchFn = vi.fn(async () => HIGH_CONFIDENCE_MATCH);
    const router = {
      list: createMockWorkflowRouter({ matchResult: HIGH_CONFIDENCE_MATCH }).list,
      match: matchFn,
      run: createMockWorkflowRouter({ runResult: SAMPLE_RESULT }).run,
    };

    const result = await dispatch({
      agent: testAgent,
      prompt: "Build a DCF for AAPL",
      provider: spy.provider,
      mcp: makeMCPClient([tinyTool]),
      workflow: router,
      workflowMode: "disabled",
    });

    // router.match was never called
    expect(matchFn).not.toHaveBeenCalled();
    // LLM was called
    expect(spy.calls).toBeGreaterThan(0);
    expect(result.finalText).toBe("LLM fallback answer");

    // Workflow tools should NOT be injected when disabled
    const toolsOnFirstTurn = spy.toolsSeen[0]!;
    const toolNames = toolsOnFirstTurn.map((t) => t.name);
    expect(toolNames).not.toContain("list_workflows");
    expect(toolNames).not.toContain("run_workflow");
  });

  // -------------------------------------------------------------------------
  // Test 4: workflow_failed → LLM fallback (no rethrow)
  // -------------------------------------------------------------------------

  it("Test 4: workflow run() throws → workflow_failed emitted, LLM fallback taken", async () => {
    const spy = makeSpy();
    const throwingRouter = {
      list: async () => ({ total: 0, workflows: [] }),
      match: async () => HIGH_CONFIDENCE_MATCH,
      run: async () => {
        throw new WorkflowRouterError("binary not found", "CLI_ERROR");
      },
    };

    const events: DispatchEvent[] = [];

    const result = await dispatch({
      agent: testAgent,
      prompt: "Build a DCF for AAPL",
      provider: spy.provider,
      mcp: makeMCPClient([tinyTool]),
      workflow: throwingRouter,
      workflowMode: "auto",
      onEvent: (e) => events.push(e),
    });

    // LLM fallback was taken
    expect(spy.calls).toBeGreaterThan(0);
    expect(result.finalText).toBe("LLM fallback answer");

    // workflow_failed event was emitted
    const failedEvt = events.find((e) => e.type === "workflow_failed");
    expect(failedEvt).toBeDefined();
    if (failedEvt?.type === "workflow_failed") {
      expect(failedEvt.slug).toBe("fa-dcf-model");
      expect(failedEvt.reason).toContain("binary not found");
    }
  });

  // -------------------------------------------------------------------------
  // Test 5: AuditRecord contains path="workflow" with slug + audit_hash
  // -------------------------------------------------------------------------

  it("Test 5: AuditRecord.path=workflow + slug + audit_hash when workflow path taken", async () => {
    const spy = makeSpy();
    const router = createMockWorkflowRouter({
      matchResult: HIGH_CONFIDENCE_MATCH,
      runResult: SAMPLE_RESULT,
    });

    const result = await dispatch({
      agent: testAgent,
      prompt: "Build a DCF for AAPL",
      provider: spy.provider,
      mcp: makeMCPClient([tinyTool]),
      workflow: router,
      workflowMode: "auto",
      audit,
    });

    expect(result.auditId).toBeDefined();
    const record = await audit.read(result.auditId!);
    expect(record).not.toBeNull();
    expect(record?.path).toBe("workflow");
    expect(record?.workflow_slug).toBe("fa-dcf-model");
    expect(record?.workflow_audit_hash).toBe(SAMPLE_RESULT.audit_hash);
    expect(record?.total_tool_uses).toBe(1);
  });

  // -------------------------------------------------------------------------
  // Test 6: AuditRecord.path is absent when no workflow configured
  // -------------------------------------------------------------------------

  it("Test 6: AuditRecord.path is absent when no workflow configured (LLM path)", async () => {
    const spy = makeSpy();

    const result = await dispatch({
      agent: testAgent,
      prompt: "Any prompt",
      provider: spy.provider,
      mcp: makeMCPClient([tinyTool]),
      // no workflow option
      audit,
    });

    expect(result.auditId).toBeDefined();
    const record = await audit.read(result.auditId!);
    expect(record).not.toBeNull();
    expect(record?.path).toBeUndefined();
    expect(record?.workflow_slug).toBeUndefined();
    expect(record?.workflow_audit_hash).toBeUndefined();
  });
});
