# ADR-036: Audit Chain via Existing corp-finance-core Audit Module

## Status: Accepted

## Date: 2026-05-09

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`audit`, `hash-chain`, `sha2`, `corp-finance-core`, `traceability`, `phase-31`

## Context

The Phase 31 master plan states: `"Auditable — every tool call recorded, every agent invocation hashed, every numerical result traceable to a tool with logged inputs and outputs (the Phase 26 audit chain pattern)."` Auditability is not optional for an institutional CFA analyst platform — regulators, compliance teams, and investment committee members need to trace a specific number (e.g., a per-warrant price of C$0.0208) back to the tool that computed it, the inputs it received, and the model context in which it was invoked.

Phase 26 shipped `corp_finance_core::audit`: a sha2-hashed invocation record system that produces a deterministic hash chain over `(agent_id, prompt_hash, tool_calls[], result_hash)`. This module was designed as a reusable audit primitive for the MCP server layer. It generates audit records compatible with the MCP tool-call audit pattern (ADR-024) and the workflow auditability requirements (ADR-009).

ADR-024 (`docs/adr/ADR-024-mcp-tool-call-audit.md`) established the canonical audit record format: each MCP `tools/call` invocation generates a sha2-hashed audit entry with the tool name, input parameters (hashed), and output (hashed). The audit log is append-only and written alongside the analysis output.

Phase 31 could build new audit infrastructure in `packages/audit/chain.ts`. However, this would duplicate what the corp-finance-core audit module already provides, introduce a second audit record format that could diverge from the Phase 26 format, and add complexity without adding capability.

The harness connects to the four plugin MCP servers via JSON-RPC (ADR-034). The MCP server layer already calls the corp-finance-core audit module for each `tools/call`. If the harness can additionally surface an aggregate audit record (one record per agent invocation, linking the individual MCP tool audit entries), it provides the two-level auditability structure without re-implementing the hash chain.

## Decision

The audit chain for the harness reuses the corp-finance-core `audit` module via MCP. No new audit infrastructure is built in Phase 31. Audit records are produced at two levels:

### Level 1: Per-tool-call audit (existing, via MCP servers)

Each `tools/call` to the four plugin MCP servers already produces an audit entry via the Phase 26 `corp_finance_core::audit` implementation. These entries flow through the MCP server's internal audit chain and are written to the tool's audit log. The harness does not need to implement per-tool-call hashing; it reads these audit entries from the MCP server response metadata (if exposed) or reconstructs them from the logged `tool_name`, `input`, and `output` fields that the harness already captures in the tool router.

### Level 2: Per-agent-invocation audit (new, in packages/audit/)

`packages/audit/chain.ts` implements a thin wrapper that:
1. Assigns a UUID `invocation_id` to each agent loop execution (chief or specialist).
2. Collects every `CanonicalToolCall` and its `CanonicalToolResult` as the loop runs.
3. At `end_turn`, computes a sha2 hash over the ordered sequence of `(tool_name, input_json, output_json)` tuples — matching the Phase 26 hash construction exactly.
4. Writes an `AgentInvocationRecord`:

```typescript
interface AgentInvocationRecord {
  invocation_id: string;     // UUID
  agent_id: string;          // e.g., "chief-analyst", "derivatives-specialist"
  timestamp_utc: string;     // ISO 8601
  prompt_hash: string;       // sha2(user message content)
  tool_calls: ToolCallRecord[];
  result_hash: string;       // sha2(final assistant message content)
  parent_invocation_id?: string; // for specialist delegations
  chain_hash: string;        // sha2(concat of all above fields)
}

interface ToolCallRecord {
  tool_name: string;
  input_hash: string;        // sha2(JSON.stringify(input))
  output_hash: string;       // sha2(JSON.stringify(output))
  duration_ms: number;
}
```

5. Appends the record to the session audit log file (e.g., `out/chaco-memo.audit.json`), a newline-delimited JSON stream.

### sha2 hash construction mirrors corp-finance-core

The sha2 hash uses SHA-256 (matching `corp_finance_core::audit`). In TypeScript:

```typescript
import { createHash } from "node:crypto";

function sha2(input: string): string {
  return createHash("sha256").update(input, "utf8").digest("hex");
}
```

This is the same algorithm used in the Phase 26 Rust audit module, ensuring cross-layer hash compatibility: a consumer can verify the harness-level audit record against the MCP-server-level audit records by re-computing the sha2 hashes over the same inputs.

### Audit log format

The session audit log is a newline-delimited JSON file at `<output-dir>/<analysis-name>.audit.json`. Each line is a JSON-serialised `AgentInvocationRecord`. This format is:
- Streamable: lines can be read incrementally as the session runs.
- Appendable: if the session is interrupted and resumed, new records append to the existing log.
- Parseable by standard tools (`jq`, Python `json.loads` per line).

The Chaco acceptance test asserts that `out/chaco-memo.audit.json` contains at least 1 `AgentInvocationRecord` with `tool_calls.length >= 21` and that all `chain_hash` values are valid SHA-256 hex strings of length 64.

### No new MCP tool for audit writing

The Phase 31 plan mentions `mcp__plugin_cfa-core_cfa-core__surface_audit_compute` as a possible route for audit writes. That tool does not exist in the current cfa-core plugin surface. Rather than add a new tool to the plugin (which would require a Phase 29/30-style MCP registration), the harness writes audit records directly to the filesystem from `packages/audit/chain.ts`. The audit module in `corp-finance-core` is reused conceptually (same hash algorithm, same record structure) but not via an MCP tool call.

## Consequences

### Positive

- The sha2 hash construction is identical to Phase 26 (`corp_finance_core::audit`), so audit records from the harness and from the MCP server layer are cross-verifiable.
- `packages/audit/chain.ts` is ~150 lines (hash function + record builder + file appender). It is not new infrastructure; it is a thin TypeScript wrapper over the established Phase 26 audit design.
- The audit log is written as a side-effect of the dispatch loop with no additional API calls or MCP invocations, keeping latency impact near zero.
- `parent_invocation_id` in the specialist record creates a two-level audit tree: chief invocation → specialist invocations, exactly matching the hierarchical dispatch structure (ADR-035).
- The audit log is a first-class output of the `cfa-harness run` command, surfaced alongside the analysis memo.

### Negative

- The harness audit records capture `input_hash` and `output_hash` (hashes of the tool inputs and outputs), not the raw inputs and outputs. A reviewer who wants to inspect the actual inputs must re-run the analysis or consult the MCP server's own audit log. This is a privacy-preserving design but requires additional tooling for full trace reconstruction.
- If `corp_finance_core::audit` changes its hash construction in a future phase (e.g., adds a version prefix), the TypeScript wrapper in `packages/audit/chain.ts` must be updated in sync. Cross-repo version coordination is required.
- The audit file is written to the local filesystem. In a multi-machine or containerised deployment (Phase 32+), the audit log must be written to shared storage or a centralised audit service. Phase 31 defers this.

### Neutral

- Phase 26's audit module was designed for the MCP server layer. The harness extends the audit discipline to the agent dispatch level without changing the MCP server audit behaviour. The two levels are additive.
- The `AgentInvocationRecord.chain_hash` field makes the audit log tamper-evident (any modification changes the hash), consistent with the Phase 26 design intent.

## Alternatives Considered

**Build new audit infrastructure from scratch** — A new `packages/audit/` implementation designed specifically for the harness dispatch loop. Rejected because the Phase 26 sha2 hash construction is exactly what is needed; starting fresh would either duplicate it identically (wasted effort) or diverge from it (breaking cross-layer verifiability). Re-use is the correct choice.

**Call a new MCP tool `surface_audit_compute` for audit writes** — Adding a new MCP tool to `plugin:cfa-core:cfa-core` for audit record writing would keep all audit logic in Rust. Rejected because: (1) the tool does not exist and adding it requires a Phase 29/30-style registration cycle; (2) the harness audit record format (with `invocation_id`, `parent_invocation_id`, dispatch tree metadata) is a harness-level concern, not a compute-core concern; (3) writing to the filesystem from TypeScript is simpler and more reliable than an MCP round-trip for a write operation.

**Use an external audit service (e.g., Datadog, Splunk)** — Centralised log shipping for production observability. Deferred to Phase 32+. For Phase 31 (single-machine, developer-focused), local file audit is sufficient and avoids external service dependencies.

**No audit in Phase 31, defer to Phase 32** — The master plan lists audit as Wave 4 (optional). Implementing a minimal audit in Wave 1 (the thin `chain.ts` wrapper, ~150 LOC) is low cost relative to the benefit: the acceptance test can assert audit correctness from the start, and the habit of writing auditable code is established before the codebase grows.

## References

- Master plan: `docs/plans/phase-31-harness.md` (Wave 4: audit and memory)
- ADR-009: Workflow Rust auditability (the Phase 26 audit module design)
- ADR-024: MCP tool-call audit (per-tool-call audit format that this ADR's Level 1 aligns with)
- ADR-031: Custom dispatch harness (audit is a cross-cutting concern of the dispatch loop)
- ADR-035: Hierarchical dispatch (the `parent_invocation_id` field reflects the chief → specialist tree)
- ADR-037: Agent registry as code (`agent_id` in the audit record corresponds to the registry key)
- `corp_finance_core::audit` module (Phase 26, in `crates/corp-finance-core/src/audit/`)
- `docs/contracts/feature_harness_audit.yml` (specflow contract for audit correctness)
