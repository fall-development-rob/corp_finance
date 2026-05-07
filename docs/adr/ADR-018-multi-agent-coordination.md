# ADR-018: Multi-Agent Coordination via Existing Surfaces

## Status: Accepted

## Date: 2026-05-06

## Deciders: CFA platform engineering, managed-agent module owners, chief-analyst orchestration owner

## Context

This ADR replaces an earlier draft titled "Cross-Cookbook Orchestration via Daemon-Mode Handoff." The earlier draft proposed a long-running daemon (`managed_agent::orchestrate_daemon`) that would consume a JSONL queue of `handoff_request` events between deployed cookbooks, dispatch fresh deploys, and enforce recursion / cost guardrails.

**What changed and why.** The smoke test that informed the second-pass revision of ADR-015 made clear that "cross-cookbook orchestration" is a phantom feature: cookbooks at `managed-agent-cookbooks/` are static deployment artefacts that users may publish to Anthropic's `/v1/agents` Managed Agents API. Once a cookbook is deployed, the resulting managed agent runs in Anthropic's infrastructure. We have no runtime relationship with deployed cookbooks. There is no place for a daemon mode in our process tree to "hand off between cookbooks", because the cookbooks aren't running in our process tree. The daemon, the JSONL queue, the handoff bus, and the per-tenant recursion guards on cookbook handoffs were architecting against a runtime layer that does not exist.

What does exist is multi-agent coordination *inside* a single Claude Code session, mediated by the four runtime surfaces ADR-015 pins down (CLI, MCP, skill, plugin). Claude Code's Agent tool lets the chief-analyst sub-agent route work to specialist sub-agents. Specialists invoke MCP tools and skills. The chief-analyst aggregates the outputs. This is a real coordination problem with real production value, and it is what analysts actually do daily. This ADR specifies that path.

The 9 specialist agents at `.claude/agents/cfa/` (chief-analyst, equity-analyst, credit-analyst, fixed-income-analyst, derivatives-analyst, quant-risk-analyst, macro-analyst, private-markets-analyst, esg-regulatory-analyst) are the unit of multi-agent decomposition. Each has a defined capability scope; chief-analyst routes natural-language requests to the appropriate specialist or fans out to multiple specialists.

## Decision

Multi-agent coordination in the CFA system happens through Claude Code's Agent tool inside a single session. There is no daemon, no queue, no handoff bus, no `managed_agent::orchestrate::route_event` runtime. The `crates/corp-finance-core/src/managed_agent/orchestrate.rs::route_event` function remains as deploy-time tooling (it validates a cookbook's declared callable-agent set against `COOKBOOK_REGISTRY` when a user is assembling a deploy payload), but it is not promoted into a runtime substrate.

Native pieces from the prior draft that survive — repurposed:

- **Native entity graph (`petgraph`-backed)** at `corp_finance_core::multi_agent::entity_graph`. Tracks issuer / ticker / fund / instrument entities across specialist outputs within a session and across sessions via the Phase 26 memory store. Used by chief-analyst aggregation when fanning out to multiple specialists on the same issuer. Consumed by ADR-020 self-learning loop.
- **A* planner (`pathfinding` crate)** at `corp_finance_core::multi_agent::goap_adapter`. Action space is the **MCP tool registry + slash command catalogue** (i.e., `~594 MCP tools + ~25 CFA slash commands`), not the cookbook registry. Chief-analyst uses the planner to decompose a multi-domain goal into a minimum-cost plan over MCP tools and skills.

### Coordination model

```
LLM session (Claude Code)
  |
  +-- chief-analyst sub-agent (.claude/agents/cfa/chief-analyst.md)
        |
        |-- (LLM calls Agent tool to route to specialists)
        |     |
        |     +-- cfa-equity-analyst (.claude/agents/cfa/equity-analyst.md)
        |     |     |
        |     |     +-- invokes MCP tools (dcf_model, comps_table, fmp_quote, ...)
        |     |     +-- invokes skills (workflow-equity-research, corp-finance-tools-core, ...)
        |     |
        |     +-- cfa-credit-analyst
        |     +-- cfa-private-markets-analyst
        |     +-- ... (other specialists as needed)
        |
        +-- aggregates outputs into final deliverable
        +-- emits run_summary.json (ADR-016) and .audit.json (ADR-017) at the surface boundaries
```

Every leaf MCP tool call and slash-command invocation is captured at the wrappers from ADR-015. The chief-analyst's aggregation step is itself a CLI invocation or a Skill-tool invocation and is captured at the corresponding surface.

### Chief-analyst planning

For multi-domain requests ("write a coverage initiation on PFE including credit, ESG, and macro context"), chief-analyst calls `corp_finance_core::multi_agent::goap_adapter::build_plan(goal, action_space)` to produce a plan tree over the MCP tool registry and slash-command catalogue. The plan tree is emitted to stdout for review before chief-analyst invokes any specialist. A* via the `pathfinding` crate is the planner. Replanning happens when a specialist returns insufficient evidence.

Action space:

- All registered MCP tools across `cfa-core`, `fmp-mcp-server`, `data-mcp-server`, `vendor-mcp-server` (~594 tools).
- All CFA slash commands at `.claude/commands/cfa/*.md` (~25 commands).
- Annotated with declared preconditions (required entity context, required prior outputs) and postconditions (entity context produced, output artefact produced).

The implementation of `build_plan` lives at `corp_finance_core::multi_agent::goap_adapter`. ADR-020 (self-learning loop) consumes this planner; this ADR owns the module location.

### Native entity graph (cross-specialist tracking)

When chief-analyst fans out to multiple specialists on the same request, each specialist's outputs may reference shared entities (an issuer ticker, a sector, a fund). The native `petgraph`-backed entity graph tracks these references so chief-analyst's aggregation step can reconcile cross-specialist findings. Entity extraction is a hand-rolled tag-and-token extractor seeded with the CFA identifier set (CUSIP, ISIN, ticker, LEI, FIGI plus issuer-name lookups).

The graph is per-session in v1; cross-session entity views are served by the Phase 26 memory layer (ADR-016) which reads `petgraph` adjacencies into BM25/HNSW retrieval. Tenant scoping is enforced per ADR-019.

The module exposes a domain trait `EntityGraph` with two implementations:

- `NativeEntityGraph` (default; `petgraph` + tag extractor)
- `NoopEntityGraph` (tests and disabled-feature builds)

### What is explicitly out of scope

- **Daemon mode**. No long-running process, no JSONL queue, no inbox/rejected file routing, no Redis adapter.
- **Handoff envelopes between deployed cookbooks**. Cookbooks are deployment artefacts running in Anthropic's infra; we do not orchestrate them after deploy.
- **Recursion accounting across cookbook deploys**. There is no recursion across deployed cookbooks; each deploy is independent. Within-session sub-agent depth is governed by Claude Code's Agent tool, not by us.
- **Cron / scheduled cookbook firing**. Out of scope. Operators who want scheduled runs can use the `loop` and `schedule` skills already available in Claude Code, or external schedulers invoking `cfa <subcommand>`.
- **Per-tenant cost ceilings on cross-cookbook chains**. Per-CLI-invocation and per-MCP-tool-call cost ceilings are defined in ADR-017; tenant scoping is defined in ADR-019. There is no separate cookbook-chain ledger.

### CLI surface

| Command | Purpose |
|---------|---------|
| `cfa plan show --goal <text>` | Print the A* plan tree for a goal (read-only; no execution) |
| `cfa plan replan --plan-id <id> --failed-step <id>` | Replan from a failed step with partial state retained |
| `cfa entities list [--session <id>]` | List entities tracked in the session entity graph |
| `cfa entities graph --format dot` | Export the entity graph for inspection |

The `cfa managed-agent <verb>` deploy-time tooling subcommand group is unchanged.

The `cfa managed-agent orchestrate` deploy-time CLI (existing) remains for users assembling cookbook payloads; it validates a cookbook's `callable_agents` field against `COOKBOOK_REGISTRY` at validate-time and returns a `DispatchDecision`. It is not invoked at runtime.

## Rationale

- The original problem (specialist coordination on multi-domain analyst requests) is real and matters daily. The original solution (daemon-mode cross-cookbook handoff) was architecting against a non-existent runtime layer.
- Claude Code's Agent tool already implements the coordination primitive we need: in-session sub-agent routing with the LLM as orchestrator. Building a daemon on top duplicates that primitive in a different runtime.
- The native entity graph and A* planner are valuable independent of the daemon framing; they make chief-analyst routing auditable (plan tree emitted before execution) and reconcile cross-specialist outputs on shared entities.
- Restricting the action space to MCP tools + slash commands matches what the LLM can actually invoke. The cookbook registry is a deploy-time artefact catalogue, not a runtime action space.
- Cookbook deploys via `cfa managed-agent deploy` are CLI invocations and are captured at the CLI surface (ADR-015, ADR-017). Nothing additional is needed.

## Consequences

### Positive

- The chief-analyst → specialist coordination story is honest: it lives where the actual coordination happens (Claude Code Agent tool inside a single session), not in a fictional daemon.
- The A* planner and entity graph deliver real audit and reconciliation value with no daemon overhead. Plan trees are emitted to stdout before execution.
- All audit / cost / observability instrumentation comes for free from the surface wrappers (ADR-017); no new event pipeline is needed.
- Implementation cost drops sharply: the daemon, queue, recursion accounting, and handoff envelope schema (~1200 LoC + tests) are deleted from scope.
- Tenant scoping (ADR-019) applies uniformly across all four surfaces; no special-case for handoff envelopes.

### Negative

- Cross-session multi-specialist coordination depends on the Phase 26 memory layer (ADR-016) for entity context that survives session boundaries. If memory ingestion is broken, cross-session reconciliation degrades.
- Scheduled execution is out of scope here; users wanting morning-note automation must use Claude Code's `schedule` / `loop` skills or external schedulers.
- The native entity graph is per-session in v1; multi-tenant cross-session graphs require the Phase 26 memory layer's tenant-partitioned reads.

### Risks

- **Action-space annotation drift**: cookbook authors and tool authors must annotate preconditions / postconditions accurately for A* to plan correctly. CI lint enforces annotation presence (TODO: define lint exact behaviour — depends on the precondition/postcondition schema; sister agent on PRD layer should specify).
- **Plan-tree quality**: A* over ~620 actions is well within `pathfinding`'s budget; if the action space grows past a few thousand, plan caching by goal hash and a hand-tuned heuristic may be needed.
- **Entity-extraction accuracy**: hand-rolled tag-and-token extraction is a starting point. Mitigated by treating the entity graph as a hint layer (chief-analyst still reads the actual specialist outputs); inaccurate entities cost recall, not correctness.

## Implementation Notes

- All new files follow the existing crate conventions: `serde::{Serialize, Deserialize}` boundaries, `CorpFinanceResult<T>` returns, `rust_decimal::Decimal` for any monetary fields.
- Native crate dependencies:
  - `petgraph = "0.x"` — entity graph (entity / sub-agent / MCP-tool edges) [shared with ADR-016 memory layer]
  - `pathfinding = "0.x"` — A* planner over MCP tool + slash-command action space [introduced by ADR-020]
- Module locations:
  - `crates/corp-finance-core/src/multi_agent/mod.rs` (module root)
  - `crates/corp-finance-core/src/multi_agent/agent_invoke.rs` (Agent-tool invocation tracking)
  - `crates/corp-finance-core/src/multi_agent/entity_graph.rs` (entity graph, native `petgraph`)
  - `crates/corp-finance-core/src/multi_agent/goap_adapter.rs` (A* planner adapter)
  - `crates/corp-finance-core/src/multi_agent/planner.rs` (planner driver)
  - `crates/corp-finance-core/src/multi_agent/budget.rs` (per-(tenant, specialist) budget reservation)
- Specialist agent definitions (existing): `.claude/agents/cfa/chief-analyst.md`, `.claude/agents/cfa/equity-analyst.md`, `.claude/agents/cfa/credit-analyst.md`, etc.
- CFA slash commands (existing): `.claude/commands/cfa/*.md`.
- Capture at surface wrappers (ADR-015 / ADR-017): every MCP tool call by a specialist is traced and cost-recorded; every CLI invocation initiating chief-analyst routing is the root span; every plugin Write/Edit hook on aggregated outputs emits the `.audit.json`.
- Specflow contracts: `docs/contracts/feature_orchestration.yml` (RUF-ORC-001..010 contracts; MAC-INV-001..010 invariants).

### What was scrapped from the prior draft

| Prior Module | Disposition |
|--------------|-------------|
| `orchestrate_daemon.rs` | Deleted from scope. No daemon. |
| `orchestrate_recursion.rs` | Deleted from scope. No cross-cookbook recursion. |
| `orchestrate_budget.rs` | Deleted from scope. Cost ceilings live in ADR-017 per CLI invocation / per MCP tool call. |
| `orchestrate_queue.rs` | Deleted from scope. No JSONL queue. |
| `orchestrate_entity_graph.rs` | Renamed and moved to `corp_finance_core::multi_agent::entity_graph` (consumed by ADR-020). |
| `HandoffEnvelope` schema | Deleted. No handoff between deployed cookbooks. |
| `cfa managed-agent orchestrate-daemon` CLI | Deleted. |
| `cfa managed-agent orchestrate-replay` CLI | Deleted. |
| `cfa managed-agent patterns` CLI | Renamed to `cfa entities ...` and scoped to the session entity graph. |
| Native scheduled workers (`tokio-cron-scheduler`) | Deleted from scope. Scheduling delegated to Claude Code skills (`schedule`, `loop`) or external. |
| `route_event` pure router | Retained at `crates/corp-finance-core/src/managed_agent/orchestrate.rs` as deploy-time tooling for cookbook payload assembly; not a runtime entry point. |

### Test Targets

| Module | Test Count | Key Scenarios |
|--------|-----------|---------------|
| `multi_agent::entity_graph` | ~25 | EntityGraph trait coverage, NoopEntityGraph behaviour, NativeEntityGraph petgraph operations, entity-kind validation, CFA identifier extraction, cross-specialist reconciliation |
| `multi_agent::goap_adapter` / `multi_agent::planner` | ~30 | A* termination on small action space (~620 actions), precondition/postcondition propagation, replan with partial state retained, plan-tree emission, missing-annotation rejection |
| `multi_agent::agent_invoke` / `multi_agent::budget` (integration) | ~15 | Multi-specialist fanout, entity-graph aggregation, plan-tree review checkpoint, surface-event capture per specialist call |
| **Total** | **~70** | |

These tests bring the projected workspace test count from approximately 6,587 to approximately 6,657. (The prior daemon design contributed ~110 tests; replacing it with the surface-anchored design reduces test count by ~40 because the daemon, queue, recursion, and budget modules are out.)

## Options Considered

### Option 1 (chosen): Multi-agent coordination via Claude Code's Agent tool, with native entity graph and A* planner over MCP tool + slash-command action space

Selected. Honest scoping, low engineering cost, leverages existing primitives.

### Option 2: Daemon-mode cross-cookbook handoff (the prior draft of this ADR)

Rejected. Cookbooks are deployment artefacts running in Anthropic's infra; we have no runtime hook for daemon-mode handoff between them. The daemon was solving a problem that does not exist in our runtime model.

### Option 3: Anthropic-native callable_agents instead of chief-analyst routing

Considered. Callable agents nest synchronously inside a parent agent's turn — they consume the parent's token budget and run inside Anthropic's infrastructure. We continue to use callable_agents inside *deployed* cookbook payloads (the `callable_agents` field in cookbook YAML). For coordination at the Claude Code session level, the Agent tool is the right primitive because it sits inside our runtime where instrumentation lives.

### Option 4: Implement coordination in the TypeScript MCP server layer

Rejected. The coordination primitive is the LLM's Agent-tool invocation, which Claude Code already implements. Adding a TypeScript orchestrator above it duplicates the primitive and fragments the audit story across two runtimes.

## Related Decisions

- ADR-015: Native Orchestration Umbrella (the four runtime surfaces; this ADR honours that boundary)
- ADR-016: Memory Architecture (entity graph reads/writes through the Phase 26 memory layer)
- ADR-017: Audit / Cost / Observability (specialist routing is captured automatically at the CLI/MCP/plugin wrappers)
- ADR-019: Multi-Tenant Federation (tenant scoping at every surface; entity graph and plan trees inherit it)
- ADR-020: Self-Learning Loop (owns the implementation of `entity_graph` and `goap_adapter` modules referenced here)
- ADR-009: Workflow Auditability (audit hashing pattern for plan tree manifests)

## References

- `crates/corp-finance-core/src/managed_agent/orchestrate.rs` — deploy-time pure router preserved for cookbook payload assembly
- `crates/corp-finance-core/src/managed_agent/types.rs` — `COOKBOOK_REGISTRY`, `ALLOWED_SLUGS`, `OrchestrateEvent`, `CookbookTier` (deploy-time types)
- `.claude/agents/cfa/chief-analyst.md` — chief-analyst routing definition
- `.claude/agents/cfa/equity-analyst.md`, `credit-analyst.md`, `private-markets-analyst.md`, etc. — specialist agents
- `.claude/commands/cfa/*.md` — CFA slash commands (part of the A* action space)
- `petgraph`, `pathfinding` — crate documentation on docs.rs
- Reference deployment templates (deployment artefacts only): `managed-agent-cookbooks/`
- Concept inspiration (not runtime dependencies): https://github.com/ruvnet/ruflo
- Smoke test findings (May 2026): `/tmp/ruflo-smoke-test.md`
- Anthropic Managed Agents API documentation
