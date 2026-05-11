/**
 * Indexer — Phase 34 Wave 2.
 *
 * Bridges the audit chain into the Reasoning Bank. Every successful dispatch
 * with both `audit` and `reasoning` configured produces an `AuditRecord` (via
 * `audit.write`) and, fire-and-forget, a `ReasoningEntry` derived from the
 * same record + the original prompt + the dispatch's `finalText`.
 *
 * The agent loop calls `indexAuditRecord` after `audit.write` returns and
 * awaits nothing — failures emit a `reasoning_index_failed` DispatchEvent and
 * never propagate to the caller. Semantic recall is best-effort.
 *
 * Design notes:
 *   - `prompt_summary` and `result_excerpt` are bounded (200 / 500 chars) so
 *     the bank stays compact even for long prompts.
 *   - `tool_calls` aggregates real tool invocations only; delegations are
 *     surfaced separately under `delegations` to keep the chief's routing
 *     pattern queryable independently of which tools each specialist used.
 *   - `queryText` defaults to `summarizePrompt(prompt)` so a recall against
 *     the same prompt rounds-trip to the same vector neighbourhood.
 */
import type { AuditRecord } from "../types.js";
import type { ReasoningBank, ReasoningEntry } from "./bank.js";

export interface IndexAuditRecordOptions {
  bank: ReasoningBank;
  record: AuditRecord;
  prompt: string;
  finalText: string;
  metadata?: Record<string, unknown>;
  /**
   * Phase 41 Wave 2 — validation result from the Phase 39 output_schema
   * validator. When provided, `validation_failed` and `validation_errors` are
   * written into the indexed `ReasoningEntry.metadata` so the outlier-detector
   * can query `recallByGraph({ metadata: { validation_failed: true } })`.
   *
   * Backward-compat: absent ⟹ treated as `validation_failed: false` by
   * detectors (they use `=== true` which is falsy on undefined).
   */
  validationResult?: {
    ok: boolean;
    errors?: Array<{ path: string; message: string }>;
  };
}

const DELEGATE_PREFIX = "delegate_to_";
const DEFAULT_PROMPT_SUMMARY_CHARS = 200;
const DEFAULT_RESULT_EXCERPT_CHARS = 500;

/**
 * Truncate `text` at the last sentence-ending boundary (`.`, `!`, `?`, or
 * newline) before `maxChars`. If no boundary exists, hard-truncate at
 * `maxChars` and append `...`. Inputs already ≤ maxChars pass through
 * untouched.
 */
function truncateAtSentenceBoundary(text: string, maxChars: number): string {
  if (text.length <= maxChars) return text;
  const slice = text.slice(0, maxChars);
  // Find the last sentence-ending boundary in the slice.
  let cut = -1;
  for (let i = slice.length - 1; i >= 0; i--) {
    const ch = slice[i];
    if (ch === "." || ch === "!" || ch === "?" || ch === "\n") {
      cut = i + 1; // include the boundary char
      break;
    }
  }
  if (cut > 0) return slice.slice(0, cut);
  // No boundary found — hard truncate with ellipsis suffix.
  return `${slice}...`;
}

/**
 * ≤ 200-char human-readable prompt summary at sentence boundary. Used as both
 * the persisted `prompt_summary` and the default embedding `queryText`.
 */
export function summarizePrompt(
  prompt: string,
  maxChars: number = DEFAULT_PROMPT_SUMMARY_CHARS,
): string {
  return truncateAtSentenceBoundary(prompt, maxChars);
}

/**
 * ≤ 500-char excerpt of the dispatch's final assistant text.
 */
export function summarizeResult(
  finalText: string,
  maxChars: number = DEFAULT_RESULT_EXCERPT_CHARS,
): string {
  return truncateAtSentenceBoundary(finalText, maxChars);
}

/**
 * Aggregates real tool invocations by name with counts. Skips
 * `delegate_to_*` entries — those are surfaced under `delegations` so the
 * chief's routing pattern is queryable independently of the specialists'
 * tool usage.
 *
 * Sort order: count descending, then name ascending — stable, deterministic
 * regardless of insertion order in `record.tool_calls`.
 */
export function aggregateToolCalls(
  record: AuditRecord,
): { name: string; count: number }[] {
  const counts = new Map<string, number>();
  for (const call of record.tool_calls) {
    if (call.name.startsWith(DELEGATE_PREFIX)) continue;
    counts.set(call.name, (counts.get(call.name) ?? 0) + 1);
  }
  const out: { name: string; count: number }[] = [];
  for (const [name, count] of counts) {
    out.push({ name, count });
  }
  out.sort((a, b) => {
    if (b.count !== a.count) return b.count - a.count;
    return a.name.localeCompare(b.name);
  });
  return out;
}

/**
 * Extract the unique specialist target slugs from `delegate_to_*` tool
 * calls. Returned alphabetically — order is deterministic across runs.
 */
export function extractDelegations(record: AuditRecord): string[] {
  const targets = new Set<string>();
  for (const call of record.tool_calls) {
    if (!call.name.startsWith(DELEGATE_PREFIX)) continue;
    const slug = call.name.slice(DELEGATE_PREFIX.length);
    if (slug.length > 0) targets.add(slug);
  }
  return [...targets].sort((a, b) => a.localeCompare(b));
}

/**
 * Build a `ReasoningEntry` (sans embedding) from an `AuditRecord` plus the
 * original prompt and the dispatch's final text, then call `bank.index()`.
 *
 * Pure side-effect on the bank. Returns `void` on success; rejects on
 * failure. The agent-loop fire-and-forgets this — it never awaits the
 * promise inline.
 */
export async function indexAuditRecord(
  opts: IndexAuditRecordOptions,
): Promise<void> {
  const { bank, record, prompt, finalText, metadata, validationResult } = opts;
  const promptSummary = summarizePrompt(prompt);

  // Phase 41 W2 — merge validation_failed / validation_errors into metadata.
  const validationMeta: Record<string, unknown> =
    validationResult !== undefined
      ? {
          validation_failed: !validationResult.ok,
          validation_errors: validationResult.ok
            ? undefined
            : (validationResult.errors ?? [])
                .map((e) => `${e.path}: ${e.message}`)
                .slice(0, 10),
        }
      : {};

  const entry: Omit<ReasoningEntry, "embedding"> = {
    audit_id: record.audit_id,
    agent_id: record.agent_id,
    prompt_hash: record.prompt_hash,
    prompt_summary: promptSummary,
    tool_calls: aggregateToolCalls(record),
    delegations: extractDelegations(record),
    result_excerpt: summarizeResult(finalText),
    metadata: { ...(metadata ?? {}), ...validationMeta },
    timestamp: record.timestamp,
  };
  // Pass `promptSummary` explicitly as queryText so the bank embeds the same
  // text we persist as `prompt_summary`. A subsequent `recallSimilar(prompt)`
  // will summarize identically and round-trip to the same vector.
  await bank.index(entry, promptSummary);
}
