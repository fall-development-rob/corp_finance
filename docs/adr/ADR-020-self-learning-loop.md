# ADR-020: Native Self-Learning Loop Over CLI / MCP / Skill Surface Events

## Status: Accepted

## Date: 2026-05-06

## Deciders

- CFA Agent platform owners
- Chief analyst orchestration owner
- Compliance / audit owner

## Context

Phase 26 (ADR-015 native orchestration umbrella, ADR-016 memory architecture, ADR-017 audit / cost / observability / security) gave every CLI invocation, every MCP tool call, and every plugin hook fire a durable footprint. Each surface event now emits a `run_summary.json` indexed in our native HNSW + BM25 store and a `.audit.json` companion via the plugin Write/Edit hook. Phase 27 (ADR-018 multi-agent coordination via existing surfaces, ADR-019 federation and tenant scoping) added native chief-analyst → specialist coordination through Claude Code's Agent tool, the native `petgraph` entity store, and tenant scoping at every output / state boundary across the four runtime surfaces.

What is still open after Phase 27:

| Gap | Why It Matters |
|-----|---------------|
| No trajectory learning | Two analysts running similar slash commands or invoking similar MCP tool sequences in the same sector each rediscover the same path. The Phase 26 hybrid retriever can surface a prior `RunSummary`, but the agent has no mechanism to bias its tool selection toward trajectories that historically scored highest on eval. |
| No goal decomposition | The chief-analyst agent receives natural-language multi-domain goals ("write a coverage initiation on PFE including credit, ESG, and macro context") and constructs an ad-hoc plan in-prompt. There is no auditable plan tree, no dependency graph between MCP tools and slash commands, and no replanning loop when a specialist returns insufficient evidence. |
| No regression guard for surface-event drift | CLI subcommand definitions, MCP tool registrations, and skill prompts evolve. Phase 26 captures every surface event, but nothing compares the byte content of a current CLI/MCP invocation against a frozen golden-set input from acceptance. Silent drift can ship to production. |
| No cross-surface signal layer | Phase 27 extracts entities (issuer, sector, instrument) into the native entity graph at the chief-analyst aggregation step, but there is no rule layer that turns recurring entity co-occurrence into actionable signals (covenant deterioration patterns, sector rotation early warnings, multi-issuer correlation alerts). |

The first two gaps were originally ruflo territory: `ruflo-intelligence` advertises SONA neural pattern training and trajectory learning, and `ruflo-goals` advertises GOAP A* planning. The May 2026 smoke test (`/tmp/ruflo-smoke-test.md`) showed neither plugin ships as a separately-installable artefact: `ruflo-intelligence` is monorepo-source-only (capabilities surfaced through the central CLI's `hooks_intelligence_*` MCP tools, with MoE and Flash Attention "not loaded"), and `ruflo-goals` has no installable plugin and no GOAP A* MCP tool — the closest match (`autopilot_predict`) is Q-Learning, not A*. We therefore build native Rust modules drawing inspiration from those plugin designs (trajectory schema shape, GOAP-style action space + A* search) but without runtime coupling.

## Decision

Build a native self-learning module at `corp_finance_core::self_learning` inspired by `ruflo-intelligence` (trajectory learning) and `ruflo-goals` (GOAP A* planning) concepts. Reuse the Phase 26 native memory backend (HNSW + BM25 + petgraph) for trajectory storage and retrieval. Implement A* over the **MCP tool registry + slash command catalogue** (the runtime action space, ~620 actions) using the `pathfinding` crate. Capture trajectory embeddings and apply a hand-rolled k-means / nearest-neighbor cluster step for pattern extraction; we do not implement SONA neural pattern training in v1.

A trajectory in this system is the sequence of (CLI subcommand | MCP tool | skill) calls leading to a deliverable. Capture is automatic at the Phase 26 surface wrappers (CLI binary entry, MCP `server.tool` wrapper, plugin hook). No special per-surface code is needed.

Phase 28 closes the self-learning loop with four native modules:

1. **Trajectory capture and retrieval**. Every CLI invocation that produces an output, and every MCP tool call that produces a non-trivial result, contributes a step to the active trajectory. At surface-event completion, the trajectory record is written to the Phase 26 native memory store. The trajectory schema is `{trajectory_id, surface_event_chain[], input_hash, output_hash, eval_grade, tenant_id, run_id}` where each element of `surface_event_chain` is `{surface, surface_event_id, ts}`. Trajectories inherit tenant scoping from ADR-019. A background worker computes trajectory embeddings and applies k-means clustering. Future surface events query past trajectories at the start of the run via `corp_finance_core::memory::SimilarRunQuery` and bias planning toward shapes with the highest eval-grade.

2. **Goal decomposition (native A*)**. The chief-analyst agent gains a native planning layer using the `pathfinding` crate's A* implementation over the **MCP tool registry + slash command catalogue** as the action space. Each MCP tool and each slash command is annotated with declared preconditions and postconditions; A* finds a minimum-cost plan satisfying the goal predicate. Plan trees are emitted to stdout for chief-analyst review before any specialist agent is invoked. Goal decomposition (turning a natural-language ask into a goal predicate) is performed by the LLM at chief-analyst level. When a specialist returns insufficient evidence (eval-grade below threshold, or required entities missing from the entity graph), the planner re-runs A* with the failed step's postconditions removed and any partial state retained. This module is consumed by ADR-018 (multi-agent coordination via existing surfaces).

3. **Replay-driven contract tests**. A frozen golden-set of 10 inputs per CLI subcommand and per MCP tool is captured at acceptance. CI replays each golden input against the current surface event handler and computes a byte-diff plus a structural-diff via recursive `serde_json::Value` comparison; the surface event fails closed if the combined drift ratio exceeds a configurable threshold (default 5%). The golden-set manifest is signed (ed25519 via `ed25519-dalek`); tampering invalidates the run. Replay reuses the Phase 26 trajectory storage for input/output capture. Drift detection runs at the CLI / MCP boundary: golden-set inputs feed the same wrappers production traffic does, expected output digests are compared against fresh outputs.

4. **Financial-domain pattern detection (in-house)**. `corp_finance_core::self_learning::patterns` reads the native entity-graph nodes (Phase 27 `entity_graph` module from ADR-018) and emits cross-surface signals via three rule families: covenant deterioration (issuer-level metric trend rules), sector rotation early signals (issuer-cluster co-movement rules), multi-issuer correlation alerts (graph-edge-frequency rules). Signals are written back to the entity graph as typed nodes and consumed by chief-analyst's aggregation step.

### Integration Boundaries

| Capability | Native module | Inspired by | What is out of scope |
|------------|---------------|-------------|----------------------|
| Trajectory capture / retrieval | `corp_finance_core::self_learning::trajectory_repo` over the Phase 26 native memory store (`hnsw_rs` + `tantivy`) | `ruflo-intelligence` schema and SONA concept | SONA neural pattern training; embedding model selection (we use the same model as the memory layer); online RL. |
| Trajectory clustering (planning-bias source) | `corp_finance_core::self_learning::clusters` (hand-rolled k-means over trajectory embeddings) | SONA pattern shapes | Neural pattern matching; deferred to a future ADR if v1 clusters underperform. |
| Goal decomposition | `corp_finance_core::multi_agent::goap_adapter` + `corp_finance_core::multi_agent::planner` (`pathfinding` crate's A*; MCP tool registry + slash command catalogue as action space) — owned by ADR-018, consumed here | `ruflo-goals` GOAP A* design | Heuristic tuning beyond admissible Manhattan-style cost; HTN; behavior trees. |
| Native entity graph | `corp_finance_core::multi_agent::entity_graph` (`petgraph` + tag extractor) — owned by ADR-018, consumed here | Generic graph traversal patterns | Generic NLP entity extraction. |
| Pattern rules | `corp_finance_core::self_learning::patterns/` (covenant_deterioration, sector_rotation, correlation_alert) | Generic graph traversal patterns | Pattern rule families beyond the three specified; deferred to a future ADR. |
| Memory store + tenant scoping | Phase 26 ADR-016 + Phase 27 ADR-019 | -- | -- |

The Rust module at `corp_finance_core::self_learning` exposes:

- `capture_trajectory_step(surface: Surface, event_id: &str, input: &Input, output: &Output) -> StepId`
- `finalize_trajectory(trajectory_id: TrajectoryId, eval: EvalGrade) -> ()`
- `retrieve_similar_trajectories(input: &Input, k: usize) -> Vec<TrajectoryRef>`
- `build_plan(goal: &Goal, action_space: &ActionSpace) -> GoapPlan`
- `replan(plan: &GoapPlan, failed_step: &PlanStep, evidence: &EvidenceState) -> GoapPlan`
- `replay_golden_set(surface_event_id: &str) -> ReplayReport`
- `detect_patterns(graph: &EntityGraphView) -> Vec<DomainSignal>`

All inputs and outputs are domain types defined in our crates. The `ActionSpace` is constructed from the live MCP tool registry plus the slash-command catalogue at `.claude/commands/cfa/`.

### What We Build Ourselves

| Component | Location | Reason |
|-----------|----------|--------|
| Self-learning module root | `crates/corp-finance-core/src/self_learning/mod.rs` | Native module; ACL boundary is preserved as future-proofing. |
| Trajectory schema and repo | `crates/corp-finance-core/src/self_learning/trajectory.rs` | Surface-event-keyed (`surface`, `surface_event_id`, `eval_grade`, tenant scoping); persisted in Phase 26 memory store. |
| Trajectory clustering | `crates/corp-finance-core/src/self_learning/clusters.rs` | Hand-rolled k-means over trajectory embeddings. |
| Action space + A* planner | `crates/corp-finance-core/src/multi_agent/goap_adapter.rs` and `crates/corp-finance-core/src/multi_agent/planner.rs` | Maps the MCP tool registry + slash-command catalogue to the GOAP action format and runs A* via the `pathfinding` crate. Owned by ADR-018. |
| Native entity graph | `crates/corp-finance-core/src/multi_agent/entity_graph.rs` | `petgraph` + tag extractor. Owned by ADR-018. |
| Domain pattern rules | `crates/corp-finance-core/src/self_learning/patterns/` | Covenant deterioration, sector rotation, multi-issuer correlation. Finance rules, not generic graph traversal. |
| Replay harness | `scripts/replay-golden-set.sh` and CI integration | Byte-diff plus `serde_json::Value` recursive structural diff; signed-manifest verification; thresholding at the CLI / MCP boundary. |
| Golden-set manifests | `tests/golden-sets/<surface>/<surface_event_id>/` | Frozen, signed (ed25519 via `ed25519-dalek`), immutable post-acceptance. |

### What We Reuse From Earlier Phases

- **Phase 26 (ADR-015, ADR-016, ADR-017)**: native memory store (HNSW + BM25 + petgraph), `run_summary.json` schema, hybrid retriever, cost and observability instrumentation. Trajectories are stored in the same store the run summaries already use; no new database is provisioned. The hybrid retriever defined in ADR-016 already satisfies the trajectory-retrieval lookup pattern. Trajectory step capture is wired at the same CLI binary entry, MCP `server.tool` wrapper, and plugin hook fire points the run-summary capture uses.
- **Phase 27 (ADR-018, ADR-019)**: chief-analyst → specialist routing through Claude Code's Agent tool consumes the A* planner at `corp_finance_core::multi_agent::goap_adapter`; the entity-graph module at `corp_finance_core::multi_agent::entity_graph` is owned by ADR-018 and consumed by self-learning pattern detection. Trajectories and goal plans inherit tenant scoping from ADR-019.

### Data Flow

```
   cfa <subcommand>           MCP tool call            chief-analyst multi-domain query
   (CLI binary entry)         (MCP wrapper)                   |
        |                           |                         v
        v                           v               +--------------------------+
   +------------------+   +--------------------+    | build_plan(goal,         |
   | capture_         |   | capture_           |    |   action_space)          |
   | trajectory_step  |   | trajectory_step    |    | -> A* Plan               |
   +-----+------------+   +-----+--------------+    +-----------+--------------+
         |                      |                               |
         +----------+-----------+                               v
                    v                                +--------------------------+
         +------------------+                        | plan tree -> stdout      |
         | native memory    |                        | (chief-analyst review)   |
         | (ADR-016 HNSW    |                        +-----------+--------------+
         |  + BM25)         |                                    |
         +-----+------------+                                    v
               |                                       +--------------------------+
               v                                       | specialist invocation    |
         +------------------+                          | (via Agent tool)         |
         | k-means cluster  |                          | + replan on failure      |
         | (native worker)  |                          +-----------+--------------+
         +-----+------------+                                      |
               |                                                   v
               v                                       +--------------------------+
         +-----------------------+                     | finalize_trajectory(     |
         | retrieve_similar_     |<--------------------|   eval_grade)            |
         | trajectories(input,k) |  (planning bias on  +--------------------------+
         +-----------------------+   next surface event)

   Native entity graph (ADR-018) ---> detect_patterns() ----> signal nodes
                                                              (back into graph;
                                                               consumed by chief-analyst
                                                               aggregation step)
```

### Implementation Plan

5 days of engineering, sequenced to keep the integration boundary clean:

| Day | Scope |
|-----|-------|
| 1 | Self-learning module scaffold at `corp_finance_core::self_learning`. Trajectory schema and persistence wired to the Phase 26 native memory store. Capture step wired at the CLI binary entry, MCP `server.tool` wrapper, and plugin hook fire — reusing the same wrappers Phase 26 introduced. |
| 2 | Trajectory retrieval and k-means clustering worker; `retrieve_similar_trajectories` returns top-k references; planning-bias plumbing exposed to the MCP wrapper. |
| 3 | A* planner via the `pathfinding` crate: MCP tool registry + slash command catalogue annotated with preconditions and postconditions; `build_plan` emits plan trees; chief-analyst review checkpoint added; replan loop wired with the configured bound (default 3). Native entity graph (`corp_finance_core::multi_agent::entity_graph`, `petgraph` + tag extractor) is owned by ADR-018; self-learning consumes it. |
| 4 | Golden-set capture tooling; ed25519 manifest signing utilities (`ed25519-dalek`); replay harness with byte-diff + recursive `serde_json::Value` diff at the CLI / MCP boundary; CI integration with configurable threshold (default 5%); fail-closed gate in CI, warn-only in dev. |
| 5 | Pattern-detection rules: covenant deterioration, sector rotation, multi-issuer correlation. Native entity-graph signal-node writes. Tenant-scoping verification test. Specflow contract suite green. |

### Out of Scope for Phase 28

- SONA-style neural pattern training; deferred to a future ADR if v1 k-means underperforms.
- Custom planner heuristic beyond the `pathfinding` crate's default A*; tuning is post-v1.
- Online reinforcement learning; trajectory bias is read-only batch.
- Cross-tenant trajectory sharing; tenant scoping from ADR-019 is preserved.
- Pattern rule families beyond the three specified (covenant deterioration, sector rotation, multi-issuer correlation); additional rule families are deferred.
- Generation of Excel/PowerPoint output formats; output remains markdown text per Phase 20.
- Trajectory capture from deployed managed-agent cookbooks. Cookbooks deploy via `cfa managed-agent deploy` (a CLI invocation captured at the CLI surface), but once deployed they run in Anthropic's infrastructure and we do not capture their internal trajectories.

## Consequences

### Positive

- Trajectory bias measurably reduces tool-call count on repeat-pattern surface events (target: 30% reduction; see PRD success metrics).
- Chief-analyst plan trees over MCP tools and slash commands are auditable before execution; multi-domain queries no longer rely on ad-hoc in-prompt planning.
- Replay-driven contract tests catch surface-event drift (CLI subcommand definitions, MCP tool registrations) before it ships to production.
- Domain-pattern signals turn the entity graph from a passive store into an active alerting layer (covenant deterioration, sector rotation, correlation alerts).
- Engineering effort: 5 days. Building uses mature crates (`pathfinding` for A*, `ed25519-dalek` for signatures); the only hand-rolled algorithms are k-means clustering (~100 LoC) and the three pattern rule families (~300 LoC each).
- Native code keeps surface-event handlers free of external plugin types; if a future ADR swaps the planner or clustering implementation, the swap is local to one module.

### Negative

- Adds well-known crate dependencies (`pathfinding`, `ed25519-dalek`) — both stable, widely-deployed.
- Trajectory storage scales with surface-event volume; native HNSW + BM25 capacity planning from ADR-016 must absorb the extra rows. At expected volume (single-digit thousand surface events/month/tenant) this is comfortably within the Phase 26 budget.
- Golden-set maintenance is a real operational cost. Every accepted CLI subcommand or MCP tool change requires regenerating the golden manifest, signing it, and committing.
- k-means cluster quality is parameter-sensitive (k, distance metric, init strategy); we own that tuning. Mitigated by the eval-grade filter (only high-grade trajectories influence bias) and by replay tests catching regressions.
- A* over the MCP tool + slash command action space assumes well-typed preconditions and postconditions. We must annotate every MCP tool and every slash command with them, which is an upfront one-time cost.

## Options Considered

### Option 1 (chosen): Build native modules using `pathfinding` (A*) + hand-rolled k-means + Phase 26 native memory; capture trajectories at the existing surface wrappers

Lowest engineering cost (5 days), proven Rust crate dependencies, no runtime coupling to ruflo, no special per-surface capture code. Selected.

### Option 2: Adopt ruflo-intelligence + ruflo-goals as runtime substrate (Rejected after smoke test)

May 2026 smoke test (`/tmp/ruflo-smoke-test.md`) showed `ruflo-intelligence` is monorepo-source-only with MoE and Flash Attention "not loaded", and `ruflo-goals` has no GOAP A* MCP tool. The closest substitute (`autopilot_predict`) is Q-Learning, not A*. Adopting these as runtime would mean either vendoring monorepo source or building shims against capabilities that do not exist as advertised.

### Option 3: Build trajectory learning natively; defer goal planning

Trajectory bias provides immediate value, and chief-analyst could continue to plan in-prompt. Rejected because plan auditability is a compliance requirement raised by the audit owner; deferring it leaves a multi-domain query gap that worsens as more specialist routes are added. Native A* via `pathfinding` is 1-2 days of additional work — keep it in scope.

### Option 4: Build goal planning natively; skip trajectory learning

Plan trees would be auditable but without trajectory bias the tool-call count remains high on repeat-pattern surface events. Rejected because the highest-frequency cost in production is repeat-pattern runs (initiate-coverage on adjacent tickers, earnings updates within a sector); trajectory bias is where most of the cost reduction comes from.

### Option 5: Use a different planner (e.g., HTN, Behavior Trees) instead of GOAP A*

GOAP A* is sufficient for surface-event-as-action planning; HTN adds complexity (task decomposition trees) we do not need at the current MCP tool + slash command count. Behavior Trees are better suited to reactive runtime control than to one-shot plan generation. Rejected on fit.

### Option 6: Action space = cookbook registry instead of MCP tools + slash commands (Rejected)

The cookbook registry is a deploy-time artefact catalogue, not a runtime action space. The LLM cannot directly invoke a cookbook from the chief-analyst session; what it can invoke are MCP tools and slash commands. Planning over the runtime action space matches the LLM's actual capabilities and produces plans the LLM can execute. The cookbook registry remains relevant only at validate-time when a user assembles a deploy payload via `cfa managed-agent`.

## Risks and Open Questions

| Risk / Question | Disposition |
|-----------------|-------------|
| Cluster bias surfaces stale tool paths after an MCP tool is materially refactored | Trajectories are tagged with the MCP tool's `surface_audit_hash` (ADR-017); bias is filtered to trajectories whose hash matches the current registration by default. Older trajectories remain in the memory store for audit but do not influence planning until a manual override is set. |
| GOAP A* search blows up the action space | The action space is currently ~620 actions (~594 MCP tools + ~25 CFA slash commands + a small fixed overhead). A* with the precondition / postcondition annotations terminates well within the `pathfinding` crate's default budget. If the action space grows past 5,000, plan caching by goal hash and a hand-tuned heuristic may be needed; revisit at that scale. |
| Pattern rule thresholds are wrong out of the gate | All three rule families ship in shadow mode for the first 14 days post-deploy; analyst confirmation feedback recalibrates thresholds before signals route to active alerting channels. |
| MCP tool authors and slash command authors forget to annotate preconditions / postconditions | A CI lint enforces that every MCP tool and every slash command in the action space carries the annotation; missing annotation fails the build. |
| Crate upgrade breaks A* or signing | `pathfinding` and `ed25519-dalek` are version-pinned in `Cargo.toml`; major-version bumps go through a dedicated PR that runs the replay-driven contract tests. |
| Replay harness boundary | Replay invokes CLI binaries and MCP tool handlers directly with golden-set inputs; the wrappers from ADR-015 / ADR-017 are the same code production traffic exercises. No separate replay code path. |

## Wave 3 Amendment (2026-05-07): replay argv-mode expansion

### Context

The original ADR-020 replay dispatcher (Phase 28) fed golden-set inputs to subprocess stdin because the first batch of replayable surface events were stdin-fed CLI subcommands. Post-Phase-28 coverage analysis showed that the majority of `cfa` subcommands consume flag-based argv, not stdin JSON. The stdin-only dispatcher therefore covered only a small fraction of CLI surface events, leaving most subcommands outside the replay safety net.

### Decision

Extend `cfa replay run` with an `--argv-mode <stdin|flags|template>` flag (default `stdin`) and a companion `--argv-template <STRING>` flag (required when mode is `template`).

- **stdin** (default) — unchanged; JSON written to subprocess stdin. All existing golden-sets work without modification.
- **flags** — top-level JSON object keys are kebab-cased and emitted as `--key value` argv pairs. Boolean values emit the flag name only (true) or omit it (false); arrays emit repeated pairs; nested objects are rejected with a hard error before subprocess invocation.
- **template** — `{key}` placeholders in the template string are substituted with top-level JSON scalars, then whitespace-split into argv; unknown keys are a hard error before subprocess invocation.

Additionally, multi-word `--target` strings (e.g. `--target "workflow audit"`) are split on whitespace before being prepended to the rendered argv vector.

### Consequences

Replay coverage extends to every CLI subcommand regardless of its input shape. Template mode handles edge cases (positional args, repeated flags) that flags-mode cannot express. Backwards compatibility is preserved: stdin remains the default and no existing golden-set requires migration.

New contract identifiers covering this amendment: RUF-LEARN-014, RUF-LEARN-015, RUF-LEARN-016 (feature contracts) and RUF-LEARN-INV-013, RUF-LEARN-INV-014, RUF-LEARN-INV-015 (invariants).

## Related Decisions

- ADR-015: Native Orchestration Umbrella (Phase 26 — pins the four runtime surfaces; trajectory capture is wired at those wrappers)
- ADR-016: Memory Architecture (Phase 26 — HNSW + BM25 + petgraph store; trajectories live here)
- ADR-017: Audit / Cost / Observability (Phase 26 — the surface-event capture points used to capture trajectory steps; `surface_audit_hash` for trajectory filtering)
- ADR-018 (revised): Multi-Agent Coordination via Existing Surfaces (Phase 27 — consumes the A* planner and entity graph defined here)
- ADR-019: Federation and Tenant Scoping (Phase 27 — trajectory data and goal plans inherit tenant scoping)
- ADR-009: Workflow Auditability (audit hashing patterns reused for golden-set manifest signing)
- ADR-008: Financial Workflow Integration (workflow skill structure that contributes to the slash command portion of the action space)

## References

- Concept inspiration (not runtime dependencies): https://github.com/ruvnet/ruflo (`ruflo-intelligence` for trajectory schema shape, `ruflo-goals` for GOAP A* design)
- Smoke test findings (May 2026): `/tmp/ruflo-smoke-test.md`
- `pathfinding`, `ed25519-dalek` -- crate documentation on docs.rs
- MADR template: <https://adr.github.io/madr/>
- MCP tool registry: `packages/mcp-server/src/`, `packages/fmp-mcp-server/src/`, `packages/data-mcp-server/src/`, `packages/vendor-mcp-server/src/`
- Slash command catalogue: `.claude/commands/cfa/*.md`
- CLI binary entry: `crates/corp-finance-cli/src/main.rs`
