/**
 * Phase 34 Wave 3 — `recall_similar` virtual tool.
 *
 * Mirrors the `delegate_to_<id>` virtual-tool pattern in `agents/delegate.ts`.
 * The agent loop injects this single tool when a `ReasoningBank` is configured
 * on `DispatchOptions.reasoning`. When the model invokes it, the loop calls
 * `bank.recallSimilar(query, { k, filter })` instead of routing through MCP,
 * and returns the formatted entries as a tool_result block.
 *
 * The tool is appended AFTER `filterToolsForAgent`, which means it bypasses
 * the per-agent allowlist. In practice only the chief (which uses `tools: "*"`)
 * receives it, because specialists are dispatched at depth 1 and their inner
 * `dispatch()` call does not pass `reasoning` (the indexer hook only fires for
 * the top-level chief dispatch via the audit + reasoning combo).
 */
import type { CanonicalTool, ToolCall, ToolResult } from "../types.js";
import type { ReasoningBank, ReasoningEntry } from "./bank.js";

export const RECALL_TOOL_NAME = "recall_similar";

/** Predicate: is this tool call a `recall_similar` virtual call? */
export function isRecallToolName(name: string): boolean {
  return name === RECALL_TOOL_NAME;
}

/**
 * Build the single virtual `recall_similar` tool spec. The description is
 * verbose on purpose — the model only sees descriptions, so this is where the
 * usage policy (call BEFORE delegating, ignore <0.7 similarity) lives.
 */
export function createRecallTool(): CanonicalTool {
  return {
    name: RECALL_TOOL_NAME,
    description: [
      "Retrieve up to k semantically-similar past dispatches from the chief's reasoning bank.",
      "Use this BEFORE delegating a new sub-task to discover whether a similar prompt has already been analyzed —",
      "surfaced priors can inform routing decisions and serve as starting points.",
      "Do NOT use it as a substitute for actual analysis; treat returned entries as historical reference.",
      "Ignore entries with similarity < 0.7.",
      "Returns a list of `ReasoningEntry` objects with `audit_id`, `agent_id`, `prompt_summary`, `tool_calls`, `delegations`, `result_excerpt`, `metadata`, `timestamp`.",
    ].join(" "),
    input_schema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description:
            "The natural-language query to embed and search against past prompt summaries. Be specific — e.g. 'Argentine convertible debenture pricing under inflation pass-through' beats 'EM credit'.",
        },
        k: {
          type: "number",
          description: "Top-k results to return. Default 5. Hard upper bound: 20.",
          default: 5,
        },
        filter: {
          type: "object",
          description: "Optional filters narrowing the result set.",
          properties: {
            agent_id: {
              type: "string",
              description:
                "Restrict to entries produced by this agent id (e.g. 'chief-analyst', 'equity-analyst').",
            },
            since: {
              type: "string",
              description:
                "ISO 8601 datetime; only return entries with `timestamp >= since`.",
            },
            metadata: {
              type: "object",
              description:
                "Structured metadata filter — entries must contain every supplied key/value pair (deep equality not required; presence + value match).",
              additionalProperties: true,
            },
          },
          additionalProperties: false,
        },
      },
      required: ["query"],
    },
  };
}

/**
 * Format a list of `ReasoningEntry` into a plaintext block the model can read
 * back as tool_result content. Empty list returns the explicit "none found"
 * sentinel so the model can branch cleanly.
 */
export function formatRecallResult(entries: ReasoningEntry[]): string {
  if (entries.length === 0) {
    return "No similar prior dispatches found.";
  }
  const lines: string[] = [
    `Found ${entries.length} prior dispatch${entries.length === 1 ? "" : "es"}:`,
    "",
  ];
  for (let i = 0; i < entries.length; i++) {
    const e = entries[i]!;
    const toolCallsStr =
      e.tool_calls.length === 0
        ? "(none)"
        : e.tool_calls.map((tc) => `${tc.name}×${tc.count}`).join(", ");
    const delegationsStr =
      e.delegations.length === 0 ? "(none)" : e.delegations.join(", ");
    lines.push(`${i + 1}. audit_id: ${e.audit_id}`);
    lines.push(`   agent_id: ${e.agent_id}`);
    lines.push(`   timestamp: ${e.timestamp}`);
    lines.push(`   prompt_summary: ${e.prompt_summary}`);
    lines.push(`   tool_calls: ${toolCallsStr}`);
    lines.push(`   delegations: ${delegationsStr}`);
    lines.push(`   result_excerpt: ${e.result_excerpt}`);
    if (Object.keys(e.metadata).length > 0) {
      lines.push(`   metadata: ${JSON.stringify(e.metadata)}`);
    }
    lines.push("");
  }
  return lines.join("\n").trimEnd();
}

interface ParsedRecallInput {
  query: string;
  k: number;
  filter?: {
    agent_id?: string;
    since?: Date;
    metadata?: Record<string, unknown>;
  };
}

/**
 * Parse and validate the loose Anthropic-shaped tool input. Throws on any
 * shape we cannot handle — caller wraps in a tool_result with is_error=true.
 */
function parseRecallInput(input: Record<string, unknown>): ParsedRecallInput {
  const rawQuery = input["query"];
  if (typeof rawQuery !== "string" || rawQuery.trim().length === 0) {
    throw new Error("`query` must be a non-empty string");
  }
  const rawK = input["k"];
  let k = 5;
  if (rawK !== undefined) {
    const n = typeof rawK === "number" ? rawK : Number(rawK);
    if (!Number.isFinite(n) || n <= 0) {
      throw new Error("`k` must be a positive number");
    }
    k = Math.min(20, Math.max(1, Math.floor(n)));
  }
  const rawFilter = input["filter"];
  if (rawFilter === undefined || rawFilter === null) {
    return { query: rawQuery, k };
  }
  if (typeof rawFilter !== "object") {
    throw new Error("`filter` must be an object");
  }
  const f = rawFilter as Record<string, unknown>;
  const out: ParsedRecallInput["filter"] = {};
  if (typeof f["agent_id"] === "string") {
    out.agent_id = f["agent_id"];
  } else if (f["agent_id"] !== undefined) {
    throw new Error("`filter.agent_id` must be a string");
  }
  if (typeof f["since"] === "string") {
    const d = new Date(f["since"]);
    if (Number.isNaN(d.getTime())) {
      throw new Error("`filter.since` must be a valid ISO 8601 datetime");
    }
    out.since = d;
  } else if (f["since"] !== undefined) {
    throw new Error("`filter.since` must be an ISO 8601 string");
  }
  const meta = f["metadata"];
  if (meta !== undefined && meta !== null) {
    if (typeof meta !== "object" || Array.isArray(meta)) {
      throw new Error("`filter.metadata` must be an object");
    }
    out.metadata = meta as Record<string, unknown>;
  }
  return { query: rawQuery, k, filter: out };
}

/**
 * Execute a `recall_similar` tool call against a `ReasoningBank`. Returns a
 * fully-formed `ToolResult` ready to be added to the conversation. Errors are
 * captured into `is_error: true` results — never thrown — to preserve the
 * dispatch hot path.
 */
export async function executeRecallCall(
  call: ToolCall,
  bank: ReasoningBank,
): Promise<ToolResult> {
  let parsed: ParsedRecallInput;
  try {
    parsed = parseRecallInput(call.input);
  } catch (err) {
    return {
      call_id: call.id,
      content: `recall_similar failed: ${err instanceof Error ? err.message : String(err)}`,
      is_error: true,
    };
  }
  try {
    const entries = await bank.recallSimilar(parsed.query, {
      k: parsed.k,
      ...(parsed.filter ? { filter: parsed.filter } : {}),
    });
    return {
      call_id: call.id,
      content: formatRecallResult(entries),
      is_error: false,
    };
  } catch (err) {
    return {
      call_id: call.id,
      content: `recall_similar failed: ${err instanceof Error ? err.message : String(err)}`,
      is_error: true,
    };
  }
}
