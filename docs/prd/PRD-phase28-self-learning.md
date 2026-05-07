# PRD: Phase 28 — Self-Learning Loop

## Overview

Phase 28 closes the self-learning loop on top of the Phase 26 surface-level memory infrastructure (ADR-015, ADR-016, ADR-017) and the Phase 27 multi-agent coordination layer (ADR-018, ADR-019). It builds a native Rust self-learning module at `corp_finance_core::self_learning`: trajectory capture and retrieval reuse the Phase 26 native HNSW + BM25 + petgraph store; trajectory clustering uses hand-rolled k-means; chief-analyst goal decomposition uses A* via the `pathfinding` crate over the **MCP-tool + slash-command action space**; replay-driven contract tests are guarded by ed25519-signed golden-set manifests (`ed25519-dalek`); financial-domain pattern-detection rules ride over the Phase 27 native entity graph.

See ADR-020 for the architecture decision and option analysis.

The CFA runtime is exclusively four surfaces: CLI, MCP, Skills, Plugin. Trajectories are captured at every surface event boundary (CLI invocation completion, MCP tool handler return, slash-command emission, plugin hook fire). Cookbooks under `managed-agent-cookbooks/` are deployment artefacts; their deploys are CLI invocations (`cfa managed-agent deploy`) and inherit surface-level trajectory capture.

## Problem Statement

After Phase 27, the platform has durable per-surface memory and cross-specialist entity tracking, but four feedback-loop gaps remain:

1. Two analysts running the same CLI subcommand or invoking the same MCP tool on adjacent inputs each rediscover the same tool path; trajectory bias is missing at the surface level.
2. Multi-domain queries to the chief-analyst rely on ad-hoc in-prompt planning; no auditable plan tree over the MCP-tool + slash-command action space exists.
3. Surface entry-point drift (CLI subcommand spec, MCP tool registration, plugin hook config, slash-command body) can ship to production silently; no replay-driven regression guard at the CLI/MCP boundary exists.
4. The Phase 27 native entity graph is a passive store; no domain-rule layer turns recurring entity co-occurrence across surfaces into actionable signals.

Phase 28 closes all four.

## User Stories

1. **As an equity analyst**, when I run the `/cfa:initiate-coverage` slash command on a new ticker in a sector where the same skill has already been invoked 5+ times, I want the slash-command emit path to retrieve my prior thesis trajectories and bias toward the trajectory shape that produced the highest-grade past output, so that I do not redo work I have already done. (RUF-LEARN-001, RUF-LEARN-002)

2. **As a chief analyst**, when I receive a multi-domain query, I want a printable A* plan tree over the MCP-tool + slash-command action space showing exactly which MCP tools and slash commands will execute and in what order, with explicit dependencies, before I approve the run. (RUF-LEARN-003, RUF-LEARN-004)

3. **As a chief analyst**, when an MCP tool call or slash command in the plan returns insufficient evidence (eval-grade below floor or required entities missing from the entity graph), I want the planner to automatically replan and re-route, up to a bounded number of replans (default 3), so that the run completes without my manual intervention. (RUF-LEARN-005)

4. **As an engineer**, I want a CLI / MCP / plugin / skill surface to fail closed if its byte-diff vs the golden-set captured payload (CLI subcommand output, MCP tool return, plugin hook output, or slash-command emission) exceeds a configurable threshold (default 5%), so that surface drift never ships to production silently. (RUF-LEARN-006, RUF-LEARN-007)

5. **As a compliance officer**, I want every surface event to capture a trajectory record with required fields `{surface, surface_event_id, input_hash, tool_call_sequence, output_hash, eval_grade, tenant_id, run_id}`, so that I can audit any historical CLI invocation, MCP tool call, plugin hook fire, or skill execution. (RUF-LEARN-008)

6. **As a compliance officer**, I want golden-set manifests to be signed and immutable post-acceptance; any tampering must invalidate CI runs until re-signed by an accepted ADR. (RUF-LEARN-009)

7. **As a credit analyst**, I want the system to emit a covenant-deterioration signal when a portfolio issuer's credit metrics trend negatively across consecutive surface events (multiple CLI invocations of credit subcommands, multiple credit MCP tool calls), so that I can react before the next IC. (RUF-LEARN-010)

8. **As a portfolio manager**, I want sector-rotation early signals when issuers within a sector cluster show co-movement in the native entity graph aggregated across surfaces. (RUF-LEARN-011)

9. **As a risk officer**, I want multi-issuer correlation alerts when graph-edge frequency between two issuers crosses a threshold across surface events. (RUF-LEARN-012)

10. **As a tenant administrator**, I want trajectory data and goal plans scoped per tenant such that no bias from tenant A's surface history influences tenant B's planning, consistent with ADR-019 federation rules. (RUF-LEARN-013)

11. **As a chief analyst**, when I have already approved a plan tree for a similar goal in the past, I want subsequent matching goals to auto-approve based on plan hash, so that routine multi-domain queries do not require re-review. (success metric: plan-cache hit rate)

12. **As an observability engineer**, I want trajectory captures, plan generations, replans, drift reports, and signal emissions all surfaced in the existing observability dashboard from Phase 26 (ADR-017), so that one pane of glass covers the self-learning loop end to end across all four surfaces.

## Non-Functional Requirements

| Requirement | Target |
|-------------|--------|
| Trajectory capture latency | <50 ms added to surface event completion path; capture is asynchronous so it does not block CLI return / MCP tool return / plugin hook completion. |
| Trajectory retrieval latency | <200 ms for top-3 candidates; reuses the Phase 26 native hybrid retriever. |
| Plan generation latency | <500 ms for goals decomposing to <=10 action-space steps (A* via the `pathfinding` crate). |
| Replay drift compute | <2 s per surface entry-point in CI; parallelizable across CLI subcommands, MCP tools, plugin hooks, and slash commands. |
| Pattern detection schedule | Runs on a 15-minute interval over the native entity graph; results visible within one cycle. |
| Storage growth | Linear in surface-event volume; expected growth fits within the Phase 26 native HNSW + BM25 capacity plan with margin. |

## Observability

- Trajectory captures, plan generations, replans, drift events, and signal emissions are all instrumented with the Phase 26 observability hooks (ADR-017) at every surface boundary.
- The observability dashboard gains four panels: trajectory volume per surface (CLI / MCP / plugin / skill), plan-tree branching factor distribution, drift ratio histogram per surface entry-point, and per-rule-family signal counts.
- Plan-tree audit log is queryable by goal hash; chief-analyst can pull the decision history for any past multi-domain query.

## Features

### Trajectory Learning

- **Trajectory capture hook** at every surface event boundary: CLI subcommand completion (via the Phase 26 CLI tracing wrapper at `crates/corp-finance-cli/src/main.rs`), MCP tool handler return (via the Phase 26 MCP wrapper in `packages/*-mcp-server/src/`), plugin hook completion (via `plugins/cfa-core/hooks/hooks.json` `PostToolUse`), and slash-command emit completion. Schema: `{surface, surface_event_id, input_hash, tool_call_sequence[], output_hash, eval_grade, tenant_id, run_id, timestamp}`. Persisted to the Phase 26 native memory store.
- **Trajectory clustering** runs as a background worker via hand-rolled k-means over trajectory embeddings, partitioned by surface. Clusters require >=10 trajectories at eval-grade B or higher before they influence planning.
- **Trajectory retrieval** at the start of a CLI subcommand, MCP tool call, or slash-command emission via `retrieve_similar_trajectories(input, surface, surface_event_id, k=3)`. Returns >=3 candidates when 10 or more prior surface events exist for the same `(surface, surface_event_id)` pair. Reuses `corp_finance_core::memory::SimilarRunQuery`.
- **Tool-selection bias** from retrieved cluster centroids is exposed to the surface entry point via the self-learning module; the surface implementation may consult bias and override when domain context demands.

### Goal Decomposition

- **A* plan generation** via the `pathfinding` crate for chief-analyst multi-domain queries. Action space is the union of registered MCP tools (across the four MCP servers) and registered slash commands (under `.claude/commands/cfa/`), each annotated with preconditions and postconditions. Goal decomposition (turning a natural-language ask into a goal predicate) is performed by the LLM at chief-analyst level.
- **Plan-tree review checkpoint**: plans are printed to CLI stdout and require chief-analyst approval before any MCP tool or slash command is invoked.
- **Bounded replanning**: default max 3 replans per goal; configurable per query but bounded.
- **Plan audit**: every generated and replanned plan is serialized to the Phase 26 native memory store with a deterministic plan hash for audit.

### Replay-Driven Contract Tests

- **Golden-set capture** at acceptance: 10 frozen inputs per surface entry-point (per CLI subcommand, per MCP tool, per plugin hook, per slash command), with expected output payload.
- **Manifest signing**: `tests/golden-sets/<surface>/<surface_event_id>/manifest.sig` signed via `ed25519-dalek`; signature verification on every CI run.
- **Drift threshold**: default 5%, configurable per surface entry-point with chief-analyst sign-off. Drift = byte-diff plus recursive `serde_json::Value` structural diff.
- **CI gate**: drift breach blocks deploy; dev-mode breach emits warning only.
- **Replay command**: `cfa replay-golden-set --surface <surface> --surface-id <id>` runs locally for development.

### Financial-Domain Pattern Detection

- **Covenant deterioration rules**: per-issuer trend detection over credit metrics emitted across surface events (CLI credit subcommands, credit MCP tool calls, credit slash-command emissions).
- **Sector rotation early signals**: cluster co-movement detection across issuers tagged with the same sector aggregated over the cross-surface entity graph.
- **Multi-issuer correlation alerts**: graph-edge frequency thresholds emit a signal node when two issuers co-occur above threshold within a window across surface events.
- Signals are written back as typed nodes in the native entity graph; consumed by Phase 27 chief-analyst coordination.

## Acceptance Criteria

| Specflow ID | Criterion |
|-------------|-----------|
| RUF-LEARN-001 | Every surface event (CLI invocation, MCP tool call, plugin hook fire, slash-command emit) captures a trajectory record with all required fields. |
| RUF-LEARN-002 | Trajectory retrieval for similar inputs returns >=3 candidates when >=10 prior surface events exist for the same `(surface, surface_event_id)` pair. |
| RUF-LEARN-003 | The A* planner produces a plan tree before any MCP tool or slash command is invoked when chief-analyst receives a multi-domain query. |
| RUF-LEARN-004 | Plan trees include explicit dependencies between steps; preconditions of each step are satisfied by upstream postconditions or goal context. |
| RUF-LEARN-005 | Replanning triggers automatically when a plan step returns eval-grade below floor or required entities missing; bounded to default 3 replans. |
| RUF-LEARN-006 | Replay byte-diff against the golden-set captured payload is computed for every CLI subcommand, MCP tool, plugin hook, and slash command on every CI run. |
| RUF-LEARN-007 | Drift threshold breach (default 5%) fails CI deploy; dev mode emits warning only. |
| RUF-LEARN-008 | Trajectory schema is enforced; surface events lacking eval_grade or tenant_id cannot be persisted. |
| RUF-LEARN-009 | Golden-set manifests are signed; signature mismatch fails CI until re-signed via accepted ADR. |
| RUF-LEARN-010 | Covenant deterioration rule emits a signal when an issuer's credit metric trend across consecutive surface events crosses the configured negative-trend threshold. |
| RUF-LEARN-011 | Sector rotation early signal emits when >=N issuers within a sector cluster show co-movement above threshold across surface events. |
| RUF-LEARN-012 | Multi-issuer correlation alert emits when graph-edge frequency between two issuers crosses the configured threshold within a window. |
| RUF-LEARN-013 | Trajectory and plan data are tenant-scoped; cross-tenant reads are rejected. |

## Estimated Work

5 days, assuming Phase 26 (ADR-015, ADR-016, ADR-017) and Phase 27 (ADR-018, ADR-019) are merged.

| Day | Scope |
|-----|-------|
| 1 | Self-learning module scaffold (`corp_finance_core::self_learning`); trajectory schema with `surface` + `surface_event_id`; capture hook wired into the four surface wrappers (CLI tracing wrapper, MCP `server.tool` wrapper, plugin hook emitter, slash-command emit wrapper). New cargo deps: `pathfinding = "0.x"`, `ed25519-dalek = "2"`. |
| 2 | Trajectory persistence to Phase 26 native memory store; retrieval API keyed by `(surface, surface_event_id)`; native k-means clustering worker partitioned by surface; planning-bias plumbing. |
| 3 | A* planner via `pathfinding` crate over the **MCP-tool + slash-command action space** (precondition/postcondition annotations read from `packages/*-mcp-server/src/` registrations and `.claude/commands/cfa/`); plan-tree generation; replan loop; chief-analyst review checkpoint. |
| 4 | Golden-set capture tooling per surface entry-point; ed25519 manifest signing; replay harness with byte-diff + recursive `serde_json::Value` diff at CLI / MCP / plugin / skill boundaries; CI gate with threshold config. |
| 5 | Pattern-detection rules (covenant, sector rotation, correlation) over the cross-surface entity graph; native entity-graph signal writes; tenant-scoping verification; specflow tests. |

## Success Metrics

- **Tool-call reduction**: >=30% reduction in tool-call count on repeat-pattern surface events (measured against the trajectory baseline captured during the first 30 days post-deploy). Repeat-pattern is defined as inputs whose retrieval matches >=3 prior trajectories for the same `(surface, surface_event_id)` pair.
- **Plan auditability**: 100% of multi-domain chief-analyst queries produce an A* plan tree reviewable before execution; 0 silent multi-domain runs.
- **Drift detection coverage**: replay drift detection blocks >=95% of regressions in the test set without false positives. False positive rate is measured as drift-blocked CI runs that, on chief-analyst review, are accepted.
- **Domain signal precision**: of emitted covenant-deterioration / sector-rotation / correlation signals, >=80% are confirmed actionable by the consuming analyst on first review.
- **Tenant isolation**: 0 cross-tenant trajectory leaks in audit; verified by an automated test in CI.
- **Module purity**: 0 plugin imports anywhere in `corp_finance_core` (the v1 implementation is fully native); verified by a clippy lint or equivalent CI grep.

## Surface Annotation Requirement

Every surface entry-point in the action space (each MCP tool registration in `packages/*-mcp-server/src/` and each slash command under `.claude/commands/cfa/`) must declare preconditions and postconditions. Without these annotations the A* planner cannot reason over the action space. A CI lint enforces presence of the annotations; missing annotations fail the build.

Example annotation shape (illustrative):

- preconditions: `[issuer_identified, sector_known]`
- postconditions: `[credit_metrics_attached, issuer_thesis_drafted]`

The annotation vocabulary is closed and curated; new precondition / postcondition tags require an ADR amendment so that the action space stays consistent across MCP tools and slash commands.

## Out of Scope

- Online reinforcement learning; trajectory bias is read-only batch.
- Cross-tenant trajectory sharing; ADR-019 tenant scoping is preserved.
- Heuristic tuning of A* beyond the `pathfinding` crate's default; deferred to a future ADR if the action space grows past 100.
- SONA-style neural pattern training; deferred to a future ADR if v1 k-means underperforms.
- Pattern rules outside the three families specified (covenant deterioration, sector rotation, multi-issuer correlation).
- Generation of Excel/PowerPoint output formats (continues to be markdown-text per Phase 20).
- Cookbook-level golden-set capture; cookbooks are deployment artefacts. Cookbook deploys are CLI invocations (`cfa managed-agent deploy`) and inherit surface-level golden-set capture for the CLI subcommand surface. Cookbook content drift detection is out of scope.

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| k-means cluster bias produces poor tool selection | eval-grade floor (default B/70) on training data; replay-driven contract tests catch regressions; surface implementations may override bias when domain context demands. |
| Golden-set maintenance overhead across four surfaces | Tooling automates capture from accepted runs per surface; ed25519 signing is one command; the golden-set update workflow is documented in surface-owner runbooks. |
| Plan tree audit fatigue | Concise plan-tree printer; chief-analyst can pre-approve plans matching a hash from a prior approved plan; routine plans cache to the native memory store and auto-approve on hash match. |
| Crate-version drift | `pathfinding` and `ed25519-dalek` are version-pinned in Cargo.toml; major-version bumps go through a dedicated PR that runs the replay-driven contract tests. |
| Pattern false positives | Threshold-based rules are calibrated against the first 30 days of production data before signals route to alerts. |
| Tenant data leakage | RUF-LEARN-013 contract test runs in CI; ADR-019 federation rules carried forward; no read API in self_learning accepts a query without tenant_id (RUF-LEARN-INV-009). |
| Replan loop runaway | Bounded by configurable max (default 3); exceeding the bound aborts the run with an error rather than silently looping. |

## Dependencies

| Dependency | Source | Status |
|-----------|--------|--------|
| Native memory store (HNSW + BM25 + petgraph) | Phase 26 (ADR-016) | Required; Phase 28 reuses, does not provision a new store. |
| run_summary.json schema and hybrid retriever | Phase 26 (ADR-015, ADR-016) | Required; trajectory retrieval reuses the hybrid graph + vector search. |
| Surface event emit hooks (CLI / MCP / plugin / skill) | Phase 26 (ADR-017) | Required; trajectory capture attaches at every surface boundary. |
| Multi-agent coordination layer | Phase 27 (ADR-018) | Required; plan trees are emitted to chief-analyst from this layer; pattern signals feed back into it. |
| Tenant federation primitives | Phase 27 (ADR-019) | Required; trajectory and plan data inherit tenant scoping. |
| `pathfinding` crate (A*) | crates.io | New cargo dep in Phase 28. |
| `ed25519-dalek` crate | crates.io | New cargo dep in Phase 28 (signing of golden-set manifests). |
| Native entity graph | Phase 27 | Read for entity nodes; write for signal nodes. |

## Test Plan

| Layer | Coverage |
|-------|----------|
| Unit (Rust) | Trajectory schema serialization with `surface`+`surface_event_id`; tenant-scoping checks; pattern-rule logic for the three families; plan-hash determinism. |
| Integration | End-to-end capture -> retrieve cycle with seeded surface events across all four surfaces; plan-tree generation for representative multi-domain goals; replan trigger paths; golden-set replay against captured payloads at every surface boundary; signal emission flowing into the entity graph. |
| Contract (specflow) | All RUF-LEARN-001 through RUF-LEARN-013 contracts pass; all twelve invariants hold. |
| CI gate | Replay drift gate fails closed in CI; emits warning only in dev. Manifest signing verification on every CI run. |

## Rollout

- Phase 28 ships behind a feature flag in `corp_finance_core::self_learning::config` so trajectory capture can be enabled per tenant during the first 30 days.
- k-means cluster bias is enabled tenant-by-tenant after the first 10 trajectories per `(surface, surface_event_id)` accumulate at eval-grade B or higher.
- Drift threshold defaults to a conservative 5%; per-surface-entry-point overrides require chief-analyst approval recorded in the surface registry.
- Pattern-rule alerts route to the observability dashboard in shadow mode for the first 14 days post-deploy.
- The A* planner is enabled for chief-analyst multi-domain queries from day one; routine single-surface queries bypass the planner so there is no latency impact on the common path.
- Replay-driven contract tests run in shadow on the first CI build after merge to surface drift in existing CLI / MCP / plugin / skill entry points; thresholds are tuned during this window before fail-closed is activated.
