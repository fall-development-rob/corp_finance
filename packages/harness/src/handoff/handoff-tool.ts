/**
 * REC-4 Wave 2 — `initiate_handoff` virtual tool.
 *
 * Mirrors the `recall_similar` virtual-tool pattern in `reasoning/recall-tool.ts`.
 * The agent loop injects this tool when a `HandoffOrchestrator` is configured
 * on `DispatchOptions.handoff` AND the dispatch is at depth=0 (chief only).
 *
 * The model calls `initiate_handoff(target, payload, context?)` to hand off a
 * sub-task to another agent in its declared `callable_agents` allowlist. The
 * orchestrator validates the allowlist, schema, and depth before dispatching.
 *
 * This mirrors the Managed Agents API `callable_agents:` deployment semantics
 * so local dev catches handoff bugs before they ship to deployment.
 */
import type { CanonicalTool, ToolCall, ToolResult } from "../types.js";
import type { HandoffOrchestrator, HandoffResult } from "./types.js";

export const HANDOFF_TOOL_NAME = "initiate_handoff";

/** Predicate: is this tool call an `initiate_handoff` virtual call? */
export function isHandoffToolName(name: string): boolean {
  return name === HANDOFF_TOOL_NAME;
}

/**
 * Build the single virtual `initiate_handoff` tool spec. The description is
 * verbose on purpose — the model only sees descriptions, so usage policy lives
 * here (e.g., allowlist constraint, sequential-vs-parallel distinction).
 */
export function createHandoffTool(): CanonicalTool {
  return {
    name: HANDOFF_TOOL_NAME,
    description: [
      "Hand off a sub-task to another agent in your declared `callable_agents` allowlist.",
      "The target agent runs as an isolated dispatch with a fresh context.",
      "Use when the sub-task requires a sequential pipeline",
      "(data-reader → analyst → publisher style) rather than the chief's parallel-delegation pattern.",
      "Returns the target's structured response.",
      "Mirrors the Managed Agents API `callable_agents:` deployment semantics.",
    ].join(" "),
    input_schema: {
      type: "object",
      properties: {
        target: {
          type: "string",
          description:
            "Target agent slug (must be in the source agent's callable_agents allowlist)",
        },
        payload: {
          type: "object",
          additionalProperties: true,
          description:
            "Structured payload (validated against target's input schema)",
        },
        context: {
          type: "object",
          additionalProperties: true,
          description: "Optional structured context the target needs",
        },
      },
      required: ["target", "payload"],
    },
  };
}

/**
 * Format a HandoffResult as markdown for the model to read back. Includes
 * request_id and duration_ms for observability, the target slug, and either
 * the result body or the rejection reason.
 */
export function formatHandoffResult(result: HandoffResult): string {
  const header = [
    `# Handoff to \`${result.target_agent}\` — ${result.ok ? "completed" : "rejected"}`,
    `request_id: ${result.request_id}  duration_ms: ${result.duration_ms}`,
    "",
  ].join("\n");

  if (!result.ok) {
    return header + `**Rejection reason:** ${result.reason ?? "(unknown)"}`;
  }

  const body =
    result.result !== undefined
      ? typeof result.result === "string"
        ? result.result
        : JSON.stringify(result.result, null, 2)
      : "(no result body)";

  return header + body;
}

interface ParsedHandoffInput {
  target: string;
  payload: Record<string, unknown>;
  context?: Record<string, unknown>;
}

/**
 * Parse and validate the loose Anthropic-shaped tool input. Throws on any
 * shape we cannot handle — caller wraps in a tool_result with is_error=true.
 */
function parseHandoffInput(input: Record<string, unknown>): ParsedHandoffInput {
  const rawTarget = input["target"];
  if (typeof rawTarget !== "string" || rawTarget.trim().length === 0) {
    throw new Error("`target` must be a non-empty string");
  }
  const rawPayload = input["payload"];
  if (
    rawPayload === undefined ||
    rawPayload === null ||
    typeof rawPayload !== "object" ||
    Array.isArray(rawPayload)
  ) {
    throw new Error("`payload` must be an object");
  }
  const out: ParsedHandoffInput = {
    target: rawTarget.trim(),
    payload: rawPayload as Record<string, unknown>,
  };
  const rawContext = input["context"];
  if (rawContext !== undefined && rawContext !== null) {
    if (typeof rawContext !== "object" || Array.isArray(rawContext)) {
      throw new Error("`context` must be an object");
    }
    out.context = rawContext as Record<string, unknown>;
  }
  return out;
}

/**
 * Execute an `initiate_handoff` tool call against a `HandoffOrchestrator`.
 * Returns a fully-formed `ToolResult` ready to be added to the conversation.
 * Errors are captured into `is_error: true` results — never thrown — to
 * preserve the dispatch hot path.
 *
 * The `sourceAgent` id is taken from the enclosing dispatch context; the loop
 * passes `agent.id` here so the orchestrator can gate the source→target pair.
 */
export async function executeHandoffCall(
  call: ToolCall,
  orchestrator: HandoffOrchestrator,
  sourceAgent: string,
  currentDepth: number,
): Promise<ToolResult> {
  let parsed: ParsedHandoffInput;
  try {
    parsed = parseHandoffInput(call.input);
  } catch (err) {
    return {
      call_id: call.id,
      content: `initiate_handoff failed: ${err instanceof Error ? err.message : String(err)}`,
      is_error: true,
    };
  }

  const result = await orchestrator.execute(
    {
      source_agent: sourceAgent,
      target_agent: parsed.target,
      payload: parsed.payload,
      ...(parsed.context ? { context: parsed.context } : {}),
    },
    currentDepth,
  );

  return {
    call_id: call.id,
    content: formatHandoffResult(result),
    is_error: !result.ok,
  };
}
