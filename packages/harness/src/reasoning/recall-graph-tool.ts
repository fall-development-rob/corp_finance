/**
 * Phase 34 Wave 4 — `recall_by_graph` virtual tool.
 *
 * Mirrors `recall-tool.ts` exactly in shape: the agent loop appends this
 * tool when a `ReasoningBank` is configured, intercepts the call instead of
 * routing through MCP, and feeds back a formatted entry list.
 *
 * Where `recall_similar` is for prose-similar prior work (k-NN over prompt
 * embeddings), `recall_by_graph` is for EXACT-criteria queries over the
 * reasoning bank's structured metadata: jurisdiction, instrument, agent_id,
 * which specialists ran, which tools fired, when. The chief composes the
 * two surfaces — graph-filter to narrow, prose-rank within.
 *
 * The implementation under the hood uses a JSONL scan sidecar maintained
 * alongside the RuVector store (see `rv-index.ts` and ADR-040). When
 * ruvector publishes a Cypher / hyperedge graph surface upstream, this tool
 * spec and `ReasoningBank.recallByGraph` are stable — we swap the
 * implementation without touching the chief skill or the wire schema.
 */
import type { CanonicalTool, ToolCall, ToolResult } from "../types.js";
import {
  GRAPH_RECALL_DEFAULT_LIMIT,
  GRAPH_RECALL_MAX_LIMIT,
  type GraphRecallQuery,
  type ReasoningBank,
  type ReasoningEntry,
} from "./bank.js";

export const RECALL_GRAPH_TOOL_NAME = "recall_by_graph";

/** Predicate: is this tool call a `recall_by_graph` virtual call? */
export function isRecallGraphToolName(name: string): boolean {
  return name === RECALL_GRAPH_TOOL_NAME;
}

/**
 * Build the single virtual `recall_by_graph` tool spec. The description is
 * verbose on purpose so the model sees enough usage policy to pick this
 * over `recall_similar` when the query is exact-match shaped.
 */
export function createRecallGraphTool(): CanonicalTool {
  return {
    name: RECALL_GRAPH_TOOL_NAME,
    description: [
      "Retrieve past dispatches by structured metadata query, NOT by semantic similarity.",
      "Use this when you need EXACT matches on jurisdiction / instrument / agent_id / tool usage / delegation patterns.",
      "For prose-similar past work use `recall_similar` instead.",
      `Returns up to \`limit\` (default ${GRAPH_RECALL_DEFAULT_LIMIT}, max ${GRAPH_RECALL_MAX_LIMIT}) entries sorted by timestamp descending.`,
      "At least one filter is recommended; an unfiltered query returns the most recent entries globally.",
      "Combines well with recall_similar: graph-filter first to narrow the corpus, then prose-rank within that subset.",
    ].join(" "),
    input_schema: {
      type: "object",
      properties: {
        agent_id: {
          type: "string",
          description:
            "Restrict to entries produced by this agent id (e.g. 'chief-analyst', 'derivatives-analyst').",
        },
        metadata: {
          type: "object",
          description:
            "Structured metadata equality filter — entries must match every supplied key/value pair.",
          additionalProperties: true,
        },
        hasTools: {
          type: "array",
          items: { type: "string" },
          description:
            "Match entries that invoked at least one of these tool names (e.g. ['option_pricer', 'wacc_calculator']).",
        },
        hasDelegations: {
          type: "array",
          items: { type: "string" },
          description:
            "Match entries that delegated to at least one of these specialist ids (e.g. ['derivatives-analyst']).",
        },
        since: {
          type: "string",
          description:
            "ISO 8601 datetime; only return entries with `timestamp >= since`.",
        },
        until: {
          type: "string",
          description:
            "ISO 8601 datetime; only return entries with `timestamp <= until`.",
        },
        limit: {
          type: "number",
          description: `Maximum entries to return. Default ${GRAPH_RECALL_DEFAULT_LIMIT}, hard cap ${GRAPH_RECALL_MAX_LIMIT}.`,
          default: GRAPH_RECALL_DEFAULT_LIMIT,
        },
      },
    },
  };
}

/**
 * Format a list of `ReasoningEntry` into a plaintext block. Empty list
 * returns the explicit "none found" sentinel so the model can branch
 * cleanly. Mirrors `formatRecallResult` from recall-tool.ts but with a
 * graph-flavored header.
 */
export function formatRecallGraphResult(entries: ReasoningEntry[]): string {
  if (entries.length === 0) {
    return "No matching prior dispatches found.";
  }
  const lines: string[] = [
    `Matched ${entries.length} prior dispatch${entries.length === 1 ? "" : "es"}:`,
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

/**
 * Parse the loose tool input into a typed `GraphRecallQuery`. Unlike
 * `recall_similar`, EVERY field is optional — an unfiltered call returns
 * the most recent `limit` entries. Throws on shape errors only.
 */
function parseRecallGraphInput(
  input: Record<string, unknown>,
): GraphRecallQuery {
  const out: GraphRecallQuery = {};

  if (input["agent_id"] !== undefined) {
    if (typeof input["agent_id"] !== "string") {
      throw new Error("`agent_id` must be a string");
    }
    out.agent_id = input["agent_id"];
  }

  if (input["metadata"] !== undefined && input["metadata"] !== null) {
    const m = input["metadata"];
    if (typeof m !== "object" || Array.isArray(m)) {
      throw new Error("`metadata` must be an object");
    }
    out.metadata = m as Record<string, unknown>;
  }

  if (input["hasTools"] !== undefined) {
    const v = input["hasTools"];
    if (!Array.isArray(v) || !v.every((x) => typeof x === "string")) {
      throw new Error("`hasTools` must be an array of strings");
    }
    out.hasTools = v as string[];
  }

  if (input["hasDelegations"] !== undefined) {
    const v = input["hasDelegations"];
    if (!Array.isArray(v) || !v.every((x) => typeof x === "string")) {
      throw new Error("`hasDelegations` must be an array of strings");
    }
    out.hasDelegations = v as string[];
  }

  if (input["since"] !== undefined) {
    if (typeof input["since"] !== "string") {
      throw new Error("`since` must be an ISO 8601 string");
    }
    const d = new Date(input["since"]);
    if (Number.isNaN(d.getTime())) {
      throw new Error("`since` must be a valid ISO 8601 datetime");
    }
    out.since = d;
  }

  if (input["until"] !== undefined) {
    if (typeof input["until"] !== "string") {
      throw new Error("`until` must be an ISO 8601 string");
    }
    const d = new Date(input["until"]);
    if (Number.isNaN(d.getTime())) {
      throw new Error("`until` must be a valid ISO 8601 datetime");
    }
    out.until = d;
  }

  if (input["limit"] !== undefined) {
    const raw = input["limit"];
    const n = typeof raw === "number" ? raw : Number(raw);
    if (!Number.isFinite(n) || n <= 0) {
      throw new Error("`limit` must be a positive number");
    }
    out.limit = Math.min(
      GRAPH_RECALL_MAX_LIMIT,
      Math.max(1, Math.floor(n)),
    );
  }

  return out;
}

/**
 * Execute a `recall_by_graph` tool call against a `ReasoningBank`. Returns
 * a fully-formed `ToolResult`. Errors are captured into `is_error: true`
 * results — never thrown — to preserve the dispatch hot path.
 */
export async function executeRecallGraphCall(
  call: ToolCall,
  bank: ReasoningBank,
): Promise<ToolResult> {
  let parsed: GraphRecallQuery;
  try {
    parsed = parseRecallGraphInput(call.input);
  } catch (err) {
    return {
      call_id: call.id,
      content: `recall_by_graph failed: ${err instanceof Error ? err.message : String(err)}`,
      is_error: true,
    };
  }
  try {
    const entries = await bank.recallByGraph(parsed);
    return {
      call_id: call.id,
      content: formatRecallGraphResult(entries),
      is_error: false,
    };
  } catch (err) {
    return {
      call_id: call.id,
      content: `recall_by_graph failed: ${err instanceof Error ? err.message : String(err)}`,
      is_error: true,
    };
  }
}
