/**
 * Cross-agent handoff event loop types — REC-4 Wave 1.
 *
 * Mirrors the design of upstream's scripts/orchestrate.py reference
 * implementation, translated to TypeScript. Provides a security-first
 * allowlist-gated handoff mechanism for agent-to-agent chaining.
 *
 * Security invariants:
 *   - Target agents are HARD-allowlisted; no LLM-discovered routing.
 *   - Payloads are schema-validated before dispatch to prevent injected
 *     handoff blobs crossing the trust boundary.
 *   - Chain depth is capped (default 5) to prevent A→B→A→B... loops.
 *   - Null/undefined payloads are rejected before dispatch.
 */

import type { AgentDef } from "../types.js";

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

export type HandoffEventType =
  | "handoff_request"
  | "handoff_executed"
  | "handoff_rejected"
  | "handoff_completed";

export interface HandoffEvent {
  type: HandoffEventType;
  source_agent: string;
  target_agent: string;
  request_id: string;
  payload?: unknown;
  reason?: string;   // populated on handoff_rejected
  result?: unknown;  // populated on handoff_completed
  timestamp: string; // ISO 8601
}

// ---------------------------------------------------------------------------
// Request and result
// ---------------------------------------------------------------------------

export interface HandoffRequest {
  source_agent: string;
  target_agent: string;    // must be in allowlist for source
  payload: unknown;        // validated against target's payload_schema
  context?: Record<string, unknown>;
}

export interface HandoffResult {
  ok: boolean;
  request_id: string;
  source_agent: string;
  target_agent: string;
  reason?: string;   // when ok: false (rejection or execution error)
  result?: unknown;  // when ok: true (target's structured response)
  duration_ms: number;
}

// ---------------------------------------------------------------------------
// Allowlist
// ---------------------------------------------------------------------------

export interface HandoffAllowlistEntry {
  /** Source agent slug that is permitted to initiate handoffs. */
  source: string;
  /** Permitted target agent slugs for this source. */
  targets: string[];
  /**
   * Optional JSON Schema to validate the handoff payload. When present,
   * the payload must pass before dispatch. When absent, any payload is
   * accepted (permissive mode). Acts as fallback when targetSchemas has
   * no entry for the specific target.
   */
  payload_schema?: Record<string, unknown>;
  /**
   * Phase 38 W3: per-target payload schemas keyed by target slug.
   * Takes precedence over payload_schema for the specific target.
   * Populated by buildAllowlistFromAgent from each callable agent's inputSchema.
   */
  targetSchemas?: Record<string, Record<string, unknown>>;
}

// ---------------------------------------------------------------------------
// Orchestrator options and interface
// ---------------------------------------------------------------------------

/**
 * Dispatch callback signature with depth threading for chained handoffs.
 *
 * `parentDepth` is the orchestrator's `currentDepth + 1` at the point of
 * dispatch — it lets the callback (agent-loop in W5) construct nested
 * HandoffRequest objects with `currentDepth = parentDepth + 1`.
 *
 * Backward-compatible: existing stubs that accept only `(agent, prompt)` still
 * satisfy this type because TS function parameter contravariance allows callers
 * to ignore trailing parameters.
 */
export type DispatchAgentFn = (
  agent: AgentDef,
  prompt: string,
  parentDepth: number,
) => Promise<{ finalText: string; auditId?: string }>;

export interface HandoffOrchestratorOptions {
  /** Top-level allowlist (used when no per-source resolver or resolver returns undefined). */
  allowlist: HandoffAllowlistEntry[];
  /**
   * Optional per-source allowlist resolver. Given a source agent slug, returns
   * the allowlist entries for THAT source (its callable_agents). Falls back to
   * the top-level allowlist when undefined is returned.
   *
   * Enables per-target allowlist scoping: when chief→A is allowed and A has its
   * own callable_agents [B, C], A→B is allowed but A→unrelated is not.
   */
  resolveAllowlist?: (sourceSlug: string) => HandoffAllowlistEntry[] | undefined;
  /**
   * Resolves a target agent slug to its AgentDef. Callers should pass a
   * closure over their registry (e.g., `(slug) => registry.get(slug)`).
   */
  resolveAgent: (slug: string) => Promise<AgentDef | undefined> | AgentDef | undefined;
  /**
   * Dispatches the target agent with the serialised payload as its prompt.
   * Returns the final text and an optional audit id.
   *
   * `parentDepth` is `currentDepth + 1` at dispatch time; pass it through as
   * the depth for any nested HandoffRequest initiated inside the callback (W5).
   */
  dispatchAgent: DispatchAgentFn;
  /** Optional lifecycle event sink for observability (audit logs, telemetry). */
  onEvent?: (e: HandoffEvent) => void;
  /** Maximum chain depth before a handoff is auto-rejected. Default: 5. */
  maxDepth?: number;
}

export interface HandoffOrchestrator {
  /** Execute a single handoff request. `currentDepth` is set by recursive calls. */
  execute(req: HandoffRequest, currentDepth?: number): Promise<HandoffResult>;
  /** Returns true iff the source→target pair appears in the allowlist. */
  isAllowed(source: string, target: string): boolean;
  /**
   * Validates a payload against the target's first-match allowlist schema.
   * When no schema is configured for the target, returns ok: true (permissive).
   */
  validatePayload(target: string, payload: unknown): { ok: boolean; errors?: string[] };
  /**
   * W6 Gap 1: Resolve a target agent slug to its AgentDef.
   * Exposes the orchestrator's internal resolveAgent callback so callers
   * (agent-loop handoff branch) can look up the target's outputSchema
   * and apply output_schema validation on handoff results — parity with
   * the delegation path's parseAndValidate gate.
   */
  resolveAgent(slug: string): Promise<AgentDef | undefined> | AgentDef | undefined;
}

// ---------------------------------------------------------------------------
// Factory (implemented in orchestrator.ts)
// ---------------------------------------------------------------------------

export declare function createHandoffOrchestrator(
  opts: HandoffOrchestratorOptions,
): HandoffOrchestrator;
