# Domain Model: Multi-Agent Coordination

## Bounded Context: Multi-Agent Coordination

This bounded context owns coordination across the CFA agents at runtime: dispatch from the chief analyst to specialist agents, cross-domain entity tracking across specialist outputs, and goal-directed planning over the available action space (MCP tools and slash commands). It is the operational surface that turns a single user goal into a composed multi-agent answer.

The runtime surfaces this context covers are the only four CFA surfaces:

- **CLI** (`cfa <subcommand>`) — every subcommand of the `cfa` binary.
- **MCP** (every `server.tool(...)` registration in `packages/*-mcp-server/src/`).
- **Skills** (slash commands and `.claude/skills/*` invoked by the LLM via the Skill tool).
- **Plugin** (PreToolUse / PostToolUse / Write / Edit hooks at `plugins/cfa-core/hooks/hooks.json`).

Cookbooks (`managed-agent-cookbooks/`) are deployment artefacts and outside this bounded context's runtime scope.

The implementation lives in native Rust modules under `corp_finance_core::multi_agent`: a `petgraph`-backed entity graph and an A* planner over the action space (the `pathfinding` crate). The ACL boundary keeps domain types clean of any external-MCP-server-shape leakage. There is no daemon, no JSONL queue, and no recursion frame: the chief-analyst agent is the orchestrator at session time and the runtime surfaces are the only execution paths.

### Domain Language (Ubiquitous Language)

| Term | Definition |
|------|-----------|
| **Surface Invocation** | A single CLI subcommand call, OR a single MCP tool handler execution, OR a plugin hook fire. The unit of multi-agent runtime activity. |
| **Agent Invocation** | A Claude Code Agent tool call: typically the chief-analyst dispatching to a specialist (equity, credit, fixed-income, derivatives, quant-risk, macro, private-markets, esg-regulatory). |
| **Specialist Output** | The structured result returned by a specialist agent invocation: text plus any artefacts written to the working directory. |
| **Entity Reference** | A typed identifier (issuer, ticker, fund, property, counterparty) extracted from a specialist output and registered in the entity graph. |
| **Entity Relation** | A directed edge between two entity references, scoped to the originating CLI/MCP invocation id. |
| **GOAP Plan** | An A* plan tree over the action space (MCP tool actions and slash-command actions) emitted by the chief-analyst's goal decomposition. |
| **Plan Step** | One node of a GOAP plan: either an MCP tool call or a slash command invocation. |
| **Pattern** | A signal raised when N specialist outputs touch the same entity within a window. |

### Aggregates

#### AgentInvocation (Aggregate Root)

A single Claude Code Agent tool call dispatched by the chief-analyst (or any other agent). The aggregate root for runtime coordination.

| Field | Type | Description |
|-------|------|-------------|
| `invocation_id` | `Uuid` (v7) | Unique identifier, sortable by time |
| `parent_invocation_id` | `Option<Uuid>` | Parent agent invocation in a multi-agent chain |
| `caller_agent` | `String` | The dispatching agent (e.g., `cfa-chief-analyst`) |
| `target_agent` | `String` | The specialist (e.g., `cfa-equity-analyst`) |
| `goal` | `String` | The decomposed sub-goal handed to the specialist |
| `surface` | `Surface` | `Cli`, `Mcp`, `Skill`, or `Plugin` (the surface that triggered the chain) |
| `input_hash` | `String` | `djb2:0x...` over the canonical input |
| `output_hash` | `Option<String>` | `djb2:0x...` over the captured output (set on completion) |
| `entity_refs` | `Vec<EntityRef>` | Entities extracted from the specialist output |
| `started_at` | `DateTime<Utc>` | Open time |
| `completed_at` | `Option<DateTime<Utc>>` | Close time |
| `status` | `InvocationStatus` | `Running`, `Completed`, `Failed`, `Cancelled` |

**Invariants**:

- `started_at <= completed_at` when the latter is set.
- `target_agent` must be in the registered CFA agent set under `.claude/agents/cfa/`.
- `caller_agent` and `target_agent` differ.
- Cycles in the parent chain are forbidden (cycle detection on registration).
- `output_hash` is set if and only if `status == Completed`.

#### EntityGraph (Aggregate)

The native `petgraph`-backed entity graph that accumulates entity references and relations across specialist outputs within a single user-facing session.

| Field | Type | Description |
|-------|------|-------------|
| `graph_id` | `String` | Stable identifier for the graph instance (per session) |
| `nodes` | `Vec<EntityNode>` | Typed entity references |
| `edges` | `Vec<EntityEdge>` | Directed relations carrying the originating `invocation_id` |
| `tenant_id` | `String` | Federation tenant scope (per `domain-federation.md`) |

**Commands**:

- `register_entity(invocation_id, entity_ref) -> Result<()>`
- `register_relation(from, to, kind, invocation_id) -> Result<()>`
- `query_pattern(entity_ref, window) -> Vec<PatternMatch>`

**Invariants**:

- Entity `kind` is one of: `issuer`, `ticker`, `fund`, `property`, `counterparty`.
- Edges are directed; carry the originating `invocation_id`.
- All nodes and edges share a single `tenant_id`; cross-tenant entity merge is forbidden unless an explicit federation session bridges (see `domain-federation.md`).

#### GoapPlan (Aggregate)

An A* plan tree produced by the chief-analyst's goal decomposition, ranging over MCP tool actions and slash-command actions. Backed by the `pathfinding` crate's A* implementation.

| Field | Type | Description |
|-------|------|-------------|
| `plan_id` | `Uuid` | Identifier |
| `goal` | `String` | Predicate the plan satisfies |
| `steps` | `Vec<PlanStep>` | Ordered actions; each is either an MCP tool call or a slash-command invocation |
| `dependencies` | `Vec<PlanDependency>` | DAG edges between steps (precondition → step) |
| `replan_count` | `u32` | Number of replans applied to this plan |
| `plan_hash` | `String` | `djb2:0x...` deterministic given goal + registry version |

**Commands**:

- `build_plan(goal, registry) -> GoapPlan`
- `replan(plan, failed_step, evidence) -> GoapPlan`
- `execute_step(step) -> StepResult`

**Invariants**:

- Each step's preconditions are satisfied by the goal context or an upstream step's postconditions.
- Replanning is bounded (default max 3 replans per goal).
- Plan hash is deterministic given the same goal + the same registry version.
- A plan must be emitted to stdout for chief-analyst review before any sub-agent or tool is invoked.

### Value Objects

#### Surface

Enum: `Cli`, `Mcp`, `Skill`, `Plugin`. The four runtime entry points.

#### EntityRef

| Field | Type | Description |
|-------|------|-------------|
| `kind` | `String` | `issuer`, `ticker`, `fund`, `property`, `counterparty` |
| `value` | `String` | The identifier value (canonical form) |

Shared kernel with the Memory bounded context (see `domain-memory.md`).

#### PlanStep

| Field | Type | Description |
|-------|------|-------------|
| `step_id` | `u32` | 1-indexed sequence position within the plan |
| `action_kind` | `ActionKind` | `McpTool` or `SlashCommand` |
| `action_name` | `String` | Tool name or slash command name |
| `preconditions` | `Vec<String>` | Required entities/inputs |
| `postconditions` | `Vec<String>` | Entities/outputs produced |

#### PatternMatch

| Field | Type | Description |
|-------|------|-------------|
| `entity_ref` | `EntityRef` | The entity at the centre of the pattern |
| `invocation_ids` | `Vec<Uuid>` | Agent invocations that touched this entity |
| `window_seconds` | `u32` | Window over which the pattern was detected |

### Domain Events

| Event | Raised By | Payload | Consumers |
|-------|-----------|---------|-----------|
| `agent_invocation_started` | AgentInvocation | `invocation_id`, `caller_agent`, `target_agent`, `goal`, `surface`, `started_at` | Audit, Memory, Observability |
| `agent_invocation_completed` | AgentInvocation | `invocation_id`, `output_hash`, `entity_refs`, `duration_ms`, `status` | Audit, Memory, Observability, Self-Learning |
| `entity_extracted` | EntityGraph | `invocation_id`, `entity_ref` | Memory, Self-Learning |
| `plan_generated` | GoapPlan | `plan_id`, `goal`, `step_count`, `plan_hash` | Audit, chief-analyst review surface |
| `plan_step_executed` | GoapPlan | `plan_id`, `step_id`, `result_summary` | Audit, Observability |
| `plan_replanned` | GoapPlan | `plan_id`, `failed_step`, `replan_count` | Audit, chief-analyst review surface |
| `pattern_detected` | EntityGraph | `entity_ref`, `invocation_ids`, `window_seconds` | Memory, Self-Learning, operator notification |

Note: there are no `handoff_emitted` / `handoff_dispatched` / `recursion_depth_exceeded` / `queue_backpressure` events. Those concepts belonged to the daemon-mode orchestration sketch and are not part of the runtime.

### Anti-Corruption Layer

The `corp_finance_core::multi_agent` Rust module is the boundary. It wraps the surface event format so domain types stay clean of any external-MCP-server-shape leakage: events arriving from MCP handlers are translated into `AgentInvocation`, `EntityRef`, and `PlanStep` domain types at the module edge. Specialists never see raw MCP envelopes; the chief-analyst never sees raw plugin hook payloads.

| Direction | ACL function | Wire format |
|-----------|--------------|-------------|
| MCP tool handler completion → AgentInvocation | `from_mcp_tool_event(&Value) -> AgentInvocation` | Validated against the surface event JSON shape |
| CLI subcommand completion → AgentInvocation | `from_cli_event(&CliEvent) -> AgentInvocation` | Direct from the CLI event struct |
| Plugin hook fire → AgentInvocation | `from_plugin_hook(&HookEvent) -> AgentInvocation` | Direct from `plugins/cfa-core/hooks/hooks.json` payload |
| AgentInvocation → audit pipeline | `to_audit_record(&AgentInvocation) -> Value` | JSON envelope per `domain-audit-observability.md` |
| Specialist output → EntityRef | `extract_entities(&str) -> Vec<EntityRef>` | Tag-based extractor seeded with the CFA identifier set |

There is no daemon. There is no JSONL inbox. There is no scheduled-worker emitter. The `EntityGraph` lives in-process for the duration of a session; persistence is via the Memory bounded context.

### Context Map

```
+---------------------------------------------------------------+
|              MULTI-AGENT COORDINATION CONTEXT                 |
|                                                               |
|  +-------------------+    +---------------------+             |
|  | AgentInvocation   |--->|   EntityGraph    |             |
|  | (root)            |    |  (petgraph store)   |             |
|  +---------+---------+    +----------+----------+             |
|            |                          |                        |
|            v                          v                        |
|     +------+----------+      +--------+--------+               |
|     |    GoapPlan     |      |  PatternMatch   |               |
|     |  (A* via        |      |  (window query) |               |
|     |   pathfinding)  |      +-----------------+               |
|     +-----------------+                                        |
|                                                               |
+---+----------+---------------+----------+----------+----------+
    |          |               |          |          |
    |          |               |          |          |
    v          v               v          v          v
 agent_     agent_       entity_   plan_        pattern_
 invocation_invocation_  extracted generated   detected
 started    completed              plan_
                                   step_
                                   executed
    |          |               |          |          |
    v          v               v          v          v
+---+----+ +---+-----+    +----+-----+ +--+------+ +-+--------+
| Memory | | Audit / |    |  Memory  | | Audit / | | Memory   |
|        | | Obs.    |    |  Self-   | | review  | | Self-    |
|        | |         |    | Learning | | surface | | Learning |
+--------+ +---------+    +----------+ +---------+ +----------+
```

### Context Relationships

| Upstream | Downstream | Relationship | Detail |
|----------|------------|--------------|--------|
| **Memory (`domain-memory.md`)** | Multi-Agent Coordination | Shared Kernel | `EntityRef` value object lives here and is consumed by Memory. Trajectories produced by Self-Learning depend on the same shape. |
| **Audit / Observability (`domain-audit-observability.md`)** | Multi-Agent Coordination | Conformist | All events flow through the audit pipeline; we conform to the audit envelope shape. |
| **Federation (`domain-federation.md`)** | Multi-Agent Coordination | Customer / Supplier | `tenant_id` is supplied by Federation per CLI invocation / per MCP tool call / per plugin hook fire. Cross-tenant entity merge is forbidden unless an explicit federation session bridges. |
| **Self-Learning (`domain-self-learning.md`)** | Multi-Agent Coordination | Customer / Supplier | Self-Learning consumes `agent_invocation_completed` and `entity_extracted` to assemble trajectories; feeds `GoapPlan` candidates back to chief-analyst. |
| **CLI / MCP / Skill / Plugin surfaces** | Multi-Agent Coordination | Anti-Corruption Layer | The four runtime surfaces deliver events; the ACL translates surface event shapes into `AgentInvocation`, `EntityRef`, `PlanStep` domain types so external surface shape never leaks. |

### Invariants Summary

| ID | Invariant | Enforced By |
|----|-----------|-------------|
| MAC-INV-001 | Every `AgentInvocation` carries `tenant_id` (via `EntityGraph`) | AgentInvocation aggregate, Federation boundary |
| MAC-INV-002 | `target_agent` is in the registered CFA agent set | AgentInvocation aggregate |
| MAC-INV-003 | No cycles in the parent agent invocation chain | AgentInvocation aggregate |
| MAC-INV-004 | Entity `kind` is one of the five typed values | EntityGraph aggregate |
| MAC-INV-005 | Entity references are tenant-scoped unless federation bridges | EntityGraph aggregate, Federation context |
| MAC-INV-006 | Plan replans are bounded (default 3) | GoapPlan aggregate |
| MAC-INV-007 | Plan hash is deterministic given goal + registry version | GoapPlan aggregate |
| MAC-INV-008 | A plan is emitted for chief-analyst review before any step executes | GoapPlan aggregate |
| MAC-INV-009 | `output_hash` is set iff `status == Completed` | AgentInvocation aggregate |
| MAC-INV-010 | Surface events are translated through the ACL; specialists never see raw MCP/plugin shapes | `corp_finance_core::multi_agent` boundary |

### Sub-Domain Breakdown

1. **Agent Invocation (`multi_agent::agent_invoke`)**: Wraps the Claude Code Agent tool. The chief-analyst dispatches via `agent_invoke(target_agent, goal)` returning a typed `AgentInvocation`. Pure function over the agent registry.
2. **Entity Graph (`multi_agent::entity_graph`)**: `petgraph`-backed in-process store with a tag-and-token entity extractor seeded with the CFA identifier set (issuer, ticker, fund, property, counterparty).
3. **Goal Decomposition (`multi_agent::goap_adapter` + `multi_agent::planner`)**: A* over the action space using the `pathfinding` crate. Action space is `(MCP tools registered in packages/*-mcp-server) ∪ (slash commands under .claude/commands/cfa)`.
4. **Budget Reservation (`multi_agent::budget`)**: Per-(tenant, specialist) reserve/settle keyed on parent run id. Failed reservation aborts dispatch.
5. **Pattern Detection**: Window-scan over the entity graph; emits `pattern_detected` when N invocations touch the same entity. Implemented as a function on the entity graph aggregate.

### MCP Tool Mapping

| Sub-Domain | Tool Name | Description |
|------------|-----------|-------------|
| Plan emission | `chief_plan_emit` | Emit an A* plan tree for a multi-domain goal before any specialist runs |
| Pattern detection | `chief_pattern_detect` | Return patterns surfaced since a timestamp from the entity graph |
| Agent invocation trace | `agent_trace_get` | Return the trace for an agent invocation (parent + children) by run_id |

## Bounded Context BC4 — Reasoning Bank (Phase 34)

This bounded context owns semantic and structured recall over past
dispatches. It exists to give the chief-analyst priors when routing a new
prompt — "have I seen something like this before?" and "what did I do
last time I had an Argentine convertible deal?" Phase 31 documented this
context (BC4 "Learning & Adaptation") but did not implement it; Phase 34
ships the implementation backed by `ruvector` (see ADR-040).

### Responsibilities

- Index every successful dispatch into a vector + metadata store keyed
  by `audit_id`, with prompt embedding, prompt summary, tool-call
  counts, delegation list, result excerpt, and free-form metadata.
- Serve two recall surfaces to the chief: `recallSimilar` (k-NN over
  prompt embeddings) and `recallByGraph` (structured-metadata query
  over the same entries).
- Persist as a single portable `.rvf` file plus an append-only JSONL
  scan sidecar — no external server, no separate database.
- Never block the dispatch hot path; never mutate the audit chain.

### Aggregate root

**`ReasoningBank`** (`packages/harness/src/reasoning/bank.ts`) — the
outbound port. Wraps `RVIndex` with embedding-step factored out so
embedding-provider choice is orthogonal to vector storage.

### Value objects

| Type | Description |
|------|-------------|
| `ReasoningEntry` | One indexed dispatch — `audit_id`, `agent_id`, `prompt_hash`, `prompt_summary`, `embedding`, `tool_calls`, `delegations`, `result_excerpt`, `metadata`, `timestamp`. |
| `RecallOptions` | k-NN recall params — `k`, `filter` (`agent_id`, `since`, `metadata`). |
| `GraphRecallQuery` | Structured-metadata recall — `metadata`, `hasTools`, `hasDelegations`, `agent_id`, `since`, `until`, `limit`. Default `limit` 50, max 500. |

### Domain events

| Event | When | Consumed by |
|-------|------|-------------|
| `entry_indexed` | After `bank.index()` succeeds | (Future) trajectory pipeline |
| `recall_hit` | `recallSimilar` / `recallByGraph` returned ≥1 entry | Audit chain (logged as a tool_call entry) |
| `recall_miss` | Recall returned `[]` | Audit chain |
| `reasoning_index_failed` | Indexing threw (embedding API down, native binding crash, sidecar I/O failure) | `DispatchEvent` stream (`onEvent`); does NOT propagate to caller (ADR-041) |

### Context relationships

| Upstream | Relationship | Detail |
|----------|--------------|--------|
| **Audit Chain (`domain-audit-observability.md`)** | Conformist — bank consumes `AuditRecord` shape verbatim | Each `ReasoningEntry` back-references the source `audit_id`. Bank is regenerable from the audit chain via the future `cfa-harness --rebuild-reasoning` CLI. |
| **Multi-Agent Coordination (above)** | Customer/Supplier — coordination context dispatches; bank derives entries from completed dispatches | Indexer fires fire-and-forget AFTER `audit.write()`; failures emit `reasoning_index_failed` events but never break the dispatch. |

| Downstream | Relationship | Detail |
|------------|--------------|--------|
| **Agent Runtime (Phase 31 BC1)** | Customer — agent loop exposes `recall_similar` and `recall_by_graph` virtual tools when a bank is configured on `DispatchOptions.reasoning` | Virtual tools bypass MCP; agent loop invokes `bank.recallSimilar` / `bank.recallByGraph` directly and returns formatted entries as `tool_result`. |

### Invariants

| ID | Invariant | Enforced By |
|----|-----------|-------------|
| RB-INV-001 | Bank entries reference the source `audit_id` and never mutate the audit chain | `ReasoningBank.index` writes only to its own store |
| RB-INV-002 | Index failure does not propagate to the caller; emits `reasoning_index_failed` instead | `agent-loop.ts` fire-and-forget hook (ADR-041) |
| RB-INV-003 | `recall_*` virtual-tool calls are themselves audit entries | `agent-loop.ts` records them in `auditToolCalls` like any MCP tool |
| RB-INV-004 | Specialists at depth ≥1 do not see `recall_*` tools | The reasoning bank is only attached to top-level chief dispatches |

### See also

- ADR-040 — RuVector for the Reasoning Bank
- ADR-041 — Indexer Hook in the Agent Loop
- Plan: `docs/plans/phase-34-reasoning-bank.md`
- Implementation: `packages/harness/src/reasoning/`
