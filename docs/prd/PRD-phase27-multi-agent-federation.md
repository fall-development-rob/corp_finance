# PRD: Phase 27 — Multi-Agent Coordination + Federation

## Overview

Phase 27 layers two capabilities onto the four CFA surfaces (CLI, MCP, Skills, Plugin):

1. **Multi-Agent Coordination** — native instrumentation that tracks how Claude Code's Agent tool delegates work to specialist CFA agents (cfa-chief-analyst, cfa-equity-analyst, cfa-credit-analyst, cfa-fixed-income-analyst, cfa-derivatives-analyst, cfa-quant-risk-analyst, cfa-macro-analyst, cfa-private-markets-analyst, cfa-esg-regulatory-analyst). The CFA platform does not run a separate multi-agent runtime; specialist agents are invoked by the host LLM (Claude Code) and execute by calling MCP tools and slash commands. Phase 27 adds a native entity graph (`petgraph`) that aggregates `EntityRef` extractions across specialist outputs and an A* planner (`pathfinding` crate) for chief-analyst goal decomposition over the MCP-tool + slash-command action space.

2. **Federation** — multi-tenant scoping at the four surface output boundaries (CLI output paths, MCP handler state, plugin hook output writes, skill output files), plus optional cross-installation collaboration with cryptographic provenance (`rustls` + `ed25519-dalek`).

Phase 27 delivers the multi-agent coordination layer (ADR-018) and the federation tenancy substrate (ADR-019), built natively in Rust using mature crates without runtime coupling to external frameworks.

## Problem Statement

Phases 24 and 25 shipped 15 cookbooks (deployment artefacts) and 9 standalone specialist analyst agents. Phase 26 instruments every CLI invocation, MCP tool call, plugin hook fire, and skill execution with audit manifests, cost ledgers, and structured spans. Three gaps remain:

1. **No cross-specialist entity tracking.** When `cfa-equity-analyst` produces a thesis on AAPL and `cfa-credit-analyst` produces a covenant analysis on AAPL the same week, no shared entity graph links them. The chief-analyst cannot ask "show me everything we've concluded about AAPL across specialists" without manually scanning surface outputs.
2. **No auditable multi-domain plan.** When a chief-analyst receives a multi-domain query, plan decomposition happens implicitly inside the LLM. There is no printable plan tree showing which MCP tools and slash commands will be invoked in what order.
3. **No tenant scoping at the surface output boundary.** All CLI output writes, MCP outputs, and plugin hook writes share a single output namespace. A fund administrator cannot host LP A and LP B on a single deployment without contamination risk.

Phase 27 closes all three.

## Dependencies

Phase 27 depends on Phase 26 deliverables:

| Phase 26 Contract | Used By Phase 27 |
|-------------------|------------------|
| `RUF-MEM-001..010` | Memory partitioned by `tenant_id`; consumes `entity_extracted`, `agent_invoked`, `plan_emitted` events; shares the `EntityRef` value object with Multi-Agent Coordination |
| `RUF-AUD-001..005` | Audit pipeline carries every Phase 27 surface-level event; `tenant_id` propagates through audit namespace |
| `RUF-COST-001..004` | Per-tenant cost ledger consumed by `BudgetLedger` for per-specialist-invocation reservation and settlement |

Phase 27 does not redefine memory, audit, or cost shapes.

## User Stories

### Multi-Agent Coordination (RUF-ORC-*)

1. **As a chief analyst**, when I delegate an equity initiating-coverage on AAPL via Claude Code's Agent tool to `cfa-equity-analyst`, I want every MCP tool call and slash-command invocation made by that specialist to be recorded against a parent agent invocation id, so that I can audit which specialist produced which numeric output.
2. **As a chief analyst**, when I run a multi-domain query that spans equity, credit, and macro, I want a printable A* plan tree (over the MCP-tool + slash-command action space) showing which specialists will be invoked in what order, with explicit dependencies, before any specialist runs.
3. **As a chief analyst**, when three specialists produce outputs that mention the same issuer entity, I want the native entity graph to surface that co-occurrence as a `pattern_detected` event via `cfa patterns` so that I can investigate cross-domain signals.
4. **As a platform engineer**, I want chief-analyst plan trees to bound depth at 3 by default (hard ceiling 8) over the action space of MCP tools and slash commands, so that runaway specialist invocation chains cannot drain my token budget.
5. **As a finance operator**, I want every specialist agent invocation tracked through Claude Code's Agent tool to reserve cost from a per-tenant budget before dispatch and settle the actual cost on completion.

### Multi-Tenant Federation (RUF-FED-*)

6. **As a fund administrator**, I want LP A's CLI output paths, MCP handler state, plugin hook outputs, and skill output files to be invisible to LP B at the file system, environment variable, and audit log boundaries simultaneously, so that I can host both LPs on a single deployment.
7. **As a wealth advisor**, I want each family's CLI and skill output directories to be isolated per-tenant with POSIX 0700 directory permissions.
8. **As a deal team lead**, I want to share a sector-research output (a specific surface output file from a CLI subcommand or skill execution) with a counterparty firm via federation, with PII default-BLOCK and explicit operator confirmation before any data leaves my installation.
9. **As a security officer**, I want trust score downgrade on threat detection to immediately place the affected federated session in Hold with no new outbound surface outputs accepted.
10. **As an operator**, I want PaidVendor MCP server outputs (e.g., from `vendor-mcp-server` LSEG / Moody's / S&P tools) to require an explicit cross-firm handshake before federation.

## Acceptance Criteria

### Multi-Agent Coordination

| Story | Specflow ID | Criterion |
|-------|-------------|-----------|
| 1 | RUF-ORC-001 | Every specialist invocation by Claude Code's Agent tool produces an `agent_invoked` event with parent agent id, child agent id, and the specialist's MCP tool calls and slash-command invocations recorded under it |
| 1 | RUF-ORC-002 | Specialist agent identifiers are validated against the registered set in `.claude/agents/cfa/`; unknown identifiers raise `unknown_specialist` |
| 2 | RUF-ORC-003 | When an LLM agent invokes the chief-analyst plan-emit tool, the A* planner (`pathfinding` crate) over the MCP-tool + slash-command action space produces a serialized `GoapPlan` printed to the CLI output and persisted to memory before any specialist is invoked |
| 2 | RUF-ORC-004 | Native `petgraph`-backed entity extraction succeeds on >= 90% of specialist outputs (measured by entity-ref count > 0 over MCP tool returns and CLI output writes) |
| 3 | RUF-ORC-005 | When >=3 specialist invocations within a configurable window touch the same `EntityRef`, a `pattern_detected` event is emitted and surfaced via `cfa patterns` |
| 4 | RUF-ORC-006 | Plan-tree depth default ceiling is 3; hard ceiling is 8; both enforced by `PlanFrame` aggregate at plan generation time |
| 4 | RUF-ORC-007 | Cycle in plan-tree action chain (same MCP tool / slash command revisited under same precondition) results in `plan_cycle_detected` event without dispatch |
| 5 | RUF-ORC-008 | Every specialist invocation calls `BudgetLedger::reserve` before dispatch and `settle` on completion, keyed by (tenant_id, specialist_id, parent_run_id) |
| 5 | RUF-ORC-009 | Budget exhaustion results in `budget_exhausted` event without dispatch |
| 1 | RUF-ORC-010 | Every accepted specialist invocation produces exactly one terminal event (`agent_completed`, `agent_dispatch_rejected`, `plan_depth_exceeded`, or `budget_exhausted`); no invocation is silently dropped |

### Multi-Tenant Federation

| Story | Specflow ID | Criterion |
|-------|-------------|-----------|
| 6 | RUF-FED-001 | Tenant scoping prevents cross-tenant access to CLI output paths, MCP handler state files, plugin hook output files, and skill output files at file system level (POSIX 0700) |
| 6 | RUF-FED-002 | Env-var substitution in CLI configs and MCP server configs is namespaced by `tenant.env_namespace` |
| 6 | RUF-FED-003 | Audit records (per Phase 26) carry `tenant_id` and `audit_namespace`; cross-tenant audit reads require explicit operator role |
| 7 | RUF-FED-004 | First CLI invocation under a new tenant creates `<out_dir_root>` with mode 0700 |
| 8 | RUF-FED-005 | Default PII redaction policy for new tenants is BLOCK for all 14 types at every surface output boundary |
| 8 | RUF-FED-006 | PII redaction in BLOCK mode prevents transmission of any of the 14 PII types in outbound federation messages, regardless of which surface produced the data |
| 9 | RUF-FED-007 | Trust-score downgrade is instantaneous on threat detection (no smoothing window) |
| 9 | RUF-FED-008 | Federated session enters `Hold` immediately on trust downgrade; no new outbound surface outputs accepted while held |
| 10 | RUF-FED-009 | Federation handshake fails closed (deny by default) on cert validation error |
| 10 | RUF-FED-010 | PaidVendor MCP-tool output federation requires both peers signing a session-scoped data redistribution agreement before `Active` state |

## Features

### Multi-Agent Coordination (ADR-018)

- New top-level Rust module at `crates/corp-finance-core/src/multi_agent/` with files: `mod.rs`, `agent_invoke.rs`, `entity_graph.rs`, `goap_adapter.rs`, `planner.rs`, `budget.rs`.
- Existing `orchestrate.rs::route_event` preserved unchanged as a pure validator (used by both managed-agent cookbook envelopes and the new agent-invocation events).
- New CLI commands: `cfa plan show --goal <text>`, `cfa plan replan --plan-id <id>`, `cfa entities list`, `cfa entities graph`.
- New MCP tools registered in `packages/cfa-core/src/`: `chief_plan_emit`, `chief_pattern_detect`, `agent_trace_get`.
- Feature flag: `multi_agent_coordination` (default off).
- New cargo deps (under the `multi_agent_coordination` feature): `petgraph = "0.x"`, `pathfinding = "0.x"`.

### Multi-Tenant Federation (ADR-019)

- New top-level Rust module at `crates/corp-finance-core/src/federation/` with files: `mod.rs`, `tenant.rs`, `pii_redaction.rs`, `trust_score.rs`, `session.rs`.
- Tenant registry at `<repo>/var/tenants/registry.toml`.
- New CLI commands: `cfa tenant list`, `cfa tenant scope <id>`, `cfa tenant init <id>`, `cfa federation status`, `cfa federation handshake`, `cfa federation audit`.
- `--tenant <id>` flag added to all CLI subcommands and surfaced in MCP tool input schemas where outputs are written; defaults to `local`.
- Feature flag: `federation` (default off).
- New cargo deps (under the `federation` feature): `rustls = "0.x"`, `ed25519-dalek = "2"`, `rcgen = "0.x"`. PII regex set is shared with the Phase 26 native scanner.
- v1 ships within-installation tenant isolation only; v2 adds the cross-installation crypto stack.

### Native Entity Graph

- Module `crates/corp-finance-core/src/multi_agent/entity_graph.rs` exposes domain trait `EntityGraph`.
- Default implementation `NativeEntityGraph` is `petgraph`-backed with a tag-and-token entity extractor seeded with the CFA identifier set (issuers, tickers, fund ids, property ids, counterparties).
- Entity extraction runs at every MCP tool handler completion (via the Phase 26 MCP wrapper) and every CLI output write (via the Phase 26 CLI wrapper), aggregating into a single tenant-scoped graph.
- Test implementation `NoopEntityGraph` used when feature disabled.

### A* Planner

- Modules `crates/corp-finance-core/src/multi_agent/goap_adapter.rs` and `crates/corp-finance-core/src/multi_agent/planner.rs` implement chief-analyst goal decomposition via the `pathfinding` crate.
- Action space is the union of registered MCP tools (across the four MCP servers) and registered slash commands (under `.claude/commands/cfa/`), each annotated with preconditions and postconditions.
- Plan trees are printed to CLI stdout for chief-analyst review and persisted via Phase 26 memory ingest with a deterministic plan hash.

## Success Metrics

| Metric | Target |
|--------|--------|
| Specialist agent invocations traced end-to-end | 100% of Agent-tool delegations to CFA specialists produce `agent_invoked` + terminal event |
| LP-A / LP-B isolation enforced across surface outputs | Zero cross-contamination across 100 paired CLI / MCP / plugin / skill invocations |
| Plan-tree generation success | >= 99% of multi-domain chief-analyst plan-emit calls return a serialized `GoapPlan` |
| Plan-tree depth violations | Zero successful dispatches above depth 3 (default) or depth 8 (hard ceiling) |
| Budget exhaustion behaviour | 100% of specialist invocations with insufficient budget rejected before dispatch |
| Native entity-graph extraction success | >= 90% of MCP tool outputs and CLI output writes yield at least one entity ref |
| Federation handshake failure mode | 100% deny-by-default on any cert validation error in test corpus |
| Trust downgrade response | Hold state asserted in <= 100ms after threshold crossing |
| PII default-BLOCK enforcement | 100% of new tenants start with all 14 types set to BLOCK at every surface boundary |
| Coordination stability | 7-day continuous run with zero unhandled crashes in staging across CLI and MCP surfaces |

## Estimated Work

7 days, broken down:

| Day | Work |
|-----|------|
| 1 | Multi-agent coordination scaffolding (`multi_agent/agent_invoke.rs`, `multi_agent/planner.rs`); feature flags; basic Agent-tool invocation tests |
| 2 | Plan-tree depth + cycle detection (`multi_agent/planner.rs`); hard ceiling and cycle property tests |
| 3 | Budget integration (`multi_agent/budget.rs`); reserve/settle keyed on (tenant, specialist, parent_run); failure-mode tests |
| 4 | Tenancy foundation (`federation/mod.rs`, `federation/tenant.rs`); `TenantContext` plumbed through CLI tracing wrapper, MCP wrapper, plugin hook emitter, and skill output paths; per-tenant out-dir 0700 enforcement |
| 5 | PII policy (`federation/pii_redaction.rs`) at every surface boundary and trust score consumption (`federation/trust_score.rs`); 14-type policy roundtrip |
| 6 | Federated session lifecycle (`federation/session.rs`); handshake state machine; deny-by-default cert validation |
| 7 | Native entity graph (`multi_agent/entity_graph.rs`) + A* planner adapter (`multi_agent/goap_adapter.rs`); end-to-end smoke (3 specialist coordination chains, 2 tenant pair invocations across all four surfaces, 1 multi-domain plan emit); CLI surface; specflow contract test pass |

## Out of Scope

- Replacing Claude Code's Agent tool with a custom multi-agent runtime; specialists are invoked by the host LLM and execute by calling existing CFA MCP tools and slash commands.
- Daemon-mode managed-agent cookbook orchestration; cookbook deploys remain CLI invocations (`cfa managed-agent deploy`) and inherit Phase 26 surface-level instrumentation. Cookbooks themselves are deployment artefacts outside this PRD's runtime scope.
- Cross-cookbook handoff buses, JSONL inboxes, recursion frames at the cookbook layer; these are not real CFA features.
- Automated peer discovery for federation (peers added by signed registry entry only).
- Windows-native federation socket implementation (Linux and macOS are primary targets).
- Cross-tenant entity merge (forbidden by FED-INV-008 unless an explicit federation session bridges).
- New cookbook personas (Phase 27 wires the existing 9 specialist analysts; new personas are out-of-scope).
- Cross-installation handshake stack (mTLS + ed25519) ships in v2; v1 is within-installation tenancy only.
- Smoke testing of the optional `agentdb_backend` feature is deferred.

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Crate-version drift in `petgraph` / `pathfinding` / `rustls` / `ed25519-dalek` | Version pinning in Cargo.toml; CI runs the coordination and federation contract sets on every PR |
| Specialist invocation explosion under load | Per-(tenant, specialist) concurrency cap of 1; observability via the Phase 26 audit pipeline |
| Tenant misconfiguration leaking data across surfaces | Default PII policy BLOCK at every surface; POSIX 0700 directory creation; audit namespace mandatory |
| Plan-tree runaway despite default cap | Hard ceiling 8 cannot be overridden by tenant config |
| Phase 26 contracts not finalised at integration time | All Phase 26 references called out for verification before Phase 27 sign-off |
| A* planner action-space drift | Action-space registry is read at startup from registered MCP tools and `.claude/commands/cfa/`; CI lint enforces precondition/postcondition annotations on every entry |

## Cross-Doc References Requiring Verification

Phase 26 contract references require verification when Phase 26 lands:

- RUF-MEM-001..010 (memory partitioning by `tenant_id`; `EntityRef` shared kernel)
- RUF-AUD-001..005 (audit pipeline shape; tenant namespace propagation)
- RUF-COST-001..004 (cost ledger keys and reserve/settle semantics)

## Related Decisions

- ADR-018: Multi-agent coordination (Agent-tool invocation tracking, native entity graph, A* planner)
- ADR-019: Multi-tenant federation (surface-output tenant scoping and cross-installation collaboration)
- ADR-015 (Phase 26): Persistent agent memory
- ADR-016 (Phase 26): Audit pipeline
- ADR-017 (Phase 26): Cost telemetry and per-tenant ledgers
- ADR-009: Workflow auditability (audit hashing pattern reused for surface audit hash)
