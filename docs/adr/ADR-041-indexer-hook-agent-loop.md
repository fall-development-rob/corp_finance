# ADR-041: Indexer Hook in the Agent Loop

## Status: Accepted

## Date: 2026-05-10

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`harness`, `agent-loop`, `reasoning-bank`, `indexer`, `fire-and-forget`,
`audit-chain`, `hot-path`

## Context

Phase 34 Wave 2 introduced the `indexAuditRecord` indexer that converts
an `AuditRecord` (plus the original prompt and the dispatch's `finalText`)
into a `ReasoningEntry` and persists it via `ReasoningBank.index`. The
question Wave 2 had to answer: where does the agent loop call this?

Constraints:

- The reasoning bank must NOT block the dispatch hot path. A failure to
  embed or persist an entry must not break the user-visible dispatch.
- The bank must NEVER mutate the audit chain. The audit record is the
  immutable source of truth; the reasoning bank is a derived,
  regenerable cache.
- Indexing must observe the SAME data the audit chain saw — same
  `audit_id`, same hashed prompt, same hashed result. Otherwise the two
  surfaces drift and `recall_by_graph` queries can't be cross-referenced
  with audit forensics.
- Index failures must be observable. Silent dropped entries would
  corrode the chief's trust in `recall_similar` over time.

## Decision

### 1. Fire-and-forget AFTER `audit.write()`

Inside `dispatch()` in `agent-loop.ts`, the indexer is called AFTER the
audit record is written. The promise is `void`-prefixed — the agent loop
does not await it:

```typescript
if (options.reasoning && auditRecord && auditId) {
  void indexAuditRecord({
    bank: options.reasoning,
    record: auditRecord,
    prompt,
    finalText,
  }).catch((err) => {
    emit(onEvent, {
      type: "reasoning_index_failed",
      reason: err instanceof Error ? err.message : String(err),
      audit_id: auditId,
    });
  });
}
```

The dispatch return path resolves with the same shape regardless of
whether the index succeeds, fails, or is still in flight.

### 2. Hoist `auditRecord` out of the audit-write block

The audit-record object is allocated once in the dispatch's success path
and reused — both `audit.write()` and `indexAuditRecord()` receive the
SAME `AuditRecord` instance. This guarantees the two writes describe the
same dispatch (same `audit_id`, same `prompt_hash`, same
`tool_calls`, same `child_audit_ids`).

### 3. Failures emit a typed `DispatchEvent`, not a thrown exception

Index failures fire a `reasoning_index_failed` event through `onEvent`
(carrying `reason` and `audit_id`). Consumers (CLI, structured-log
collector, observability hook) can subscribe and react. The dispatch's
return value is unaffected.

### 4. Wave 3 + Wave 4 virtual tools follow the same audit pattern

When the chief calls `recall_similar` or `recall_by_graph` mid-dispatch,
those are recorded as ordinary entries in `auditToolCalls` with the same
`input_hash` / `result_hash` discipline as MCP tool calls. Reasoning
recall is a first-class auditable action — the chief can never "secretly"
look at priors.

## Consequences

### Positive

- Reasoning index is best-effort: dispatch latency unaffected.
- An indexing failure does not break the dispatch — users see results
  on time, the failure surfaces through `onEvent` for ops to investigate.
- Audit and bank describe the SAME dispatch (shared `AuditRecord`
  instance + audit_id).
- Consumers of `DispatchEvent` get structured visibility into bank
  health without polling the bank.
- `recall_*` virtual-tool calls are themselves audit entries.

### Negative

- Eventual consistency between audit chain and reasoning bank. The
  audit chain is the durable record; the bank may lose entries on
  process crash between `audit.write()` returning and the index
  promise resolving. This is acceptable because:
  - The audit chain is the source of truth.
  - The bank is regenerable from the audit chain.
  - The future `cfa-harness --rebuild-reasoning` CLI command is the
    operator-facing recovery path.
- No retry mechanism in Wave 2/3/4. A transient embedding-API failure
  loses one entry per failure. This is intentional for the initial
  implementation; if the failure rate climbs, retries are the right
  next step (with idempotency guaranteed by `audit_id`).
- Indexer test surface must mock the bank to assert the
  `reasoning_index_failed` path without flaking on real native
  bindings.

### Neutral

- The indexer hook only fires when BOTH `options.audit` and
  `options.reasoning` are configured — by design. Reasoning without
  audit would lose the canonical `audit_id` back-reference; audit
  without reasoning is the Phase 31 default.
- Specialists at depth 1 do NOT receive `reasoning` in their nested
  `dispatch()` options, so the indexer fires only for the top-level
  chief dispatch. Specialists are entered via delegation tool calls,
  which the chief's audit record records as tool_calls; the chief's
  reasoning entry summarizes the entire delegated arc.

## Alternatives Considered

**Synchronous index inside `dispatch()`** — Rejected. Couples failure
modes: an embedding-API outage would break dispatches.

**Background queue (BullMQ / Redis Streams)** — Rejected. Adds a Redis
dependency for what is intrinsically a single-machine workload at
Phase 34 scale. If multi-machine orchestration becomes a goal, the queue
is the right next step but it is not Phase 34's concern.

**Indexer as a separate process consuming the audit log** — Deferred.
A daemon that tails `<audit-dir>/*.jsonl` and indexes asynchronously
would isolate the bank from the dispatch process entirely. Wave 5+ if
the dispatch process's memory/cpu cost from in-process indexing
becomes material.

**Index BEFORE `audit.write()`** — Rejected. The bank's
`prompt_hash` field is meant to back-reference the audit record;
producing the bank entry first would either duplicate hashing logic or
require a forward-reference that the audit chain doesn't actually have.

## Future work

- `cfa-harness --rebuild-reasoning` CLI command that scans the audit
  directory and re-indexes any audit records the bank is missing.
- Optional retry-with-jitter on index failures (gated by audit_id
  idempotency).
- Out-of-process indexer daemon for multi-host deployments (Wave 5+).
- Bank backpressure signal — if the bank is down for >N seconds, the
  agent loop could surface a one-time warning to the user that
  `recall_*` will return empty results until recovery.

## Links

- Plan: `docs/plans/phase-34-reasoning-bank.md`
- Implementation: `packages/harness/src/core/agent-loop.ts` — search
  for `// Phase 34 Wave 2: fire-and-forget reasoning index`
- Indexer module: `packages/harness/src/reasoning/indexer.ts`
- Companion ADR: ADR-040 (RuVector for the reasoning bank)
- Depends on: ADR-024 (MCP tool-call audit) — the audit-record shape
  the indexer consumes
