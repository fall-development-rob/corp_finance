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

/**
 * Phase 38 W4: Build a per-source allowlist resolver from a registry of agents.
 *
 * Returns a function `(sourceSlug) => HandoffAllowlistEntry[] | undefined`.
 * Each source slug maps to the allowlist derived from ITS own `callableAgents`.
 * Returns `undefined` when the source slug isn't in the registry, allowing the
 * orchestrator to fall back to the top-level allowlist.
 *
 * This mirrors the Managed Agents API `callable_agents:` scoping: agent A can
 * only hand off to agents listed in A's own `callable_agents`, not arbitrarily.
 */
export function buildAllowlistResolver(
  agents: AgentDef[],
): (sourceSlug: string) => HandoffAllowlistEntry[] | undefined {
  const byId = new Map<string, AgentDef>();
  for (const a of agents) byId.set(a.id, a);
  return (slug) => {
    const a = byId.get(slug);
    if (!a) return undefined;
    return buildAllowlistFromAgent(a);
  };
}
