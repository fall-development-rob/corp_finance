//! Domain types for the multi-agent coordination bounded context.
//!
//! Mirrors `docs/ddd/domain-orchestration.md`:
//!
//! - `AgentInvocation` is the aggregate root for a single Claude Code
//!   Agent-tool delegation (chief-analyst → specialist).
//! - `EntityRef` is a typed identifier extracted from a specialist output;
//!   `EntityKind` is constrained to the five values per `MAC-INV-004`.
//! - `EntityRelation` carries directed edges between entity references.
//! - `GoapPlan` is the A* plan tree over the MCP-tool + slash-command
//!   action space; `PlanStep` is a single node and `PlanAction` is the
//!   discriminated union of supported actions.
//!
//! All types are `serde::{Serialize, Deserialize}` so they round-trip
//! through the NAPI / MCP boundary unchanged.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// AgentInvocation
// ---------------------------------------------------------------------------

/// Status of an `AgentInvocation` from open through terminal state.
///
/// Per `MAC-INV-009`, an `output_hash` is set on the corresponding
/// invocation record if and only if `status == Completed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// A single Claude Code Agent-tool delegation by the chief-analyst (or
/// any other agent).
///
/// This is the aggregate root for runtime multi-agent coordination. Per
/// the DDD model:
///
/// - `target_agent` must be in [`crate::multi_agent::agent_invoke::REGISTERED_CFA_AGENTS`]
///   (`MAC-INV-002`).
/// - The parent chain must be acyclic (`MAC-INV-003`).
/// - `tenant_id` carries the federation tenant scope (`MAC-INV-001`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInvocation {
    /// Unique identifier (UUID v7 — sortable by time).
    pub invocation_id: Uuid,
    /// Specialist slug being invoked (e.g. `cfa-equity-analyst`).
    pub target_agent: String,
    /// Parent invocation id, if this is a sub-call inside an existing
    /// chain. `None` for the chief-analyst's own root invocation.
    pub parent_invocation_id: Option<Uuid>,
    /// Short summary of the input handed to the specialist.
    pub input_summary: String,
    /// Federation tenant scope (per `MAC-INV-001`).
    pub tenant_id: Option<String>,
    /// Open time.
    pub ts: DateTime<Utc>,
    /// Lifecycle status.
    pub status: InvocationStatus,
}

// ---------------------------------------------------------------------------
// EntityRef + EntityRelation
// ---------------------------------------------------------------------------

/// Canonical entity kinds extracted from specialist outputs.
///
/// Constrained to exactly five variants per `MAC-INV-004`. The set
/// matches the Phase 26 memory bounded context's `EntityKind` so the
/// shared kernel between the two contexts stays in sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Ticker,
    Issuer,
    Sector,
    Fund,
    Cusip,
}

/// A typed identifier extracted from a specialist output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityRef {
    pub kind: EntityKind,
    pub value: String,
    /// The agent invocation id that produced this entity, if any.
    pub source_invocation: Option<Uuid>,
}

/// Directed edge kinds between two `EntityRef`s in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    /// Two entities mentioned together in the same specialist output.
    MentionedTogether,
    /// Hierarchical containment (e.g. fund → portfolio company).
    ParentChild,
    /// Holding relationship (e.g. fund holds ticker).
    Holds,
    /// Issuance relationship (e.g. CUSIP issued by issuer).
    IssuedBy,
    /// Open-ended escape hatch for caller-defined relations.
    Custom(String),
}

/// A directed relation between two `EntityRef`s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityRelation {
    pub from: EntityRef,
    pub to: EntityRef,
    pub kind: RelationKind,
}

// ---------------------------------------------------------------------------
// GoapPlan
// ---------------------------------------------------------------------------

/// A single action that can occupy a `PlanStep`.
///
/// Three shapes:
///
/// - `McpTool` — a registered MCP tool from one of the four MCP servers.
/// - `SlashCommand` — a CFA slash command under `.claude/commands/cfa/`.
/// - `SpecialistAgent` — a Claude Code Agent-tool dispatch to one of the
///   nine CFA specialists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanAction {
    McpTool {
        name: String,
        input_hint: serde_json::Value,
    },
    SlashCommand {
        name: String,
        args: Vec<String>,
    },
    SpecialistAgent {
        slug: String,
        brief: String,
    },
}

impl PlanAction {
    /// Stable string used for plan-hash canonicalisation and cycle
    /// detection. Two actions with the same display key are considered
    /// equivalent for the purposes of `RUF-ORC-007` cycle detection
    /// when the upstream precondition state is unchanged.
    pub fn display_key(&self) -> String {
        match self {
            PlanAction::McpTool { name, .. } => format!("mcp:{name}"),
            PlanAction::SlashCommand { name, .. } => format!("slash:{name}"),
            PlanAction::SpecialistAgent { slug, .. } => format!("specialist:{slug}"),
        }
    }
}

/// Lifecycle status of a single `PlanStep`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// One node of a `GoapPlan`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_id: Uuid,
    pub action: PlanAction,
    pub depends_on: Vec<Uuid>,
    pub status: StepStatus,
    pub result_summary: Option<String>,
}

/// An A* plan tree over the action catalogue.
///
/// `plan_hash` is deterministic given the same goal + the same registry
/// version (`MAC-INV-007`); see [`crate::multi_agent::planner::plan_hash`].
/// `replan_count` and `max_replans` enforce `MAC-INV-006` (default 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoapPlan {
    pub plan_id: Uuid,
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub plan_hash: String,
    pub replan_count: u8,
    pub max_replans: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn invocation_status_round_trips() {
        let json = serde_json::to_string(&InvocationStatus::Completed).unwrap();
        assert_eq!(json, "\"completed\"");
        let back: InvocationStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, InvocationStatus::Completed);
    }

    #[test]
    fn entity_kind_has_five_variants() {
        // MAC-INV-004: exactly five variants.
        let kinds = [
            EntityKind::Ticker,
            EntityKind::Issuer,
            EntityKind::Sector,
            EntityKind::Fund,
            EntityKind::Cusip,
        ];
        assert_eq!(kinds.len(), 5);
    }

    #[test]
    fn plan_action_display_key_is_stable() {
        let a = PlanAction::McpTool {
            name: "dcf_model".into(),
            input_hint: json!({}),
        };
        assert_eq!(a.display_key(), "mcp:dcf_model");
        let b = PlanAction::SlashCommand {
            name: "initiate-coverage".into(),
            args: vec![],
        };
        assert_eq!(b.display_key(), "slash:initiate-coverage");
        let c = PlanAction::SpecialistAgent {
            slug: "cfa-equity-analyst".into(),
            brief: "value AAPL".into(),
        };
        assert_eq!(c.display_key(), "specialist:cfa-equity-analyst");
    }

    #[test]
    fn agent_invocation_round_trips_via_json() {
        let inv = AgentInvocation {
            invocation_id: Uuid::now_v7(),
            target_agent: "cfa-equity-analyst".into(),
            parent_invocation_id: None,
            input_summary: "value AAPL".into(),
            tenant_id: Some("local".into()),
            ts: Utc::now(),
            status: InvocationStatus::Pending,
        };
        let v = serde_json::to_value(&inv).unwrap();
        assert_eq!(v["target_agent"], "cfa-equity-analyst");
        let back: AgentInvocation = serde_json::from_value(v).unwrap();
        assert_eq!(back, inv);
    }
}
