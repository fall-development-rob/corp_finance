# Product Requirements Document: Phase 26 — Memory + Audit Hardening

**Product**: Autonomous CFA Analyst Platform
**Package**: @robotixai/corp-finance-mcp
**Version**: 1.1
**Date**: 2026-05-06
**Author**: RobotixAI Engineering

---

## 1. Overview

The CFA runtime is exclusively four surfaces: the **CLI** (`cfa <subcommand>` shipped from `crates/corp-finance-cli/`), **MCP** (every `server.tool(...)` registration in `packages/*-mcp-server/src/`), **Skills** (slash commands and `.claude/skills/*` invoked by the LLM), and the **Plugin** (Claude Code hooks declared at `plugins/cfa-core/hooks/hooks.json`). Cookbooks under `managed-agent-cookbooks/` are static deployment templates. They are submitted to Anthropic's Managed Agents API via a CLI invocation (`cfa managed-agent deploy`) and execute in Anthropic infrastructure thereafter; they are not runtime concepts of the CFA system itself.

Phase 25 hardened those four surfaces and introduced cost-tiered MCP servers (`cfa-core` free, `data-mcp-server` free, `fmp-mcp-server` freemium, `vendor-mcp-server` paid), the Rust `managed_agent` module, 5 governance hooks at `plugins/cfa-core/hooks/hooks.json`, and 39+ cookbook tests.

What is missing for production: durable memory across CLI invocations, MCP tool calls, and skill executions; defensible audit trails on every numeric output produced through any surface; cost discipline on paid-vendor MCP traffic; structured observability across the four surfaces; and broad-spectrum PII / prompt-injection security at every surface boundary.

Phase 26 closes those gaps by building native Rust modules using mature crates: `hnsw_rs` for HNSW vector index, `tantivy` for BM25, `petgraph` for graph traversal, `flate2` for portable session archives, `tracing` + `tracing-subscriber` for structured spans, `rusqlite` for the cost ledger, and `regex` for PII / prompt-injection detection. We also ship the finance-specific pieces: MCP tool-name lint, surface output audit hashing, and a CI workflow.

The architectural framing for Phase 26 is captured in three ADRs:

- **ADR-015** — umbrella decision to build native orchestration / memory / audit modules.
- **ADR-016** — memory architecture detail (`run_summary.json`, indexing, retention, native portable session archive, hybrid retrieval).
- **ADR-017** — audit / cost / observability / security detail (`.audit.json`, cost CLI, traces, native PII / injection scanner).

Two DDD bounded contexts:

- **`docs/ddd/domain-memory.md`** — RunSummary, TrajectoryIndex, SimilarRunQuery, CfaSession.
- **`docs/ddd/domain-audit-observability.md`** — AuditManifest, CostBudget, TraceSpan, SecurityScan.

Two contract files:

- **`docs/contracts/feature_memory.yml`** — RUF-MEM-001..N executable invariants.
- **`docs/contracts/feature_audit_observability.yml`** — RUF-AUD-001..N, RUF-COST-001..N, RUF-OBS-001..N, RUF-SEC-001..N executable invariants.

---

## 2. Problem Statement

| Capability | Phase 25 state | Phase 26 target |
|-----------|---------------|----------------|
| Cross-surface memory | CLI invocations, MCP tool calls, and skill runs are amnesic. | Every CLI invocation that returns structured output, every MCP tool handler that produces a numeric result, and every plugin hook write produces a `RunSummary` indexed via the native `hnsw_rs` HNSW backend; analysts retrieve similar past surface events in one query. |
| Cross-session persistence | Long-running workspaces lost between conversations. | Native portable session archive (`.cfa-session`, JSON + `flate2` gzip) persists CLI session state and the working set of MCP tool outputs; analysts restore last week's working session with one CLI call. |
| Per-output audit | No canonical provenance for numeric recommendations emitted through CLI / MCP / plugin write paths. | Every audit-required output written by a CLI subcommand, MCP tool, or plugin hook has a sibling `.audit.json` with surface audit hash and tool-call ledger. |
| Cost tracking | Token counts in API responses; no aggregation, no budget. | `cfa cost summary` aggregates per CLI invocation and per MCP tool call by surface / `CookbookTier` / `McpServerTier` / analyst from a `rusqlite` ledger; budgets and threshold alerts fire on the surface event boundary. |
| Observability | `println!` and stdout-only hooks. | Structured spans for every CLI subcommand invocation, every `server.tool(...)` handler, and every plugin hook fire via `tracing` + `tracing-subscriber`; optional OTLP exporter via `tracing-opentelemetry`. |
| Security | One regex hook for SSN. | Native `corp_finance_core::observability::security_scan` covers 14 PII categories plus prompt-injection patterns at three surface hook points (PreToolUse, PreMemoryWrite, PostToolUse). |

---

## 3. User Stories

### Memory

**US-26-01** As a **CLI user**, I want every `cfa <subcommand>` invocation that returns structured JSON to produce a `run_summary.json` with `surface`, `subcommand`, `recommendation` (when applicable), `sources_hash`, `model`, and `ts` so that the invocation is durably searchable later.
- Acceptance: RUF-MEM-001
- Dependency: ADR-016, `corp-finance-core::memory`

**US-26-02** As an **LLM agent invoking MCP tools**, I want every `server.tool(...)` handler completion to emit a `RunSummary` so that I can query past tool calls by free-form text and filter by tool name, MCP server, or date range and find the most relevant prior result in seconds.
- Acceptance: RUF-MEM-002, RUF-MEM-003
- Dependency: ADR-016, native hybrid retriever (`tantivy` BM25 + `hnsw_rs` vector + `petgraph` graph + MMR)

**US-26-03** As a **CLI user driving long-running diligence**, I want my surface-level working state (recent CLI invocations, recent MCP tool outputs, last skill output) to persist between conversations so that multi-day work is not lost.
- Acceptance: RUF-MEM-005
- Dependency: ADR-016, native portable session archive (`flate2`)

**US-26-04** As a **compliance reviewer**, I want indexed surface events to age out of the hot tier at 90 days but stay retrievable from cold storage so that retrieval latency is bounded while regulatory retention (7 years) is honoured.
- Acceptance: RUF-MEM-006
- Dependency: ADR-016 retention policy

### Audit

**US-26-05** As a **compliance reviewer**, I want every output file containing a numeric recommendation written by a CLI subcommand, an MCP tool handler, or a plugin `PostToolUse` hook to have a matching `.audit.json` so that I can reconstruct the surface invocation and tool-call ledger that produced it.
- Acceptance: RUF-AUD-001, RUF-AUD-002
- Dependency: ADR-017 audit manifest schema

**US-26-06** As a **compliance reviewer**, I want the `surface_audit_hash` (covering CLI subcommand spec, MCP tool registration, or plugin hook config) to be deterministic so that two invocations of the same surface entry point produce the same hash, and any change produces a different hash.
- Acceptance: RUF-AUD-003
- Dependency: ADR-009 djb2 algorithm reused per ADR-017

### Cost

**US-26-07** As an **operations lead**, I want to see aggregated token cost by CLI subcommand, by MCP tool, by `McpServerTier`, and by analyst so that paid-vendor usage is visible before invoicing.
- Acceptance: RUF-COST-001, RUF-COST-002
- Dependency: ADR-017, native `rusqlite` cost ledger

**US-26-08** As an **operations lead**, I want budget thresholds (e.g., 80% of monthly cap) to fire alerts within 1 second of a CLI invocation or MCP tool call crossing the threshold so that runaway spend is caught quickly.
- Acceptance: RUF-COST-003
- Dependency: ADR-017 alert latency target

### Observability

**US-26-09** As an **SRE**, I want every `cfa` CLI invocation, every MCP `server.tool` handler, and every plugin hook fire to produce a structured trace span with parent / child spans for surface phases so that production failures are diagnosable from telemetry rather than logs.
- Acceptance: RUF-OBS-001, RUF-OBS-002
- Dependency: ADR-017, `tracing` + `tracing-subscriber` (OTLP via optional `tracing-opentelemetry` feature)

### Security

**US-26-10** As a **Claude Code user with the cfa-core plugin installed**, I want the existing SSN-only regex hook replaced with the native scanner at `corp_finance_core::observability::security_scan` covering all 14 PII categories plus prompt-injection patterns at three plugin hook points (`PreToolUse`, `PreMemoryWrite`, `PostToolUse`) so that PII coverage is comprehensive at every surface boundary and prompt injection is defended.
- Acceptance: RUF-SEC-001, RUF-SEC-002, RUF-SEC-003
- Dependency: ADR-017, native scanner, `plugins/cfa-core/hooks/hooks.json` rewrite

---

## 4. Functional Requirements

### FR-26-01: Memory module (`corp-finance-core::memory`)

- New Rust module exposing four aggregates: `RunSummary`, `TrajectoryIndex`, `SimilarRunQuery`, `CfaSession`.
- Native backends: `hnsw_rs` HNSW index, `tantivy` BM25 inverted index, `petgraph` graph, `flate2` portable session archive. Optional cargo features: `agentdb_backend`, `rvf_backend`.
- New cargo deps in `crates/corp-finance-core/Cargo.toml`: `hnsw_rs = "0.x"`, `tantivy = "0.x"`, `petgraph = "0.x"`, `flate2 = "1"`, `uuid = { version = "1", features = ["v7"] }`.
- Unit tests validate aggregate invariants (recommendation enum constraints, hash format, etc.).
- Integration tests validate end-to-end ingest/retrieve/save/restore against the native backends.

### FR-26-02: `run_summary.json` emission

- Every CLI subcommand whose output is structured JSON emits exactly one `run_summary.json` at the surface output directory root via a CLI tracing wrapper installed in `crates/corp-finance-cli/src/main.rs`.
- Every MCP `server.tool(...)` handler whose return value contains a numeric recommendation emits a `RunSummary` via the MCP wrapper installed in each `packages/*-mcp-server/src/`.
- Every plugin hook `PostToolUse` write of a numeric output emits a `RunSummary` via the hook emitter declared in `plugins/cfa-core/hooks/hooks.json`.
- Schema as per ADR-016. Required fields: `schema_version`, `run_id`, `ts`, `surface` (`cli` | `mcp` | `plugin` | `skill`), `surface_event_id` (subcommand name, MCP tool name, hook id, slash-command id), `surface_audit_hash`, `model`, `sources_hash`, `status`.
- Failure to emit causes the surface invocation to be marked `partial` and an `audit_failure` event to be raised.

### FR-26-03: Indexing

- After `run_summary.json` is written, `corp-finance-core::memory::TrajectoryIndex::ingest()` is called.
- Failure is non-blocking for the surface invocation itself; sidecar `indexed: false` flag set on disk.
- Retry: 3 attempts with exponential backoff.

### FR-26-04: Retrieval CLI / MCP

- New CLI subcommands:
  - `cfa memory ingest --run-summary <path>`
  - `cfa memory find --query <text> [--surface <cli|mcp|plugin|skill>] [--surface-id <id>] [--k 5]`
  - `cfa session save --session-id <id>`
  - `cfa session restore --session-id <id>`
- New MCP tools registered in `packages/cfa-core/src/`: `surface_memory_ingest`, `surface_memory_find`, `surface_session_save`, `surface_session_restore`.

### FR-26-05: Audit manifest emission

- For every output file containing a numeric recommendation produced by any of the four surfaces, a sibling `<output>.audit.json` is written.
- Schema as per ADR-017. Required fields: `schema_version`, `run_id`, `surface`, `surface_event_id`, `surface_audit_hash`, `model`, `ts`, `output_path`, `output_sha256`.
- `tool_call_ledger` recommended; required for surface invocations whose contracts flag it (e.g., MCP tools that compose multiple downstream calls).

### FR-26-06: Surface audit hash

- `corp-finance-core::observability::audit::compute_surface_hash(&SurfaceSpec) -> String` returns `djb2:0x...` over canonical JSON of the surface entry-point definition: for CLI, the subcommand spec (clap definition); for MCP, the `server.tool(...)` registration (name + schema + handler module path); for plugin, the hook entry from `hooks.json`; for skill, the slash-command `.md` and skill `.md` content.
- Identical content produces identical hash; any change to the entry-point definition produces a different hash.
- Hash is computed once at process start (CLI) or handler registration (MCP) or hook fire (plugin) and reused for all downstream artefacts (`run_summary.json`, `.audit.json`, memory index).

### FR-26-07: Cost tracking CLI / MCP

- New CLI subcommands:
  - `cfa cost summary [--since <date>] [--by surface|tool|tier|analyst]`
  - `cfa cost budget get`
  - `cfa cost budget set --surface-id <id> --monthly-limit <usd>`
  - `cfa cost alerts`
- New MCP tools: `surface_cost_summary`, `surface_cost_budget_set`, `surface_cost_budget_get`.
- Persistent ledger: `rusqlite`-backed SQLite at `<repo>/var/observability/cost-ledger.sqlite`. New cargo dep: `rusqlite = { version = "0.x", features = ["bundled"] }`.
- Tier classification consumes existing `CookbookTier` (`crates/corp-finance-core/src/managed_agent/types.rs`) and `McpServerTier` (`crates/corp-finance-core/src/mcp_servers/types.rs`); no new tier type or `cost_tier` field is introduced. CLI invocations and plugin hook fires inherit tier classification from the MCP servers they touch.
- Threshold defaults: `CoreOnly` no limit, `Freemium` 80%/100%, `PaidVendor` 50%/80%/95%/100%.

### FR-26-08: Structured tracing

- A `tracing` initialiser installed in `crates/corp-finance-cli/src/main.rs` opens a root span for every `cfa` CLI invocation. New cargo deps: `tracing = "0.1"`, `tracing-subscriber = "0.3"`. Optional: `tracing-opentelemetry = "0.x"` behind the `otlp_export` feature.
- A `tracing` initialiser in each `packages/*-mcp-server/src/index.ts` (via napi binding to `corp-finance-core::observability::tracing`) opens a span for every `server.tool(...)` handler invocation.
- A plugin hook emitter at `plugins/cfa-core/hooks/hooks.json` writes a span event per fire (PreToolUse, PostToolUse, PreMemoryWrite).
- Required span attributes: `cfa.surface` (`cli`|`mcp`|`plugin`|`skill`), `cfa.surface.id`, `cfa.run.id`, `cfa.user` (redacted to first-initial-last-name).
- Child spans for: validate, sub-handler calls, downstream MCP tool calls, memory ingest.

### FR-26-09: Security hook replacement

- Replace `plugins/cfa-core/hooks/hooks.json` SSN regex with native scanner at `corp_finance_core::observability::security_scan` exposed via the `surface_pii_scan` MCP tool, called at three plugin hook points: `PreToolUse` (block on prompt injection, redact PII), `PreMemoryWrite` (redact PII), `PostToolUse` (alert only). New cargo dep: `regex = "1"`.
- 14 PII categories: names+address tuples, emails, phones, addresses, SSN, passport, drivers' licence, bank account, credit card, IBAN, IP address, MAC address, date of birth, government ID / EIN.
- Injection pattern set: instruction_override, role_swap, context_truncation, encoded_payload, delimiter_collision, tool_chain_hijack.
- CFA allowlist: CUSIP, ISIN, SEDOL, FIGI, LEI, ticker symbols.
- Optional npm fallback for HTTP transport: `aidefence@2.2.4` (Express middleware; documented in deployment guide; not used by stdio-MCP).

### FR-26-10: MCP tool-name lint at validate time

- `cfa validate` (and `cfa managed-agent validate` for cookbook deploy artefacts) rejects any reference to MCP tool names not in the registered set across `cfa-core`, `data-mcp-server`, `fmp-mcp-server`, `vendor-mcp-server`. The registered set is read from each `packages/*-mcp-server/src/` at build time.
- Error message identifies the unrecognised tool name and the source file path.

### FR-26-11: CI workflow

- New file `.github/workflows/phase26-checks.yml` runs on every PR.
- Steps: `cargo build --workspace --all-features`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-features -- -D warnings`, `cargo fmt --check`, contract evaluation for `feature_memory.yml` and `feature_audit_observability.yml`, native PII regression suite (14-category positive/negative test set), CLI subcommand surface lint, MCP `server.tool` registration lint.

---

## 5. Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-26-01 | Indexing success rate per surface | >= 80% of CLI invocations producing structured JSON, MCP tool calls returning numeric output, and plugin hook writes successfully indexed in the native HNSW + BM25 store |
| NFR-26-02 | Cost alert latency | Threshold crossed by a CLI invocation or MCP tool call -> alert event emitted in <= 1000 ms |
| NFR-26-03 | Audit coverage | 100% of audit-required outputs from CLI / MCP / plugin surfaces produce a `run_summary.json` and `.audit.json` |
| NFR-26-04 | PII regression | Zero critical PII regressions (the new system catches everything the old SSN regex caught, plus the additional 13 categories) |
| NFR-26-05 | Trace overhead | Span open / close adds <= 15 ms to CLI invocation, MCP tool call, or plugin hook fire wall-clock |
| NFR-26-06 | Memory query latency | `cfa memory find` returns top-5 in <= 500 ms p95 over 10k indexed surface events |
| NFR-26-07 | Storage retention | Hot 90 days, cold 7 years (`run_summary.json` and `.audit.json` only; surface output bodies follow each surface's own retention) |

---

## 6. Acceptance Criteria

Each user story maps to one or more Specflow contract IDs. The implementation is accepted when all contracts pass and all NFR targets are met in the integration test suite.

| User Story | Specflow IDs |
|-----------|--------------|
| US-26-01 | RUF-MEM-001 |
| US-26-02 | RUF-MEM-002, RUF-MEM-003 |
| US-26-03 | RUF-MEM-005 |
| US-26-04 | RUF-MEM-006 |
| US-26-05 | RUF-AUD-001, RUF-AUD-002 |
| US-26-06 | RUF-AUD-003 |
| US-26-07 | RUF-COST-001, RUF-COST-002 |
| US-26-08 | RUF-COST-003 |
| US-26-09 | RUF-OBS-001, RUF-OBS-002 |
| US-26-10 | RUF-SEC-001, RUF-SEC-002, RUF-SEC-003 |

---

## 7. Success Metrics

| Metric | Before (Phase 25) | After (Phase 26) |
|--------|------------------|------------------|
| Indexed CLI invocations producing structured JSON | 0 | >= 80% |
| Indexed MCP tool calls returning numeric output | 0 | >= 80% |
| Plugin hook writes producing audit manifest | 0 | 100% of audit-required writes |
| PII categories detected at every surface boundary | 1 (SSN) | 14 |
| Prompt-injection defence | None | PreToolUse + PreMemoryWrite scan |
| Cost-budget alerts | None | Per CLI subcommand and per MCP tool with 1s emission latency |
| Structured CLI traces | None | Every `cfa` invocation |
| Structured MCP traces | None | Every `server.tool` handler invocation |
| Cross-session restore | Not supported | Round-trip lossless via portable session archive |
| Cross-surface similar-event retrieval | Not supported | Hybrid (BM25 + HNSW + petgraph) + MMR re-rank |
| MCP tool-name lint | None | Validate-time error on unregistered tool names |
| CI coverage of Phase 26 | None | Dedicated workflow, runs on every PR |

---

## 8. Dependencies

### Prior phases
- **Phase 20 / ADR-008** — workflow skills layer.
- **Phase 21 / ADR-009** — djb2 audit-hash algorithm reused for surface audit hash.
- **Phase 24** — 15 cookbooks under `managed-agent-cookbooks/` (deployment artefacts only).
- **Phase 25** — `corp-finance-core::managed_agent` module; `CookbookTier` / `McpServerTier` classification; 5 governance hooks at `plugins/cfa-core/hooks/hooks.json`; 39+ tests covering deployment artefacts.

### External
- New cargo crate dependencies (added to `crates/corp-finance-core/Cargo.toml`):
  - `hnsw_rs = "0.x"` — HNSW vector index
  - `tantivy = "0.x"` — BM25 inverted index
  - `petgraph = "0.x"` — entity / surface-event graph
  - `flate2 = "1"` — portable session archive compression
  - `tracing = "0.1"`, `tracing-subscriber = "0.3"` — structured spans
  - `tracing-opentelemetry = "0.x"` (optional, behind `otlp_export` feature) — OTLP exporter
  - `rusqlite = { version = "0.x", features = ["bundled"] }` — cost ledger
  - `regex = "1"` — PII / injection patterns
  - `uuid = { version = "1", features = ["v7"] }` — `run_id`
- Optional npm fallback: `aidefence@2.2.4` (HTTP middleware; not used by stdio-MCP transport).

### Internal
- `corp-finance-core::managed_agent` (existing; extended).
- `corp-finance-cli` (existing; extended with new subcommands and tracing wrapper).
- `cfa-core` MCP server (existing; extended with new tools and tracing wrapper).
- `plugins/cfa-core/hooks/hooks.json` (existing; rewritten to invoke `surface_pii_scan` and emit hook spans).

---

## 9. Estimated Work

6 days, allocated as:

| Day | Focus |
|-----|-------|
| 1 | Memory module skeleton; `RunSummary` + native HNSW backend (`hnsw_rs`); `corp-finance-core::memory` |
| 2 | TrajectoryIndex + SimilarRunQuery (BM25 via `tantivy` + HNSW + MMR); CLI `memory find`; MCP tools |
| 3 | CfaSession + portable session archive round-trip (JSON + `flate2`); CLI `cfa session save` / `cfa session restore` |
| 4 | AuditManifest emission + djb2 surface hash; CLI tracing wrapper; MCP `server.tool` wrapper; plugin hook emitter rewrite; tool-name lint |
| 5 | `rusqlite` cost ledger + budget thresholds; tracing exporter wiring; observability spans across all four surfaces |
| 6 | Native PII / injection scanner replaces SSN-only hook at all three hook points; CI workflow; contract test suite green |

---

## 10. Out of Scope

- Daily compaction job that moves entries past 90 days from hot to cold (planned for Phase 27).
- Quarterly reconciliation between the native cost ledger and Anthropic billing (planned for Phase 27).
- Custom finance-domain extensions to the 14 PII categories beyond the configured allowlist.
- Replacement of any existing MCP server; the four servers shipped in Phase 25 remain as-is.
- New cookbooks; the 15 existing cookbooks gain audit / memory / cost integration via the surface instrumentation only (no rewrite of cookbook content). Cookbook deploys are CLI invocations (`cfa managed-agent deploy`) and inherit surface-level instrumentation; cookbooks themselves are deployment artefacts outside this PRD's runtime scope.
- A web dashboard for cost / audit visualisation; CLI and MCP only in Phase 26.
- Multi-tenant isolation; single-tenant assumed throughout (Phase 27 federation).
- Optional `agentdb_backend` and `rvf_backend` cargo features are scaffolded but disabled by default; smoke testing of those backends is deferred.
