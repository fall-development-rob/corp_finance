/**
 * REC-4 Wave 2 — Build handoff allowlist and resolver from an AgentDef.
 *
 * Derives the HandoffOrchestrator's allowlist and resolver function from the
 * `callableAgents` field on an AgentDef (populated by the YAML manifest loader
 * from `callable_agents:` — the same field that Managed Agents API deployment
 * uses). This wires local-dev handoff semantics to the production contract.
 *
 * Phase 38 W3: per-target payload schemas are now populated from each callable
 * agent's `inputSchema` field, enabling the orchestrator to enforce each
 * target's declared input contract before dispatch.
 */
import type { AgentDef } from "../types.js";
import type { HandoffAllowlistEntry } from "./types.js";

/**
 * Build a HandoffAllowlistEntry array from the agent's `callableAgents` list.
 * Returns an empty array when the agent declares no callable agents.
 *
 * Per-target schemas are keyed by target slug in `targetSchemas`. Callable
 * agents without an `inputSchema` are omitted from targetSchemas (permissive
 * for that specific target). Existing `payload_schema` semantics are preserved
 * for backwards compatibility.
 */
export function buildAllowlistFromAgent(
  agent: AgentDef,
): HandoffAllowlistEntry[] {
  if (!agent.callableAgents || agent.callableAgents.length === 0) return [];
  const entry: HandoffAllowlistEntry = {
    source: agent.id,
    targets: agent.callableAgents.map((a) => a.id),
  };
  const schemaEntries = agent.callableAgents
    .filter((a) => a.inputSchema !== undefined)
    .map((a) => [a.id, a.inputSchema] as [string, Record<string, unknown>]);
  if (schemaEntries.length > 0) {
    entry.targetSchemas = Object.fromEntries(schemaEntries);
  }
  return [entry];
}

/**
 * Build a resolver function from the agent's `callableAgents` list.
 * The returned function maps a slug to its AgentDef, or undefined if
 * the slug is not in the list.
 */
export function buildResolverFromAgent(
  agent: AgentDef,
): (slug: string) => AgentDef | undefined {
  return (slug) => agent.callableAgents?.find((a) => a.id === slug);
}
