/**
 * Delegation mechanism for chief→specialist dispatch (Phase 31 Wave 2).
 *
 * The agent loop injects one virtual `delegate_to_<id>` tool per specialist
 * in `DispatchOptions.delegates`. When the model invokes a delegation tool,
 * the loop runs a nested dispatch on the specialist agent with the
 * `sub_prompt` as the input. The specialist's `finalText` (plus a brief
 * usage summary) is returned to the chief as the tool result.
 *
 * Recursion is bounded by `agent.maxRecursionDepth`. Phase 31 caps at
 * depth 1: chief (depth 0) can delegate to specialists (depth 1);
 * specialists at depth 1 cannot delegate further (their delegates array
 * is dropped at depth 1).
 */

import type { AgentDef, CanonicalTool } from "../types.js";

const DELEGATE_PREFIX = "delegate_to_";

/**
 * Build a virtual tool spec for delegating to one specialist agent.
 * The tool's name is `delegate_to_<agent-id-with-hyphens-replaced>`.
 *
 * Tool ids must be valid Anthropic tool names (regex `^[a-zA-Z0-9_-]{1,64}$`).
 * We replace hyphens with underscores in the suffix to keep Anthropic happy
 * with combined `delegate_to_credit_analyst` (not `delegate_to_credit-analyst`).
 */
export function delegationToolName(agentId: string): string {
  const suffix = agentId.replace(/-/g, "_");
  return `${DELEGATE_PREFIX}${suffix}`;
}

export function isDelegationToolName(name: string): boolean {
  return name.startsWith(DELEGATE_PREFIX);
}

/**
 * Map a delegation tool name back to the specialist agent id.
 * Returns undefined if the name doesn't match any specialist in `delegates`.
 */
export function resolveDelegationTarget(
  toolName: string,
  delegates: AgentDef[],
): AgentDef | undefined {
  if (!isDelegationToolName(toolName)) return undefined;
  return delegates.find((d) => delegationToolName(d.id) === toolName);
}

/**
 * Build virtual `delegate_to_<id>` CanonicalTool entries to inject into
 * the chief's tool list. Each tool's description embeds the specialist's
 * own description so the model knows when to delegate.
 */
export function createDelegationTools(delegates: AgentDef[]): CanonicalTool[] {
  return delegates.map((spec) => ({
    name: delegationToolName(spec.id),
    description: [
      `Delegate a sub-task to the ${spec.id} specialist agent. ${spec.description}`,
      "",
      "Use this tool when the sub-task requires depth in this specialist's domain.",
      "Provide a self-contained `sub_prompt` describing what the specialist should produce — the specialist will not see the parent conversation.",
      "Optionally include `context` with key facts (figures, identifiers) the specialist needs.",
      "The specialist will execute its own tool calls and return a structured analysis.",
    ].join("\n"),
    input_schema: {
      type: "object",
      properties: {
        sub_prompt: {
          type: "string",
          description:
            "Self-contained prompt for the specialist. Must include all context the specialist needs — they cannot see the parent conversation.",
        },
        context: {
          type: "object",
          description:
            "Optional structured context (numbers, identifiers, prior findings) the specialist should anchor on. Pass-through to the specialist as JSON in the prompt body.",
          additionalProperties: true,
        },
      },
      required: ["sub_prompt"],
    },
  }));
}

/**
 * Format the input received by a delegation tool into the prompt the
 * specialist will dispatch on. Stable shape so the specialist can rely on it.
 */
export function buildSubPrompt(input: Record<string, unknown>): string {
  const subPrompt = typeof input["sub_prompt"] === "string" ? input["sub_prompt"] : "";
  const ctx = input["context"];
  if (ctx && typeof ctx === "object" && Object.keys(ctx as object).length > 0) {
    return [
      subPrompt,
      "",
      "## Context (structured pass-through from chief)",
      "",
      "```json",
      JSON.stringify(ctx, null, 2),
      "```",
    ].join("\n");
  }
  return subPrompt;
}

/**
 * Format the specialist's dispatch result into the tool_result content
 * returned to the chief. Includes the final text plus a brief usage summary
 * so the chief can audit the work.
 */
export function formatDelegationResult(
  specialistId: string,
  result: { finalText: string; toolUses: number },
): string {
  return [
    `# ${specialistId} — completed`,
    `(tool_uses: ${result.toolUses})`,
    "",
    result.finalText,
  ].join("\n");
}
