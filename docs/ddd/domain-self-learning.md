# Domain Model: Self-Learning Loop

## Bounded Context: Self-Learning

This bounded context closes the CFA feedback loop. It captures runtime trajectories from the four CFA surfaces, distills recurring shapes via native k-means clustering over trajectory embeddings, decomposes natural-language goals into auditable A* plan trees over the action space (using the `pathfinding` crate), freezes golden-set replay tests to detect prompt drift (with ed25519-signed manifests via `ed25519-dalek`), and emits financial-domain signals from the native entity graph back into Multi-Agent Coordination.

The runtime activity captured is exclusively from the four CFA surfaces:

- **CLI** (`cfa <subcommand>`)
- **MCP** (every `server.tool(...)` registration in `packages/*-mcp-server/src/`)
- **Skills** (slash commands and `.claude/skills/*` invoked by the LLM via the Skill tool — recorded via the MCP wrapper)
- **Plugin** (PreToolUse / PostToolUse / Write / Edit hooks at `plugins/cfa-core/hooks/hooks.json`)

Cookbooks (`managed-agent-cookbooks/`) are deployment artefacts and outside this bounded context's runtime scope.

The context is implemented as native Rust modules. The ACL boundary at `corp_finance_core::self_learning` wraps the surface event format so domain types stay clean of any external-MCP-server-shape leakage; v1 builds depend only on `pathfinding`, `ed25519-dalek`, and the native memory store. Concept inspiration: `ruflo-intelligence` (trajectory schema shape) and `ruflo-goals` (GOAP A* design).

### Domain Language (Ubiquitous Language)

| Term | Definition |
|------|-----------|
| **Surface Event** | A single CLI subcommand call, OR a single MCP tool handler execution, OR a skill invocation (recorded via its MCP wrapper), OR a plugin hook fire. |
| **Trajectory** | The complete record of one user-facing session: an ordered sequence of Surface Events plus the final eval-grade. Immutable once written. |
| **Eval Grade** | Quality score for a completed trajectory (A/B/C/D/F or numeric 0-100), assigned by the responsible agent's quality gates plus chief-analyst review when applicable. |
| **Trajectory Cluster** | A recurring shape extracted from many trajectories by native k-means clustering over trajectory embeddings. Used to bias future runs toward high-grade tool paths. |
| **Action** | One step in the action space: an MCP tool call OR a slash command invocation. Annotated with preconditions and postconditions for planning. |
| **GOAP Plan** | An A* plan tree over the action space (MCP tools and slash commands). Each node is one action; edges encode dependencies. |
| **Replan** | The act of regenerating part of a GOAP plan when an action returns insufficient evidence. |
| **Golden Set** | A frozen, signed bundle of `{golden input, expected action sequence, expected output bundle}` per CFA surface target, captured at acceptance. Immutable post-acceptance. |
| **Drift Detection** | Byte-level comparison of a current surface invocation's output bundle against the golden-set bundle. Threshold-based pass/fail. |
| **Domain Signal** | A typed cross-invocation pattern emitted from the entity graph (covenant deterioration, sector rotation early signal, multi-issuer correlation alert). |

### Aggregates

#### Trajectory Aggregate

- **Root**: `Trajectory`
- **Entities**: `SurfaceEventRef` (ordered: each is a CLI subcommand call, MCP tool call, skill invocation, or plugin hook fire), `EvalGrade`, `RunMetadata` (surface, surface_event_id, tenant_id, run_id, timestamp)
- **Value Objects**: `InputHash`, `OutputHash`, `TrajectoryId`
- **Repository**: native memory store (HNSW + BM25 + petgraph from ADR-016) via `corp_finance_core::self_learning::trajectory_repo`
- **Invariants**:
  - Trajectory is immutable once captured; no in-place updates
  - Every trajectory carries the tenant_id (per `domain-federation.md`); cross-tenant reads are forbidden
  - input_hash and output_hash are content-addressed (SHA-256)
  - eval_grade is required; sessions without an evaluation cannot be persisted as trajectories
  - SurfaceEventRef sequence is non-empty; preserves the order in which the events fired

#### TrajectoryCluster Aggregate

- **Root**: `TrajectoryCluster`
- **Entities**: `PatternShape` (cluster of trajectory shapes), `BiasWeights` (per-action weighting derived from eval-grade distribution within the cluster)
- **Value Objects**: `ClusterId`, `Centroid`, `ConfidenceScore`
- **Repository**: native k-means worker writes centroids to the native memory store; cluster membership is recomputed on a configurable schedule via `corp_finance_core::self_learning::clusters`
- **Invariants**:
  - A cluster must reference at least N trajectories (configurable, default 10) before it influences planning
  - Clusters are tenant-scoped; bias from tenant A's clusters never leaks into tenant B's planning
  - Clusters derived only from trajectories above an eval-grade floor (default B or 70/100)

#### GoapPlan Aggregate

- **Root**: `GoapPlan`
- **Entities**: `PlanStep` (one action in the action space), `Dependency` (edge between steps), `Goal` (predicate the plan satisfies)
- **Value Objects**: `PlanId`, `PlanHash`, `ReplanCount`
- **Repository**: in-memory during the user-facing session; serialized to the native memory store for audit. The planner uses the `pathfinding` crate's A* implementation over the union of `(MCP tools registered in packages/*-mcp-server) ∪ (slash commands under .claude/commands/cfa)`.
- **Invariants**:
  - A plan must be emitted to stdout for chief-analyst review before any action is invoked
  - Each step's preconditions must be satisfied by either the goal context or an upstream step's postconditions
  - Replanning is bounded (default max 3 replans per goal)
  - Plan hash is deterministic given the same goal and the same action-space registry version

Note: the GoapPlan aggregate is shared with the Multi-Agent Coordination context (`domain-orchestration.md`); Self-Learning consumes plan results to feed the planner forward, while Multi-Agent Coordination owns plan execution.

#### GoldenSet Aggregate

- **Root**: `GoldenSet` (per CFA surface target — a CLI subcommand, an MCP tool, or a slash command)
- **Entities**: `GoldenInput` (10 frozen inputs), `GoldenActionSequence`, `GoldenOutputBundle`
- **Value Objects**: `ManifestHash`, `Signature`, `AcceptanceTimestamp`
- **Repository**: filesystem under `tests/golden-sets/<surface>/<surface_event_id>/` with a signed manifest
- **Invariants**:
  - Golden sets are immutable post-acceptance; signature mismatch fails CI
  - Each surface target with golden-set replay enabled must have exactly 10 golden inputs
  - Manifest signing uses ed25519 (`ed25519-dalek`); key is rotated only at accepted ADR; key rotation invalidates all manifests until re-signed

#### DriftDetection Aggregate

- **Root**: `DriftReport`
- **Entities**: `ByteDiff`, `Threshold`, `Verdict` (Pass/Warn/Fail)
- **Value Objects**: `DriftRatio` (0.0-1.0), `SurfaceTarget`
- **Repository**: CI artifact storage; latest report kept in the native memory store for the observability dashboard
- **Invariants**:
  - Drift threshold default is 5%; configurable per surface target with chief-analyst sign-off
  - Threshold breach in CI blocks deploy (fail closed)
  - Threshold breach in dev environment emits a warning, never blocks
  - Drift verdict is computed deterministically from a byte-diff plus a recursive `serde_json::Value` structural diff; the same algorithm runs across environments

### Domain Events

| Event | Producer | Consumers |
|-------|----------|-----------|
| `trajectory_captured` | Self-Learning trajectory_repo (subscribed to surface completion events from Memory) | native k-means worker, observability dashboard |
| `pattern_extracted` | native k-means worker | Future surface invocations (planning bias query) |
| `goal_received` | chief-analyst | native A* planner (`pathfinding` crate) |
| `plan_generated` | native A* planner | chief-analyst (review), audit log, Multi-Agent Coordination |
| `plan_replanned` | native A* planner | chief-analyst (review), audit log |
| `golden_set_updated` | Acceptance workflow (manual ADR step) | CI replay job |
| `drift_detected` | CI replay job | Deploy gate, observability dashboard |
| `drift_blocked_deploy` | Deploy gate | Engineer (CI failure), observability dashboard |
| `domain_signal_emitted` | `corp_finance_core::self_learning::patterns` | Native entity graph (signal node), Multi-Agent Coordination |

### Anti-Corruption Layer

The Rust module `corp_finance_core::self_learning` is implemented as native first-class types. The ACL wraps the surface event format so domain types (Trajectory, GoapPlan, GoldenSet) stay clean of any external-MCP-server-shape leakage: surface events arriving from Memory are translated into `SurfaceEventRef` and `Trajectory` domain types at the module edge.

v1 builds depend only on the `pathfinding` and `ed25519-dalek` crates plus the native memory store. No CLI subcommand, no MCP tool handler, and no plugin hook imports plugin types. Public API exposes domain types only.

Public API (domain types only):

- `capture_trajectory(events: Vec<SurfaceEventRef>, eval: EvalGrade) -> TrajectoryId`
- `retrieve_similar_trajectories(input, k) -> Vec<TrajectoryRef>`
- `build_plan(goal, registry) -> GoapPlan`
- `replan(plan, failed_step, evidence) -> GoapPlan`
- `replay_golden_set(surface_event_id) -> ReplayReport`
- `detect_patterns(graph_view) -> Vec<DomainSignal>`

### Context Map

```
+----------------------------------------------------------+
|             Self-Learning Bounded Context                 |
|                                                          |
|  +-------------+ +---------------+ +-------------+       |
|  | Trajectory  | | TrajectoryClu | |  GoapPlan   |       |
|  |  Aggregate  | |  ster Aggreg. | |  Aggregate  |       |
|  +------+------+ +------+--------+ +------+------+       |
|         |               |                 |              |
|  +------+------+ +------+------+                         |
|  | GoldenSet   | | Drift       |                         |
|  | Aggregate   | | Detection   |                         |
|  +-------------+ +-------------+                         |
|         |                                                |
|         v   (native module: corp_finance_core::self_learning)
+---------|------------------------------------------------+
          |
          | consumes                produces signals
          v                          ^
+-------------------+        +-------------------+
| Memory Bounded    |        | Multi-Agent       |
| Context           |        | Coordination      |
| (ADR-015,         |        | (`domain-orchest- |
|  ADR-016,         |        |  ration.md`)      |
|  ADR-017)         |        |                   |
| - Surface         |        | - petgraph entity |
|   invocation      |        |   graph           |
|   events          |        | - GoapPlan        |
| - run_summary     |        |   execution       |
| - hybrid retrieve |        | - Tenant scoping  |
| - Cost / obs      |        +-------------------+
+-------------------+
```

Dependency direction:

- **Self-Learning depends on Memory** (consumes): trajectories are assembled from `cli_invocation_completed`, `mcp_tool_completed`, `plugin_hook_fired`, and skill events (recorded via the MCP wrapper) flowing out of the Memory bounded context. Trajectory retrieval uses Memory's hybrid retriever; cost and observability instrumentation reuse Memory's hooks.
- **Self-Learning depends on Multi-Agent Coordination** (consumes): the chief-analyst Coordination layer sends goals into Self-Learning and consumes plan trees back; trajectory and plan data inherit tenant scoping from `domain-federation.md`.
- **Self-Learning produces back to Multi-Agent Coordination**: domain signals from `patterns` are written as typed nodes into the native entity graph and consumed by the chief-analyst's cross-domain routing layer; high-grade `GoapPlan` candidates feed the planner forward.

There is no direct dependency on Phase 1-23 computation modules; Self-Learning operates above them.

### Event Flow

1. User invokes a CFA surface (`/cfa/initiate-coverage PFE`, `cfa initiate-coverage`, or an MCP tool call from a Claude Code session).
2. The chief-analyst (or a specialist) executes a sequence of MCP tool calls and slash commands; each fires a surface event.
3. On session completion, Memory has emitted one event per surface invocation. Self-Learning's `trajectory_repo` reads the sequence and writes a Trajectory aggregate, emitting `trajectory_captured`.
4. Native k-means worker periodically consumes `trajectory_captured` events → recomputes cluster centroids → emits `pattern_extracted`.
5. Next session starts: planning queries `retrieve_similar_trajectories(...)` → top-k high-grade refs returned → action selection biased.
6. For multi-domain goals, chief-analyst calls `build_plan(...)` (A* via the `pathfinding` crate) → `plan_generated` event → plan tree printed for review → chief-analyst proceeds or aborts.
7. If a plan step returns insufficient evidence: `replan(...)` → `plan_replanned` event → revised plan executed.
8. CI runs `replay_golden_set(...)` for every CFA surface target with golden-set replay enabled, on every commit (byte-diff + recursive `serde_json::Value` diff) → `drift_detected` events when threshold breached → deploy gate blocks (`drift_blocked_deploy`).
9. Continuously: `detect_patterns(graph_view)` over the native entity graph → `domain_signal_emitted` events → consumed by Multi-Agent Coordination.
