# ADR-024: MCP Tool-Call Audit Middleware (Phase 29 Wave 10)

## Status: Accepted

## Date: 2026-05-08

## Deciders

- CFA Agent platform engineering
- MCP tool surface owner
- Compliance / audit owner
- Security and privacy owner

## Tags

audit, mcp, middleware, observability, privacy, jsonl, typescript

## Context

Phase 29 Wave 10 lands invocation-level audit coverage for all MCP tools. Prior to this wave, the platform has 90+ MCP tools registered across mcp-server with no call-level audit trail. The existing Rust-side audit (ADR-017, `corp-finance-core::audit`) records workflow-compute hashing at execution time — it is a compute-layer primitive, not an invocation-level record. There is no mechanism today to answer: which tools were called, by whom, with what frequency, and whether they succeeded.

Three production needs motivate Wave 10:

1. **Cost analysis** — tool-call counts and latencies are the primary input for usage-based cost modelling as the platform moves toward production deployment.
2. **Reproducibility and abuse detection** — `input_sha256` stability across reruns enables detection of repeated identical calls (replay attacks, runaway agents) and provides a stable key for deduplication.
3. **Observability** — operators need a structured, queryable audit log to investigate incidents, verify tool call outcomes, and satisfy compliance obligations without exposing raw user inputs or trade-sensitive data.

Privacy is a first-class constraint: MCP tool inputs and outputs may contain PII, proprietary data, or trade-sensitive parameters. The audit record must never persist raw payloads — only their SHA-256 hashes.

## Decision

### Single Insertion Point

A wrapper `withAudit(server: McpServer): McpServer` is introduced at `packages/mcp-server/src/middleware/audit.ts`. It is applied once in `packages/mcp-server/src/index.ts` when constructing the server. Every tool registered via `server.tool()` is automatically intercepted; no per-tool changes are required. There is one rollback point: remove the `withAudit()` call in `index.ts`.

### Audit Record Schema

Each invocation emits one JSONL record with the following fields:

- `call_id` — UUID v4, unique per invocation
- `timestamp` — ISO 8601 UTC string
- `tool` — tool name as registered with `server.tool()`
- `input_sha256` — SHA-256 hex digest (64 characters) of the serialized input params
- `latency_ms` — wall-clock duration from handler entry to handler return
- `ok` — boolean; true if the handler resolved without throwing
- `output_sha256` — SHA-256 hex digest of the serialized output (present on success only)
- `error` — error message string (present on failure only)

Raw input params and raw output values are never written to the audit log.

### Log Destination

Default log path: `~/.cfa-agent/audit/mcp-{YYYY-MM-DD}.jsonl` (daily rotation by date). Override via environment variable `CFA_AUDIT_LOG_PATH`. The file is created on first write; the parent directory must exist or be creatable. Append-only; records are never modified after write.

### Append Strategy (v1)

Synchronous append (Node.js `fs.appendFileSync`) for v1 simplicity and atomicity on single-process deployments. The synchronous write occurs after the tool handler returns, so it does not extend the handler's critical path in terms of business logic — latency overhead is the file I/O only. If hot-path overhead becomes measurable in production profiling, async buffered append is the natural v2 evolution and will be addressed in a future ADR.

### Audit Failure Isolation

A failure during the audit append (disk full, permissions denied, path unwritable) is caught by the middleware, logged to stderr, and does not propagate to the caller. The tool call's success or failure outcome is fully independent of audit infrastructure health. This invariant is tested explicitly (AUDIT-INV-004).

### Relationship to Rust-Side Audit

The Wave 10 middleware is additive and orthogonal to `corp-finance-core::audit`. The Rust layer records workflow-compute hashing at the time of workflow execution (a compute primitive). The TypeScript middleware records invocation metadata at the MCP boundary (a call primitive). Neither replaces the other. Together they provide a two-layer audit trail: call-level at the MCP boundary and computation-level at the workflow engine.

## Consequences

### Positive

- Every MCP tool receives audit coverage automatically. No per-tool instrumentation is required; new tools registered in future waves are covered immediately.
- One rollback point: removing `withAudit()` from `index.ts` disables all audit in a single line.
- Uniform audit schema across all 90+ tools enables cross-tool analytics (call volume, error rates, latency distribution) without schema differences.
- `input_sha256` is byte-stable for identical params (AUDIT-INV-003), enabling replay detection, deduplication, and reproducibility assertions without storing raw inputs.
- Privacy-by-default: hash-only storage eliminates the risk of PII or trade-sensitive data appearing in audit logs. No redaction logic is required because no raw payload is ever written.

### Negative

- Synchronous file write on every tool call adds measurable (but expected to be small) latency overhead. On a hot path with high-frequency tool calls this may be perceptible; async buffered append is deferred to v2.
- Daily log rotation by date only. No size-based rotation, compression, or log-shipping in v1. Operators on high-throughput deployments must manage log growth manually or via external tooling (logrotate, cron).
- Raw payloads are not recoverable from the audit log by design. Incident investigations that require actual parameter values must rely on other telemetry (e.g., agent trace, re-run under debug mode). This is acceptable given the privacy invariant.

### Neutral

- The `uuid` and `crypto` (Node built-in) dependencies cover `call_id` generation and SHA-256 hashing respectively; no new heavyweight dependency is introduced.
- `fs.appendFileSync` is a Node.js built-in; no additional file-I/O library is needed.
- The JSONL format is line-delimited, making it trivially parseable by `jq`, `grep`, and standard log-aggregation tools without a custom parser.

## Alternatives Considered

### Option 1: Per-tool decoration via withAudit(handler) at each registerXxxTools call (Rejected)

Applying a per-handler `withAudit()` helper at the 10+ `registerXxxTools` call sites (each covering a subset of tools) was considered. Rejected because it requires touching ~90 call sites today and every new tool registration in future waves — a high-friction pattern where omission is undetectable until audit records are found missing. The single-server-wrapper approach eliminates this class of error structurally.

### Option 2: OpenTelemetry / Prometheus integration (Rejected for v1)

OTel spans and Prometheus counters would provide richer observability (distributed tracing, dashboards, alerting). Rejected for v1 because: OTel adds a non-trivial dependency footprint; the platform does not yet have a collector or backend configured; and the overhead of span creation and export on every tool call exceeds the lightweight JSONL append approach. Deferred to a future ADR if production observability requirements mandate it.

### Option 3: Full payload logging with PII redaction (Rejected)

Logging raw inputs and outputs with a PII-scrubbing pass was considered. Rejected because redaction logic is fragile — it must be updated as tool schemas evolve, and a single missed field leaks PII. Hash-only storage is the provably safe default: the hash reveals nothing about the payload content while still providing a stable identity key.

## Related Decisions

- ADR-017: Audit / Cost / Observability — Wave 10 extends the platform's audit posture from workflow-compute hashing to MCP call-level recording
- ADR-009: Workflow Rust Auditability — Rust-side audit that Wave 10 complements at the MCP boundary layer
- ADR-015: Native Orchestration Umbrella — MCP server is one of the four runtime surfaces; middleware applies at this surface
- ADR-018: Multi-Agent Coordination — audit records provide observability into agent-driven tool invocations in multi-agent scenarios

## References

- `packages/mcp-server/src/middleware/audit.ts` — withAudit() middleware implementation
- `packages/mcp-server/src/index.ts` — single application point of withAudit()
- `corp-finance-core::audit` — Rust-side workflow-compute audit (orthogonal layer)
- Specflow contracts: `docs/contracts/feature_mcp_audit.yml`
