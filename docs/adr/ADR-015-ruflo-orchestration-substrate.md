# ADR-015: Native Orchestration / Memory / Audit Layer Inspired by Ruflo Concepts

## Status: Accepted

## Date: 2026-05-06

## Deciders: RobotixAI Engineering, CFA Platform Lead, Compliance Sponsor

## Context

Phase 24 shipped 15 reference cookbook templates and 9 specialist CFA agents. Phase 25 added the cost-tiered MCP server matrix (`cfa-core` free, `data-mcp-server` free, `fmp-mcp-server` freemium, `vendor-mcp-server` paid), a Rust `managed_agent` module (validate / check_all / deploy / orchestrate / sync / list — deploy-time tooling for users wiring up cookbooks), 5 governance hooks at `plugins/cfa-core/hooks/hooks.json`, and 39+ cookbook-validation tests.

Phase 26 must close the remaining gaps required to take this stack into supervised production:

| Capability | Today (Phase 25) | Production gap |
|------------|-----------------|----------------|
| Cross-run memory | None. Each CLI invocation and MCP tool call is amnesic. | Analysts cannot retrieve "the last time we modeled this issuer" without re-running. |
| Cross-session persistence | None. Long-running deal workspaces are lost between conversations. | Multi-day diligence loses context. |
| Per-output audit | Manual; no canonical hash for "what code + skills + sub-agents produced this number". | Compliance cannot reconstruct provenance for regulators. |
| Cost tracking | Token usage emitted by the MCP transport but not aggregated, no budget alerts. | Paid-vendor MCP tools (Bloomberg/FactSet/Capital IQ surfaces) silently overrun. |
| Observability | `println!` and a few hook stdouts. No structured trace per CLI invocation or per MCP tool call. | Cannot answer "why did this run fail at 02:14 UTC". |
| Security | One PII hook at `plugins/cfa-core/hooks/hooks.json` matching SSN-shaped strings via regex. | Misses 13 other PII categories; no prompt-injection defence at all. |

Three viable paths were considered:

1. **BUILD NATIVE**: write vector indexing, RAG retrieval, session persistence, cost aggregation, structured tracing, and PII / prompt-injection scanners directly inside `corp-finance-core` and `cfa-core`, leaning on best-in-class Rust crates and a minimal set of npm packages. Instrument at the four runtime surfaces: CLI subcommands, MCP `server.tool` registrations, plugin hook fire points, and (passively) skill invocations which collapse to MCP calls.
2. **BORROW (ruflo plugin family)**: adopt the `ruflo-*` plugin set installed under `~/.claude/plugins/cache/ruflo/` as the substrate, and constrain finance-domain logic to our crates with an anti-corruption layer.
3. **HYBRID**: build native modules for everything, and keep an *optional* fallback to the ruflo MCP backend for the two capabilities where ruflo ships a real packaged artefact (`agentdb` npm + `ruvector-*` crates, `rvf-cli`/`rvf-federation` crates).

A package-availability and end-to-end smoke test in May 2026 (see `/tmp/ruflo-smoke-test.md` and `/tmp/ruflo-package-availability.md`) showed that only 4 of 11 ruflo plugins worked end-to-end as separately-installed plugins, and only 3 ruflo capabilities ship as real registry artefacts (`agentdb` on npm, `aidefence@2.2.4` on npm, `rvf-cli` / `rvf-federation` on crates.io). The other 8 capabilities exist only as markdown skills and Node stdlib scripts inside the ruflo monorepo, with no `npm install` or `cargo add` shortcut. Treating ruflo as a runtime substrate would mean either vendoring monorepo source into our tree or carrying integration shims for capabilities that do not exist as expected packages.

## Decision

Adopt path 3: **build native finance-domain orchestration / memory / audit modules inspired by ruflo concepts; optionally fall back to the ruflo MCP backend for `agentdb` and `rvf` if the user has the upstream plugins installed at user-scope; explicitly avoid runtime dependence on ruflo for production deploys.** Status: Accepted.

The decision preserves our determinism story (no external MCP backend dependency in production), gives finance-domain fit (we own retrieval ranking, audit hashing, PII categories), and avoids carrying broken integration shims for plugins that do not actually exist as expected packages.

### Runtime surfaces vs deployment artefacts

The CFA runtime executes through exactly four surfaces:

- **CLI** — every subcommand of the `cfa` binary at `crates/corp-finance-cli/src/main.rs` (e.g., `cfa workflow audit`, `cfa managed-agent validate`, `cfa memory find`).
- **MCP** — every `server.tool(...)` registration across `packages/mcp-server/`, `packages/fmp-mcp-server/`, `packages/data-mcp-server/`, `packages/vendor-mcp-server/`, and any future MCP server in `packages/`.
- **Skills** — slash commands in `.claude/commands/cfa/` and skill packages in `.claude/skills/*` invoked by the LLM via the Skill tool. Skills emit nothing themselves at runtime; they document trigger patterns and route to MCP tool calls (which are captured at the MCP wrapper) or to other skills.
- **Plugin** — hooks fired on `PreToolUse` / `PostToolUse` / `Write` / `Edit` lifecycle events at `plugins/cfa-core/hooks/hooks.json`. This is the only surface that observes file-write events.

Cookbooks at `managed-agent-cookbooks/` are **deployment artefacts**, not a runtime surface. They are reference templates a user may publish to Anthropic's `/v1/agents` Managed Agents API via `cfa managed-agent deploy`. Once a cookbook is deployed, the resulting managed agent runs in Anthropic's infrastructure; we have no runtime relationship with it. Cookbooks are not subject to this ADR's runtime concerns: we do not capture memory from deployed cookbooks, we do not aggregate cost from them, we do not trace them, and we do not observe them. The only runtime event we record is the local CLI invocation that built and posted the deploy payload.

This second decision — "the runtime is the four surfaces; cookbooks are static deployment templates" — is what determines where every Phase 26 capability is wired:

| Surface | Capability wired |
|---------|------------------|
| CLI binary (`crates/corp-finance-cli/src/main.rs`) | Root tracing span per invocation, cost-ledger event on tool execution, audit emit on file write, PII scan on stdin/stdout boundaries. |
| MCP `server.tool(...)` wrapper (`packages/*-mcp-server/src/`) | Per-tool trace span, per-tool cost record, PII scan on inputs and outputs, memory-ingest hook on tool completion. |
| Plugin hook (`plugins/cfa-core/hooks/hooks.json`) | File-write triggered `.audit.json` companion emit, PII scan on Write/Edit content, audit-failure event on hook reject. |
| Skill invocation | Passive — captured at the MCP wrapper when the skill instructs the LLM to call a tool. |

### Native Modules Built (Phase 26)

| Capability | Native module | Inspired by ruflo plugin (concepts only) | Optional fallback |
|------------|---------------|------------------------------------------|-------------------|
| HNSW vector store | `corp_finance_core::memory::TrajectoryIndex` (`hnsw_rs` crate) | `ruflo-agentdb` | If `agentdb@^3` npm or `ruvector-core` crate is installed at user-scope, native module can dispatch through MCP backend |
| Portable session memory | `corp_finance_core::memory::CfaSession` (JSON+`flate2` archive) | `ruflo-rvf` | If `rvf-cli` crate is installed, native module can read/write RVF format via crate API |
| Hybrid + graph retrieval | `corp_finance_core::memory::SimilarRunQuery` (BM25 over `tantivy` + HNSW + `petgraph` traversal + MMR) | `ruflo-rag-memory` | None (capability covered by native code) |
| Token cost aggregation | `corp_finance_core::observability::cost_ledger` (`rusqlite` ledger) | `ruflo-cost-tracker` | None |
| Structured tracing | `corp_finance_core::observability::tracing` (`tracing` + `tracing-subscriber` + OTLP exporter) | `ruflo-observability` | OTel-compatible backend (Datadog / Honeycomb / etc.) |
| 14-type PII + prompt-injection scanner | `corp_finance_core::observability::security_scan` (hand-rolled regex set + injection pattern lib) | `ruflo-aidefence` (categories list) | `aidefence@2.2.4` npm if user wants the upstream Express middleware (HTTP-only, currently N/A) |

Finance-domain logic (manifest validation, audit hashing, MCP tool-name lint, cost tier classification, cookbook deploy payload assembly) stays in `corp-finance-core::managed_agent`. That module is **deploy-time tooling** for users wiring up cookbooks, not a runtime substrate. The `corp_finance_core::memory` and `corp_finance_core::observability` modules expose first-class Rust types with no required external dependency; future ruflo MCP fallback (if added) flows through an anti-corruption layer.

### What Changed Versus the Earlier Drafts of This ADR

The first draft committed to adopting the ruflo plugin family as a runtime substrate. The May 2026 smoke test showed 7 of the 11 referenced plugins are not separately installable. We pivoted to native build, drawing inspiration from the ruflo concept set without carrying the runtime coupling.

The second-pass revision (this version) corrects a structural framing error in the first revision: it spoke of cookbook deploys as a runtime concept ("every cookbook deploy emits a run summary", "cookbook lifecycle events"). That conflated the deploy-time CLI action (which is a runtime event we own) with the deployed managed agent (which runs outside our infrastructure and is not a runtime concept for us). Phase 26 instrumentation is now anchored at the four surfaces above, and "cookbook" appears in this ADR only where strictly relevant — as a static deployment artefact a user may publish.

### Integration Boundary

Two responsibilities sit firmly inside our crates:

1. **Financial-domain semantics** — what a cookbook template declares, which skills it references, which sub-agents it lists, which cost tier it occupies. Owned by `corp-finance-core::managed_agent` (deploy-time validation and payload assembly). Note: `CookbookTier` (in `crates/corp-finance-core/src/managed_agent/types.rs`) and `McpServerTier` (in `crates/corp-finance-core/src/mcp_servers/types.rs`) are the existing tier types from Phase 25 — no `cost_tier` field is introduced.
2. **Audit hashing of finance artefacts** — djb2 fingerprint over output content and the surface event that produced it (CLI subcommand + MCP tool sequence + skill chain), as defined in ADR-009. Owned by `corp-finance-core::managed_agent::audit` (the function library; the trigger is a plugin Write/Edit hook).

The optional ruflo-MCP fallback path exists as a future-proofing boundary should a deployment want to substitute a ruflo-managed agentdb or rvf backend; for v1 the modules are first-class Rust types with no external dependency.

### Phase 26 Finance-Domain Work

| Item | Owner | Notes |
|------|-------|-------|
| MCP tool-name lint at deploy-time `validate` | `corp-finance-core::managed_agent::validate` | Cookbook YAML must reference only registered MCP tools; no invented names. Static check against the cookbook file before POST. |
| Per-output audit hashing (djb2) | `corp-finance-core::managed_agent::audit` | Same algorithm as ADR-009 workflow audit, reused; trigger is the plugin Write/Edit hook on output files. |
| Native memory module | `corp-finance-core::memory` | New module; depends on `hnsw_rs`, `tantivy`, `petgraph`, `flate2`. Indexed by surface event (CLI invocation digest + MCP tool sequence). |
| Native observability module | `corp-finance-core::observability` | New module; depends on `tracing`, `tracing-subscriber`, `tracing-opentelemetry` (optional), `rusqlite`. Spans rooted at CLI invocation; child spans per MCP tool and per plugin hook fire. |
| Native PII / injection scanner | `corp-finance-core::observability::security_scan` | Hand-rolled regex set covering 14 PII types; injection pattern lib (instruction_override, role_swap, encoded_payload, etc.). Wired at the MCP `server.tool` wrapper for inputs/outputs and at the plugin Write/Edit hook for file content. |
| CI workflow | `.github/workflows/phase26-checks.yml` | Runs `cargo test`, contract checks, manifest lint, and the native PII regression suite on every PR. |

### Phase 26 Day Allocation

The native build is sized at 6 working days. The breakdown is recorded here so future ADRs can reason about the cost of similar BUILD decisions:

| Day | Focus |
|-----|-------|
| 1 | Memory module skeleton, `RunSummary` aggregate (a CLI-invocation / MCP-tool-call summary, not a cookbook deploy summary), `hnsw_rs` integration |
| 2 | TrajectoryIndex + SimilarRunQuery (BM25 over `tantivy` + HNSW + MMR), CLI `memory find`, MCP `surface_memory_find` tool, MCP wrapper memory-ingest hook |
| 3 | CfaSession + portable session archive (JSON + `flate2`), CLI `cfa session save` / `cfa session restore` subcommands |
| 4 | AuditManifest emission, djb2 hash, CLI `audit show`, plugin Write/Edit hook wired to emit `.audit.json` companions, MCP tool-name lint at validate-time |
| 5 | Cost CLI + `rusqlite` budget ledger, structured tracing root span at CLI binary entry, child spans wrapped around every MCP `server.tool` registration and every plugin hook fire |
| 6 | Native PII / injection scanner replaces SSN-only hook; wired at MCP wrapper (input/output) and plugin Write/Edit hook (file content); CI workflow; contract suite green |

## Rationale

### BUILD vs BORROW analysis (revised)

| Dimension | BUILD NATIVE (chosen) | BORROW (ruflo plugin family, prior plan) |
|-----------|------------------------|------------------------------------------|
| Engineering cost | ~6 days using mature Rust crates (`hnsw_rs`, `tantivy`, `petgraph`, `tracing`, `rusqlite`) | ~6 days *if* every plugin existed as expected; smoke test shows 7 of 11 do not, so actual cost is higher (vendor source, build shims, monitor upstream drift). |
| Maintenance | We own the boundary; depend on stable, well-trafficked crates | Tracks ruflo upstream; the monorepo bundles ~314 capabilities into one CLI + one 269-tool MCP server, making per-capability version pinning hard. |
| Domain alignment | Tuned to finance vocabulary (CFA allowlist, MCP tool registry, CookbookTier, audit hash format) and to the four runtime surfaces (CLI / MCP / skill / plugin). | Generic; finance allowlist must be configured atop opaque plugin internals. |
| Risk of NIH | Low — we use commodity Rust crates for HNSW/BM25/graph/tracing/SQLite. The 14-PII regex set is the only piece we hand-roll, and its category list is a finance-platform decision regardless of upstream. | High dependency surface on a single rapidly-evolving upstream. |
| Compliance posture | Auditable Rust code in our tree; reproducible builds | Plugin internals are markdown skills + Node `.mjs` stdlib scripts in some cases — harder to audit. |
| Replaceability | Boundary is clean Rust types; future ruflo-MCP fallback is opt-in | Boundary required either way. |
| Determinism | All in-process; no external MCP runtime needed | Adds an MCP server runtime dependency to every production deploy. |

The BUILD decision is reinforced by the observation that none of these capabilities are differentiating for a finance platform. The differentiating layers are the 9 specialist agents, the ~206 MCP computation tools, the slash commands, and the audit semantics over them. HNSW and BM25 ranking, structured tracing, SQLite ledgering, and regex PII detection are commodity infrastructure where the win is in choosing the right crates.

### Boundary discipline

The anti-corruption layer is non-negotiable for any future ruflo-MCP fallback path. If a downstream user opts in to a ruflo backend, the dispatch path is constrained to:

- `corp-finance-core::memory::backend::ruflo_agentdb` (optional cargo feature)
- `corp-finance-core::memory::backend::ruflo_rvf` (optional cargo feature)

Domain types (`RunSummary`, `AuditManifest`, `CfaSession`) are defined in our crates with `serde` round-trip; the optional ruflo-MCP backend translates at the wire boundary only.

## Consequences

### Positive

- Six commodity capabilities (vector indexing, hybrid retrieval, session persistence, cost ledger, tracing, PII / injection) ship as native Rust modules with well-known crate dependencies.
- CLI invocations and MCP tool calls become first-class citizens: every output gets a `run_summary.json` indexed in our HNSW store and an `.audit.json` companion (the latter emitted by the plugin Write/Edit hook), enabling "show me how we got here" replay across the four surfaces.
- Cost overruns on paid-vendor MCP tools (`McpServerTier::PaidVendor`) are caught before invoice surprises via SQLite-backed budget thresholds keyed on per-tool-call events.
- Compliance gets a defensible PII story (14 categories owned by us, allowlisted to CFA identifiers) and prompt-injection defence in our regex / pattern library, applied at both MCP and plugin boundaries.
- The boundary discipline keeps finance differentiation in our crates, so a future swap to a ruflo-MCP backend (or any other) is feasible.
- No external MCP runtime is required for production; deploys are reproducible from the cargo workspace alone.
- Cookbook deploys via `cfa managed-agent deploy` are CLI invocations and inherit the surface-level instrumentation automatically — no special-case handling.

### Negative

- Phase 26 introduces well-known Rust crate dependencies (`hnsw_rs`, `tantivy`, `petgraph`, `tracing`, `tracing-subscriber`, `rusqlite`, `flate2`, `regex`). Each is a stable, widely-deployed crate; combined LoC added to the workspace is moderate (~1500-2500 LoC of native code plus tests).
- We own the 14-PII regex set and its allowlist tuning. False positives and negatives surface in the regression suite and require maintenance.
- Hybrid (BM25 + HNSW + graph) retrieval ranking is now a parameter we own rather than inherit; the MMR diversity weight, BM25 b/k1, and HNSW M/efConstruction are all tunables we have to test.
- An optional ruflo-MCP backend adds a feature flag and conditional code path, even though it is unused in default builds.
- Wrapping every MCP `server.tool(...)` registration with a tracing/cost/PII shim means every MCP server in `packages/` must be migrated; the wrapper is one shared TypeScript helper but the migration is fan-out work.

### Risks

- **Crate churn**: a major-version bump in `hnsw_rs` or `tantivy` could require migration. Mitigated by version pinning and an integration-test suite that catches behaviour drift.
- **Ranking quality**: hand-rolled BM25 + HNSW + MMR may underperform a mature retrieval engine on very large corpora. Mitigated by treating Phase 26 as the v1 target (10^4 to 10^6 indexed surface events) and by the optional ruflo-agentdb fallback path for installations that exceed that.
- **PII false positives**: 14-category detection is broader than SSN-only and may mis-flag legitimate financial data (CUSIPs, ISINs). Mitigated by the CFA allowlist in `security_scan` (CUSIP, ISIN, SEDOL, FIGI, LEI, ticker symbols) and by the regression test set.

## Implementation Notes

Native dependencies declared by Phase 26 (full list consolidated in PRD-phase26-memory-audit):

- `hnsw_rs = "0.x"` — HNSW vector index in `corp-finance-core/Cargo.toml`
- `tantivy = "0.x"` — BM25 inverted index for hybrid retrieval
- `petgraph = "0.x"` — graph traversal for entity / sub-agent / MCP-tool edges
- `flate2 = "1"` — gzip for portable session archives
- `tracing = "0.1"`, `tracing-subscriber = "0.3"` — structured spans
- `tracing-opentelemetry = "0.x"` (optional) — OTLP exporter
- `rusqlite = "0.x"` (with `bundled` feature) — cost ledger persistence
- `regex = "1"` — PII detection patterns
- `uuid = "1"` (with `v7` feature) — `run_id` allocation

Surface-level instrumentation points:

- **CLI binary** at `crates/corp-finance-cli/src/main.rs` — root tracing span opened at process entry; CLI subcommand name attached as span attribute; duration and exit-status recorded on drop. Every one of the ~200 `cfa <subcommand>` paths inherits this.
- **MCP wrapper** in each `packages/*-mcp-server/src/` index — every `server.tool(name, schema, handler)` registration is wrapped by a shared TypeScript helper that opens a child span, records cost/latency, runs PII scan on input and output, and dispatches a memory-ingest hook on completion. The wrapper is the single integration point for all ~594 MCP tools across `cfa-core`, `fmp-mcp-server`, `data-mcp-server`, and `vendor-mcp-server`.
- **Plugin hooks** at `plugins/cfa-core/hooks/hooks.json` — extended to emit `.audit.json` companions on `Write` / `Edit` events for output files containing numeric recommendations, and to invoke the native PII/injection scanner at `PreToolUse` and `PostToolUse` boundaries.
- **Skills** — no per-skill instrumentation. Skills route the LLM toward MCP tool calls, which are captured at the MCP wrapper.

Module layout:

- `corp_finance_core::memory` — TrajectoryIndex, RunSummary, SimilarRunQuery, CfaSession; backends: `native_hnsw` (default), `ruflo_agentdb` (optional feature)
- `corp_finance_core::observability::tracing` — `tracing` setup + OTel exporter
- `corp_finance_core::observability::cost_ledger` — `rusqlite`-backed ledger; budget threshold logic
- `corp_finance_core::observability::security_scan` — 14 PII regex categories + injection pattern set + CFA allowlist
- `corp_finance_core::managed_agent::audit` — extended; djb2 hash, manifest emission (consumed by the plugin Write/Edit hook)

References:

- Memory architecture detail and `RunSummary` schema: see ADR-016.
- Audit / cost / observability detail and the SSN-hook replacement plan: see ADR-017.
- Multi-agent coordination via the existing surfaces (chief-analyst routing): see ADR-018.
- Tenant scoping at every output / state boundary: see ADR-019.
- Self-learning loop over surface events: see ADR-020.
- Bounded contexts and aggregates: see `docs/ddd/domain-memory.md` and `docs/ddd/domain-audit-observability.md`.
- Functional requirements and acceptance criteria: see `docs/prd/PRD-phase26-memory-audit.md`.
- Executable invariants: see `docs/contracts/feature_memory.yml` and `docs/contracts/feature_audit_observability.yml`.
- Concept references (inspiration only, not runtime dependencies): https://github.com/ruvnet/ruflo
- Cookbook layout (deployment artefacts only): `managed-agent-cookbooks/`.
- Hook to be extended: `plugins/cfa-core/hooks/hooks.json` (current SSN-only regex; ADR-017 specifies the migration to native scanner and Write/Edit hook for `.audit.json`).

## Related Decisions

- **ADR-008** — Financial Services Workflow Integration (workflow skills layer; the skill surface).
- **ADR-009** — Workflow Auditability (djb2 hashing pattern reused for output audit hashing in ADR-017).
- **ADR-016** — Memory Architecture (detail on `RunSummary`, `TrajectoryIndex`, `SimilarRunQuery`, `CfaSession`; native HNSW backend; capture happens at CLI invocation and MCP tool-call boundaries).
- **ADR-017** — Audit, Cost, Observability, Security (detail on `.audit.json` companion via plugin Write/Edit hook, `cfa cost` CLI subcommand, structured tracing across CLI/MCP/plugin, native PII / injection scanner at MCP and plugin boundaries).
- **ADR-018** — Multi-Agent Coordination via Existing Surfaces (chief-analyst → specialist routing through Claude Code's Agent tool; native entity graph and A* planner over MCP tool action space).
- **ADR-019** — Multi-Tenant Federation (tenant scoping at every output / state boundary across the four surfaces).
- **ADR-020** — Self-Learning Loop (trajectory = sequence of CLI / MCP / skill calls; A* over MCP tools + slash commands).

## References

- ruflo source repo (concept inspiration only): https://github.com/ruvnet/ruflo
- Smoke test findings (May 2026): `/tmp/ruflo-smoke-test.md`
- Package availability findings (May 2026): `/tmp/ruflo-package-availability.md`
- ADR-008: `docs/adr/ADR-008-financial-workflow-integration.md`
- ADR-009: `docs/adr/ADR-009-workflow-rust-auditability.md`
- Reference deployment templates: `managed-agent-cookbooks/`
- Deploy-time managed-agent CLI: `crates/corp-finance-cli/src/commands/managed_agent.rs`
- Deploy-time managed-agent module: `crates/corp-finance-core/src/managed_agent/`
- `CookbookTier` definition: `crates/corp-finance-core/src/managed_agent/types.rs`
- `McpServerTier` definition: `crates/corp-finance-core/src/mcp_servers/types.rs`
