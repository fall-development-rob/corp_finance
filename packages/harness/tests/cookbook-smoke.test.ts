/**
 * Cookbook deployment smoke tests — Phase 33 smoke gate.
 *
 * Verifies that every managed-agent-cookbook:
 *   1. Loads into a valid AgentDef without runtime errors (loader chain ok)
 *   2. Has structurally valid output_schema on each subagent
 *   3. Can dispatch with a stub provider + stub MCP without exceptions
 *
 * These tests gate against regressions where a manifest change lints clean
 * but breaks at the harness loader (e.g., malformed enum, bad from_plugin path).
 */

import { describe, expect, it } from "vitest";
import { resolve } from "node:path";
import { createCookbookLoader } from "../src/manifests/cookbook-loader.js";
import { parseAndValidate } from "../src/manifests/validator.js";
import { dispatch } from "../src/core/agent-loop.js";
import type {
  AgentDef,
  CanonicalTool,
  MCPClient,
  Provider,
  ProviderTurnRequest,
  ProviderTurnResponse,
} from "../src/types.js";
import type { SkillLoader } from "../src/skills/types.js";
import type { ParsedSkill } from "../src/skills/types.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const REPO_ROOT = resolve(import.meta.dirname, "../../..");
const COOKBOOKS_ROOT = resolve(REPO_ROOT, "managed-agent-cookbooks");

const EXPECTED_SLUGS = [
  "credit-analyst",
  "earnings-reviewer",
  "equity-analyst",
  "gl-reconciler",
  "kyc-screener",
  "lp-statement-auditor",
  "lseg-rates-monitor",
  "model-builder",
  "month-end-closer",
  "pitch-deck-builder",
  "private-markets-analyst",
  "sector-research",
  "sp-credit-research",
  "skill-editor",
  "valuation-reviewer",
  "wealth-meeting-prep",
] as const;

const EXPECTED_PARENT_NAME: Record<string, string> = {
  "credit-analyst": "cfa-credit-analyst",
  "earnings-reviewer": "cfa-earnings-reviewer",
  "equity-analyst": "cfa-equity-analyst",
  "gl-reconciler": "cfa-gl-reconciler",
  "kyc-screener": "cfa-kyc-screener",
  "lp-statement-auditor": "cfa-lp-statement-auditor",
  "lseg-rates-monitor": "cfa-lseg-rates-monitor",
  "model-builder": "cfa-model-builder",
  "month-end-closer": "cfa-month-end-closer",
  "pitch-deck-builder": "cfa-pitch-deck-builder",
  "private-markets-analyst": "cfa-private-markets-analyst",
  "sector-research": "cfa-sector-research",
  "skill-editor": "skill-editor",
  "sp-credit-research": "cfa-sp-credit-research",
  "valuation-reviewer": "cfa-valuation-reviewer",
  "wealth-meeting-prep": "cfa-wealth-meeting-prep",
};

// ---------------------------------------------------------------------------
// Stub helpers
// ---------------------------------------------------------------------------

/** Null skill loader — cookbooks use from_plugin exclusively; from_skill not expected. */
function makeNullSkillLoader(): SkillLoader {
  return {
    async loadSkill(id: string): Promise<ParsedSkill> {
      throw new Error(`Unexpected from_skill reference to "${id}" in cookbook`);
    },
    async loadAgent() {
      throw new Error("not used in cookbook-smoke");
    },
    clearCache() {},
  };
}

function makeStubMCPClient(tools: CanonicalTool[] = []): MCPClient {
  return {
    initialize: async () => {},
    listTools: async () => tools,
    callTool: async () => ({ content: [{ type: "text", text: "stub" }] }),
    close: async () => {},
  };
}

/** Provider that returns a single text turn immediately. */
function makeStubProvider(text: string): Provider {
  return {
    name: "anthropic" as const,
    async turn(_req: ProviderTurnRequest): Promise<ProviderTurnResponse> {
      return {
        message: {
          role: "assistant",
          content: [{ type: "text", text }],
        },
        stopReason: "end_turn",
        usage: { inputTokens: 10, outputTokens: 5 },
      };
    },
  };
}

// ---------------------------------------------------------------------------
// Shared loader instance (created once; all tests are read-only)
// ---------------------------------------------------------------------------

const loader = createCookbookLoader({
  cookbooksRoot: COOKBOOKS_ROOT,
  skillLoader: makeNullSkillLoader(),
});

// ---------------------------------------------------------------------------
// 1. Inventory test
// ---------------------------------------------------------------------------

describe("CookbookLoader.list()", () => {
  it("returns exactly the 16 expected cookbook slugs", async () => {
    const slugs = await loader.list();
    expect(new Set(slugs)).toEqual(new Set(EXPECTED_SLUGS));
    expect(slugs).toHaveLength(16);
  });
});

// ---------------------------------------------------------------------------
// 2. Per-cookbook load tests (16 × generated)
// ---------------------------------------------------------------------------

describe("CookbookLoader.load() — per-cookbook", () => {
  for (const slug of EXPECTED_SLUGS) {
    describe(`cookbook: ${slug}`, () => {
      it("loads parent and 3 subagents without errors", async () => {
        const cookbook = await loader.load(slug);

        expect(cookbook.slug).toBe(slug);
        expect(cookbook.parent.id).toBe(EXPECTED_PARENT_NAME[slug]);
        expect(cookbook.subagents).toHaveLength(3);
        // No structural warnings
        expect(cookbook.warnings).toHaveLength(0);
      });

      it("each subagent output_schema is structurally valid (parseAndValidate does not throw)", async () => {
        const cookbook = await loader.load(slug);

        for (const sub of cookbook.subagents) {
          const schema = sub.outputSchema;
          expect(schema).toBeDefined();
          // parseAndValidate(empty-string, schema) must not throw; ok:false is fine
          // (an empty value naturally fails required-field checks)
          let threw = false;
          let result: ReturnType<typeof parseAndValidate> | undefined;
          try {
            result = parseAndValidate("{}", schema as Record<string, unknown>);
          } catch {
            threw = true;
          }
          expect(threw, `subagent ${sub.id} output_schema threw during parseAndValidate`).toBe(false);
          // The result exists and has a boolean ok field — schema is parseable
          expect(result).toBeDefined();
          expect(typeof result?.ok).toBe("boolean");
        }
      });

      it("each subagent blockTools (if present) is an array of strings", async () => {
        const cookbook = await loader.load(slug);

        for (const sub of cookbook.subagents) {
          if (sub.blockTools !== undefined) {
            expect(Array.isArray(sub.blockTools)).toBe(true);
            for (const tool of sub.blockTools) {
              expect(typeof tool).toBe("string");
            }
          }
        }
      });

      it("parent systemPrompt is non-empty (skills + system.file resolved)", async () => {
        const cookbook = await loader.load(slug);
        expect(cookbook.parent.systemPrompt.trim().length).toBeGreaterThan(0);
      });

      it("parent model field is set to a claude model string", async () => {
        const cookbook = await loader.load(slug);
        expect(cookbook.parent.model).toMatch(/^claude-/);
      });
    });
  }
});

// ---------------------------------------------------------------------------
// 3. Stub dispatch test (credit-analyst as representative)
// ---------------------------------------------------------------------------

describe("CookbookLoader — stub dispatch", () => {
  it.skipIf(process.env["SKIP_HEAVY"] === "1")(
    "credit-analyst parent dispatches with stub provider without error",
    async () => {
      const cookbook = await loader.load("credit-analyst");
      const agent: AgentDef = {
        ...cookbook.parent,
        // Stub out tools to avoid real MCP requirements
        tools: "*",
        maxRecursionDepth: 0,
      };

      const mcp = makeStubMCPClient();
      const provider = makeStubProvider("stub credit analyst response");

      const result = await dispatch({
        agent,
        prompt: "smoke test: analyse AAPL credit",
        provider,
        mcp,
        maxTurns: 2,
      });

      expect(result).toBeDefined();
      expect(typeof result.finalText).toBe("string");
    },
  );
});

// ---------------------------------------------------------------------------
// 4. Aggregate test
// ---------------------------------------------------------------------------

describe("CookbookLoader.loadAll()", () => {
  it("loads all 16 cookbooks and total subagent count is 48", async () => {
    const all = await loader.loadAll();

    expect(all).toHaveLength(16);

    const totalSubagents = all.reduce((sum, c) => sum + c.subagents.length, 0);
    expect(totalSubagents).toBe(48);

    // Every cookbook's slug appears exactly once
    const seenSlugs = new Set(all.map((c) => c.slug));
    expect(seenSlugs).toEqual(new Set(EXPECTED_SLUGS));
  });
});
