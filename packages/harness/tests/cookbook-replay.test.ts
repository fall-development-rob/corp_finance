/**
 * Phase 25 Tier A3 — cookbook replay contracts.
 *
 * Pure-function tests using inline LoadedCookbook fixtures. No file I/O,
 * no real cookbook loads. Covers fingerprint determinism, projection of
 * the AgentDef shape into the contract, serialisation stability, and
 * diff classification.
 */

import { describe, it, expect } from "vitest";
import { createHash } from "node:crypto";

import {
  replayCookbook,
  serialiseReplayCatalog,
  parseReplayCatalog,
  diffReplayCatalogs,
  type CookbookReplayCatalog,
} from "../src/manifests/cookbook-replay.js";
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
    id: "test-agent",
    description: "",
    systemPrompt: "",
    tools: [],
    ...overrides,
  };
}

function makeLoadedCookbook(
  slug: string,
  parent: AgentDef,
  subagents: AgentDef[] = [],
): LoadedCookbook {
  return { slug, parent, subagents, warnings: [] };
}

// ---------------------------------------------------------------------------
// replayCookbook — fingerprint projection
// ---------------------------------------------------------------------------

describe("replayCookbook — tools", () => {
  it("represents tools=* as ['*'] with tool_count=-1", () => {
    const replay = replayCookbook(
      makeLoadedCookbook("x", makeAgent({ tools: "*" })),
    );
    expect(replay.parent.tools).toEqual(["*"]);
    expect(replay.parent.tool_count).toBe(-1);
  });

  it("sorts tool allowlists alphabetically", () => {
    const replay = replayCookbook(
      makeLoadedCookbook(
        "x",
        makeAgent({ tools: ["zebra", "apple", "mango"] }),
      ),
    );
    expect(replay.parent.tools).toEqual(["apple", "mango", "zebra"]);
    expect(replay.parent.tool_count).toBe(3);
  });

  it("represents empty tool allowlist as [] with tool_count=0", () => {
    const replay = replayCookbook(
      makeLoadedCookbook("x", makeAgent({ tools: [] })),
    );
    expect(replay.parent.tools).toEqual([]);
    expect(replay.parent.tool_count).toBe(0);
  });

  it("sorts block_tools alphabetically", () => {
    const replay = replayCookbook(
      makeLoadedCookbook(
        "x",
        makeAgent({ tools: "*", blockTools: ["zzz", "aaa"] }),
      ),
    );
    expect(replay.parent.block_tools).toEqual(["aaa", "zzz"]);
  });

  it("defaults block_tools to empty array when undefined", () => {
    const replay = replayCookbook(
      makeLoadedCookbook("x", makeAgent({ tools: "*" })),
    );
    expect(replay.parent.block_tools).toEqual([]);
  });
});

describe("replayCookbook — system prompt fingerprint", () => {
  it("hashes the system prompt content with sha256", () => {
    const prompt = "You are an analyst.";
    const replay = replayCookbook(
      makeLoadedCookbook(
        "x",
        makeAgent({ systemPrompt: prompt, tools: [] }),
      ),
    );
    expect(replay.parent.system_prompt_sha256).toBe(sha(prompt));
    expect(replay.parent.system_prompt_bytes).toBe(
      Buffer.byteLength(prompt, "utf8"),
    );
  });

  it("produces different hashes for different prompts", () => {
    const a = replayCookbook(
      makeLoadedCookbook("x", makeAgent({ systemPrompt: "alpha" })),
    );
    const b = replayCookbook(
      makeLoadedCookbook("x", makeAgent({ systemPrompt: "beta" })),
    );
    expect(a.parent.system_prompt_sha256).not.toBe(
      b.parent.system_prompt_sha256,
    );
  });
});

describe("replayCookbook — schemas", () => {
  it("emits empty string for output_schema_sha256 when schema is absent", () => {
    const replay = replayCookbook(
      makeLoadedCookbook("x", makeAgent({ tools: "*" })),
    );
    expect(replay.parent.output_schema_sha256).toBe("");
    expect(replay.parent.input_schema_sha256).toBe("");
  });

  it("hashes output_schema canonically (key order independent)", () => {
    const schemaA = makeAgent({
      tools: "*",
      outputSchema: { type: "object", required: ["x"], properties: {} },
    });
    const schemaB = makeAgent({
      tools: "*",
      outputSchema: { properties: {}, required: ["x"], type: "object" },
    });
    const a = replayCookbook(makeLoadedCookbook("x", schemaA));
    const b = replayCookbook(makeLoadedCookbook("x", schemaB));
    expect(a.parent.output_schema_sha256).toBe(b.parent.output_schema_sha256);
    expect(a.parent.output_schema_sha256).toMatch(/^[a-f0-9]{64}$/);
  });

  it("produces different output_schema hashes for different schemas", () => {
    const a = replayCookbook(
      makeLoadedCookbook(
        "x",
        makeAgent({ tools: "*", outputSchema: { type: "object" } }),
      ),
    );
    const b = replayCookbook(
      makeLoadedCookbook(
        "x",
        makeAgent({ tools: "*", outputSchema: { type: "string" } }),
      ),
    );
    expect(a.parent.output_schema_sha256).not.toBe(
      b.parent.output_schema_sha256,
    );
  });
});

describe("replayCookbook — subagents", () => {
  it("records subagent IDs in load order on the parent", () => {
    const replay = replayCookbook(
      makeLoadedCookbook(
        "x",
        makeAgent({ tools: "*" }),
        [
          makeAgent({ id: "first" }),
          makeAgent({ id: "second" }),
          makeAgent({ id: "third" }),
        ],
      ),
    );
    expect(replay.parent.subagent_ids).toEqual(["first", "second", "third"]);
    expect(replay.subagents).toHaveLength(3);
  });

  it("fingerprints each subagent independently", () => {
    const replay = replayCookbook(
      makeLoadedCookbook(
        "x",
        makeAgent({ tools: "*" }),
        [
          makeAgent({
            id: "worker",
            systemPrompt: "do work",
            tools: ["tool_a", "tool_b"],
          }),
        ],
      ),
    );
    expect(replay.subagents[0]?.id).toBe("worker");
    expect(replay.subagents[0]?.tools).toEqual(["tool_a", "tool_b"]);
    expect(replay.subagents[0]?.system_prompt_sha256).toBe(sha("do work"));
  });

  it("preserves optional model field when present", () => {
    const replay = replayCookbook(
      makeLoadedCookbook(
        "x",
        makeAgent({ tools: "*", model: "claude-opus-4-7" }),
      ),
    );
    expect(replay.parent.model).toBe("claude-opus-4-7");
  });

  it("omits model field entirely when absent", () => {
    const replay = replayCookbook(
      makeLoadedCookbook("x", makeAgent({ tools: "*" })),
    );
    expect("model" in replay.parent).toBe(false);
  });
});

describe("replayCookbook — determinism", () => {
  it("two calls against the same LoadedCookbook produce identical replays", () => {
    const cookbook = makeLoadedCookbook(
      "x",
      makeAgent({
        tools: ["b", "a"],
        systemPrompt: "p",
        outputSchema: { type: "object" },
      }),
      [makeAgent({ id: "s1", tools: ["x"] })],
    );
    const a = JSON.stringify(replayCookbook(cookbook));
    const b = JSON.stringify(replayCookbook(cookbook));
    expect(a).toBe(b);
  });
});

// ---------------------------------------------------------------------------
// serialiseReplayCatalog
// ---------------------------------------------------------------------------

describe("serialiseReplayCatalog", () => {
  it("emits cookbooks sorted alphabetically by slug", () => {
    const catalog: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [
        {
          slug: "zzz",
          parent: {
            id: "z",
            tool_count: 0,
            tools: [],
            block_tools: [],
            system_prompt_sha256: "z",
            system_prompt_bytes: 0,
            output_schema_sha256: "",
            input_schema_sha256: "",
            subagent_ids: [],
          },
          subagents: [],
        },
        {
          slug: "aaa",
          parent: {
            id: "a",
            tool_count: 0,
            tools: [],
            block_tools: [],
            system_prompt_sha256: "a",
            system_prompt_bytes: 0,
            output_schema_sha256: "",
            input_schema_sha256: "",
            subagent_ids: [],
          },
          subagents: [],
        },
      ],
    };
    const out = serialiseReplayCatalog(catalog);
    expect(out.indexOf('"aaa"')).toBeLessThan(out.indexOf('"zzz"'));
  });

  it("is byte-deterministic across two calls", () => {
    const c: CookbookReplayCatalog = { version: "1", cookbooks: [] };
    expect(serialiseReplayCatalog(c)).toBe(serialiseReplayCatalog(c));
  });

  it("ends with a trailing newline", () => {
    const c: CookbookReplayCatalog = { version: "1", cookbooks: [] };
    expect(serialiseReplayCatalog(c).endsWith("\n")).toBe(true);
  });

  it("round-trips through parseReplayCatalog", () => {
    const c: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [
        {
          slug: "a",
          parent: {
            id: "p",
            tool_count: -1,
            tools: ["*"],
            block_tools: [],
            system_prompt_sha256: "h",
            system_prompt_bytes: 0,
            output_schema_sha256: "",
            input_schema_sha256: "",
            subagent_ids: [],
          },
          subagents: [],
        },
      ],
    };
    expect(parseReplayCatalog(serialiseReplayCatalog(c))).toEqual(c);
  });
});

describe("parseReplayCatalog — validation", () => {
  it("rejects JSON without a cookbooks array", () => {
    expect(() => parseReplayCatalog("{}")).toThrow(/cookbooks/);
  });
});

// ---------------------------------------------------------------------------
// diffReplayCatalogs
// ---------------------------------------------------------------------------

describe("diffReplayCatalogs", () => {
  function cb(slug: string, parentTools: string[]) {
    return {
      slug,
      parent: {
        id: `${slug}-parent`,
        tool_count: parentTools.length,
        tools: [...parentTools].sort(),
        block_tools: [],
        system_prompt_sha256: sha(slug),
        system_prompt_bytes: slug.length,
        output_schema_sha256: "",
        input_schema_sha256: "",
        subagent_ids: [],
      },
      subagents: [],
    };
  }

  it("returns empty diff for identical catalogs", () => {
    const c: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [cb("a", ["x"]), cb("b", ["y"])],
    };
    const diff = diffReplayCatalogs(c, c);
    expect(diff.added).toEqual([]);
    expect(diff.removed).toEqual([]);
    expect(diff.changed).toEqual([]);
  });

  it("flags added cookbooks", () => {
    const before: CookbookReplayCatalog = { version: "1", cookbooks: [] };
    const after: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [cb("new", ["x"])],
    };
    expect(diffReplayCatalogs(before, after).added).toEqual(["new"]);
  });

  it("flags removed cookbooks", () => {
    const before: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [cb("old", ["x"])],
    };
    const after: CookbookReplayCatalog = { version: "1", cookbooks: [] };
    expect(diffReplayCatalogs(before, after).removed).toEqual(["old"]);
  });

  it("flags parent tool-list changes with field='tools'", () => {
    const before: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [cb("a", ["x"])],
    };
    const after: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [cb("a", ["x", "y"])],
    };
    const diff = diffReplayCatalogs(before, after);
    expect(diff.changed).toHaveLength(1);
    const fields = diff.changed[0]?.parent_changes.map((c) => c.field);
    expect(fields).toContain("tools");
    expect(fields).toContain("tool_count");
  });

  it("flags subagent additions and removals", () => {
    const before: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [
        {
          ...cb("a", ["x"]),
          subagents: [
            {
              id: "s1",
              tool_count: 0,
              tools: [],
              block_tools: [],
              system_prompt_sha256: "h",
              system_prompt_bytes: 0,
              output_schema_sha256: "",
              input_schema_sha256: "",
            },
          ],
        },
      ],
    };
    const after: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [
        {
          ...cb("a", ["x"]),
          subagents: [
            {
              id: "s2",
              tool_count: 0,
              tools: [],
              block_tools: [],
              system_prompt_sha256: "h",
              system_prompt_bytes: 0,
              output_schema_sha256: "",
              input_schema_sha256: "",
            },
          ],
        },
      ],
    };
    const diff = diffReplayCatalogs(before, after);
    expect(diff.changed).toHaveLength(1);
    const subIds = diff.changed[0]?.subagent_changes.map((s) => s.id).sort();
    expect(subIds).toEqual(["s1", "s2"]);
  });

  it("flags subagent_ids drift (reordering, addition, removal)", () => {
    const before: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [
        {
          ...cb("a", []),
          parent: { ...cb("a", []).parent, subagent_ids: ["s1", "s2"] },
        },
      ],
    };
    const after: CookbookReplayCatalog = {
      version: "1",
      cookbooks: [
        {
          ...cb("a", []),
          parent: { ...cb("a", []).parent, subagent_ids: ["s2", "s1"] },
        },
      ],
    };
    const diff = diffReplayCatalogs(before, after);
    const fields = diff.changed[0]?.parent_changes.map((c) => c.field);
    expect(fields).toContain("subagent_ids");
  });
});
