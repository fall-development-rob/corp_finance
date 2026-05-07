# ADR-016: Memory Architecture for CLI / MCP / Plugin Surface Events and Cross-Session Context

## Status: Accepted

## Date: 2026-05-06

## Deciders: RobotixAI Engineering, CFA Platform Lead

## Context

ADR-015 commits the platform to building native finance-domain orchestration / memory / audit modules inspired by ruflo concepts, with optional fallback to a ruflo MCP backend if the user installs the upstream plugins at user-scope. ADR-015 also pins the runtime to four surfaces: CLI, MCP, skill, plugin. This ADR specifies the memory bounded context: how surface events become persistent, retrievable, and replayable across sessions.

Today, CLI invocations write to per-run output directories, MCP tool calls return data to the LLM and disappear, plugin hook fires emit a stdout line and exit, and skill invocations route the LLM to MCP calls without leaving a record of their own. There is no canonical record of "what happened in this analyst session", no index over past sessions, and no way to retrieve "the most similar past run to my current one". For institutional finance use, this is a hard requirement — analysts and compliance both expect to be able to ask:

- "What was our last published view on this issuer?"
- "Show me the three closest comparables to this current diligence."
- "Reload the deal workspace I had open last Friday."
- "Audit every CLI subcommand and MCP tool call that touched ticker XYZ in Q1."

The capabilities we need are HNSW vector search, hybrid (BM25 + vector) retrieval with MMR diversity ranking, portable session memory, and graph traversal for entity → MCP tool → sub-agent links. The `ruflo-agentdb`, `ruflo-rag-memory`, and `ruflo-rvf` plugin descriptions in https://github.com/ruvnet/ruflo describe the same capability shape, but only `agentdb` (npm) and `rvf-cli`/`rvf-federation` (crates.io) ship as packaged artefacts; `ruflo-rag-memory` is monorepo-source-only. We therefore build native Rust modules using mature crates (`hnsw_rs`, `tantivy`, `petgraph`, `flate2`) and expose an optional fallback to the ruflo MCP backend behind a cargo feature.

## Decision

Define a Memory bounded context owned by `corp-finance-core::memory`. The context has four aggregates: `RunSummary`, `TrajectoryIndex`, `SimilarRunQuery`, `CfaSession`. Every CLI invocation that produces an output, and every MCP tool call that produces a non-trivial result, generates exactly one `run_summary.json` written to disk and indexed via the native HNSW backend (`hnsw_rs` crate). Cross-session similar-run retrieval is served by a native hybrid retriever combining `tantivy` (BM25), `hnsw_rs` (vector), `petgraph` (graph traversal), and a hand-rolled MMR re-ranker. Cross-session persistence uses a portable session archive format (JSON + `flate2` gzip) inspired by RVF; an optional ruflo-rvf crate-backed fallback is available behind a cargo feature. Retention is 90 days hot in the HNSW index, archive cold thereafter.

Capture is automatic at the wrappers specified in ADR-015:

- CLI binary at `crates/corp-finance-cli/src/main.rs` emits one `RunSummary` per invocation when the invocation produces an output artefact.
- MCP `server.tool(...)` wrapper in `packages/*-mcp-server/src/` emits one `RunSummary` per tool call when the tool produces an output artefact (configurable per tool via the wrapper's emit policy).
- Plugin hook at `plugins/cfa-core/hooks/hooks.json` does not emit `RunSummary` directly; it only emits `.audit.json` companions per ADR-017. Memory ingestion is the responsibility of the CLI/MCP wrapper.
- Skills emit nothing themselves; their effects are captured at the MCP wrapper when the LLM follows the skill's instructions.

### `run_summary.json` schema

Every surface event emits one and only one `run_summary.json` for the artefact it produced. The file is the canonical record of the surface event.

```json
{
  "schema_version": "1.0",
  "run_id": "uuid-v7",
  "ts": "2026-05-06T14:22:01Z",
  "surface": "cli",
  "surface_event_id": "cfa.workflow.audit",
  "surface_audit_hash": "djb2:0x7ab3c910",
  "model": "claude-opus-4-7[1m]",
  "ticker": "ACME",
  "asset_class": "equity",
  "recommendation": "BUY",
  "price_target": "142.50",
  "target_currency": "USD",
  "horizon_months": 12,
  "key_assumptions": [
    "WACC 9.2%",
    "terminal_growth 2.5%",
    "EBITDA margin expansion 180bps over plan"
  ],
  "sources_hash": "djb2:0x44e1289c",
  "skills_invoked": ["workflow-equity-research", "corp-finance-tools-core"],
  "mcp_tools_invoked": ["dcf_model", "comps_table", "wacc"],
  "sub_agents": ["cfa-equity-analyst"],
  "input_hash": "djb2:0x91002a55",
  "output_dir": "/var/cfa/runs/2026-05-06/uuid-v7/",
  "status": "completed",
  "duration_ms": 184321,
  "token_usage": {
    "input_tokens": 412009,
    "output_tokens": 18402,
    "cache_read_tokens": 380122
  }
}
```

The `surface` field takes values `"cli"`, `"mcp"`, or `"plugin"`. Skill invocations are not a separate surface for capture purposes; they collapse to whichever MCP tool the LLM ultimately calls.

The `surface_event_id` field identifies the surface event:
- For `surface == "cli"`, this is the dotted CLI subcommand path (e.g., `cfa.workflow.audit`, `cfa.managed-agent.deploy`).
- For `surface == "mcp"`, this is the MCP tool name (e.g., `dcf_model`, `fmp_quote`).
- For `surface == "plugin"`, this is the hook event (e.g., `plugin.write.audit_emit`). Reserved; plugin hook events do not normally produce a `RunSummary`.

The `surface_audit_hash` field carries the djb2 hash of the surface event's manifest — for CLI, the subcommand definition; for MCP, the tool registration. It supersedes the prior `cookbook_audit_hash` framing.

Required fields: `schema_version`, `run_id`, `ts`, `surface`, `surface_event_id`, `surface_audit_hash`, `model`, `sources_hash`, `status`. Where applicable for finance outputs, `ticker` (or asset identifier) and `recommendation` (or equivalent decision token) are required as well. Other fields are recommended and used by retrieval ranking.

The `recommendation` field is normalised across output families: equity outputs use BUY/HOLD/SELL; PE / IB outputs use PROCEED/PASS/REVISIT; wealth outputs use APPROVED/CONDITIONAL/REJECTED. The CLI subcommand or MCP tool that produces the output is responsible for the mapping; the `RunSummary` aggregate validates that the value belongs to the surface event's permitted set.

### Indexing trigger

Indexing happens at the close of the surface event, after the output is written and the `.audit.json` companion (per ADR-017, emitted by the plugin Write/Edit hook) is sealed. The trigger sequence is:

1. CLI subcommand or MCP tool handler completes and writes outputs to `output_dir`.
2. Plugin Write/Edit hook fires on the output file and emits the `.audit.json` companion.
3. The CLI binary or MCP wrapper writes `run_summary.json`.
4. `corp-finance-core::memory::TrajectoryIndex::ingest(&run_summary)` is invoked.
5. The native backend computes the embedding (via the configured embedding model), writes the `RunSummary` record into the on-disk HNSW index (`hnsw_rs`) and the `tantivy` BM25 inverted index, and updates the `petgraph` adjacency for surface-event / entity / sub-agent / MCP-tool edges.
6. On commit, the `surface_event_indexed` integration event is emitted.

Failures at step 5 are non-blocking for the surface event itself (the output already exists) but raise an `audit_failure` event consumed by ADR-017's observability path. The retry policy is 3 attempts with exponential backoff; persistent failure leaves the `run_summary.json` on disk with an `indexed: false` sidecar to be picked up by a daily reconciliation job.

### Retention policy

| Tier | Storage | Retention | Access |
|------|---------|-----------|--------|
| Hot | Native `hnsw_rs` index plus `tantivy` BM25 inverted index, both rooted at `<repo>/var/memory/` | 90 days from `ts` | Sub-second similarity queries |
| Cold | Object store archive (`run_summary.json` + `*.audit.json` only; output blobs follow the surface event's own retention) | 7 years (regulatory minimum) | Re-ingestion required for retrieval |

The 90-day hot window is chosen because it covers a full quarter of analyst workflow plus the typical IC memo refresh cycle. The 7-year cold retention aligns with FINRA / SEC books-and-records requirements (17 CFR 240.17a-4) and is comfortably above MiFID II's 5-year threshold.

A daily compaction job (out of scope for Phase 26 — to be specified in Phase 27) moves entries past 90 days from hot to cold. For Phase 26 we ship the boundary and the schema; the compaction is manual.

### Cross-session persistence (portable session archive)

Long-running analyst sessions (e.g., a multi-day diligence with intermediate state) persist via a portable session archive: a JSON document compressed with `flate2` gzip, written with extension `.cfa-session` (RVF-compatible structure where feasible). The boundary is:

- `corp-finance-core::memory::CfaSession::save(session_id) -> PathBuf` writes a `.cfa-session` archive.
- `corp-finance-core::memory::CfaSession::restore(session_id)` reads and inflates the archive.
- The session captures: working notes, intermediate computations, partial outputs, the active CLI subcommand, and the sequence of MCP tool calls made so far. It does **not** capture credentials, vendor API keys, or PII (filtered by ADR-017's native PII / injection scanner pre-write hook).
- An optional cargo feature `rvf_backend` swaps the format for the `rvf-cli` crate's RVF format if a deployment wants strict compatibility with the upstream ruflo session format.

Round-trip correctness is enforced by contract test RUF-MEM-005 (see `docs/contracts/feature_memory.yml`).

### Cross-surface retrieval (hybrid + graph)

The `SimilarRunQuery` aggregate exposes one operation: `find_similar(query, k, filters) -> Vec<RunSummary>`. Implementation is native: `tantivy` (BM25 keyword search) and `hnsw_rs` (vector similarity) candidate lists are merged by reciprocal-rank fusion, then a hand-rolled MMR re-ranker (lambda configurable in [0,1]) removes near-duplicates. `petgraph` traversal over surface-event → ticker, surface-event → sub-agent, and surface-event → MCP-tool edges adds a graph-based secondary retrieval path; the union of vector + BM25 + 1-hop graph candidates is the candidate pool fed to MMR.

Filters supported in the ACL:

- `surface` (cli | mcp | plugin)
- `surface_event_id` (exact match — CLI subcommand path or MCP tool name)
- `ticker` (exact match)
- `ts` range (after / before)
- `recommendation` (exact match)
- `surface_audit_hash` prefix (for "find runs of this surface-event manifest version")

The query interface is exposed as a CLI subcommand `cfa memory find` and as MCP tool `surface_memory_find`. Both delegate through the same ACL.

## Rationale

### Why a canonical `run_summary.json`?

A flat schema disjoint from the output prose makes indexing tractable and audit-friendly. The output prose is unbounded, narrative, and varies per surface event; the summary is bounded, structured, and uniform. Putting the structured fields in a separate file means we can hash, index, and compare without parsing markdown.

### Why HNSW (native via `hnsw_rs`)?

HNSW is the de-facto standard for approximate nearest-neighbour search at the scale we expect (10^4 to 10^6 surface events over 7 years). The `hnsw_rs` crate is a mature, well-maintained Rust implementation; using it as a library keeps the index in-process, avoids an external MCP runtime in production, and gives us deterministic builds. An optional `agentdb_backend` cargo feature can route through the `agentdb` npm package (or the `ruvector-core` crate) for installations that want to share an index with a separately-managed ruflo agentdb backend.

### Why 90 days hot?

A quarterly IC cycle plus a buffer for re-opens. Shorter would lose recent context for active deals; longer inflates the HNSW index and degrades query latency. 90 days is a Phase 26 starting point and may be tuned in Phase 27.

### Why an ACL even though the v1 backend is native?

Domain types stay clean. The ACL exists as a future-proofing boundary should we later choose to integrate the ruflo MCP backend or another vector store; for v1 the modules are first-class Rust types with no external dependency. Constraining the blast radius to one module means a future backend swap is a focused change, not a cross-repo refactor.

### Why surface-keyed indexing rather than cookbook-keyed?

Cookbooks are deployment artefacts, not runtime events. A `cfa managed-agent deploy <slug>` invocation is itself a CLI surface event and is captured as such; the deployed managed agent runs in Anthropic's infrastructure and is outside our memory scope. Keying on the runtime surface makes capture uniform across all 200+ CLI subcommands and ~594 MCP tools without special-casing the cookbook deploy path.

## Consequences

### Positive

- Every CLI invocation and MCP tool call that matters is retrievable, comparable, and replayable.
- Compliance can answer "show me runs that produced X recommendation on Y issuer" without grepping output files, regardless of which surface produced the output.
- Analysts get cross-surface context: "what was our PE view of this issuer when equity was last covered" becomes a single RAG query that spans CLI, MCP, and skill-driven invocations alike.
- Long-running diligence survives session boundaries via the portable session archive.
- The ACL keeps finance-domain types decoupled from ruflo's wire format.

### Negative

- One additional file (`run_summary.json`) per emitting surface event; the wrapper must produce it correctly or fail validation.
- The 90-day hot window is a guess; will need tuning based on real query patterns.
- The portable session archive introduces a state file that must be excluded from version control and from PII export paths.
- Indexing failures, while non-blocking, accumulate work for the daily reconciliation job; if that job is not built in Phase 27 it becomes a debt.
- Hybrid retriever ranking parameters (BM25 b/k1, HNSW M/efConstruction, MMR lambda) are now ours to tune.
- The MCP wrapper must decide per tool whether to emit a `RunSummary` (TODO: configurable per-tool emit policy default — pending product input).

### Risks

- **Schema drift**: surface events added in later phases may want fields not in v1.0. Mitigated by `schema_version` and additive evolution rules — new fields are optional, removed fields are forbidden until a major version bump.
- **PII leaks into `run_summary.json`**: client names, account numbers, etc. could end up in `key_assumptions`. Mitigated by ADR-017's native PII / injection scanner pre-write hook, which scans the summary file before persistence.
- **Index corruption**: HNSW or BM25 index corruption would force a full rebuild from cold storage. Mitigated by retaining `run_summary.json` on disk regardless of index state.
- **Crate-version drift**: a major-version bump in `hnsw_rs` or `tantivy` could require an index migration. Mitigated by version pinning and integration tests.

## Implementation Notes

- Module: `corp-finance-core::memory` (new in Phase 26).
- Aggregates: `RunSummary`, `TrajectoryIndex`, `SimilarRunQuery`, `CfaSession`.
- Native crate dependencies (added to `crates/corp-finance-core/Cargo.toml`):
  - `hnsw_rs = "0.x"` — HNSW index
  - `tantivy = "0.x"` — BM25 inverted index
  - `petgraph = "0.x"` — surface-event / entity / sub-agent / MCP-tool graph
  - `flate2 = "1"` — portable session archive compression
  - `uuid = { version = "1", features = ["v7"] }` — `run_id`
- Backend abstraction:
  - `corp_finance_core::memory::backend::native_hnsw` (default)
  - `corp_finance_core::memory::backend::ruflo_agentdb` (optional `agentdb_backend` cargo feature; calls the `agentdb` npm via MCP or the `ruvector-core` crate)
  - `corp_finance_core::memory::backend::ruflo_rvf` (optional `rvf_backend` cargo feature; calls the `rvf-cli` crate)
- ACL functions (only used when an optional backend feature is enabled): `to_agentdb_record`, `from_agentdb_record`, `to_rvf_session`, `from_rvf_session`.
- Capture wiring:
  - CLI capture point: end of `crates/corp-finance-cli/src/main.rs` invocation, after subcommand dispatch returns.
  - MCP capture point: shared `packages/*-mcp-server/src/wrapper.ts` (TODO: confirm shared wrapper module name; sister agents may have moved this) wraps every `server.tool(name, schema, handler)` and ingests on completion.
  - Plugin: no direct capture; plugin emits `.audit.json` only.
- CLI subcommands (new):
  - `cfa memory ingest --run-summary <path>`
  - `cfa memory find --query <text> [--surface cli|mcp] [--surface-event-id <id>] [--ticker <ticker>] [--k 5]`
  - `cfa session save --session-id <id>`
  - `cfa session restore --session-id <id>`
- MCP tools (added to `cfa-core` MCP server):
  - `surface_memory_ingest`
  - `surface_memory_find`
  - `surface_session_save`
  - `surface_session_restore`
- Domain events emitted: `surface_event_indexed`, `cfa_session_saved`, `cfa_session_restored`, `similar_runs_retrieved`.
- Contract IDs: RUF-MEM-001 through RUF-MEM-N (see `docs/contracts/feature_memory.yml`).
- DDD bounded context: `docs/ddd/domain-memory.md`.

## Related Decisions

- **ADR-015** — Native Orchestration / Memory / Audit Layer Inspired by Ruflo Concepts (umbrella decision; pins the four runtime surfaces; this ADR is the memory slice).
- **ADR-017** — Audit / Cost / Observability (the `.audit.json` companion file referenced here, emitted by the plugin Write/Edit hook; the native PII pre-write hook).
- **ADR-009** — Workflow Auditability (djb2 hashing reused for `surface_audit_hash`, `sources_hash`, `input_hash`).

## References

- Concept inspiration (not runtime dependencies in default builds): https://github.com/ruvnet/ruflo
- Optional backend artefacts: `agentdb@^3.0.0-alpha.14` (npm), `ruvector-core@2.2.0` (crates.io), `rvf-cli@0.1.0` (crates.io)
- `hnsw_rs`, `tantivy`, `petgraph`, `flate2` — crate documentation on docs.rs
- 17 CFR 240.17a-4: SEC books-and-records retention rules.
- Reference deployment templates (deployment artefacts, not runtime): `managed-agent-cookbooks/`.
