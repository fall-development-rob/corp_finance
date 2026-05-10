/**
 * Tool schema helpers — filtering and allowlist management.
 */
import type { AgentDef, CanonicalTool } from "../types.js";

/**
 * Filters the full tool catalog to the agent's allowlist.
 * - `"*"` returns all tools unchanged.
 * - An explicit string[] returns only tools whose name appears in the list.
 */
export function filterToolsForAgent(
  tools: CanonicalTool[],
  allowlist: string[] | "*",
): CanonicalTool[] {
  if (allowlist === "*") return tools;
  const permitted = new Set(allowlist);
  return tools.filter((t) => permitted.has(t.name));
}

export interface ToolCatalogValidationIssue {
  agent_id: string;
  unknown_tool: string;
}

export interface ToolCatalogValidationResult {
  ok: boolean;
  issues: ToolCatalogValidationIssue[];
}

/**
 * Validates that every bare tool name in every AgentDef.tools allowlist
 * exists in the live tool catalog. Agents with tools: "*" are skipped
 * (they consume the full catalog by definition).
 *
 * Use this at startup, after MCPClient.initialize() and before the first
 * dispatch(), to convert silent runtime tool_uses=0 failures into loud
 * startup errors.
 */
export function validateAllowlists(
  agentDefs: AgentDef[],
  catalog: Set<string>,
): ToolCatalogValidationResult {
  const issues: ToolCatalogValidationIssue[] = [];
  for (const def of agentDefs) {
    if (def.tools === "*") continue;
    for (const tool of def.tools) {
      if (!catalog.has(tool)) {
        issues.push({ agent_id: def.id, unknown_tool: tool });
      }
    }
  }
  issues.sort((a, b) =>
    a.agent_id !== b.agent_id
      ? a.agent_id.localeCompare(b.agent_id)
      : a.unknown_tool.localeCompare(b.unknown_tool),
  );
  return { ok: issues.length === 0, issues };
}

/**
 * Throws an Error if validateAllowlists reports issues. Convenience
 * wrapper for the strict-startup pattern. Error message names every
 * (agent, missing-tool) pair plus a hint to check tool registration.
 */
export function assertAllowlistsValid(
  agentDefs: AgentDef[],
  catalog: Set<string>,
): void {
  const result = validateAllowlists(agentDefs, catalog);
  if (result.ok) return;
  const lines = result.issues
    .map((i) => `  - ${i.agent_id}: unknown tool "${i.unknown_tool}"`)
    .join("\n");
  throw new Error(
    `ToolCatalog validation failed (${result.issues.length} issues):\n${lines}\n` +
      "Hint: ensure all 4 plugin MCP servers are connected and the tool name in the agent's " +
      "`tools` array matches a registered tool's bare name.",
  );
}
