/**
 * REC-4 Wave 2 — Build handoff allowlist and resolver from an AgentDef.
 *
 * Derives the HandoffOrchestrator's allowlist and resolver function from the
 * `callableAgents` field on an AgentDef (populated by the YAML manifest loader
 * from `callable_agents:` — the same field that Managed Agents API deployment
 * uses). This wires local-dev handoff semantics to the production contract.
 *
 * Per-target payload schemas are deferred to Phase 38 W3 — allowlist entries
 * are created in permissive mode (no schema) unless a future caller supplies
 * explicit overrides.
 */
import type { AgentDef } from "../types.js";
import type { HandoffAllowlistEntry } from "./types.js";

/**
 * Build a HandoffAllowlistEntry array from the agent's `callableAgents` list.
 * Returns an empty array when the agent declares no callable agents.
 *
 * Phase 38 W3 TODO: pull per-target payload_schema from each target's
 * `outputSchema` or a dedicated `input_schema` field.
 */
export function buildAllowlistFromAgent(
  agent: AgentDef,
): HandoffAllowlistEntry[] {
  if (!agent.callableAgents || agent.callableAgents.length === 0) return [];
  return [
    {
      source: agent.id,
      targets: agent.callableAgents.map((a) => a.id),
      // payload_schema: deferred to Phase 38 W3
    },
  ];
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
