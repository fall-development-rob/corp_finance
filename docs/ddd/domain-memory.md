# Domain Model: Surface-Invocation Memory

## Bounded Context: Memory

This bounded context handles the canonical record, persistence, and retrieval of CFA runtime activity at the surface level. It is the durable substrate that lets the platform answer "what happened in past invocations", "what is the most similar past invocation", and "restore my deal workspace from last week".

The runtime activity captured by this context is exclusively from the four CFA surfaces:

- **CLI** (`cfa <subcommand>`)
- **MCP** (every `server.tool(...)` registration in `packages/*-mcp-server/src/`)
- **Skills** (slash commands and `.claude/skills/*` invoked by the LLM via the Skill tool)
- **Plugin** (PreToolUse / PostToolUse / Write / Edit hooks at `plugins/cfa-core/hooks/hooks.json`)

Cookbooks (`managed-agent-cookbooks/`) are deployment artefacts and outside this bounded context's runtime scope.

The context is implemented as native Rust modules using mature crates (`hnsw_rs` for HNSW, `tantivy` for BM25, `petgraph` for graph traversal, `flate2` for portable session archives). An anti-corruption layer wraps the surface event format so domain types stay clean of any external-MCP-server-shape leakage; an optional cargo feature can swap the backend for `agentdb` npm or the `rvf-cli` crate. See ADR-015 for the umbrella decision and ADR-016 for the architectural detail.

### Domain Language (Ubiquitous Language)

| Term | Definition |
|------|-----------|
| **Surface Invocation** | A single CLI subcommand call, OR a single MCP tool handler execution, OR a plugin hook fire. The unit of indexing and retrieval. Skills are recorded via the MCP wrapper through which they execute. |
| **Run Summary** | The canonical structured record of one Surface Invocation; the unit indexed in the trajectory store. |
| **Surface Audit Hash** | djb2 fingerprint over the surface configuration that produced the invocation (CLI subcommand version, MCP tool registration, plugin hook configuration). |
| **Trajectory** | The sequence of MCP tool calls, sub-agent invocations, and intermediate states that an invocation traversed. The HNSW index is a trajectory index. |
| **CfaSession** | A persisted, restorable workspace — typically a multi-day diligence with intermediate state. |
| **Portable Session Archive** | The native `.cfa-session` archive format (JSON + `flate2` gzip). An optional `rvf_backend` cargo feature can swap the format for the upstream `rvf-cli` crate's RVF format. |
| **Similar Run** | A past `RunSummary` retrieved via hybrid (BM25 + vector) search and ranked by composite similarity to a query. |
| **MMR Diversity** | Maximal Marginal Relevance — retrieval re-ranking that penalises near-duplicates so the returned set covers different aspects. |
| **Graph Retrieval** | Retrieval that traverses graph edges (invocation -> ticker, invocation -> sub-agent, invocation -> skill) via `petgraph` in addition to vector similarity. |

### Aggregates

#### RunSummary (Aggregate Root)

The canonical record of a single Surface Invocation. The aggregate root for the Memory context. A RunSummary may originate from any of the four surfaces; the `surface` field discriminates.

| Field | Type | Description |
|-------|------|-------------|
| `run_id` | `Uuid` (v7) | Unique identifier; v7 sorts lexicographically by time |
| `ts` | `DateTime<Utc>` | Invocation start timestamp |
| `surface` | `Surface` | `Cli`, `Mcp`, `Skill`, or `Plugin` |
| `surface_event_id` | `String` | The CLI subcommand name, MCP tool name, slash command name, or plugin hook id |
| `surface_audit_hash` | `String` | `djb2:0x...` per ADR-017 |
| `model` | `String` | Model identifier including 1M-context flag where applicable |
| `ticker` | `Option<String>` | Asset identifier where applicable |
| `asset_class` | `Option<String>` | `equity`, `credit`, `pe`, `wealth`, etc. |
| `recommendation` | `Recommendation` | Normalised decision token where applicable |
| `price_target` | `Option<Decimal>` | Where applicable |
| `target_currency` | `Option<String>` | ISO 4217 |
| `horizon_months` | `Option<u32>` | Recommendation horizon |
| `key_assumptions` | `Vec<String>` | Free-form analyst assumptions |
| `sources_hash` | `String` | `djb2:0x...` over the source-document set |
| `skills_invoked` | `Vec<String>` | Sorted list |
| `mcp_tools_invoked` | `Vec<String>` | Sorted list |
| `sub_agents` | `Vec<String>` | Sorted list |
| `input_hash` | `String` | `djb2:0x...` over the canonical user input |
| `output_dir` | `String` | Filesystem path to the invocation's output directory (CLI) or owning workspace (MCP / Skill / Plugin) |
| `status` | `Status` | `completed`, `failed`, `partial` |
| `duration_ms` | `u64` | Wall-clock duration |
| `token_usage` | `TokenUsage` | Input, output, cache_read counts |

**Invariants enforced at the aggregate root**:

- `surface_audit_hash` matches the result of `compute_surface_hash()` over the surface configuration used in the invocation.
- `recommendation` value (when present) is in the permitted set declared by the surface family (BUY/HOLD/SELL for equity research, PROCEED/PASS/REVISIT for PE, etc.).
- `sources_hash` is non-empty if and only if the invocation consumed external sources.
- `output_dir` exists at `ts` (CLI / Skill case) or references an existing workspace path (MCP / Plugin case).
- `status == completed` implies `duration_ms > 0`.
- All fields marked required (`run_id`, `ts`, `surface`, `surface_event_id`, `surface_audit_hash`, `model`, `sources_hash`, `status`) are non-null.

#### TrajectoryIndex (Aggregate)

The HNSW-indexed view of past invocations. Backed by the native `hnsw_rs` crate (default); an optional `agentdb_backend` cargo feature can route through the `agentdb` npm or `ruvector-core` crate.

| Field | Type | Description |
|-------|------|-------------|
| `index_id` | `String` | Stable identifier for the trajectory index instance |
| `embedding_dim` | `u32` | Vector dimensionality |
| `entries` | Conceptual; managed by the native `hnsw_rs` index | One entry per `RunSummary` |
| `hot_window_days` | `u32` | Default 90 (per ADR-016) |

**Commands**:

- `ingest(run_summary)` — adds a `RunSummary` to the index.
- `evict_cold(before_ts)` — moves entries older than the hot window to cold archive.
- `rebuild_from_archive(window)` — restores from cold storage on index corruption.

**Invariants**:

- Every `RunSummary` indexed in the hot window has a matching `run_summary.json` on disk.
- Index entries past `hot_window_days` are eligible for eviction; eviction does not delete the underlying file.
- `embedding_dim` is constant for the lifetime of an index instance; a dim change forces a rebuild.

#### SimilarRunQuery (Aggregate)

A query against the trajectory index. Backed by the native hybrid retriever combining `tantivy` (BM25), `hnsw_rs` (vector), `petgraph` (1-hop graph), reciprocal-rank fusion, and a hand-rolled MMR re-ranker.

| Field | Type | Description |
|-------|------|-------------|
| `query_text` | `String` | Free-form natural language query |
| `query_embedding` | `Vec<f32>` | Computed by the configured embedding model |
| `filters` | `QueryFilters` | Surface, surface_event_id, ticker, ts range, recommendation, audit-hash prefix |
| `k` | `u32` | Number of results to return; default 5 |
| `mmr_lambda` | `f32` | Diversity weight in [0, 1]; default 0.5 |
| `results` | `Vec<RunSummary>` | Returned set, ranked by composite similarity |

**Commands**:

- `find_similar()` — executes the query and populates `results`.

**Invariants**:

- `k >= 1`.
- `mmr_lambda` in [0, 1].
- `results.len() <= k`.
- Filters are AND-combined.

#### CfaSession (Aggregate)

A persisted workspace state. Backed by the native portable session archive (JSON + `flate2` gzip); an optional `rvf_backend` feature can swap the format for the upstream `rvf-cli` crate.

| Field | Type | Description |
|-------|------|-------------|
| `session_id` | `String` | Analyst-supplied or auto-generated id |
| `surface` | `Surface` | Surface that owns the session |
| `surface_event_id` | `String` | Originating surface target (subcommand / tool / slash command) |
| `created_at` | `DateTime<Utc>` | First save |
| `updated_at` | `DateTime<Utc>` | Most recent save |
| `working_notes` | `String` | Free-form notes |
| `intermediate_outputs` | `Vec<IntermediateArtefact>` | Partial outputs not yet promoted to final |
| `archive_path` | `String` | Path to the `.cfa-session` archive file backing this session |

**Commands**:

- `save()` — serialises to a `.cfa-session` archive (JSON + `flate2` gzip).
- `restore(session_id)` — inflates the archive and reconstructs the aggregate.

**Invariants**:

- `created_at <= updated_at`.
- `archive_path` exists after the first successful `save()`.
- The archive file does not contain credentials or PII (enforced by the native pre-memory-write hook in ADR-017).
- Round-trip: `restore(save())` produces an aggregate field-equal to the original.

### Value Objects

#### Surface

Enum: `Cli`, `Mcp`, `Skill`, `Plugin`. The four runtime entry points. Shared kernel with the Multi-Agent Coordination context (see `domain-orchestration.md`).

#### Recommendation

| Field | Type | Description |
|-------|------|-------------|
| `family` | `String` | `equity`, `pe`, `wealth`, etc. |
| `value` | `String` | Permitted token within the family's set |

Permitted sets:

- `equity`: `BUY`, `HOLD`, `SELL`, `NOT_RATED`
- `pe`: `PROCEED`, `PASS`, `REVISIT`
- `wealth`: `APPROVED`, `CONDITIONAL`, `REJECTED`
- `credit`: `APPROVE`, `DECLINE`, `WATCH`
- `ib`: `ACCEPT`, `DECLINE`, `MODIFY`

#### TokenUsage

| Field | Type | Description |
|-------|------|-------------|
| `input_tokens` | `u64` | Anthropic API input count |
| `output_tokens` | `u64` | Anthropic API output count |
| `cache_read_tokens` | `u64` | Cache hits |

#### QueryFilters

| Field | Type | Description |
|-------|------|-------------|
| `surface` | `Option<Surface>` | Exact match |
| `surface_event_id` | `Option<String>` | Exact match |
| `ticker` | `Option<String>` | Exact match |
| `ts_after` | `Option<DateTime<Utc>>` | Inclusive |
| `ts_before` | `Option<DateTime<Utc>>` | Inclusive |
| `recommendation` | `Option<String>` | Exact match on `value` |
| `surface_audit_hash_prefix` | `Option<String>` | Prefix match |

#### IntermediateArtefact

| Field | Type | Description |
|-------|------|-------------|
| `artefact_id` | `String` | Local id within the session |
| `kind` | `String` | `note`, `partial_model`, `draft_section`, etc. |
| `payload` | `String` | UTF-8 content; opaque to the aggregate |

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `cli_invocation_completed` | CLI surface | `run_id`, `surface_event_id`, `surface_audit_hash`, `ts`, `status` | Memory (ingest), Audit, Observability |
| `mcp_tool_completed` | MCP surface (every `server.tool` handler) | `run_id`, `surface_event_id`, `surface_audit_hash`, `ts`, `status` | Memory (ingest), Audit, Observability |
| `plugin_hook_fired` | Plugin surface (PreToolUse / PostToolUse / Write / Edit) | `run_id`, `surface_event_id`, `surface_audit_hash`, `ts`, `status` | Memory (ingest), Audit, Observability |
| `run_indexed` | TrajectoryIndex | `run_id`, `surface`, `surface_event_id`, `surface_audit_hash`, `ts` | Audit context, Cost context |
| `cfa_session_saved` | CfaSession | `session_id`, `surface`, `archive_path`, `updated_at` | Audit context |
| `cfa_session_restored` | CfaSession | `session_id`, `surface`, `restored_at` | Observability context |
| `similar_runs_retrieved` | SimilarRunQuery | `query_text` (redacted), `k`, `result_count`, `latency_ms` | Observability context |
| `audit_failure` | TrajectoryIndex (on ingest fail) | `run_id`, `error_kind`, `attempts` | Audit context, Observability context |

There are no `cookbook_deploy_started` / `cookbook_deploy_completed` events: cookbooks are deployment artefacts and run outside this context's runtime scope.

### Anti-Corruption Layer

The `corp_finance_core::memory` Rust module is the boundary. Its primary job is to wrap the surface event format so domain types stay clean of any external-MCP-server-shape leakage: events arriving from MCP handlers, CLI subcommands, and plugin hooks are translated into `RunSummary`, `Surface`, and `IntermediateArtefact` domain types at the module edge.

The default v1 implementation is fully native; with optional cargo features (`agentdb_backend`, `rvf_backend`), the same domain types translate at the wire boundary only — no external backend types leak into Memory consumers.

| Direction | ACL function | Wire format |
|-----------|--------------|-------------|
| MCP tool handler completion → RunSummary | `from_mcp_tool_event(&Value) -> RunSummary` | Validated against the MCP completion event JSON shape |
| CLI subcommand completion → RunSummary | `from_cli_event(&CliEvent) -> RunSummary` | Direct from the CLI event struct |
| Plugin hook fire → RunSummary | `from_plugin_hook(&HookEvent) -> RunSummary` | Direct from `plugins/cfa-core/hooks/hooks.json` payload |
| RunSummary → agentdb backend (optional) | `to_agentdb_record(&RunSummary)` | JSON document with embedding-relevant fields flattened |
| agentdb backend → RunSummary (optional) | `from_agentdb_record(&Value)` | Reverse mapping; rejects unknown fields |
| Session → RVF (optional) | `to_rvf_session(&Session)` | RVF frame produced via `rvf-cli` |
| RVF → Session (optional) | `from_rvf_session(&Bytes)` | Reverse mapping; validates round-trip |

### Context Map

```
+--------------------------------------------------------------+
|                   Memory Bounded Context                      |
|                                                               |
|  +------------------+        +-----------------------+        |
|  |   RunSummary     |<-------|   TrajectoryIndex     |        |
|  |   (root)         |        |   (HNSW via           |        |
|  |  surface = CLI |  |        |    hnsw_rs crate)     |        |
|  |    MCP|Skill|Plug|        +-----------+-----------+        |
|  +--------+---------+                    |                    |
|           |                              v                    |
|           v                    +-----------+-----------+      |
|  +--------+---------+          |  SimilarRunQuery      |      |
|  |    CfaSession    |          |  (BM25 via tantivy +  |      |
|  | (.cfa-session    |          |   HNSW + petgraph     |      |
|  |  archive,        |          |   + MMR re-rank)      |      |
|  |  flate2 gzip)    |          +-----------------------+      |
|  +------------------+                                         |
|                                                               |
+----------------+----------------------------+-----------------+
                 |                            |
                 | run_indexed                | similar_runs_retrieved
                 | session_saved              | audit_failure
                 v                            v
        +--------+------------+      +--------+-----------+
        |   Audit Context     |      | Observability      |
        |   (ADR-017)         |      |   Context (ADR-017)|
        +---------------------+      +--------------------+
                 ^
                 | surface_audit_hash (computed in ADR-017)
                 |
        +--------+------------+
        |  Workflow Audit     |
        |  Context (ADR-009)  |
        +---------------------+
```

### Relationship to Other Contexts

| Upstream | Downstream | Relationship | Detail |
|----------|------------|--------------|--------|
| CLI / MCP / Skill / Plugin surfaces | Memory | Customer/Supplier | Each surface emits a completion event at the end of an invocation; Memory is the customer that translates and ingests it. Skills inherit via the MCP wrapper through which they execute. |
| Multi-Agent Coordination (`domain-orchestration.md`) | Memory | Shared Kernel | Shares the `EntityRef` value object; Memory persists entity references extracted by Coordination. |
| Memory | Audit / Observability (ADR-017) | Publisher/Subscriber | Memory publishes `run_indexed` and `audit_failure`; Audit consumes for the audit ledger; Observability consumes for span enrichment. |
| Memory | Workflow Audit (ADR-009) | Conformist | Memory adopts the djb2 hashing format for `surface_audit_hash`, `sources_hash`, `input_hash`. |
| Self-Learning (`domain-self-learning.md`) | Memory | Customer/Supplier | Self-Learning consumes `RunSummary` records to assemble Trajectories. |
| Optional ruflo MCP backend | Memory | Anti-Corruption Layer | When the `agentdb_backend` or `rvf_backend` cargo feature is enabled, JSON wire formats stay behind the ACL; default v1 builds use the native backend with no external dependency. |
