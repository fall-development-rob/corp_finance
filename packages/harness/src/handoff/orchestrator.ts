/**
 * Cross-agent handoff orchestrator — REC-4 Wave 1.
 *
 * Implements the HandoffOrchestrator interface defined in types.ts, mirroring
 * the design of upstream's scripts/orchestrate.py reference implementation.
 *
 * Event ordering for a successful chain:
 *   handoff_request → handoff_executed → handoff_completed
 *
 * Event ordering for a rejected request (any gate):
 *   handoff_request → handoff_rejected
 *
 * Rejection gates (in order):
 *   1. Payload null/undefined check
 *   2. Depth cap (>= maxDepth)
 *   3. Allowlist check (source→target pair)
 *   4. Agent resolution (resolveAgent returns undefined)
 *   5. Payload schema validation (validatePayload)
 *   6. Dispatch error (dispatchAgent throws)
 */

import { validateAgainstSchema } from "../manifests/validator.js";
import type {
  HandoffAllowlistEntry,
  HandoffEvent,
  HandoffEventType,
  HandoffOrchestrator,
  HandoffOrchestratorOptions,
  HandoffRequest,
  HandoffResult,
} from "./types.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_MAX_DEPTH = 5;

// ---------------------------------------------------------------------------
// UUID-lite — no external dependency; sufficient for request_id correlation.
// Generates a UUID v4-shaped string: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
// where y is one of [8, 9, a, b].
// ---------------------------------------------------------------------------

function generateRequestId(): string {
  const r = () => Math.floor(Math.random() * 0x10000).toString(16).padStart(4, "0");
  const r8 = () => r() + r();
  const r12 = () => r() + r() + r();
  const r3 = () => Math.floor(Math.random() * 0x1000).toString(16).padStart(3, "0");
  const variant = (8 | Math.floor(Math.random() * 4)).toString(16);
  return `${r8()}-${r()}-4${r3()}-${variant}${r3()}-${r12()}`;
}

// ---------------------------------------------------------------------------
// Prompt builder
// ---------------------------------------------------------------------------

function buildHandoffPrompt(req: HandoffRequest): string {
  const payloadJson = JSON.stringify(req.payload, null, 2);
  const lines: string[] = [
    `Handoff from ${req.source_agent} via cross-agent event loop.`,
    "",
    "## Payload (JSON, schema-validated)",
    "",
    "```json",
    payloadJson,
    "```",
  ];

  if (req.context && Object.keys(req.context).length > 0) {
    lines.push("", "## Context", "", JSON.stringify(req.context, null, 2));
  }

  return lines.join("\n");
}

// ---------------------------------------------------------------------------
// Allowlist helpers
// ---------------------------------------------------------------------------

/** Returns the first allowlist entry that names this source, or undefined. */
function findSourceEntries(
  allowlist: HandoffAllowlistEntry[],
  source: string,
): HandoffAllowlistEntry[] {
  return allowlist.filter((e) => e.source === source);
}

/**
 * Returns the schema to use for `target` across ALL allowlist entries.
 * Precedence per entry: targetSchemas[target] > payload_schema > absent.
 * Uses the first entry that names the target and has any schema.
 */
function findPayloadSchema(
  allowlist: HandoffAllowlistEntry[],
  target: string,
): Record<string, unknown> | undefined {
  for (const entry of allowlist) {
    if (!entry.targets.includes(target)) continue;
    const perTarget = entry.targetSchemas?.[target];
    if (perTarget) return perTarget;
    if (entry.payload_schema) return entry.payload_schema;
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

export function createHandoffOrchestrator(
  opts: HandoffOrchestratorOptions,
): HandoffOrchestrator {
  const { allowlist, resolveAllowlist, resolveAgent, dispatchAgent, onEvent } = opts;
  const maxDepth = opts.maxDepth ?? DEFAULT_MAX_DEPTH;

  function emit(event: HandoffEvent): void {
    onEvent?.(event);
  }

  function makeEvent(
    type: HandoffEventType,
    req: HandoffRequest,
    requestId: string,
    extra?: Partial<HandoffEvent>,
  ): HandoffEvent {
    return {
      type,
      source_agent: req.source_agent,
      target_agent: req.target_agent,
      request_id: requestId,
      payload: req.payload,
      timestamp: new Date().toISOString(),
      ...extra,
    };
  }

  /**
   * Resolve the effective allowlist for a given source agent.
   * Per-source resolver takes priority; falls back to the top-level allowlist.
   */
  function effectiveAllowlist(source: string): HandoffAllowlistEntry[] {
    const perSource = resolveAllowlist?.(source);
    return perSource ?? allowlist;
  }

  function isAllowed(source: string, target: string): boolean {
    return effectiveAllowlist(source).some(
      (e) => e.source === source && e.targets.includes(target),
    );
  }

  function validatePayload(
    target: string,
    payload: unknown,
    sourceSlug?: string,
  ): { ok: boolean; errors?: string[] } {
    const list = sourceSlug !== undefined ? effectiveAllowlist(sourceSlug) : allowlist;
    const schema = findPayloadSchema(list, target);
    if (!schema) return { ok: true };

    const result = validateAgainstSchema(payload, schema);
    if (result.ok) return { ok: true };

    return {
      ok: false,
      errors: result.errors.map((e) =>
        e.path ? `${e.path}: ${e.message}` : e.message,
      ),
    };
  }

  async function execute(
    req: HandoffRequest,
    currentDepth = 0,
  ): Promise<HandoffResult> {
    const requestId = generateRequestId();
    const startMs = Date.now();

    function reject(reason: string): HandoffResult {
      emit(
        makeEvent("handoff_rejected", req, requestId, { reason, payload: req.payload }),
      );
      return {
        ok: false,
        request_id: requestId,
        source_agent: req.source_agent,
        target_agent: req.target_agent,
        reason,
        duration_ms: Date.now() - startMs,
      };
    }

    // Always emit handoff_request first (mirrors orchestrate.py line 1 of loop)
    emit(makeEvent("handoff_request", req, requestId));

    // Gate 1: payload required
    if (req.payload === null || req.payload === undefined) {
      return reject("payload required");
    }

    // Gate 2: depth cap — currentDepth is the Nth hop; reject when >= maxDepth
    if (currentDepth >= maxDepth) {
      return reject(
        `max chain depth (${maxDepth}) exceeded for source '${req.source_agent}'`,
      );
    }

    // Gate 3: per-source allowlist check
    const sourceEntries = findSourceEntries(effectiveAllowlist(req.source_agent), req.source_agent);
    if (sourceEntries.length === 0) {
      return reject("source agent has no handoff permissions");
    }
    if (!isAllowed(req.source_agent, req.target_agent)) {
      return reject(
        `target "${req.target_agent}" not allowed for source "${req.source_agent}"`,
      );
    }

    // Gate 4: resolve target agent
    const targetAgent = await resolveAgent(req.target_agent);
    if (!targetAgent) {
      return reject(`target agent "${req.target_agent}" not found in registry`);
    }

    // Gate 5: validate payload using per-source effective allowlist
    const validation = validatePayload(req.target_agent, req.payload, req.source_agent);
    if (!validation.ok) {
      return reject((validation.errors ?? []).join("; "));
    }

    // All gates passed — build prompt and dispatch.
    // Pass parentDepth = currentDepth + 1 so the callback can thread depth
    // through nested HandoffRequests (W5 agent-loop integration).
    const prompt = buildHandoffPrompt(req);
    emit(makeEvent("handoff_executed", req, requestId, { payload: req.payload }));

    try {
      const { finalText, auditId } = await dispatchAgent(
        targetAgent,
        prompt,
        currentDepth + 1,
      );
      const result = { finalText, auditId };

      emit(makeEvent("handoff_completed", req, requestId, { result, payload: undefined }));

      return {
        ok: true,
        request_id: requestId,
        source_agent: req.source_agent,
        target_agent: req.target_agent,
        result,
        duration_ms: Date.now() - startMs,
      };
    } catch (err) {
      const reason = err instanceof Error ? err.message : String(err);
      return reject(reason);
    }
  }

  // Public validatePayload exposes the two-arg variant from HandoffOrchestrator interface
  function publicValidatePayload(
    target: string,
    payload: unknown,
  ): { ok: boolean; errors?: string[] } {
    return validatePayload(target, payload);
  }

  // W6 Gap 1: expose resolveAgent so agent-loop can look up target outputSchema
  function publicResolveAgent(slug: string) {
    return resolveAgent(slug);
  }

  return { execute, isAllowed, validatePayload: publicValidatePayload, resolveAgent: publicResolveAgent };
}
