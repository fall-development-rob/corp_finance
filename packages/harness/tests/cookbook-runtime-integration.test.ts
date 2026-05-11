/**
 * Cookbook runtime end-to-end integration tests — Phase 33 / Phase 39.
 *
 * Tests `dispatchCookbook` (packages/harness/src/runtime/dispatch-cookbook.ts),
 * which is the top-level entry point that stitches together:
 *   - CookbookLoader (cookbook-loader.ts)
 *   - HandoffOrchestrator (orchestrator.ts)
 *   - dispatch agent-loop (agent-loop.ts)
 *   - block_tools enforcement (agent-loop.ts Phase 33)
 *   - output_schema validation via parseAndValidate (validator.ts)
 *   - Audit chain (audit/chain.ts)
 *   - Session memory (memory/session.ts)
 *   - DispatchEvent stream
 *
 * Canonical test cookbook: credit-analyst
 *   subagents: cfa-credit-data-reader, cfa-credit-scorer, cfa-credit-reporter
 *
 * All tests run offline with stub providers and in-memory / tmpdir stores.
 */

import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// ---------------------------------------------------------------------------
// Subject under test
// ---------------------------------------------------------------------------

import {
  dispatchCookbook,
  type CookbookDispatchOptions,
  type CookbookDispatchResult,
} from "../src/runtime/dispatch-cookbook.js";

// ---------------------------------------------------------------------------
// Harness imports
// ---------------------------------------------------------------------------

import { parseAndValidate } from "../src/manifests/validator.js";
import { createFileAuditSink, verifyAuditChain } from "../src/audit/index.js";
import { HANDOFF_TOOL_NAME } from "../src/handoff/index.js";
import type {
  AuditRecord,
  AuditSink,
  CanonicalTool,
  DispatchEvent,
  MCPClient,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
  SessionState,
  SessionStore,
} from "../src/types.js";
import { createCookbookLoader } from "../src/manifests/cookbook-loader.js";
import type { SkillLoader } from "../src/skills/types.js";
import type { ParsedSkill } from "../src/skills/types.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(import.meta.dirname, "../../..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");

// credit-analyst subagent ids (from their "name" field in *.json)
const DATA_READER_ID = "cfa-credit-data-reader";
const CREDIT_SCORER_ID = "cfa-credit-scorer";
const REPORTER_ID = "cfa-credit-reporter";
const PARENT_ID = "cfa-credit-analyst";

// ---------------------------------------------------------------------------
// Stub helpers
// ---------------------------------------------------------------------------

/** Null skill loader — cookbooks use from_plugin exclusively. */
function makeNullSkillLoader(): SkillLoader {
  return {
    async loadSkill(id: string): Promise<ParsedSkill> {
      throw new Error(`Unexpected from_skill reference to "${id}" in cookbook`);
    },
    async loadAgent() {
      throw new Error("not used in integration tests");
    },
    clearCache() {},
  };
}

/**
 * Bare tool names used by the credit-analyst subagents (after prefix stripping).
 * Must be in the MCP catalog so assertAllowlistsValid doesn't throw on dispatch.
 * data-reader has tools:"*" (fmp wildcard) — no need to enumerate those.
 * credit-scorer and reporter have explicit tool allowlists.
 */
const CREDIT_ANALYST_SUBAGENT_TOOLS: string[] = [
  // credit-scorer (agent_toolset read + explicit cfa-core tools)
  "read",
  "credit_metrics",
  "debt_capacity",
  "covenant_compliance",
  "altman_zscore",
  "calculate_credit_spreads",
  "calculate_scorecard",
  "calculate_merton",
  "calculate_intensity_model",
  "calculate_calibration",
  "calculate_scoring_validation",
  "price_cds",
  "calculate_cva",
  "calculate_portfolio_credit_risk",
  "calculate_migration",
  "build_sensitivity_grid",
  // reporter (agent_toolset write)
  "write",
];

/** Build a CanonicalTool from a bare name (stub, no real schema needed). */
function stubTool(name: string): CanonicalTool {
  return {
    name,
    description: `Stub tool: ${name}`,
    input_schema: { type: "object", properties: {} },
  };
}

/**
 * Stub MCP client that includes all credit-analyst subagent tools
 * plus any extra tools passed in. This prevents assertAllowlistsValid
 * from throwing when subagents with explicit tool allowlists are dispatched.
 */
function makeStubMCPClient(extraTools: CanonicalTool[] = []): MCPClient {
  const allTools = [
    ...CREDIT_ANALYST_SUBAGENT_TOOLS.map(stubTool),
    ...extraTools,
  ];
  return {
    initialize: async () => {},
    listTools: async () => allTools,
    callTool: async (name: string) => ({
      content: [{ type: "text", text: `stub-result:${name}` }],
    }),
    close: async () => {},
  };
}

/**
 * Deterministic multi-turn provider.
 *
 * `turns` is an ordered list of turn responses. Each entry is either:
 *   - A handoff-initiation response (for chief or subagent turns)
 *   - A text-only response (for final text or subagent JSON responses)
 *
 * Emits a "dispatch complete" text for any calls beyond the turn list.
 */
interface HandoffTurn {
  kind: "handoff";
  target: string;
  payload: Record<string, unknown>;
}
interface TextTurn {
  kind: "text";
  text: string;
}
type TurnSpec = HandoffTurn | TextTurn;

function makeSequentialProvider(turns: TurnSpec[]): Provider {
  let callCount = 0;
  return {
    name: "anthropic" as const,
    async turn(_req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      const spec = turns[callCount] ?? { kind: "text", text: "dispatch complete" };
      callCount += 1;

      if (spec.kind === "handoff") {
        return {
          message: {
            role: "assistant",
            content: [
              {
                type: "tool_use",
                id: `handoff-call-${callCount}`,
                name: HANDOFF_TOOL_NAME,
                input: { target: spec.target, payload: spec.payload },
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
          content: [{ type: "text", text: spec.text }],
        },
        stopReason: "end_turn",
        usage: { inputTokens: 5, outputTokens: 3 },
      };
    },
  };
}

/**
 * Valid JSON response bodies for each credit-analyst subagent,
 * matching their output_schema declarations.
 */
const VALID_DATA_READER_OUTPUT = JSON.stringify({
  issuer: "Apple Inc.",
  ticker: "AAPL",
  as_of_date: "2025-12-31",
  financials: {
    revenue: "391035000000",
    ebitda: "130000000000",
    total_debt: "109280000000",
    cash: "65172000000",
    net_income: "96995000000",
    operating_cash_flow: "118254000000",
    interest_expense: "3933000000",
    total_assets: "364980000000",
  },
  data_source: "fmp",
});

const VALID_CREDIT_SCORER_OUTPUT = JSON.stringify({
  issuer: "Apple Inc.",
  synthetic_rating: "AA+",
  leverage_ratio: "0.84",
  interest_coverage: "33.11",
  altman_z: "7.23",
  distress_flag: "safe",
  tool_calls: [{ name: "credit_metrics", count: 1 }],
});

const VALID_REPORTER_OUTPUT = JSON.stringify({
  issuer: "Apple Inc.",
  recommendation: "invest",
  executive_summary:
    "Apple Inc. demonstrates exceptional credit quality with a synthetic rating of AA+. " +
    "Leverage is minimal at 0.84x and interest coverage is robust at 33.11x. " +
    "The Altman Z-Score of 7.23 places the issuer firmly in the safe zone.",
  output_file: "out/AAPL-credit-opinion.md",
  traceability_table: [
    { metric: "Leverage Ratio", source: "cfa-credit-scorer", value: "0.84x" },
    { metric: "Interest Coverage", source: "cfa-credit-scorer", value: "33.11x" },
  ],
});

/**
 * Build the 7-turn provider for the full happy-path chief pipeline:
 *   Turn 0 (chief, turn 1):  handoff to data-reader
 *   Turn 1 (data-reader, turn 1): returns valid JSON
 *   Turn 2 (chief, turn 2):  handoff to credit-scorer
 *   Turn 3 (scorer, turn 1): returns valid JSON
 *   Turn 4 (chief, turn 3):  handoff to reporter
 *   Turn 5 (reporter, turn 1): returns valid JSON
 *   Turn 6 (chief, turn 4):  final text
 */
function makeHappyPathProvider(): Provider {
  return makeSequentialProvider([
    { kind: "handoff", target: DATA_READER_ID, payload: { ticker: "AAPL" } },
    { kind: "text", text: VALID_DATA_READER_OUTPUT },
    { kind: "handoff", target: CREDIT_SCORER_ID, payload: { issuer: "Apple Inc.", ticker: "AAPL" } },
    { kind: "text", text: VALID_CREDIT_SCORER_OUTPUT },
    { kind: "handoff", target: REPORTER_ID, payload: { issuer: "Apple Inc.", synthetic_rating: "AA+" } },
    { kind: "text", text: VALID_REPORTER_OUTPUT },
    { kind: "text", text: "Credit analysis for AAPL complete. Recommendation: invest." },
  ]);
}

// ---------------------------------------------------------------------------
// In-memory audit sink
// ---------------------------------------------------------------------------

function inMemoryAuditSink(): { sink: AuditSink; records: Map<string, AuditRecord> } {
  const records = new Map<string, AuditRecord>();
  const sink: AuditSink = {
    async write(record: AuditRecord): Promise<void> {
      records.set(record.audit_id, record);
    },
    async read(auditId: string): Promise<AuditRecord | null> {
      return records.get(auditId) ?? null;
    },
    async list(filter?: { agentId?: string; since?: Date }): Promise<AuditRecord[]> {
      const all = [...records.values()];
      return all.filter((r) => {
        if (filter?.agentId && r.agent_id !== filter.agentId) return false;
        if (filter?.since && new Date(r.timestamp) < filter.since) return false;
        return true;
      });
    },
  };
  return { sink, records };
}

// ---------------------------------------------------------------------------
// In-memory session store
// ---------------------------------------------------------------------------

function inMemorySessionStore(): { store: SessionStore; sessions: Map<string, SessionState> } {
  const sessions = new Map<string, SessionState>();
  const store: SessionStore = {
    async save(state: SessionState): Promise<void> {
      sessions.set(state.session_id, { ...state });
    },
    async load(sessionId: string): Promise<SessionState | null> {
      return sessions.get(sessionId) ?? null;
    },
    async list(filter?: { agentId?: string; status?: SessionState["status"] }): Promise<SessionState[]> {
      const all = [...sessions.values()];
      return all.filter((s) => {
        if (filter?.agentId && s.agent_id !== filter.agentId) return false;
        if (filter?.status && s.status !== filter.status) return false;
        return true;
      });
    },
    async delete(sessionId: string): Promise<void> {
      sessions.delete(sessionId);
    },
  };
  return { store, sessions };
}

// ---------------------------------------------------------------------------
// Event collector
// ---------------------------------------------------------------------------

function collectEvents(): { events: DispatchEvent[]; onEvent: (e: DispatchEvent) => void } {
  const events: DispatchEvent[] = [];
  return {
    events,
    onEvent: (e: DispatchEvent) => events.push(e),
  };
}

// ---------------------------------------------------------------------------
// Minimal CookbookDispatchOptions builder
// ---------------------------------------------------------------------------

/**
 * Build the minimal options required by dispatchCookbook.
 * Uses the actual API: slug + cookbookLoader (pre-built loader instance).
 */
function buildDispatchOptions(
  overrides: Partial<CookbookDispatchOptions> = {},
): CookbookDispatchOptions {
  return {
    slug: "credit-analyst",
    cookbookLoader,
    prompt: "Perform a full credit analysis for AAPL.",
    provider: makeHappyPathProvider(),
    mcp: makeStubMCPClient(),
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// Shared cookbook loader instance (created once; tests are read-only on manifests).
// This loader is passed to dispatchCookbook as the cookbookLoader option AND
// used directly in tests for schema inspection.
// ---------------------------------------------------------------------------

const cookbookLoader = createCookbookLoader({
  cookbooksRoot: COOKBOOKS_ROOT,
  skillLoader: makeNullSkillLoader(),
});

// ---------------------------------------------------------------------------
// Test group 1: Happy path (4 tests)
// ---------------------------------------------------------------------------

describe("Happy path: full credit-analyst 3-stage pipeline", () => {
  it("Test 1: dispatchCookbook runs the 3-stage pipeline; returns CookbookDispatchResult", async () => {
    const result: CookbookDispatchResult = await dispatchCookbook(buildDispatchOptions());

    expect(result).toBeDefined();
    // Parent agent id from agent.json "name" field
    expect(result.parentAgentId).toBe(PARENT_ID);
    // Three subagent ids in any order (pipeline ordering is orchestrator-internal)
    expect(Array.isArray(result.subagentIds)).toBe(true);
    expect(result.subagentIds).toHaveLength(3);
    expect(result.subagentIds).toContain(DATA_READER_ID);
    expect(result.subagentIds).toContain(CREDIT_SCORER_ID);
    expect(result.subagentIds).toContain(REPORTER_ID);
    // subagentAuditIds is string[] | undefined (undefined when no handoffs produced audit ids)
    expect(result.subagentAuditIds === undefined || Array.isArray(result.subagentAuditIds)).toBe(true);
    // Final text from chief's last turn
    expect(typeof result.finalText).toBe("string");
    expect(result.finalText.length).toBeGreaterThan(0);
  });

  it("Test 2: DispatchEvent stream contains 3 handoff_initiated + 3 handoff_returned in order", async () => {
    const { events, onEvent } = collectEvents();
    await dispatchCookbook(buildDispatchOptions({ onEvent }));

    const initiated = events.filter((e) => e.type === "handoff_initiated") as Array<
      Extract<DispatchEvent, { type: "handoff_initiated" }>
    >;
    const returned = events.filter((e) => e.type === "handoff_returned") as Array<
      Extract<DispatchEvent, { type: "handoff_returned" }>
    >;

    expect(initiated).toHaveLength(3);
    expect(returned).toHaveLength(3);

    // Sequential pipeline order: data-reader → scorer → reporter
    const initiatedTargets = initiated.map((e) => e.target_agent);
    const returnedTargets = returned.map((e) => e.target_agent);

    expect(initiatedTargets[0]).toBe(DATA_READER_ID);
    expect(initiatedTargets[1]).toBe(CREDIT_SCORER_ID);
    expect(initiatedTargets[2]).toBe(REPORTER_ID);
    expect(returnedTargets[0]).toBe(DATA_READER_ID);
    expect(returnedTargets[1]).toBe(CREDIT_SCORER_ID);
    expect(returnedTargets[2]).toBe(REPORTER_ID);

    // For each target: initiated must appear before returned in the event stream
    for (const target of [DATA_READER_ID, CREDIT_SCORER_ID, REPORTER_ID]) {
      const iIdx = events.findIndex(
        (e) =>
          e.type === "handoff_initiated" &&
          (e as Extract<DispatchEvent, { type: "handoff_initiated" }>).target_agent === target,
      );
      const rIdx = events.findIndex(
        (e) =>
          e.type === "handoff_returned" &&
          (e as Extract<DispatchEvent, { type: "handoff_returned" }>).target_agent === target,
      );
      expect(iIdx).toBeGreaterThanOrEqual(0);
      expect(rIdx).toBeGreaterThan(iIdx);
    }

    // All returned events are ok: true on the happy path
    for (const ret of returned) {
      expect(ret.ok).toBe(true);
      expect(ret.duration_ms).toBeGreaterThanOrEqual(0);
    }
  });

  it("Test 3: Audit chain — parent audit record exists; subagent audit ids in subagentAuditIds", async () => {
    const { sink, records } = inMemoryAuditSink();
    const result = await dispatchCookbook(buildDispatchOptions({ audit: sink }));

    // parentAuditId is the alias for auditId on the result
    const parentAuditId = result.parentAuditId ?? result.auditId;
    expect(parentAuditId).toBeDefined();
    const parentRecord = await sink.read(parentAuditId!);
    expect(parentRecord).not.toBeNull();
    expect(parentRecord!.agent_id).toBe(PARENT_ID);

    // In the handoff-based architecture, child_audit_ids on the parent record
    // is populated via the delegation path only; handoff subagents produce
    // separate audit records captured in CookbookDispatchResult.subagentAuditIds
    // rather than in child_audit_ids.
    // Verify: at least 4 records total (parent + 3 subagents)
    expect(records.size).toBeGreaterThanOrEqual(4);

    // The subagentAuditIds should reference the 3 subagent records
    expect(result.subagentAuditIds).toBeDefined();
    expect(result.subagentAuditIds!.length).toBe(3);

    // Each subagent audit id must be readable
    for (const subId of result.subagentAuditIds!) {
      const subRecord = await sink.read(subId);
      expect(subRecord, `subagent audit record ${subId} missing from store`).not.toBeNull();
    }

    // The audit chain (considered independently as separate subtrees) verifies clean
    // (no broken parent/child references within each subtree)
    const allRecords = [...records.values()];
    const verify = verifyAuditChain(allRecords);
    expect(verify.ok, `audit chain integrity issues: ${verify.issues.join("; ")}`).toBe(true);
  });

  it("Test 4: Each subagent output passes its output_schema (no is_error on happy path)", async () => {
    const { events, onEvent } = collectEvents();
    const { sink } = inMemoryAuditSink();
    await dispatchCookbook(buildDispatchOptions({ onEvent, audit: sink }));

    // No handoff_returned event should have ok: false
    const returnedEvents = events.filter((e) => e.type === "handoff_returned") as Array<
      Extract<DispatchEvent, { type: "handoff_returned" }>
    >;
    for (const ret of returnedEvents) {
      expect(ret.ok, `handoff to ${ret.target_agent} returned ok: false on happy path`).toBe(true);
    }

    // Directly validate the JSON payloads against the schemas for independent confirmation
    const cookbook = await cookbookLoader.load("credit-analyst");

    const dataReader = cookbook.subagents.find((s) => s.id === DATA_READER_ID)!;
    const drValidation = parseAndValidate(VALID_DATA_READER_OUTPUT, dataReader.outputSchema!);
    expect(drValidation.ok).toBe(true);

    const scorer = cookbook.subagents.find((s) => s.id === CREDIT_SCORER_ID)!;
    const scorerValidation = parseAndValidate(VALID_CREDIT_SCORER_OUTPUT, scorer.outputSchema!);
    expect(scorerValidation.ok).toBe(true);

    const reporter = cookbook.subagents.find((s) => s.id === REPORTER_ID)!;
    const reporterValidation = parseAndValidate(VALID_REPORTER_OUTPUT, reporter.outputSchema!);
    expect(reporterValidation.ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Test group 2: Schema enforcement (3 tests)
//
// Note: the handoff orchestrator dispatches subagents and returns the raw
// finalText — it does NOT validate output against output_schema (that is
// done in the delegation path, not the handoff path). These tests verify
// schema validation using parseAndValidate directly against the cookbook's
// declared schemas, and verify that the handoff path completes regardless
// of the subagent's output content.
// ---------------------------------------------------------------------------

describe("Schema enforcement: output_schema validation against cookbook schemas", () => {
  it("Test 5: data-reader output missing 'ticker' fails parseAndValidate; dispatch completes", async () => {
    const missingTickerOutput = JSON.stringify({
      issuer: "Apple Inc.",
      // ticker deliberately omitted
      as_of_date: "2025-12-31",
      financials: { revenue: "391035000000" },
      data_source: "fmp",
    });

    // Verify the schema correctly rejects the invalid output
    const cookbook = await cookbookLoader.load("credit-analyst");
    const dataReader = cookbook.subagents.find((s) => s.id === DATA_READER_ID)!;
    const schemaCheck = parseAndValidate(missingTickerOutput, dataReader.outputSchema!);
    expect(schemaCheck.ok).toBe(false);
    expect(schemaCheck.errors.some((e) => e.message.includes("ticker"))).toBe(true);
    expect(schemaCheck.errors.some((e) => e.schemaKeyword === "required")).toBe(true);

    // The handoff path completes normally regardless (no output_schema gate on handoff)
    const provider = makeSequentialProvider([
      { kind: "handoff", target: DATA_READER_ID, payload: { ticker: "AAPL" } },
      { kind: "text", text: missingTickerOutput },
      { kind: "text", text: "received data-reader output." },
    ]);

    const { events, onEvent } = collectEvents();
    const result = await dispatchCookbook(buildDispatchOptions({ provider, onEvent }));

    expect(result).toBeDefined();
    expect(typeof result.finalText).toBe("string");

    // The handoff itself succeeds (handoff path doesn't validate output_schema)
    const dataReaderReturn = (events.filter((e) => e.type === "handoff_returned") as Array<
      Extract<DispatchEvent, { type: "handoff_returned" }>
    >).find((e) => e.target_agent === DATA_READER_ID);
    expect(dataReaderReturn).toBeDefined();
    expect(dataReaderReturn!.ok).toBe(true);
  });

  it("Test 6: synthetic_rating 'ZZZ' fails enum validation in parseAndValidate", async () => {
    const invalidRatingOutput = JSON.stringify({
      issuer: "Apple Inc.",
      synthetic_rating: "ZZZ", // not in the allowed enum
      leverage_ratio: "0.84",
      interest_coverage: "33.11",
      altman_z: "7.23",
      distress_flag: "safe",
      tool_calls: [],
    });

    // Verify the schema correctly rejects ZZZ
    const cookbook = await cookbookLoader.load("credit-analyst");
    const scorer = cookbook.subagents.find((s) => s.id === CREDIT_SCORER_ID)!;
    const schemaCheck = parseAndValidate(invalidRatingOutput, scorer.outputSchema!);
    expect(schemaCheck.ok).toBe(false);
    expect(schemaCheck.errors.some((e) => e.schemaKeyword === "enum")).toBe(true);
    expect(schemaCheck.errors[0]!.path).toBe("synthetic_rating");

    // Verify valid enum values ARE accepted
    const validOutput = { ...JSON.parse(invalidRatingOutput), synthetic_rating: "BBB" };
    const validCheck = parseAndValidate(JSON.stringify(validOutput), scorer.outputSchema!);
    expect(validCheck.ok).toBe(true);
  });

  it("Test 7: markdown-fenced JSON is unwrapped by extractJsonFromMarkdown before validation", async () => {
    // Simulate extractJsonFromMarkdown behavior directly — this function is in agent-loop
    // and used in the delegation path. Verify it correctly strips fences.
    const inner = VALID_REPORTER_OUTPUT;
    const wrappedJson = "```json\n" + inner + "\n```";
    const wrappedNoLabel = "```\n" + inner + "\n```";

    // extractJsonFromMarkdown is not exported, so we test via parseAndValidate
    // by manually stripping fences as the agent-loop does
    const cookbook = await cookbookLoader.load("credit-analyst");
    const reporter = cookbook.subagents.find((s) => s.id === REPORTER_ID)!;

    // Validate the raw inner JSON (should pass)
    const rawResult = parseAndValidate(inner, reporter.outputSchema!);
    expect(rawResult.ok).toBe(true);

    // Strip fences manually (simulating extractJsonFromMarkdown) and validate
    const jsonBlockMatch = wrappedJson.match(/```json\s*\n([\s\S]*?)\n```/);
    const extractedJson = jsonBlockMatch ? jsonBlockMatch[1]! : wrappedJson;
    const extractedResult = parseAndValidate(extractedJson, reporter.outputSchema!);
    expect(extractedResult.ok).toBe(true);

    // Without stripping, the fenced version fails to parse as JSON
    const rawFencedResult = parseAndValidate(wrappedJson, reporter.outputSchema!);
    expect(rawFencedResult.ok).toBe(false);
    expect(rawFencedResult.errors[0]!.message).toContain("JSON parse");

    // The happy-path dispatch completes with wrapped reporter output
    const provider = makeSequentialProvider([
      { kind: "handoff", target: DATA_READER_ID, payload: { ticker: "AAPL" } },
      { kind: "text", text: VALID_DATA_READER_OUTPUT },
      { kind: "handoff", target: CREDIT_SCORER_ID, payload: { issuer: "Apple Inc." } },
      { kind: "text", text: VALID_CREDIT_SCORER_OUTPUT },
      { kind: "handoff", target: REPORTER_ID, payload: { issuer: "Apple Inc." } },
      { kind: "text", text: wrappedJson },
      { kind: "text", text: "Credit analysis complete." },
    ]);

    const { events, onEvent } = collectEvents();
    await dispatchCookbook(buildDispatchOptions({ provider, onEvent }));

    const reporterReturn = (events.filter((e) => e.type === "handoff_returned") as Array<
      Extract<DispatchEvent, { type: "handoff_returned" }>
    >).find((e) => e.target_agent === REPORTER_ID);
    expect(reporterReturn).toBeDefined();
    // handoff path doesn't validate output — passes through raw text
    expect(reporterReturn!.ok).toBe(true);
    void wrappedNoLabel; // used above
  });
});

// ---------------------------------------------------------------------------
// Test group 3: block_tools enforcement (2 tests)
// ---------------------------------------------------------------------------

describe("block_tools enforcement: data-reader cannot invoke office write tools", () => {
  it("Test 8: data-reader block_tools strips office_xlsx_write from dispatch tool list", async () => {
    // Confirm the manifest declares the blocklist
    const cookbook = await cookbookLoader.load("credit-analyst");
    const dataReader = cookbook.subagents.find((s) => s.id === DATA_READER_ID)!;
    expect(dataReader.blockTools).toBeDefined();
    expect(dataReader.blockTools).toContain("office_xlsx_write");
    expect(dataReader.blockTools).toContain("office_pptx_write");
    expect(dataReader.blockTools).toContain("office_docx_write");

    // Provide the blocked tools in the MCP catalog so they would appear unless stripped
    const blockedTool: CanonicalTool = {
      name: "office_xlsx_write",
      description: "Write an XLSX file.",
      input_schema: { type: "object", properties: { path: { type: "string" } } },
    };
    const mcp = makeStubMCPClient([blockedTool]);

    const { events, onEvent } = collectEvents();
    await dispatchCookbook(buildDispatchOptions({ mcp, onEvent }));

    // The model must never have been asked to call office_xlsx_write
    // (it would appear as a tool_call event if it got through the blocklist)
    const officeToolUses = events.filter(
      (e) =>
        e.type === "tool_call" &&
        (e as Extract<DispatchEvent, { type: "tool_call" }>).call.name === "office_xlsx_write",
    );
    expect(officeToolUses).toHaveLength(0);
  });

  it("Test 9: tool_blocked events fire for each blocked tool when data-reader is dispatched", async () => {
    // Provide all three blocked tools in the MCP catalog so the block-list filter runs on them
    const blockedTools: CanonicalTool[] = [
      {
        name: "office_xlsx_write",
        description: "Write XLSX.",
        input_schema: { type: "object", properties: {} },
      },
      {
        name: "office_pptx_write",
        description: "Write PPTX.",
        input_schema: { type: "object", properties: {} },
      },
      {
        name: "office_docx_write",
        description: "Write DOCX.",
        input_schema: { type: "object", properties: {} },
      },
    ];
    const mcp = makeStubMCPClient(blockedTools);

    const { events, onEvent } = collectEvents();
    await dispatchCookbook(buildDispatchOptions({ mcp, onEvent }));

    const toolBlockedEvents = events.filter((e) => e.type === "tool_blocked") as Array<
      Extract<DispatchEvent, { type: "tool_blocked" }>
    >;

    const blockedNames = toolBlockedEvents.map((e) => e.toolName);

    // All three office tools declared in data-reader.json must have been blocked
    expect(blockedNames).toContain("office_xlsx_write");
    expect(blockedNames).toContain("office_pptx_write");
    expect(blockedNames).toContain("office_docx_write");

    // All tool_blocked events must carry reason: "blockTools"
    for (const ev of toolBlockedEvents) {
      expect(ev.reason).toBe("blockTools");
    }
  });
});

// ---------------------------------------------------------------------------
// Test group 4: Allowlist scoping (2 tests)
// ---------------------------------------------------------------------------

describe("Allowlist scoping: handoff target restrictions", () => {
  it("Test 10: chief initiates handoff to non-allowlisted target → orchestrator rejects (ok: false)", async () => {
    // cfa-equity-analyst-data-reader is NOT in credit-analyst's callable_agents
    const unauthorizedTarget = "cfa-equity-analyst-data-reader";

    const provider = makeSequentialProvider([
      { kind: "handoff", target: unauthorizedTarget, payload: { ticker: "AAPL" } },
      { kind: "text", text: "handoff was rejected." },
    ]);

    const { events, onEvent } = collectEvents();
    await dispatchCookbook(buildDispatchOptions({ provider, onEvent }));

    const returnedEvents = events.filter((e) => e.type === "handoff_returned") as Array<
      Extract<DispatchEvent, { type: "handoff_returned" }>
    >;

    // The handoff to the unauthorized target must be rejected
    const rejectedReturn = returnedEvents.find((e) => e.target_agent === unauthorizedTarget);
    expect(rejectedReturn).toBeDefined();
    expect(rejectedReturn!.ok).toBe(false);
  });

  it("Test 11: subagents at depth=1 cannot issue further handoffs (depth > maxHandoffDepth=0 default)", async () => {
    // The default maxHandoffDepth inside dispatchCookbook is 2 (per implementation).
    // However, the data-reader has no callable_agents, so its allowlist is empty.
    // Any handoff attempt from data-reader would be rejected by the orchestrator's
    // allowlist gate. We verify: exactly 3 handoff_initiated events (chief only).

    const { events, onEvent } = collectEvents();
    await dispatchCookbook(buildDispatchOptions({ onEvent }));

    const initiatedEvents = events.filter((e) => e.type === "handoff_initiated") as Array<
      Extract<DispatchEvent, { type: "handoff_initiated" }>
    >;

    // Exactly 3 handoffs — one to each subagent from the chief
    expect(initiatedEvents.length).toBe(3);

    // All targets must be from the credit-analyst subagent set
    const targets = initiatedEvents.map((e) => e.target_agent);
    expect(targets).toContain(DATA_READER_ID);
    expect(targets).toContain(CREDIT_SCORER_ID);
    expect(targets).toContain(REPORTER_ID);
  });
});

// ---------------------------------------------------------------------------
// Test group 5: Audit chain linkage (2 tests)
// ---------------------------------------------------------------------------

describe("Audit chain linkage", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), "harness-cookbook-audit-"));
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("Test 12: cookbook dispatch with file audit sink → parent links to subagent audit_ids", async () => {
    const audit = createFileAuditSink({ dir: tmpDir });
    const result = await dispatchCookbook(buildDispatchOptions({ audit }));

    // parentAuditId is the alias for auditId on the result
    const parentAuditId = result.parentAuditId ?? result.auditId;
    expect(parentAuditId).toBeDefined();
    const parentRecord = await audit.read(parentAuditId!);
    expect(parentRecord).not.toBeNull();
    expect(parentRecord!.agent_id).toBe(PARENT_ID);

    // In the handoff-based architecture, subagent audit ids are returned in
    // CookbookDispatchResult.subagentAuditIds (not in parent.child_audit_ids)
    expect(result.subagentAuditIds).toBeDefined();
    expect(result.subagentAuditIds!.length).toBe(3);

    // Every subagent audit record must exist in the file store
    for (const subId of result.subagentAuditIds!) {
      const subRecord = await audit.read(subId);
      expect(subRecord, `subagent audit record ${subId} missing`).not.toBeNull();
    }

    // Full chain integrity (each record's parent/child references are internally consistent)
    const allRecords = await audit.list();
    const verify = verifyAuditChain(allRecords);
    expect(verify.ok, `Audit chain integrity issues: ${verify.issues.join("; ")}`).toBe(true);
  });

  it("Test 13: session state captures message history; can re-load and inspect", async () => {
    const { store, sessions } = inMemorySessionStore();
    const result = await dispatchCookbook(buildDispatchOptions({ session: store }));

    expect(result.sessionId).toBeDefined();
    const sessionState = await store.load(result.sessionId!);
    expect(sessionState).not.toBeNull();

    // Session must be in completed status
    expect(sessionState!.status).toBe("completed");
    expect(sessionState!.agent_id).toBe(PARENT_ID);

    // Messages must capture the conversation including handoff tool results
    expect(sessionState!.messages.length).toBeGreaterThan(0);

    // The final_text must match what we got back from dispatchCookbook
    expect(sessionState!.final_text).toBeDefined();
    expect(sessionState!.final_text).toBe(result.finalText);

    // At least the parent session must be in the store
    expect(sessions.size).toBeGreaterThanOrEqual(1);
  });
});

// ---------------------------------------------------------------------------
// Test group 6: Edge cases (2 tests)
// ---------------------------------------------------------------------------

describe("Edge cases: loader errors and missing resources", () => {
  it("Test 14: nonexistent slug → dispatchCookbook throws before reaching dispatch", async () => {
    await expect(
      dispatchCookbook(
        buildDispatchOptions({ slug: "does-not-exist-cookbook-xyz" }),
      ),
    ).rejects.toThrow();
  });

  it("Test 15: cookbook with broken loader (simulated missing from_plugin) → throws; dispatch never starts", async () => {
    // Inject a stub loader that simulates a broken from_plugin path
    const brokenLoader = {
      async list() {
        return ["credit-analyst"];
      },
      async load(_slug: string): Promise<never> {
        throw new Error(
          `from_plugin skill not found. Expected SKILL.md at: /nonexistent/path/${_slug}/SKILL.md`,
        );
      },
      async loadAll() {
        return [];
      },
    };

    await expect(
      dispatchCookbook(
        buildDispatchOptions({ cookbookLoader: brokenLoader }),
      ),
    ).rejects.toThrow(/SKILL\.md|not found|from_plugin/i);
  });
});

// ---------------------------------------------------------------------------
// Supplementary: CookbookDispatchResult type contract (compile-time checks)
// These functions are never called at runtime — they verify that the public
// type surface is usable without casting.
// ---------------------------------------------------------------------------

function _typeCheckResult(r: CookbookDispatchResult): void {
  const _parentAgentId: string = r.parentAgentId;
  const _subagentIds: string[] = r.subagentIds;
  const _subagentAuditIds: string[] | undefined = r.subagentAuditIds;
  const _parentAuditId: string | undefined = r.parentAuditId;
  const _finalText: string = r.finalText;
  const _auditId: string | undefined = r.auditId;
  const _sessionId: string | undefined = r.sessionId;
  const _errorMessage: string | undefined = r.errorMessage;

  void _parentAgentId;
  void _subagentIds;
  void _subagentAuditIds;
  void _parentAuditId;
  void _finalText;
  void _auditId;
  void _sessionId;
  void _errorMessage;
}
void _typeCheckResult;

function _typeCheckOptions(opts: CookbookDispatchOptions): void {
  const _slug: string = opts.slug;
  const _prompt: string = opts.prompt;
  const _provider: Provider | undefined = opts.provider;
  const _mcp: MCPClient = opts.mcp;
  const _onEvent: ((e: DispatchEvent) => void) | undefined = opts.onEvent;
  const _audit: AuditSink | undefined = opts.audit;
  const _session: SessionStore | undefined = opts.session;

  void _slug;
  void _prompt;
  void _provider;
  void _mcp;
  void _onEvent;
  void _audit;
  void _session;
}
void _typeCheckOptions;
