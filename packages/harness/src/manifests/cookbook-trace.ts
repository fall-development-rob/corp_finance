/**
 * Cookbook synthetic-trace evaluation — Phase 25 Tier C4.
 *
 * Captures the full assembled deploy payload per cookbook at a fixed
 * point in time: parent + subagent definitions with the actual assembled
 * system prompt text (not just a hash), tool allowlists, block lists,
 * schemas, plus the steering events that drive the cookbook and the
 * version + audit_hash of the source.
 *
 * Complements Tier A2 audit and Tier A3 replay:
 *   - audit   → file-byte inventory (does the source match what was reviewed?)
 *   - replay  → projection fingerprint  (does the loader output match what was reviewed?)
 *   - trace   → final assembled payload (what would the deploy actually send?)
 *
 * The trace is the artifact you read in a PR when you need to know
 * "did the actual prompt that gets sent to Claude change, and how?".
 *
 * Two runs against the same LoadedCookbook + auditHash produce
 * byte-identical traces.
 */

import { createHash } from "node:crypto";

import type { LoadedCookbook } from "./cookbook-loader.js";
import type { AgentDef } from "../types.js";

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

export interface AgentTrace {
  id: string;
  model?: string;
  max_tokens?: number;
  /** Sorted bare tool names; ["*"] when allowlist is unrestricted. */
  tools: string[];
  /** Sorted block_tools entries (empty array when none declared). */
  block_tools: string[];
  /** The fully assembled system prompt — skills + system.file + system.text + system.append. */
  system_prompt: string;
  /** Byte length of system_prompt (convenience, redundant with text). */
  system_prompt_bytes: number;
  /** sha256 hex of the system_prompt — for quick equality checks. */
  system_prompt_sha256: string;
  /** Canonical JSON of the output_schema, or "" if absent. */
  output_schema_json: string;
  /** Canonical JSON of the input_schema, or "" if absent. */
  input_schema_json: string;
}

export interface CookbookTrace {
  slug: string;
  version: string;
  /** Hash of the cookbook source from data/cookbook-audits.json. */
  audit_hash: string;
  /** Event strings from steering-examples.json (descriptions stripped). */
  example_events: string[];
  parent: AgentTrace & { subagent_ids: string[] };
  subagents: AgentTrace[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function sha256Hex(s: string): string {
  return createHash("sha256").update(s, "utf8").digest("hex");
}

function canonicalJson(value: unknown): string {
  return JSON.stringify(value, sortedReplacer(), 2);
}

function sortedReplacer(): (key: string, value: unknown) => unknown {
  return (_key, value) => {
    if (value && typeof value === "object" && !Array.isArray(value)) {
      const obj = value as Record<string, unknown>;
      const sorted: Record<string, unknown> = {};
      for (const k of Object.keys(obj).sort()) sorted[k] = obj[k];
      return sorted;
    }
    return value;
  };
}

function traceAgent(def: AgentDef): AgentTrace {
  const tools: string[] =
    def.tools === "*"
      ? ["*"]
      : [...def.tools].sort((a, b) => a.localeCompare(b));
  const block_tools = def.blockTools
    ? [...def.blockTools].sort((a, b) => a.localeCompare(b))
    : [];
  const systemPrompt = def.systemPrompt ?? "";
  return {
    id: def.id,
    ...(def.model !== undefined ? { model: def.model } : {}),
    ...(def.maxTokens !== undefined ? { max_tokens: def.maxTokens } : {}),
    tools,
    block_tools,
    system_prompt: systemPrompt,
    system_prompt_bytes: Buffer.byteLength(systemPrompt, "utf8"),
    system_prompt_sha256: sha256Hex(systemPrompt),
    output_schema_json: def.outputSchema
      ? canonicalJson(def.outputSchema)
      : "",
    input_schema_json: def.inputSchema ? canonicalJson(def.inputSchema) : "",
  };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export interface BuildTraceInput {
  loaded: LoadedCookbook;
  /** Source-of-truth audit hash for this cookbook (from cookbook-audits.json). */
  auditHash: string;
  /** Cookbook version from the parent manifest. */
  version: string;
  /** Steering example events (already extracted; descriptions stripped). */
  exampleEvents: string[];
}

/**
 * Build a fully assembled synthetic trace for a single cookbook. The
 * output is deterministic — two calls with identical inputs produce
 * byte-identical traces.
 */
export function buildSyntheticTrace(input: BuildTraceInput): CookbookTrace {
  const parentTrace = traceAgent(input.loaded.parent);
  return {
    slug: input.loaded.slug,
    version: input.version,
    audit_hash: input.auditHash,
    example_events: [...input.exampleEvents],
    parent: {
      ...parentTrace,
      subagent_ids: input.loaded.subagents.map((s) => s.id),
    },
    subagents: input.loaded.subagents.map(traceAgent),
  };
}

/**
 * Stable JSON serialisation: 2-space indent, alphabetical key order
 * within every nested object, trailing newline. Two runs produce
 * byte-identical strings.
 */
export function serialiseTrace(trace: CookbookTrace): string {
  return JSON.stringify(trace, sortedReplacer(), 2) + "\n";
}

export function parseTrace(json: string): CookbookTrace {
  const parsed = JSON.parse(json) as unknown;
  if (
    typeof parsed !== "object" ||
    parsed === null ||
    typeof (parsed as { slug?: unknown }).slug !== "string" ||
    typeof (parsed as { version?: unknown }).version !== "string" ||
    !("parent" in parsed) ||
    !("subagents" in parsed) ||
    !Array.isArray((parsed as { subagents: unknown }).subagents)
  ) {
    throw new Error("trace JSON missing required fields slug/version/parent/subagents");
  }
  return parsed as CookbookTrace;
}

// ---------------------------------------------------------------------------
// Steering-example helpers (used by the CLI)
// ---------------------------------------------------------------------------

/**
 * Extract just the `event` strings from a parsed steering-examples.json
 * array. Returns an empty array on shape mismatch — the cookbook-loader
 * already validates the broader shape; trace generation must not crash
 * on malformed examples.
 */
export function extractSteeringEvents(parsed: unknown): string[] {
  if (!Array.isArray(parsed)) return [];
  const out: string[] = [];
  for (const entry of parsed) {
    if (
      entry &&
      typeof entry === "object" &&
      typeof (entry as { event?: unknown }).event === "string"
    ) {
      out.push((entry as { event: string }).event);
    }
  }
  return out;
}
