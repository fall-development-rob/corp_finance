# ADR-017: Audit Trails, Cost Tracking, Observability, and Security Hardening

## Status: Accepted

## Date: 2026-05-06

## Deciders: RobotixAI Engineering, CFA Platform Lead, Compliance Sponsor, Security Reviewer

## Context

ADR-015 commits to building native orchestration / memory / audit modules inspired by ruflo concepts and pins the runtime to four surfaces (CLI, MCP, skill, plugin). ADR-016 specifies the memory slice. This ADR specifies the four remaining production gaps: audit, cost, observability, and security — all wired at the surface boundaries defined in ADR-015.

| Gap | Today | Required for production |
|-----|-------|-------------------------|
| Per-output audit | None. CLI subcommands and MCP tools produce numbers; the provenance of those numbers is implicit in the prose. | Every output file containing a numeric recommendation must have a companion `.audit.json` that records the surface event manifest, sub-agent set, skill set, and MCP tool set behind it, hashed with djb2 (per ADR-009). The trigger is the plugin Write/Edit hook on file creation, not a separate "deploy" event. |
| Cost tracking | The MCP transport returns token counts, but we do not aggregate per CLI invocation, per MCP tool, per tier, or per analyst. Paid-vendor MCP tools can be called unconstrained. | Aggregated cost per CLI invocation and per MCP tool call, rolled up by `McpServerTier` and per analyst, with budget thresholds and alerts. |
| Observability | Hooks emit lines to stdout; CLI subcommands log inconsistently. | Structured trace span for every `cfa <subcommand>` invocation (~200+ subcommands), every MCP tool handler (~594 tools), and every plugin hook fire. |
| Security | One regex-based PII hook at `plugins/cfa-core/hooks/hooks.json` that matches SSN-shaped strings. No prompt-injection defence. | Multi-category PII detection (14 categories) wired at the MCP wrapper boundary (tool inputs/outputs) and at the plugin Write/Edit hook (file content). Prompt-injection scanning on user input arriving at MCP and on text retrieved from memory. |

ADR-015 commits this layer to native Rust modules: `tracing` + `tracing-subscriber` for spans, `rusqlite` for the cost ledger, and a hand-rolled regex set + injection pattern library for security. The optional `aidefence@2.2.4` npm package is documented as an HTTP-middleware fallback for users who want the upstream Express integration; for our stdio-MCP architecture the native scanner is the production path.

## Decision

Define an Audit / Cost / Observability / Security bounded context owned by `corp-finance-core::observability` (telemetry side) and `corp-finance-core::managed_agent::audit` (audit hashing function library). Five capabilities ship in Phase 26, each anchored at a runtime surface from ADR-015:

1. **Surface-event audit hashing**: every CLI subcommand and every MCP tool registration computes `surface_audit_hash = djb2(manifest || sub_agents || skills || mcp_tools)` over its registered manifest.
2. **`.audit.json` companion file**: emitted by the plugin Write/Edit hook when an output file containing a numeric recommendation is written. Sibling file `<output>.audit.json` carries the manifest hash, run id, model, timestamp, and tool-call ledger.
3. **`cfa cost` subcommand**: native `rusqlite` ledger aggregates token usage from CLI invocations and MCP tool calls and enforces budget thresholds.
4. **Structured traces**: every `cfa <subcommand>` CLI invocation creates a root trace span via `tracing` + `tracing-subscriber`; every MCP `server.tool(...)` registration is wrapped to emit a child span; every plugin hook fire emits a span. OTLP export is available behind an optional `tracing-opentelemetry` feature.
5. **PII / prompt-injection hardening**: replace the SSN-only regex hook at `plugins/cfa-core/hooks/hooks.json` with a native PII + injection scanner covering 14 PII types and a prompt-injection pattern set, wired at the MCP wrapper (tool inputs and outputs) and at the plugin Write/Edit hook (file content).

### 1. Surface-event audit hashing (djb2)

Reuse the algorithm specified in ADR-009 for workflow audit hashing. The hash domain is the surface event's static manifest:

```
input  = canonical_json(manifest)
       || sorted_csv(sub_agent_ids)
       || sorted_csv(skill_ids)
       || sorted_csv(mcp_tool_ids)

hash   = djb2(input)
display = "djb2:0x" + hex(hash, 8)
```

For a CLI subcommand, the manifest is the subcommand definition (name, args, description). For an MCP tool, the manifest is the tool registration (name, schema, description). For a deploy-time cookbook artefact (`managed-agent-cookbooks/<slug>/`), the manifest is the cookbook YAML — but the hash is computed at validate-time inside the `cfa managed-agent validate` CLI subcommand, which is itself a runtime CLI surface event.

Canonical JSON for the manifest means: keys sorted lexicographically, no insignificant whitespace, no trailing commas, UTF-8 encoded. The sorted CSV form for sub-agents / skills / MCP tools means: each id is a stable string (e.g., `cfa-equity-analyst`, `workflow-equity-research`, `dcf_model`), the list is sorted lexicographically, joined with single comma. This makes the hash invariant under cosmetic reordering and stable across surface events of the same content.

The hash is computed by `corp-finance-core::managed_agent::audit::compute_manifest_hash(&Manifest) -> String` and is the value of the `surface_audit_hash` field used in:

- `run_summary.json` (ADR-016)
- `<output>.audit.json` (this ADR, see below)
- Memory index entries (ADR-016)

Identical content always produces the identical hash; any change to the manifest, sub-agent set, skill set, or MCP tool set changes the hash. This is the property compliance relies on for "did the surface event manifest change between this run and that run".

### 2. `.audit.json` companion file (plugin-hook triggered)

For every output file containing a numeric recommendation (price target, IRR, NPV, recommended action, allocation percentage, etc.), the **plugin Write/Edit hook** at `plugins/cfa-core/hooks/hooks.json` fires on the file event and writes a sibling file with the same path plus `.audit.json` extension. Example:

```
output/coverage_report.md
output/coverage_report.md.audit.json
output/dcf_model.csv
output/dcf_model.csv.audit.json
```

This is a plugin-surface concern. The CLI subcommand or MCP tool that produced the output does not emit the `.audit.json` itself; it emits the output file, the plugin hook fires on the Write/Edit lifecycle event, and the hook invokes `surface_audit_compute` MCP tool to assemble and write the companion. This keeps audit emission uniform regardless of which surface produced the output.

Schema:

```json
{
  "schema_version": "1.0",
  "run_id": "uuid-v7",
  "surface": "cli",
  "surface_event_id": "cfa.workflow.audit",
  "surface_audit_hash": "djb2:0x7ab3c910",
  "model": "claude-opus-4-7[1m]",
  "ts": "2026-05-06T14:22:01Z",
  "output_path": "output/coverage_report.md",
  "output_sha256": "1f2c...e9",
  "tool_call_ledger": [
    { "step": 1, "tool": "fmp_quote", "input_hash": "djb2:0x10aa", "output_hash": "djb2:0x55bc" },
    { "step": 2, "tool": "dcf_model", "input_hash": "djb2:0x44e1", "output_hash": "djb2:0x9082" }
  ],
  "skills_in_scope": ["workflow-equity-research", "corp-finance-tools-core"],
  "sub_agents_in_scope": ["cfa-equity-analyst"]
}
```

Required fields: `schema_version`, `run_id`, `surface`, `surface_event_id`, `surface_audit_hash`, `model`, `ts`, `output_path`, `output_sha256`. The `tool_call_ledger` is recommended; it captures the deterministic sequence of MCP tool calls behind the output and is the basis for replay-style audit. The ledger is assembled by the MCP wrapper (every `server.tool` invocation appends to a per-run buffer) and consumed by the plugin hook at file-write time.

What counts as "contains a numeric recommendation" is enforced by the plugin hook's emit policy (configurable per output-path glob). The default for unspecified files is "audit not required". A future tightening (Phase 27) may flip the default the other way.

### 3. `cfa cost` subcommand

A new CLI subcommand backed by a native `rusqlite` ledger at `<repo>/var/observability/cost-ledger.sqlite`. Surface:

- `cfa cost summary [--since <date>] [--by surface|tool|tier|analyst]`
- `cfa cost budget get`
- `cfa cost budget set --tool <id> --monthly-limit <usd>`
- `cfa cost alerts`

Token counts are recorded into the SQLite ledger by event handlers subscribed at two surface points:

- **CLI binary** at `crates/corp-finance-cli/src/main.rs` records token usage on subcommand completion, attributing cost to the CLI subcommand name.
- **MCP wrapper** at `packages/*-mcp-server/src/` records token usage on tool completion, attributing cost to the MCP tool name and to `McpServerTier` (the server tier from `crates/corp-finance-core/src/mcp_servers/types.rs`).

Cookbook deploys via `cfa managed-agent deploy` are CLI invocations and inherit the CLI cost-recording path automatically; we record the deploy-time cost (assembling the payload, validating it, posting to Anthropic). We do not record costs incurred by the deployed managed agent in Anthropic's infrastructure — those are billed by Anthropic to the user directly and are outside our runtime.

Cost-tier classification comes from the existing `CookbookTier` (in `crates/corp-finance-core/src/managed_agent/types.rs`, used at deploy-time for cookbook tier classification) and `McpServerTier` (in `crates/corp-finance-core/src/mcp_servers/types.rs`, used at runtime for MCP cost classification) — both shipped in Phase 25; no new tier type or `cost_tier` field is introduced. The combination produces dollarised totals per tier.

Budget thresholds default to:

- `McpServerTier::Free`: no limit, no alert.
- `McpServerTier::Freemium`: warn at 80% of monthly tier ceiling, hard stop at 100%.
- `McpServerTier::PaidVendor`: warn at 50%, 80%, 95% of analyst-set monthly cap; hard stop at 100%.

A budget threshold crossing emits the `budget_threshold_crossed` integration event. Alerts route through the configured tracing subscriber (and optionally an OTLP exporter). The "fire within 1s" success metric in the PRD is the budget event latency from `record_usage` call to event emission, measured in-process.

### 4. Structured traces

Every `cfa <subcommand>` CLI invocation opens a root span via `tracing` + `tracing-subscriber` at the CLI binary entry point in `crates/corp-finance-cli/src/main.rs`. Span attributes:

- `cfa.cli.subcommand` (e.g., `workflow.audit`, `managed-agent.deploy`, `memory.find`)
- `cfa.run.id` (uuid-v7 generated at root)
- `cfa.user` (from environment, redacted to first-initial-last-name)

Every MCP `server.tool(...)` registration is wrapped at `packages/*-mcp-server/src/` to open a child span on each tool invocation:

- `cfa.mcp.tool` (e.g., `dcf_model`, `fmp_quote`)
- `cfa.mcp.server` (e.g., `cfa-core`, `fmp-mcp-server`)
- `cfa.mcp.tier` (`McpServerTier` value)
- inherits `cfa.run.id` from the CLI/LLM context if available

Every plugin hook fire emits a span with attribute `cfa.plugin.hook` (e.g., `pre_tool_use`, `post_tool_use`, `write`, `edit`).

Span export format is OTel-compatible. The default subscriber emits to stdout in dev; production deploys can enable the `tracing-opentelemetry` feature to ship spans to any OTLP-compatible backend (Datadog, Honeycomb, OTel collector). Backend choice is deployment configuration.

### 5. PII / prompt-injection hardening

The current hook at `plugins/cfa-core/hooks/hooks.json` matches one regex (SSN). It is replaced by a native scanner at `corp_finance_core::observability::security_scan` that runs at three points across the runtime surfaces:

| Hook point | Surface | Direction | Action on detection |
|-----------|---------|-----------|---------------------|
| MCP wrapper, before tool handler | MCP | Inbound (tool input) | Block on prompt injection; redact on PII; emit `pii_detected` event with category |
| MCP wrapper, after tool handler | MCP | Outbound (tool output) | Redact PII before returning to caller; emit `pii_detected` |
| Plugin Write/Edit hook | Plugin | Outbound (file content) | Scan file content before persistence; redact for files written into memory paths (`run_summary.json`, `.cfa-session`); emit `pii_detected` for analyst review on final-output bundles |

We define and own a hand-rolled set of 14 PII categories (inspired by aidefence's category list): names+address tuples, email addresses, phone numbers, postal addresses, social security numbers, passport numbers, driver's licence numbers, bank account numbers, credit card numbers, IBAN, IP addresses, MAC addresses, dates of birth, government IDs / EINs. The 14-count is load-bearing for our own implementation. The CFA-specific allowlist (CUSIP, ISIN, SEDOL, FIGI, LEI, ticker symbols) is consulted before adding to PII findings to suppress false positives.

The injection pattern library covers `instruction_override`, `role_swap`, `context_truncation`, `encoded_payload`, `delimiter_collision`, and `tool_chain_hijack`. Prompt-injection patterns are scanned on inbound MCP tool input and on text retrieved from memory (hybrid retriever output). On detection of prompt injection in retrieved memory, the entry is excluded from the prompt and a `prompt_injection_blocked` event is emitted.

Users who prefer the upstream `aidefence@2.2.4` npm package (Express HTTP middleware) can mount it in front of an HTTP MCP transport; our default stdio-MCP transport uses the native scanner. The optional npm fallback is documented in the deployment guide but is not required.

The migration from the old hook is:

1. Implement `corp_finance_core::observability::security_scan` with the 14-category regex set, injection pattern set, and CFA allowlist.
2. Add a `surface_pii_scan` MCP tool to the `cfa-core` server that exposes the scanner over MCP.
3. Wire the MCP wrapper at `packages/*-mcp-server/src/` to invoke the native scanner directly (in-process) on every tool input and output.
4. Update `plugins/cfa-core/hooks/hooks.json` to invoke the native scanner via the `surface_pii_scan` MCP tool at the Write/Edit hook points instead of the inline SSN regex.
5. Run the regression test set (RUF-SEC-001, see `docs/contracts/feature_audit_observability.yml`) to confirm coverage of all 14 PII categories.
6. Remove the old SSN-only regex once regression passes.

## Rationale

### Why djb2 specifically?

ADR-009 already standardised on djb2 for workflow audit hashing. Reusing the same algorithm avoids two hash schemes inside one platform and makes cross-referencing a workflow audit hash with a surface-event audit hash trivial. djb2 is not cryptographic, but the audit hash is not a security primitive — it is an identity primitive ("are these the same content?"). The accompanying `output_sha256` field handles the integrity primitive.

### Why a sibling `.audit.json` rather than embedding audit data in the output?

Many outputs are markdown or CSV consumed by humans or downstream tools. Embedding JSON audit blocks in those formats creates parsing friction. A sibling file is invariant under format and lets the audit hash chain be machine-readable while leaving the output untouched.

### Why trigger `.audit.json` at the plugin Write/Edit hook rather than inside the CLI/MCP wrapper?

The plugin Write/Edit hook fires on every file write regardless of which surface produced it (CLI subcommand, MCP tool, LLM-driven Edit, etc.). Anchoring audit emission there gives uniform coverage with one integration point. The CLI/MCP wrapper still maintains the in-memory `tool_call_ledger` so the hook has the data it needs, but the file emission is the plugin's responsibility.

### Why three hook points for the native scanner?

Defence in depth. MCP wrapper input catches injection at ingress to any tool. MCP wrapper output catches PII before it leaves the tool boundary. Plugin Write/Edit catches PII at the last moment before persistence and at file-creation by any actor (CLI, MCP, or LLM Edit).

### Why not block on PII at every point?

PII may be intentional in finance output (an investor name in a wealth management plan, for example). Hard-blocking would prevent legitimate work. Redaction at memory boundaries plus alerting at output boundaries gives compliance the visibility they need without breaking workflow.

## Consequences

### Positive

- Every output that drives a decision has a verifiable provenance chain, regardless of which of the four surfaces produced it.
- Paid-vendor MCP tool costs become predictable and bounded.
- Production failures are diagnosable from structured spans rooted at CLI invocation, with child spans across MCP tool calls and plugin hook fires.
- PII coverage moves from 1 category to 14, applied at MCP input/output and at plugin Write/Edit.
- Reusing djb2 means the audit hash format is uniform across workflows (ADR-009) and surface-event manifests (this ADR).

### Negative

- Each output that ships an audit-required file gains a sibling `.audit.json` — roughly doubles the file count for those outputs.
- A SQLite ledger at `<repo>/var/observability/cost-ledger.sqlite` is added; backups must include it.
- Structured tracing adds ~5-15ms per CLI invocation overhead from span open/close, plus per-tool MCP overhead from the wrapper.
- The native PII regression set must be maintained as the team adds new PII categories or tunes false-positive suppression.
- Wrapping every MCP `server.tool(...)` registration is fan-out work across four MCP server packages; the wrapper is a shared helper but each server must adopt it.

### Risks

- **Audit hash collision**: djb2 is 32-bit; collisions are theoretically possible. Mitigated by `output_sha256` for content integrity. The audit hash collides only if two surface-event manifests have identical canonical content, which is the desired behaviour.
- **Cost-ledger drift**: if our `rusqlite` totals and Anthropic's billing diverge (rate-card refresh, cache-discount changes), the dashboard becomes a guide rather than ground truth. Mitigated by a quarterly reconciliation job (out of scope for Phase 26).
- **Trace cardinality**: span attributes including `cfa.user` and `cfa.mcp.tool` create high cardinality at scale. Mitigated by the redaction policy on user attributes and by configurable sampling on the OTLP exporter.
- **PII allowlist gaps**: a CFA-specific identifier we forget to allowlist (e.g., a new vendor's instrument ID) triggers false positives. Mitigated by an explicit review of the allowlist as part of every new vendor MCP integration.

## Implementation Notes

- Module: `corp-finance-core::observability` (new in Phase 26) and `corp-finance-core::managed_agent::audit` (extended).
- Native crate dependencies (added to `crates/corp-finance-core/Cargo.toml`):
  - `tracing = "0.1"`, `tracing-subscriber = "0.3"` — span emission
  - `tracing-opentelemetry = "0.x"` (optional, behind `otlp_export` feature) — OTLP exporter
  - `rusqlite = { version = "0.x", features = ["bundled"] }` — cost ledger persistence
  - `regex = "1"` — PII / injection pattern matching
- Surface-level instrumentation points:
  - **CLI binary** (`crates/corp-finance-cli/src/main.rs`): root tracing span, cost record on subcommand completion.
  - **MCP wrapper** (shared helper used by `packages/mcp-server/`, `packages/fmp-mcp-server/`, `packages/data-mcp-server/`, `packages/vendor-mcp-server/`): per-tool child span, per-tool cost record, native PII scan on input and output, append to in-memory `tool_call_ledger` for the active run.
  - **Plugin hook** (`plugins/cfa-core/hooks/hooks.json`): `.audit.json` emit on Write/Edit of output files matching audit-policy globs; native PII scan on file content; span on hook fire.
- CLI subcommands (new):
  - `cfa cost summary [--since <date>] [--by surface|tool|tier|analyst]`
  - `cfa cost budget get`
  - `cfa cost budget set --tool <id> --monthly-limit <usd>`
  - `cfa cost alerts`
  - `cfa audit show --run-id <id>`
- MCP tools (added to `cfa-core` MCP server):
  - `surface_audit_compute`
  - `surface_audit_show`
  - `surface_cost_summary`
  - `surface_cost_budget_set`
  - `surface_cost_budget_get`
  - `surface_pii_scan`
- Hook update: `plugins/cfa-core/hooks/hooks.json` rewritten to invoke `surface_pii_scan` and `surface_audit_compute` MCP tools at the Write/Edit hook points instead of the inline SSN regex. (The hooks file already exists at `plugins/cfa-core/hooks/hooks.json`.)
- Tier types referenced: `CookbookTier` (`crates/corp-finance-core/src/managed_agent/types.rs`, deploy-time tier classification) and `McpServerTier` (`crates/corp-finance-core/src/mcp_servers/types.rs`, runtime MCP cost tier); both shipped in Phase 25.
- Domain events emitted: `cli_invocation_started`, `cli_invocation_completed`, `mcp_tool_called`, `audit_failure`, `budget_threshold_crossed`, `pii_detected`, `prompt_injection_blocked`.
- Contract IDs: RUF-AUD-001..N, RUF-COST-001..N, RUF-OBS-001..N, RUF-SEC-001..N (see `docs/contracts/feature_audit_observability.yml`).
- DDD bounded context: `docs/ddd/domain-audit-observability.md`.
- CI workflow: `.github/workflows/phase26-checks.yml` runs the contract test set on every PR; the native PII regression suite is part of that workflow.

## Related Decisions

- **ADR-015** — Native Orchestration / Memory / Audit Layer Inspired by Ruflo Concepts (umbrella decision; pins the four runtime surfaces; this ADR is the audit / cost / observability / security slice).
- **ADR-016** — Memory Architecture (the `run_summary.json` referenced here is defined there; the pre-memory-write PII redaction is the integration point).
- **ADR-009** — Workflow Auditability (djb2 algorithm and rationale; reused here for surface-event manifest audit hashing).
- **ADR-008** — Financial Services Workflow Integration (workflows are the inner layer; skills compose them; audit hashes nest at the surface event boundary).

## References

- Concept inspiration (not runtime dependencies in default builds): https://github.com/ruvnet/ruflo
- Optional npm fallback: `aidefence@2.2.4` (Express middleware; currently N/A for stdio-MCP)
- `tracing`, `tracing-subscriber`, `rusqlite`, `regex` — crate documentation on docs.rs
- ADR-009: `docs/adr/ADR-009-workflow-rust-auditability.md`
- Deploy-time managed-agent code (used by the `cfa managed-agent` CLI surface): `crates/corp-finance-core/src/managed_agent/`
- Existing hook configuration: `plugins/cfa-core/hooks/hooks.json` (confirmed present)
- `CookbookTier`: `crates/corp-finance-core/src/managed_agent/types.rs`
- `McpServerTier`: `crates/corp-finance-core/src/mcp_servers/types.rs`
- 17 CFR 240.17a-4: SEC books-and-records retention rules.
