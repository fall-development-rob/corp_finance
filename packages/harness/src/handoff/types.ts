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

export interface HandoffOrchestratorOptions {
  allowlist: HandoffAllowlistEntry[];
  /**
   * Resolves a target agent slug to its AgentDef. Callers should pass a
   * closure over their registry (e.g., `(slug) => registry.get(slug)`).
   */
  resolveAgent: (slug: string) => Promise<AgentDef | undefined> | AgentDef | undefined;
  /**
   * Dispatches the target agent with the serialised payload as its prompt.
   * Returns the final text and an optional audit id.
   */
  dispatchAgent: (
    agent: AgentDef,
    prompt: string,
  ) => Promise<{ finalText: string; auditId?: string }>;
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
}

// ---------------------------------------------------------------------------
// Factory (implemented in orchestrator.ts)
// ---------------------------------------------------------------------------

export declare function createHandoffOrchestrator(
  opts: HandoffOrchestratorOptions,
): HandoffOrchestrator;
