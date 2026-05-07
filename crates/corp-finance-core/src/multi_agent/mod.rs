//! Multi-Agent Coordination bounded context (Phase 27).
//!
//! Implements ADR-018 (multi-agent coordination via Claude Code's Agent
//! tool) and the `feature_orchestration.yml` contracts (`RUF-ORC-001..010`
//! plus the `MAC-INV-001..010` invariants).
//!
//! The bounded context owns runtime coordination across the nine CFA
//! specialist agents under `.claude/agents/cfa/`. The chief-analyst agent
//! is the orchestrator at session time; specialists are invoked through
//! Claude Code's Agent tool. The CFA platform itself does not run a
//! separate multi-agent runtime — this module records the trace, exposes
//! the entity graph, and emits A* plan trees over the registered MCP-tool
//! and slash-command action space.
//!
//! ## Module layout
//!
//! - [`types`] — domain types (`AgentInvocation`, `EntityRef`, `EntityKind`,
//!   `EntityRelation`, `RelationKind`, `GoapPlan`, `PlanStep`, `PlanAction`,
//!   `StepStatus`, `InvocationStatus`).
//! - [`agent_invoke`] — Claude Code Agent-tool delegation tracking, agent
//!   slug validation against the registered CFA agent set, and parent-chain
//!   cycle detection.
//! - [`entity_graph`] — `petgraph`-backed entity graph that supersedes the
//!   Phase 26 stub at [`crate::memory::entity_graph`]. Tag-based extractor
//!   for tickers, CUSIPs, sectors, issuers, and fund identifiers.
//! - [`goap_adapter`] — action-catalogue adapter exposing the registered
//!   MCP tools and CFA slash commands as the planner's action space.
//! - [`planner`] — A* planner over the action catalogue using
//!   [`pathfinding::directed::astar::astar`], with deterministic plan
//!   hashing (djb2) and bounded replanning (default 3 per `MAC-INV-006`).
//! - [`budget`] — per-`(tenant, specialist)` plan budget verdicts; default
//!   8 steps per invocation and 1 concurrent invocation per specialist
//!   (`RUF-ORC-009`).
//!
//! ## Feature gating
//!
//! The whole module is gated behind the `multi_agent` cargo feature at
//! the crate root (`lib.rs`). Internal files do not repeat the gate.

pub mod agent_invoke;
pub mod budget;
pub mod entity_graph;
pub mod goap_adapter;
pub mod planner;
pub mod types;

#[cfg(test)]
mod tests;

pub use agent_invoke::{
    complete_invocation, no_cycles_in_chain, record_invocation, validate_target_agent,
    REGISTERED_CFA_AGENTS,
};
pub use budget::{check_budget, BudgetVerdict, PlanBudget};
pub use entity_graph::{extract_entities_from_text, EntityGraph};
pub use goap_adapter::{load_action_catalogue, validate_action, ActionCatalogue};
pub use planner::{detect_cycles, plan, plan_hash, replan};
pub use types::{
    AgentInvocation, EntityKind, EntityRef, EntityRelation, GoapPlan, InvocationStatus, PlanAction,
    PlanStep, RelationKind, StepStatus,
};
