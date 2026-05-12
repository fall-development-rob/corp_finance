/**
 * Cookbook cost telemetry — Phase 25 Tier C1.
 *
 * Estimates per-invocation worst-case dollar cost for each cookbook. The
 * estimate sums every agent (parent + all callable subagents) using:
 *
 *   cost_usd = (system_prompt_tokens / 1e6) * model.input_rate_per_mtok
 *            + (max_output_tokens / 1e6) * model.output_rate_per_mtok
 *
 * The estimate is a worst-case ceiling: it assumes every agent uses its
 * full max_tokens budget. Real invocations will usually run cheaper. This
 * is intentional — overestimating cost ceilings is the safer side for
 * budget planning.
 *
 * Tool-call round-trip cost is NOT included here. A subsequent wave can
 * extend the model with an expected_tool_calls × avg_tool_result_tokens
 * term once we have replay-derived telemetry for those values. For now,
 * the model cost dominates total invocation cost on the cookbooks we ship.
 *
 * Pure library — no network, no clock. Two runs against the same
 * LoadedCookbook produce byte-identical estimates. CLI runner lives in
 * `scripts/generate-cookbook-costs.ts`.
 */

import type { LoadedCookbook } from "./cookbook-loader.js";
import type { AgentDef } from "../types.js";

// ---------------------------------------------------------------------------
// Pricing table — canonical Anthropic API rates (USD per 1M tokens)
// ---------------------------------------------------------------------------

export interface ModelPricing {
  /** Input tokens, $ per million. */
  input_per_mtok: number;
  /** Output tokens, $ per million. */
  output_per_mtok: number;
}

/**
 * Source of truth for cost telemetry. Keys are model IDs as they appear in
 * `model:` fields on agent.yaml / subagents/*.yaml. Update this table when
 * Anthropic publishes new rates or new model IDs ship.
 */
export const MODEL_PRICING: Record<string, ModelPricing> = {
  "claude-opus-4-7": { input_per_mtok: 15, output_per_mtok: 75 },
  "claude-sonnet-4-6": { input_per_mtok: 3, output_per_mtok: 15 },
  "claude-haiku-4-5": { input_per_mtok: 1, output_per_mtok: 5 },
  // Long-context variants — same per-token rate as base, surfaced for clarity.
  "claude-opus-4-7[1m]": { input_per_mtok: 15, output_per_mtok: 75 },
  "claude-haiku-4-5-20251001": { input_per_mtok: 1, output_per_mtok: 5 },
};

/** Default model when an agent has no `model:` field. */
export const DEFAULT_MODEL_ID = "claude-sonnet-4-6";

/** Default max_tokens budget when an agent has no max_tokens field. */
export const DEFAULT_MAX_OUTPUT_TOKENS = 4096;

/**
 * Rough char-per-token heuristic for English prose. Anthropic's tokenizer
 * averages ~3.7-4 characters per token; we use 4 as a conservative upper
 * bound on token count (and therefore on cost). Replace with a real
 * tokenizer if precision becomes important.
 */
export const CHARS_PER_TOKEN_HEURISTIC = 4;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface AgentCostBreakdown {
  /** Agent ID. */
  id: string;
  /** Model resolved at estimate time. */
  model: string;
  /** Whether the model was found in MODEL_PRICING (false → defaulted). */
  model_priced: boolean;
  /** Estimated system-prompt input tokens (deterministic via char heuristic). */
  input_tokens_est: number;
  /** Max output tokens (from agent or default). */
  output_tokens_max: number;
  /** Cost of the input tokens, USD, rounded to 6 dp. */
  input_cost_usd: number;
  /** Worst-case output cost (full max_tokens), USD, rounded to 6 dp. */
  output_cost_usd: number;
  /** Sum of input + output cost, USD, rounded to 6 dp. */
  total_cost_usd: number;
}

export interface CookbookCostEstimate {
  slug: string;
  /** Per-agent breakdown: index 0 is the parent, rest are subagents. */
  agents: AgentCostBreakdown[];
  /** Worst-case total: parent + all subagents, USD, rounded to 6 dp. */
  total_cost_usd: number;
}

export interface CookbookCostCatalog {
  version: string;
  /** $-amount across every cookbook (sum of total_cost_usd). */
  grand_total_usd: number;
  /** Estimates per cookbook, sorted alphabetically by slug. */
  cookbooks: CookbookCostEstimate[];
}

// ---------------------------------------------------------------------------
// Estimator (pure functions)
// ---------------------------------------------------------------------------

function round6(n: number): number {
  return Math.round(n * 1_000_000) / 1_000_000;
}

function estimateInputTokens(text: string): number {
  if (!text) return 0;
  return Math.ceil(text.length / CHARS_PER_TOKEN_HEURISTIC);
}

function resolveModelPricing(modelId: string): {
  model: string;
  pricing: ModelPricing;
  found: boolean;
} {
  const direct = MODEL_PRICING[modelId];
  if (direct) return { model: modelId, pricing: direct, found: true };
  const fallback = MODEL_PRICING[DEFAULT_MODEL_ID];
  if (!fallback) {
    throw new Error(
      `cost telemetry: DEFAULT_MODEL_ID '${DEFAULT_MODEL_ID}' missing from MODEL_PRICING`,
    );
  }
  return { model: modelId, pricing: fallback, found: false };
}

export function estimateAgentCost(def: AgentDef): AgentCostBreakdown {
  const modelId = def.model ?? DEFAULT_MODEL_ID;
  const { pricing, found } = resolveModelPricing(modelId);

  const inputTokens = estimateInputTokens(def.systemPrompt ?? "");
  const outputTokens = def.maxTokens ?? DEFAULT_MAX_OUTPUT_TOKENS;

  const inputCost = (inputTokens / 1_000_000) * pricing.input_per_mtok;
  const outputCost = (outputTokens / 1_000_000) * pricing.output_per_mtok;
  const total = inputCost + outputCost;

  return {
    id: def.id,
    model: modelId,
    model_priced: found,
    input_tokens_est: inputTokens,
    output_tokens_max: outputTokens,
    input_cost_usd: round6(inputCost),
    output_cost_usd: round6(outputCost),
    total_cost_usd: round6(total),
  };
}

export function estimateCookbookCost(
  loaded: LoadedCookbook,
): CookbookCostEstimate {
  const parentBreakdown = estimateAgentCost(loaded.parent);
  const subagentBreakdowns = loaded.subagents.map(estimateAgentCost);
  const total =
    parentBreakdown.total_cost_usd +
    subagentBreakdowns.reduce((n, a) => n + a.total_cost_usd, 0);

  return {
    slug: loaded.slug,
    agents: [parentBreakdown, ...subagentBreakdowns],
    total_cost_usd: round6(total),
  };
}

// ---------------------------------------------------------------------------
// Aggregation + serialisation
// ---------------------------------------------------------------------------

export function buildCostCatalog(
  loadedCookbooks: LoadedCookbook[],
): CookbookCostCatalog {
  const estimates = loadedCookbooks
    .map(estimateCookbookCost)
    .sort((a, b) => a.slug.localeCompare(b.slug));
  const grandTotal = estimates.reduce((n, e) => n + e.total_cost_usd, 0);
  return {
    version: "1",
    grand_total_usd: round6(grandTotal),
    cookbooks: estimates,
  };
}

function sortedJsonReplacer(_root: unknown) {
  return (_key: string, value: unknown): unknown => {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      const obj = value as Record<string, unknown>;
      const sorted: Record<string, unknown> = {};
      for (const k of Object.keys(obj).sort()) sorted[k] = obj[k];
      return sorted;
    }
    return value;
  };
}

/**
 * Stable JSON serialisation: 2-space indent, alphabetical key order within
 * each estimate, alphabetical cookbook ordering by slug, trailing newline.
 * Two runs produce byte-identical strings.
 */
export function serialiseCostCatalog(catalog: CookbookCostCatalog): string {
  const ordered: CookbookCostCatalog = {
    version: catalog.version,
    grand_total_usd: catalog.grand_total_usd,
    cookbooks: [...catalog.cookbooks].sort((a, b) =>
      a.slug.localeCompare(b.slug),
    ),
  };
  return JSON.stringify(ordered, sortedJsonReplacer(ordered), 2) + "\n";
}

export function parseCostCatalog(json: string): CookbookCostCatalog {
  const parsed = JSON.parse(json) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    !("cookbooks" in parsed) ||
    !Array.isArray((parsed as { cookbooks: unknown }).cookbooks)
  ) {
    throw new Error("cost catalog JSON missing 'cookbooks' array");
  }
  return parsed as CookbookCostCatalog;
}
