/**
 * Tool schema helpers — filtering and allowlist management.
 */
import type { CanonicalTool } from "../types.js";

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
