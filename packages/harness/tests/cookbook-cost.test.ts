/**
 * Phase 25 Tier C1 — cookbook cost telemetry.
 *
 * Pure-function tests using inline AgentDef / LoadedCookbook fixtures.
 * No file I/O, no real cookbook loads. Covers pricing-table lookup,
 * token estimation heuristic, worst-case formula, fallback for unknown
 * models, aggregation, determinism, and serialisation.
 */

import { describe, it, expect } from "vitest";

import {
  estimateAgentCost,
  estimateCookbookCost,
  buildCostCatalog,
  serialiseCostCatalog,
  parseCostCatalog,
  MODEL_PRICING,
  DEFAULT_MODEL_ID,
  DEFAULT_MAX_OUTPUT_TOKENS,
  CHARS_PER_TOKEN_HEURISTIC,
  type CookbookCostCatalog,
} from "../src/manifests/cookbook-cost.js";
import type { LoadedCookbook } from "../src/manifests/cookbook-loader.js";
import type { AgentDef } from "../src/types.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
// estimateAgentCost — pricing + formulas
// ---------------------------------------------------------------------------

describe("estimateAgentCost — model resolution", () => {
  it("uses the explicit model when listed in MODEL_PRICING", () => {
    const cost = estimateAgentCost(
      makeAgent({ model: "claude-opus-4-7", systemPrompt: "x" }),
    );
    expect(cost.model).toBe("claude-opus-4-7");
    expect(cost.model_priced).toBe(true);
  });

  it("falls back to DEFAULT_MODEL_ID rates for unknown models", () => {
    const cost = estimateAgentCost(
      makeAgent({ model: "future-model-2030", systemPrompt: "x" }),
    );
    expect(cost.model).toBe("future-model-2030");
    expect(cost.model_priced).toBe(false);
    // Should use default-model rates
    const defaultPrice = MODEL_PRICING[DEFAULT_MODEL_ID]!;
    const expectedOutput =
      (DEFAULT_MAX_OUTPUT_TOKENS / 1_000_000) * defaultPrice.output_per_mtok;
    expect(cost.output_cost_usd).toBeCloseTo(expectedOutput, 6);
  });

  it("uses DEFAULT_MODEL_ID when agent has no model field", () => {
    const cost = estimateAgentCost(makeAgent({ systemPrompt: "x" }));
    expect(cost.model).toBe(DEFAULT_MODEL_ID);
    expect(cost.model_priced).toBe(true);
  });
});

describe("estimateAgentCost — token estimation", () => {
  it(`estimates input tokens via ceil(len / ${CHARS_PER_TOKEN_HEURISTIC})`, () => {
    const cost = estimateAgentCost(
      makeAgent({ systemPrompt: "x".repeat(100) }),
    );
    expect(cost.input_tokens_est).toBe(
      Math.ceil(100 / CHARS_PER_TOKEN_HEURISTIC),
    );
  });

  it("treats empty system prompt as 0 input tokens", () => {
    const cost = estimateAgentCost(makeAgent({ systemPrompt: "" }));
    expect(cost.input_tokens_est).toBe(0);
    expect(cost.input_cost_usd).toBe(0);
  });

  it("uses agent.maxTokens for output budget when present", () => {
    const cost = estimateAgentCost(
      makeAgent({ model: "claude-haiku-4-5", maxTokens: 1000 }),
    );
    expect(cost.output_tokens_max).toBe(1000);
  });

  it("falls back to DEFAULT_MAX_OUTPUT_TOKENS when agent has no maxTokens", () => {
    const cost = estimateAgentCost(makeAgent({ model: "claude-haiku-4-5" }));
    expect(cost.output_tokens_max).toBe(DEFAULT_MAX_OUTPUT_TOKENS);
  });
});

describe("estimateAgentCost — cost formula", () => {
  it("computes worst-case Opus cost correctly", () => {
    // 10000 chars → 2500 input tokens
    const cost = estimateAgentCost(
      makeAgent({
        model: "claude-opus-4-7",
        systemPrompt: "x".repeat(10000),
        maxTokens: 4096,
      }),
    );
    expect(cost.input_tokens_est).toBe(2500);
    // 2500 tokens × $15/Mtok = $0.0375 input
    expect(cost.input_cost_usd).toBeCloseTo(2500 * 15 / 1_000_000, 6);
    // 4096 × $75/Mtok = $0.3072 output
    expect(cost.output_cost_usd).toBeCloseTo(4096 * 75 / 1_000_000, 6);
    // total = $0.3447
    expect(cost.total_cost_usd).toBeCloseTo(
      cost.input_cost_usd + cost.output_cost_usd,
      6,
    );
  });

  it("Haiku is materially cheaper than Opus for the same prompt", () => {
    const opus = estimateAgentCost(
      makeAgent({
        model: "claude-opus-4-7",
        systemPrompt: "x".repeat(4000),
      }),
    );
    const haiku = estimateAgentCost(
      makeAgent({
        model: "claude-haiku-4-5",
        systemPrompt: "x".repeat(4000),
      }),
    );
    expect(haiku.total_cost_usd).toBeLessThan(opus.total_cost_usd / 10);
  });

  it("rounds dollar amounts to 6 decimal places", () => {
    const cost = estimateAgentCost(
      makeAgent({ model: "claude-opus-4-7", systemPrompt: "x" }),
    );
    expect(cost.input_cost_usd.toString().split(".")[1]?.length ?? 0).toBeLessThanOrEqual(6);
    expect(cost.output_cost_usd.toString().split(".")[1]?.length ?? 0).toBeLessThanOrEqual(6);
    expect(cost.total_cost_usd.toString().split(".")[1]?.length ?? 0).toBeLessThanOrEqual(6);
  });

  it("is byte-deterministic — two calls produce identical output", () => {
    const agent = makeAgent({
      model: "claude-opus-4-7",
      systemPrompt: "deterministic prompt",
      maxTokens: 4096,
    });
    expect(JSON.stringify(estimateAgentCost(agent))).toBe(
      JSON.stringify(estimateAgentCost(agent)),
    );
  });
});

// ---------------------------------------------------------------------------
// estimateCookbookCost — aggregation
// ---------------------------------------------------------------------------

describe("estimateCookbookCost", () => {
  it("includes the parent + every subagent in agents[]", () => {
    const est = estimateCookbookCost(
      makeLoaded(
        "x",
        makeAgent({ id: "parent", model: "claude-opus-4-7" }),
        [
          makeAgent({ id: "s1", model: "claude-sonnet-4-6" }),
          makeAgent({ id: "s2", model: "claude-haiku-4-5" }),
        ],
      ),
    );
    expect(est.agents).toHaveLength(3);
    expect(est.agents.map((a) => a.id)).toEqual(["parent", "s1", "s2"]);
  });

  it("total = sum of all agent totals", () => {
    const est = estimateCookbookCost(
      makeLoaded(
        "x",
        makeAgent({
          model: "claude-opus-4-7",
          systemPrompt: "x".repeat(100),
        }),
        [
          makeAgent({
            id: "sub",
            model: "claude-haiku-4-5",
            systemPrompt: "y".repeat(50),
          }),
        ],
      ),
    );
    const sum = est.agents.reduce((n, a) => n + a.total_cost_usd, 0);
    // Allow a 1-µ$ rounding tolerance from the round6 step on total.
    expect(Math.abs(est.total_cost_usd - sum)).toBeLessThan(2e-6);
  });

  it("preserves the cookbook slug", () => {
    const est = estimateCookbookCost(
      makeLoaded("equity-analyst", makeAgent({ model: "claude-opus-4-7" })),
    );
    expect(est.slug).toBe("equity-analyst");
  });
});

// ---------------------------------------------------------------------------
// buildCostCatalog — grand total + sort order
// ---------------------------------------------------------------------------

describe("buildCostCatalog", () => {
  it("sorts cookbooks alphabetically by slug", () => {
    const cat = buildCostCatalog([
      makeLoaded("zzz", makeAgent({ model: "claude-haiku-4-5" })),
      makeLoaded("aaa", makeAgent({ model: "claude-haiku-4-5" })),
      makeLoaded("mmm", makeAgent({ model: "claude-haiku-4-5" })),
    ]);
    expect(cat.cookbooks.map((c) => c.slug)).toEqual(["aaa", "mmm", "zzz"]);
  });

  it("grand_total_usd is the sum of per-cookbook totals", () => {
    const cat = buildCostCatalog([
      makeLoaded("a", makeAgent({ model: "claude-opus-4-7" })),
      makeLoaded("b", makeAgent({ model: "claude-haiku-4-5" })),
    ]);
    const sum = cat.cookbooks.reduce((n, c) => n + c.total_cost_usd, 0);
    expect(Math.abs(cat.grand_total_usd - sum)).toBeLessThan(2e-6);
  });

  it("returns an empty catalog when given no cookbooks", () => {
    const cat = buildCostCatalog([]);
    expect(cat.cookbooks).toEqual([]);
    expect(cat.grand_total_usd).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Serialisation
// ---------------------------------------------------------------------------

describe("serialiseCostCatalog", () => {
  it("is byte-deterministic across two calls", () => {
    const cat: CookbookCostCatalog = {
      version: "1",
      grand_total_usd: 0,
      cookbooks: [],
    };
    expect(serialiseCostCatalog(cat)).toBe(serialiseCostCatalog(cat));
  });

  it("ends with a trailing newline", () => {
    const out = serialiseCostCatalog({
      version: "1",
      grand_total_usd: 0,
      cookbooks: [],
    });
    expect(out.endsWith("\n")).toBe(true);
  });

  it("round-trips through parseCostCatalog", () => {
    const cat: CookbookCostCatalog = {
      version: "1",
      grand_total_usd: 0.123456,
      cookbooks: [
        {
          slug: "x",
          total_cost_usd: 0.123456,
          agents: [
            {
              id: "a",
              model: "claude-opus-4-7",
              model_priced: true,
              input_tokens_est: 10,
              output_tokens_max: 1000,
              input_cost_usd: 0,
              output_cost_usd: 0.075,
              total_cost_usd: 0.075,
            },
          ],
        },
      ],
    };
    expect(parseCostCatalog(serialiseCostCatalog(cat))).toEqual(cat);
  });

  it("emits cookbooks sorted alphabetically regardless of input order", () => {
    const cat: CookbookCostCatalog = {
      version: "1",
      grand_total_usd: 0,
      cookbooks: [
        {
          slug: "zzz",
          total_cost_usd: 0,
          agents: [],
        },
        {
          slug: "aaa",
          total_cost_usd: 0,
          agents: [],
        },
      ],
    };
    const out = serialiseCostCatalog(cat);
    expect(out.indexOf('"aaa"')).toBeLessThan(out.indexOf('"zzz"'));
  });
});

describe("parseCostCatalog — validation", () => {
  it("rejects JSON without a cookbooks array", () => {
    expect(() => parseCostCatalog("{}")).toThrow(/cookbooks/);
  });
});

// ---------------------------------------------------------------------------
// Pricing-table invariants
// ---------------------------------------------------------------------------

describe("MODEL_PRICING invariants", () => {
  it("DEFAULT_MODEL_ID is in MODEL_PRICING", () => {
    expect(MODEL_PRICING[DEFAULT_MODEL_ID]).toBeDefined();
  });

  it("every priced model has positive input + output rates", () => {
    for (const [model, p] of Object.entries(MODEL_PRICING)) {
      expect(p.input_per_mtok, `${model} input rate`).toBeGreaterThan(0);
      expect(p.output_per_mtok, `${model} output rate`).toBeGreaterThan(0);
    }
  });

  it("output rate is at least input rate (Anthropic invariant)", () => {
    for (const [model, p] of Object.entries(MODEL_PRICING)) {
      expect(
        p.output_per_mtok,
        `${model} output rate should be >= input rate`,
      ).toBeGreaterThanOrEqual(p.input_per_mtok);
    }
  });
});
