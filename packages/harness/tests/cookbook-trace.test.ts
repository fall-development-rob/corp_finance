/**
 * Phase 25 Tier C4 — cookbook synthetic-trace evaluation.
 *
 * Pure-function tests using inline LoadedCookbook fixtures. No file
 * I/O, no real cookbook loads. Covers full-prompt capture, model/
 * max_tokens propagation, deterministic serialisation, schema
 * canonicalisation (key-order independent), and steering-event
 * extraction.
 */

import { describe, it, expect } from "vitest";
import { createHash } from "node:crypto";

import {
  buildSyntheticTrace,
  serialiseTrace,
  parseTrace,
  extractSteeringEvents,
  type CookbookTrace,
} from "../src/manifests/cookbook-trace.js";
import type { LoadedCookbook } from "../src/manifests/cookbook-loader.js";
import type { AgentDef } from "../src/types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sha(s: string): string {
  return createHash("sha256").update(s, "utf8").digest("hex");
}

function makeAgent(overrides: Partial<AgentDef> = {}): AgentDef {
  return {
    id: "agent",
    description: "",
    systemPrompt: "",
    tools: [],
    ...overrides,
  };
}

function makeLoaded(
  slug: string,
  parent: AgentDef,
  subagents: AgentDef[] = [],
): LoadedCookbook {
  return { slug, parent, subagents, warnings: [] };
}

// ---------------------------------------------------------------------------
// buildSyntheticTrace — content capture
// ---------------------------------------------------------------------------

describe("buildSyntheticTrace — full prompt capture", () => {
  it("includes the parent's full assembled system prompt verbatim", () => {
    const prompt = "You are an analyst.\n\nFollow these rules.";
    const trace = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent({ systemPrompt: prompt })),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(trace.parent.system_prompt).toBe(prompt);
    expect(trace.parent.system_prompt_bytes).toBe(
      Buffer.byteLength(prompt, "utf8"),
    );
    expect(trace.parent.system_prompt_sha256).toBe(sha(prompt));
  });

  it("includes each subagent's system prompt independently", () => {
    const trace = buildSyntheticTrace({
      loaded: makeLoaded(
        "x",
        makeAgent({ systemPrompt: "parent" }),
        [
          makeAgent({ id: "s1", systemPrompt: "first" }),
          makeAgent({ id: "s2", systemPrompt: "second" }),
        ],
      ),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(trace.subagents).toHaveLength(2);
    expect(trace.subagents[0]?.system_prompt).toBe("first");
    expect(trace.subagents[1]?.system_prompt).toBe("second");
  });

  it("records audit_hash and version exactly as provided", () => {
    const trace = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent()),
      auditHash: "abc123",
      version: "2.3.4",
      exampleEvents: [],
    });
    expect(trace.audit_hash).toBe("abc123");
    expect(trace.version).toBe("2.3.4");
  });

  it("copies example_events from input into the trace", () => {
    const events = ["e1", "e2", "e3"];
    const trace = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent()),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: events,
    });
    expect(trace.example_events).toEqual(events);
  });

  it("preserves subagent_ids on parent", () => {
    const trace = buildSyntheticTrace({
      loaded: makeLoaded(
        "x",
        makeAgent(),
        [makeAgent({ id: "a" }), makeAgent({ id: "b" })],
      ),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(trace.parent.subagent_ids).toEqual(["a", "b"]);
  });
});

describe("buildSyntheticTrace — tools + schemas", () => {
  it("represents tools=* as ['*']", () => {
    const trace = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent({ tools: "*" })),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(trace.parent.tools).toEqual(["*"]);
  });

  it("sorts explicit tool allowlists alphabetically", () => {
    const trace = buildSyntheticTrace({
      loaded: makeLoaded(
        "x",
        makeAgent({ tools: ["z", "a", "m"] }),
      ),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(trace.parent.tools).toEqual(["a", "m", "z"]);
  });

  it("emits canonical JSON for output_schema (key order independent)", () => {
    const a = buildSyntheticTrace({
      loaded: makeLoaded(
        "x",
        makeAgent({
          outputSchema: { type: "object", required: ["x"], properties: {} },
        }),
      ),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    const b = buildSyntheticTrace({
      loaded: makeLoaded(
        "x",
        makeAgent({
          outputSchema: { properties: {}, required: ["x"], type: "object" },
        }),
      ),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(a.parent.output_schema_json).toBe(b.parent.output_schema_json);
  });

  it("emits empty string for output_schema_json when absent", () => {
    const trace = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent()),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(trace.parent.output_schema_json).toBe("");
  });
});

describe("buildSyntheticTrace — optional fields", () => {
  it("emits model field when present, omits when absent", () => {
    const present = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent({ model: "claude-opus-4-7" })),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    const absent = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent()),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(present.parent.model).toBe("claude-opus-4-7");
    expect("model" in absent.parent).toBe(false);
  });

  it("emits max_tokens when present, omits when absent", () => {
    const present = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent({ maxTokens: 8192 })),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    const absent = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent()),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(present.parent.max_tokens).toBe(8192);
    expect("max_tokens" in absent.parent).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Determinism + serialisation
// ---------------------------------------------------------------------------

describe("buildSyntheticTrace + serialiseTrace — determinism", () => {
  it("two builds against the same input produce identical JSON", () => {
    const input = {
      loaded: makeLoaded(
        "x",
        makeAgent({
          systemPrompt: "p",
          tools: ["b", "a"],
          model: "claude-opus-4-7",
        }),
        [makeAgent({ id: "s", tools: ["x"] })],
      ),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: ["e1"],
    };
    const a = serialiseTrace(buildSyntheticTrace(input));
    const b = serialiseTrace(buildSyntheticTrace(input));
    expect(a).toBe(b);
  });

  it("serialiseTrace ends with a trailing newline", () => {
    const trace = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent()),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(serialiseTrace(trace).endsWith("\n")).toBe(true);
  });
});

describe("parseTrace — round-trip", () => {
  it("round-trips a trace through serialise + parse", () => {
    const trace = buildSyntheticTrace({
      loaded: makeLoaded(
        "x",
        makeAgent({ systemPrompt: "p", tools: "*" }),
      ),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    const parsed = parseTrace(serialiseTrace(trace));
    expect(parsed.slug).toBe(trace.slug);
    expect(parsed.version).toBe(trace.version);
    expect(parsed.parent.system_prompt).toBe(trace.parent.system_prompt);
  });

  it("rejects JSON missing required fields", () => {
    expect(() => parseTrace("{}")).toThrow(/slug.*version.*parent.*subagents/);
  });
});

// ---------------------------------------------------------------------------
// extractSteeringEvents
// ---------------------------------------------------------------------------

describe("extractSteeringEvents", () => {
  it("extracts event strings from a typed steering-examples array", () => {
    const parsed = [
      { event: "e1", description: "d1" },
      { event: "e2", description: "d2" },
    ];
    expect(extractSteeringEvents(parsed)).toEqual(["e1", "e2"]);
  });

  it("skips entries missing the event field", () => {
    const parsed = [
      { event: "e1" },
      { description: "no event here" },
      { event: "e3" },
    ];
    expect(extractSteeringEvents(parsed)).toEqual(["e1", "e3"]);
  });

  it("returns empty array on non-array input", () => {
    expect(extractSteeringEvents(null)).toEqual([]);
    expect(extractSteeringEvents({})).toEqual([]);
    expect(extractSteeringEvents("not an array")).toEqual([]);
  });

  it("returns empty array on empty input", () => {
    expect(extractSteeringEvents([])).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Catalog-level helpers
// ---------------------------------------------------------------------------

describe("CookbookTrace shape", () => {
  it("has exactly the documented top-level keys", () => {
    const trace: CookbookTrace = buildSyntheticTrace({
      loaded: makeLoaded("x", makeAgent()),
      auditHash: "h",
      version: "1.0.0",
      exampleEvents: [],
    });
    expect(Object.keys(trace).sort()).toEqual([
      "audit_hash",
      "example_events",
      "parent",
      "slug",
      "subagents",
      "version",
    ]);
  });
});
