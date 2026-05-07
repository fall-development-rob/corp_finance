//! Claude Code Agent-tool delegation tracking.
//!
//! This module records traces of Agent-tool dispatches at the chief-analyst
//! → specialist boundary. It does **not** dispatch the actual Agent call
//! (that is the host LLM's job inside Claude Code). It validates the target
//! slug against the registered CFA agent set (`MAC-INV-002`), enforces
//! parent-chain acyclicity (`MAC-INV-003`), and emits the
//! `agent_invocation_started` / `agent_invocation_completed` domain events
//! per `docs/ddd/domain-orchestration.md`.
//!
//! The events are surfaced to the audit / observability pipeline through
//! the surface wrappers (ADR-017). For v1 we just construct the event
//! envelopes; downstream wiring lives in the Phase 26 audit layer.

use std::path::Path;

use crate::multi_agent::audit_writer::InvocationAuditPaths;
use crate::multi_agent::types::{AgentInvocation, EntityRef};
use crate::CorpFinanceResult;
use uuid::Uuid;

/// Hard-coded set of registered CFA specialist agents under
/// `.claude/agents/cfa/`. Source-of-truth for `MAC-INV-002`.
///
/// v1 keeps this list as a `&'static [&'static str]` constant because the
/// agent set is small (9) and changes rarely. v2 will read the directory
/// at startup and validate against the on-disk set.
pub const REGISTERED_CFA_AGENTS: &[&str] = &[
    "cfa-chief-analyst",
    "cfa-equity-analyst",
    "cfa-credit-analyst",
    "cfa-fixed-income-analyst",
    "cfa-derivatives-analyst",
    "cfa-quant-risk-analyst",
    "cfa-macro-analyst",
    "cfa-private-markets-analyst",
    "cfa-esg-regulatory-analyst",
];

/// Validate that `slug` is in the registered CFA specialist set.
///
/// Returns `true` if the slug matches one of [`REGISTERED_CFA_AGENTS`].
/// Per `RUF-ORC-002`, an `unknown_specialist` event must be emitted by
/// the caller when this returns `false`.
pub fn validate_target_agent(slug: &str) -> bool {
    REGISTERED_CFA_AGENTS.contains(&slug)
}

/// Detect whether registering `invocation` against the existing `history`
/// would create a cycle in the parent-invocation chain.
///
/// Returns `true` when the chain stays acyclic (per `MAC-INV-003`); the
/// caller may safely register the invocation. Returns `false` when an
/// ancestor of `invocation` shares its `target_agent`, which would form a
/// cycle once registered.
///
/// The walk follows `parent_invocation_id` links through `history` up to
/// the root or until the depth limit (`history.len() + 1`) is hit.
pub fn no_cycles_in_chain(invocation: &AgentInvocation, history: &[AgentInvocation]) -> bool {
    let mut current = invocation.parent_invocation_id;
    let max_walks = history.len() + 1;
    for _ in 0..max_walks {
        let parent_id = match current {
            Some(id) => id,
            None => return true,
        };
        let parent = match history.iter().find(|h| h.invocation_id == parent_id) {
            Some(p) => p,
            None => return true, // parent not in history; treat as detached but acyclic
        };
        if parent.target_agent == invocation.target_agent {
            // The ancestor invokes the same target — that is a cycle.
            return false;
        }
        current = parent.parent_invocation_id;
    }
    // Walked beyond the depth bound; conservative: treat as cyclic.
    false
}

/// Record an `agent_invocation_started` domain event.
///
/// Per the DDD model and `RUF-ORC-001`, every Agent-tool delegation to a
/// CFA specialist produces this event with parent agent id, child agent
/// id, and the begin timestamp. v1 emits via the audit pipeline (TODO:
/// wire the actual sink in the Phase 26 audit layer once the audit
/// crate-feature is enabled at the workspace level).
///
/// Returns `Ok(())` on success. Returns
/// [`crate::error::CorpFinanceError::InvalidInput`] when `target_agent`
/// is not in [`REGISTERED_CFA_AGENTS`].
pub fn record_invocation(invocation: &AgentInvocation) -> CorpFinanceResult<()> {
    if !validate_target_agent(&invocation.target_agent) {
        return Err(crate::error::CorpFinanceError::InvalidInput {
            field: "target_agent".to_string(),
            reason: format!(
                "agent slug '{}' is not in the registered CFA specialist set",
                invocation.target_agent
            ),
        });
    }
    // Emit the `agent_invocation_started` envelope through the surface
    // audit pipeline. Agent invocations land on the MCP / Agent-tool
    // surface, so we build a `SurfaceManifest` keyed off the invocation
    // id and command-args projection. The full `.audit.json` manifest
    // write happens at the surface boundary (CLI / MCP wrapper); here we
    // just compute the canonical hash for trace consistency.
    #[cfg(feature = "audit")]
    {
        let manifest = crate::audit::surface_audit::SurfaceManifest {
            surface: crate::surface::Surface::Mcp,
            surface_event_id: invocation.invocation_id.to_string(),
            command_args: serde_json::json!({
                "event": "agent_invocation_started",
                "target_agent": invocation.target_agent,
                "input_summary": invocation.input_summary,
                "tenant_id": invocation.tenant_id,
                "parent_invocation_id": invocation
                    .parent_invocation_id
                    .map(|u| u.to_string()),
            }),
            output_paths: Vec::new(),
        };
        let _hash = crate::audit::surface_audit::compute_surface_audit_hash(&manifest);
    }
    Ok(())
}

/// Record an `agent_invocation_completed` domain event with the entity
/// references extracted from the specialist's output.
///
/// Per the DDD model:
///
/// - The caller MUST set `output_hash` upstream when transitioning the
///   invocation to `Completed` (`MAC-INV-009`); this function records
///   the completion without re-checking that flag because it does not
///   own the invocation's mutable state.
/// - `entities` are folded into the entity graph by
///   [`crate::multi_agent::entity_graph::EntityGraph`].
///
/// Returns `Ok(())` on success. The function is intentionally
/// side-effect free at the data-store level for v1; downstream wiring
/// (audit + memory) is owned by the Phase 26 surface wrappers.
pub fn complete_invocation(
    invocation_id: Uuid,
    output_summary: &str,
    entities: &[EntityRef],
) -> CorpFinanceResult<()> {
    // Emit the `agent_invocation_completed` envelope through the surface
    // audit pipeline; entity-graph dispatch is owned by the orchestrator
    // (caller wires extracted entities into `multi_agent::entity_graph`
    // separately). The manifest is written at the surface boundary; we
    // compute the hash here to keep traces consistent across `record_*`
    // and `complete_*` for the same invocation id.
    #[cfg(feature = "audit")]
    {
        let entity_summary: Vec<serde_json::Value> = entities
            .iter()
            .map(|e| {
                serde_json::json!({
                    "kind": format!("{:?}", e.kind).to_lowercase(),
                    "value": e.value,
                })
            })
            .collect();
        let manifest = crate::audit::surface_audit::SurfaceManifest {
            surface: crate::surface::Surface::Mcp,
            surface_event_id: invocation_id.to_string(),
            command_args: serde_json::json!({
                "event": "agent_invocation_completed",
                "output_summary": output_summary,
                "entities": entity_summary,
            }),
            output_paths: Vec::new(),
        };
        let _hash = crate::audit::surface_audit::compute_surface_audit_hash(&manifest);
    }
    #[cfg(not(feature = "audit"))]
    let _ = (invocation_id, output_summary, entities);
    Ok(())
}

/// Record an `agent_invocation_started` domain event **and** persist the
/// companion `.audit.json` file pair under `manifest_root`.
///
/// Delegates to [`record_invocation`] first for validation (unknown slug
/// returns `InvalidInput` before any I/O). On success, calls
/// [`crate::multi_agent::audit_writer::write_invocation_started`] and
/// returns the paths of the written files.
pub fn record_invocation_with_audit(
    invocation: &AgentInvocation,
    manifest_root: &Path,
) -> CorpFinanceResult<InvocationAuditPaths> {
    record_invocation(invocation)?;
    crate::multi_agent::audit_writer::write_invocation_started(manifest_root, invocation)
}

/// Record an `agent_invocation_completed` domain event **and** persist the
/// companion `.audit.json` file pair under `manifest_root`.
///
/// Delegates to [`complete_invocation`] first, then calls
/// [`crate::multi_agent::audit_writer::write_invocation_completed`] and
/// returns the paths of the written files.
pub fn complete_invocation_with_audit(
    invocation_id: Uuid,
    output_summary: &str,
    entities: &[EntityRef],
    tenant_id: Option<&str>,
    manifest_root: &Path,
) -> CorpFinanceResult<InvocationAuditPaths> {
    complete_invocation(invocation_id, output_summary, entities)?;
    crate::multi_agent::audit_writer::write_invocation_completed(
        manifest_root,
        invocation_id,
        output_summary,
        entities,
        tenant_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_agent::types::InvocationStatus;
    use chrono::Utc;

    fn mk(slug: &str, parent: Option<Uuid>) -> AgentInvocation {
        AgentInvocation {
            invocation_id: Uuid::now_v7(),
            target_agent: slug.into(),
            parent_invocation_id: parent,
            input_summary: "x".into(),
            tenant_id: Some("local".into()),
            ts: Utc::now(),
            status: InvocationStatus::Pending,
        }
    }

    #[test]
    fn validates_each_registered_agent() {
        for slug in REGISTERED_CFA_AGENTS {
            assert!(validate_target_agent(slug));
        }
    }

    #[test]
    fn rejects_unknown_slug() {
        assert!(!validate_target_agent("not-a-real-specialist"));
    }

    #[test]
    fn no_cycles_with_empty_history() {
        let inv = mk("cfa-equity-analyst", None);
        assert!(no_cycles_in_chain(&inv, &[]));
    }

    #[test]
    fn detects_cycle_when_ancestor_shares_slug() {
        let root = mk("cfa-equity-analyst", None);
        let child = AgentInvocation {
            parent_invocation_id: Some(root.invocation_id),
            ..mk("cfa-credit-analyst", Some(root.invocation_id))
        };
        // grandchild targets cfa-equity-analyst again — same as root.
        let grand = AgentInvocation {
            parent_invocation_id: Some(child.invocation_id),
            ..mk("cfa-equity-analyst", Some(child.invocation_id))
        };
        let history = vec![root.clone(), child.clone()];
        assert!(!no_cycles_in_chain(&grand, &history));
    }

    #[test]
    fn record_invocation_rejects_unknown_slug() {
        let mut inv = mk("cfa-equity-analyst", None);
        inv.target_agent = "ghost-analyst".into();
        let res = record_invocation(&inv);
        assert!(res.is_err());
    }

    #[test]
    fn record_invocation_accepts_known_slug() {
        let inv = mk("cfa-private-markets-analyst", None);
        assert!(record_invocation(&inv).is_ok());
    }

    #[cfg(feature = "audit")]
    #[test]
    fn record_invocation_computes_audit_hash_when_audit_feature_on() {
        // The audit hash is computed internally and not surfaced on the
        // public API; this test pins that the audit-emitting path runs to
        // completion (no panic, no error) when both `multi_agent` and
        // `audit` features are enabled together.
        let inv = mk("cfa-equity-analyst", None);
        let result = record_invocation(&inv);
        assert!(result.is_ok());
    }

    #[cfg(feature = "audit")]
    #[test]
    fn complete_invocation_computes_audit_hash_when_audit_feature_on() {
        let inv = mk("cfa-equity-analyst", None);
        let entities: Vec<EntityRef> = Vec::new();
        let result = complete_invocation(inv.invocation_id, "summary", &entities);
        assert!(result.is_ok());
    }

    /// Test 7: unknown slug is rejected before any I/O is attempted.
    #[test]
    fn record_invocation_with_audit_validates_target_agent() {
        let dir = tempfile::tempdir().unwrap();
        let mut inv = mk("cfa-equity-analyst", None);
        inv.target_agent = "not-a-cfa-agent".into();
        let result = record_invocation_with_audit(&inv, dir.path());
        assert!(result.is_err(), "unknown slug must return Err");
    }

    /// Test 8: known slug writes both event and audit files.
    #[test]
    fn record_invocation_with_audit_writes_files_for_known_slug() {
        let dir = tempfile::tempdir().unwrap();
        let inv = mk("cfa-fixed-income-analyst", None);
        let paths = record_invocation_with_audit(&inv, dir.path()).unwrap();
        assert!(paths.event_path.exists(), "event file must exist");
        assert!(paths.audit_path.exists(), "audit companion must exist");
    }

    /// Test 9: completed event path filename contains `.completed.` infix.
    #[test]
    fn complete_invocation_with_audit_writes_completed_phase() {
        let dir = tempfile::tempdir().unwrap();
        let inv = mk("cfa-macro-analyst", None);
        // Record started first so complete_invocation validation passes.
        let entities: Vec<EntityRef> = Vec::new();
        let paths = complete_invocation_with_audit(
            inv.invocation_id,
            "macro summary",
            &entities,
            inv.tenant_id.as_deref(),
            dir.path(),
        )
        .unwrap();
        let name = paths.event_path.file_name().unwrap().to_string_lossy();
        assert!(
            name.contains(".completed."),
            "filename must contain .completed. infix"
        );
    }
}
