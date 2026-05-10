/**
 * Tests for the cross-agent handoff event loop — REC-4 Wave 1.
 *
 * Covers: isAllowed, validatePayload, execute (happy + all rejection gates),
 * event ordering, request_id uniqueness, and timestamp format.
 *
 * Does NOT spin up real model calls; uses fake registry + stub dispatchAgent.
 */

import { describe, expect, it, vi } from "vitest";
import {
  createHandoffOrchestrator,
  type AgentDef,
  type HandoffAllowlistEntry,
  type HandoffEvent,
  type HandoffOrchestratorOptions,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

const makeAgent = (id: string): AgentDef => ({
  id,
  description: `Stub agent ${id}`,
  systemPrompt: `You are ${id}.`,
  tools: [],
});

const RESEARCHER = makeAgent("market-researcher");
const REVIEWER = makeAgent("earnings-reviewer");
const BUILDER = makeAgent("model-builder");

/** Registry backed by a simple Map. */
function makeRegistry(agents: AgentDef[]): Map<string, AgentDef> {
  return new Map(agents.map((a) => [a.id, a]));
}

/** Default stub dispatchAgent — returns canned response. */
function makeDispatch(
  response: { finalText: string; auditId?: string } = {
    finalText: "done",
    auditId: "audit-001",
  },
): HandoffOrchestratorOptions["dispatchAgent"] {
  return vi.fn().mockResolvedValue(response);
}

/** Collect emitted events into an array. */
function makeEventCollector(): {
  events: HandoffEvent[];
  onEvent: (e: HandoffEvent) => void;
} {
  const events: HandoffEvent[] = [];
  return { events, onEvent: (e) => events.push(e) };
}

const baseAllowlist: HandoffAllowlistEntry[] = [
  {
    source: "market-researcher",
    targets: ["earnings-reviewer", "model-builder"],
    payload_schema: {
      type: "object",
      required: ["ticker", "period"],
      properties: {
        ticker: { type: "string", maxLength: 10 },
        period: { type: "string" },
      },
    },
  },
  {
    source: "earnings-reviewer",
    targets: ["model-builder"],
  },
];

// ---------------------------------------------------------------------------
// isAllowed
// ---------------------------------------------------------------------------

describe("isAllowed", () => {
  const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
  const orchestrator = createHandoffOrchestrator({
    allowlist: baseAllowlist,
    resolveAgent: (slug) => registry.get(slug),
    dispatchAgent: makeDispatch(),
  });

  it("returns true for allowed source→target pair", () => {
    expect(orchestrator.isAllowed("market-researcher", "earnings-reviewer")).toBe(true);
    expect(orchestrator.isAllowed("market-researcher", "model-builder")).toBe(true);
    expect(orchestrator.isAllowed("earnings-reviewer", "model-builder")).toBe(true);
  });

  it("returns false for source with different target not in list", () => {
    expect(orchestrator.isAllowed("earnings-reviewer", "earnings-reviewer")).toBe(false);
    expect(orchestrator.isAllowed("earnings-reviewer", "market-researcher")).toBe(false);
  });

  it("returns false for source not in any allowlist entry", () => {
    expect(orchestrator.isAllowed("model-builder", "market-researcher")).toBe(false);
    expect(orchestrator.isAllowed("unknown-agent", "earnings-reviewer")).toBe(false);
  });

  it("returns false when allowlist is empty", () => {
    const empty = createHandoffOrchestrator({
      allowlist: [],
      resolveAgent: () => undefined,
      dispatchAgent: makeDispatch(),
    });
    expect(empty.isAllowed("market-researcher", "earnings-reviewer")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// validatePayload
// ---------------------------------------------------------------------------

describe("validatePayload", () => {
  const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
  const orchestrator = createHandoffOrchestrator({
    allowlist: baseAllowlist,
    resolveAgent: (slug) => registry.get(slug),
    dispatchAgent: makeDispatch(),
  });

  it("returns ok:true when payload matches schema", () => {
    const result = orchestrator.validatePayload("earnings-reviewer", {
      ticker: "AAPL",
      period: "Q1-2026",
    });
    expect(result.ok).toBe(true);
    expect(result.errors).toBeUndefined();
  });

  it("returns ok:false with errors when required field missing", () => {
    const result = orchestrator.validatePayload("earnings-reviewer", {
      ticker: "AAPL",
      // missing: period
    });
    expect(result.ok).toBe(false);
    expect(result.errors).toBeDefined();
    expect(result.errors!.some((e) => e.includes("period"))).toBe(true);
  });

  it("returns ok:false with field path in error message", () => {
    const result = orchestrator.validatePayload("earnings-reviewer", {
      ticker: "TOOLONGTICKERVALUE",
      period: "Q1",
    });
    expect(result.ok).toBe(false);
    expect(result.errors!.some((e) => e.includes("ticker"))).toBe(true);
  });

  it("returns ok:true (permissive) when no schema configured for target", () => {
    // Use an orchestrator whose allowlist has no payload_schema for the target.
    const noSchemaOrchestrator = createHandoffOrchestrator({
      allowlist: [
        { source: "earnings-reviewer", targets: ["model-builder"] }, // no payload_schema
      ],
      resolveAgent: () => undefined,
      dispatchAgent: makeDispatch(),
    });
    const result = noSchemaOrchestrator.validatePayload("model-builder", { anything: true });
    expect(result.ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// execute — happy path
// ---------------------------------------------------------------------------

describe("execute — happy path", () => {
  it("dispatches and returns ok:true with result", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const dispatch = makeDispatch({ finalText: "analysis complete", auditId: "audit-42" });
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: dispatch,
      onEvent,
    });

    const result = await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "MSFT", period: "Q2-2026" },
      context: { priority: "high" },
    });

    expect(result.ok).toBe(true);
    expect(result.source_agent).toBe("market-researcher");
    expect(result.target_agent).toBe("earnings-reviewer");
    expect(result.reason).toBeUndefined();
    expect(result.result).toEqual({ finalText: "analysis complete", auditId: "audit-42" });
    expect(result.duration_ms).toBeGreaterThanOrEqual(0);
  });

  it("calls resolveAgent with the target slug", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const resolve = vi.fn().mockImplementation((slug: string) => registry.get(slug));
    const dispatch = makeDispatch();

    // Use an allowlist without any payload_schema so validatePayload is permissive.
    const orchestrator = createHandoffOrchestrator({
      allowlist: [
        { source: "earnings-reviewer", targets: ["model-builder"] },
      ],
      resolveAgent: resolve,
      dispatchAgent: dispatch,
    });

    await orchestrator.execute({
      source_agent: "earnings-reviewer",
      target_agent: "model-builder",
      payload: { data: "some_data" },
    });

    expect(resolve).toHaveBeenCalledWith("model-builder");
    expect(dispatch).toHaveBeenCalledOnce();
  });

  it("passes JSON payload in prompt to dispatchAgent", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const dispatch = makeDispatch();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: dispatch,
    });

    const payload = { ticker: "GOOG", period: "Q3-2026" };
    await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload,
    });

    const [, prompt] = (dispatch as ReturnType<typeof vi.fn>).mock.calls[0] as [AgentDef, string];
    expect(prompt).toContain("market-researcher");
    expect(prompt).toContain('"GOOG"');
    expect(prompt).toContain("Payload (JSON, schema-validated)");
  });

  it("includes context section in prompt when context is provided", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const dispatch = makeDispatch();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: dispatch,
    });

    await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "META", period: "Q1-2026" },
      context: { analyst: "alice" },
    });

    const [, prompt] = (dispatch as ReturnType<typeof vi.fn>).mock.calls[0] as [AgentDef, string];
    expect(prompt).toContain("## Context");
    expect(prompt).toContain('"analyst"');
  });
});

// ---------------------------------------------------------------------------
// execute — rejection gates
// ---------------------------------------------------------------------------

describe("execute — rejection: payload null/undefined", () => {
  const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);

  it("rejects when payload is null", async () => {
    const { events, onEvent } = makeEventCollector();
    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    const result = await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: null,
    });

    expect(result.ok).toBe(false);
    expect(result.reason).toContain("payload required");
    expectEventSequence(events, ["handoff_request", "handoff_rejected"]);
  });

  it("rejects when payload is undefined", async () => {
    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
    });

    const result = await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: undefined,
    });

    expect(result.ok).toBe(false);
    expect(result.reason).toContain("payload required");
  });
});

describe("execute — rejection: target not allowlisted", () => {
  it("rejects with reason mentioning 'not allowed'", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const dispatch = makeDispatch();
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: dispatch,
      onEvent,
    });

    const result = await orchestrator.execute({
      source_agent: "model-builder",
      target_agent: "market-researcher",
      payload: { x: 1 },
    });

    expect(result.ok).toBe(false);
    expect(result.reason).toBeDefined();
    // Source has no permissions at all → "source agent has no handoff permissions"
    expect(result.reason!.toLowerCase()).toMatch(/no handoff permissions|not allowed/);

    // request + rejected, but NOT executed
    expectEventSequence(events, ["handoff_request", "handoff_rejected"]);
    expect(events.some((e) => e.type === "handoff_executed")).toBe(false);
  });

  it("rejects when source is in allowlist but target is not listed for it", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    const result = await orchestrator.execute({
      source_agent: "earnings-reviewer",
      target_agent: "market-researcher", // not in targets
      payload: { x: 1 },
    });

    expect(result.ok).toBe(false);
    expect(result.reason).toContain("not allowed");
    expectEventSequence(events, ["handoff_request", "handoff_rejected"]);
  });
});

describe("execute — rejection: payload schema invalid", () => {
  it("rejects and concatenates validator error messages", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    const result = await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "AAPL" }, // missing required: period
    });

    expect(result.ok).toBe(false);
    expect(result.reason).toBeDefined();
    expect(result.reason!).toContain("period");
    expectEventSequence(events, ["handoff_request", "handoff_rejected"]);
  });
});

describe("execute — rejection: target agent not resolved", () => {
  it("rejects when resolveAgent returns undefined", async () => {
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: () => undefined, // nothing resolves
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    const result = await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "TSLA", period: "Q4-2026" },
    });

    expect(result.ok).toBe(false);
    expect(result.reason).toContain("not found");
    expectEventSequence(events, ["handoff_request", "handoff_rejected"]);
  });
});

describe("execute — rejection: dispatchAgent throws", () => {
  it("catches error and returns ok:false with the error message", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const dispatch = vi.fn().mockRejectedValue(new Error("model timeout"));
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: dispatch,
      onEvent,
    });

    const result = await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "NVDA", period: "Q1-2026" },
    });

    expect(result.ok).toBe(false);
    expect(result.reason).toContain("model timeout");
    // handoff_executed was emitted before dispatch threw
    expectEventSequence(events, [
      "handoff_request",
      "handoff_executed",
      "handoff_rejected",
    ]);
  });
});

describe("execute — rejection: max depth exceeded", () => {
  it("rejects with 'max depth' reason when currentDepth >= maxDepth", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      maxDepth: 5,
      onEvent,
    });

    const result = await orchestrator.execute(
      {
        source_agent: "market-researcher",
        target_agent: "earnings-reviewer",
        payload: { ticker: "AMD", period: "Q2-2026" },
      },
      5, // currentDepth === maxDepth → reject
    );

    expect(result.ok).toBe(false);
    expect(result.reason).toContain("max depth");
    expectEventSequence(events, ["handoff_request", "handoff_rejected"]);
  });

  it("allows execution at depth < maxDepth", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      maxDepth: 5,
    });

    const result = await orchestrator.execute(
      {
        source_agent: "market-researcher",
        target_agent: "earnings-reviewer",
        payload: { ticker: "AMD", period: "Q2-2026" },
      },
      4, // depth 4 < maxDepth 5 → allowed
    );

    expect(result.ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Event ordering invariants
// ---------------------------------------------------------------------------

describe("event ordering invariants", () => {
  it("emits exactly one terminal event (completed or rejected), never both", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    // Happy path
    await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "AAPL", period: "Q1-2026" },
    });

    const completed = events.filter((e) => e.type === "handoff_completed").length;
    const rejected = events.filter((e) => e.type === "handoff_rejected").length;
    expect(completed + rejected).toBe(1);
    expect(completed).toBe(1);
    expect(rejected).toBe(0);
  });

  it("emits request before any other event", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "AAPL", period: "Q1-2026" },
    });

    expect(events[0].type).toBe("handoff_request");
  });

  it("does NOT emit handoff_executed on allowlist rejection", async () => {
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: () => undefined,
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    await orchestrator.execute({
      source_agent: "unknown-source",
      target_agent: "earnings-reviewer",
      payload: { x: 1 },
    });

    expect(events.some((e) => e.type === "handoff_executed")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Event metadata
// ---------------------------------------------------------------------------

describe("event metadata", () => {
  it("events carry valid request_id (UUID-like)", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "IBM", period: "Q3-2026" },
    });

    const uuidLike = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
    for (const e of events) {
      expect(e.request_id).toMatch(uuidLike);
    }
  });

  it("all events in one execute share the same request_id", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "IBM", period: "Q3-2026" },
    });

    const ids = new Set(events.map((e) => e.request_id));
    expect(ids.size).toBe(1);
  });

  it("events carry ISO 8601 timestamp", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "AMZN", period: "Q2-2026" },
    });

    const isoPattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/;
    for (const e of events) {
      expect(e.timestamp).toMatch(isoPattern);
    }
  });

  it("events carry correct source and target slugs", async () => {
    const registry = makeRegistry([RESEARCHER, REVIEWER, BUILDER]);
    const { events, onEvent } = makeEventCollector();

    const orchestrator = createHandoffOrchestrator({
      allowlist: baseAllowlist,
      resolveAgent: (slug) => registry.get(slug),
      dispatchAgent: makeDispatch(),
      onEvent,
    });

    await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { ticker: "GOOG", period: "Q1-2026" },
    });

    for (const e of events) {
      expect(e.source_agent).toBe("market-researcher");
      expect(e.target_agent).toBe("earnings-reviewer");
    }
  });
});

// ---------------------------------------------------------------------------
// Empty allowlist edge case
// ---------------------------------------------------------------------------

describe("empty allowlist", () => {
  it("rejects every request with 'no handoff permissions'", async () => {
    const orchestrator = createHandoffOrchestrator({
      allowlist: [],
      resolveAgent: () => makeAgent("dummy"),
      dispatchAgent: makeDispatch(),
    });

    const result = await orchestrator.execute({
      source_agent: "market-researcher",
      target_agent: "earnings-reviewer",
      payload: { x: 1 },
    });

    expect(result.ok).toBe(false);
    expect(result.reason).toMatch(/no handoff permissions|not allowed/);
  });
});

// ---------------------------------------------------------------------------
// Phase 38 W3: per-target schemas via buildAllowlistFromAgent + validatePayload
// ---------------------------------------------------------------------------

import { buildAllowlistFromAgent } from "../src/handoff/from-agent-def.js";

const tickerSchema: Record<string, unknown> = {
  type: "object",
  required: ["ticker", "period"],
  properties: {
    ticker: { type: "string" },
    period: { type: "string" },
  },
};

const creditSchema: Record<string, unknown> = {
  type: "object",
  required: ["company"],
  properties: {
    company: { type: "string" },
  },
};

describe("Phase 38 W3 — buildAllowlistFromAgent targetSchemas", () => {
  it("populates targetSchemas when callableAgents have inputSchema declared", () => {
    const agent: AgentDef = {
      id: "chief",
      description: "Chief",
      systemPrompt: "",
      tools: [],
      callableAgents: [
        { id: "equity-analyst", description: "", systemPrompt: "", tools: [], inputSchema: tickerSchema },
        { id: "credit-analyst", description: "", systemPrompt: "", tools: [], inputSchema: creditSchema },
      ],
    };
    const [entry] = buildAllowlistFromAgent(agent);
    expect(entry!.targetSchemas).toBeDefined();
    expect(entry!.targetSchemas!["equity-analyst"]).toEqual(tickerSchema);
    expect(entry!.targetSchemas!["credit-analyst"]).toEqual(creditSchema);
  });

  it("omits targetSchemas entries for callableAgents WITHOUT inputSchema", () => {
    const agent: AgentDef = {
      id: "chief",
      description: "Chief",
      systemPrompt: "",
      tools: [],
      callableAgents: [
        { id: "equity-analyst", description: "", systemPrompt: "", tools: [], inputSchema: tickerSchema },
        { id: "macro-analyst", description: "", systemPrompt: "", tools: [] }, // no inputSchema
      ],
    };
    const [entry] = buildAllowlistFromAgent(agent);
    expect(entry!.targetSchemas).toBeDefined();
    expect(entry!.targetSchemas!["equity-analyst"]).toEqual(tickerSchema);
    expect(entry!.targetSchemas!["macro-analyst"]).toBeUndefined();
  });

  it("validatePayload uses target-specific schema when both targetSchemas and payload_schema present", () => {
    const equitySchema: Record<string, unknown> = {
      type: "object",
      required: ["ticker"],
      properties: { ticker: { type: "string" } },
    };
    const fallbackSchema: Record<string, unknown> = {
      type: "object",
      required: ["company"],
      properties: { company: { type: "string" } },
    };
    const orchestrator = createHandoffOrchestrator({
      allowlist: [
        {
          source: "chief",
          targets: ["equity-analyst"],
          payload_schema: fallbackSchema,
          targetSchemas: { "equity-analyst": equitySchema },
        },
      ],
      resolveAgent: () => undefined,
      dispatchAgent: makeDispatch(),
    });
    // payload valid per equitySchema (has ticker), invalid per fallbackSchema (missing company)
    const result = orchestrator.validatePayload("equity-analyst", { ticker: "AAPL" });
    expect(result.ok).toBe(true);
  });

  it("validatePayload falls back to payload_schema if targetSchemas[target] missing", () => {
    const fallbackSchema: Record<string, unknown> = {
      type: "object",
      required: ["company"],
      properties: { company: { type: "string" } },
    };
    const orchestrator = createHandoffOrchestrator({
      allowlist: [
        {
          source: "chief",
          targets: ["credit-analyst"],
          payload_schema: fallbackSchema,
          targetSchemas: { "equity-analyst": tickerSchema }, // no entry for credit-analyst
        },
      ],
      resolveAgent: () => undefined,
      dispatchAgent: makeDispatch(),
    });
    // Falls back to fallbackSchema — missing required "company"
    const result = orchestrator.validatePayload("credit-analyst", { ticker: "AAPL" });
    expect(result.ok).toBe(false);
    expect(result.errors!.some((e) => e.includes("company"))).toBe(true);
  });

  it("validatePayload returns ok:true if neither targetSchemas nor payload_schema exists for target", () => {
    const orchestrator = createHandoffOrchestrator({
      allowlist: [
        {
          source: "chief",
          targets: ["macro-analyst"],
          // no payload_schema, no targetSchemas
        },
      ],
      resolveAgent: () => undefined,
      dispatchAgent: makeDispatch(),
    });
    const result = orchestrator.validatePayload("macro-analyst", { anything: true });
    expect(result.ok).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function expectEventSequence(events: HandoffEvent[], types: string[]): void {
  expect(events.map((e) => e.type)).toEqual(types);
}
