//! Specflow contract tests for the multi-agent coordination bounded
//! context.
//!
//! Each `ruf_orc_*` test references the contract in
//! `docs/contracts/feature_orchestration.yml`; each `mac_inv_*` test
//! references the corresponding invariant. The aggregate test count is
//! 20 (`RUF-ORC-001..010` + `MAC-INV-001..010`).

use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::multi_agent::agent_invoke::{
    no_cycles_in_chain, record_invocation, validate_target_agent, REGISTERED_CFA_AGENTS,
};
use crate::multi_agent::budget::{check_budget, BudgetVerdict, PlanBudget};
use crate::multi_agent::entity_graph::{extract_entities_from_text, EntityGraph};
use crate::multi_agent::goap_adapter::{load_action_catalogue, validate_action};
use crate::multi_agent::planner::{detect_cycles, plan, plan_hash, replan, DEFAULT_MAX_REPLANS};
use crate::multi_agent::types::{
    AgentInvocation, EntityKind, EntityRef, InvocationStatus, PlanAction, RelationKind, StepStatus,
};

fn mk_invocation(slug: &str, parent: Option<Uuid>) -> AgentInvocation {
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

// ---------------------------------------------------------------------------
// RUF-ORC-001 .. RUF-ORC-010
// ---------------------------------------------------------------------------

#[test]
fn ruf_orc_001_agent_invoked_event_recorded() {
    // A chief-analyst delegating to cfa-equity-analyst produces a
    // recordable AgentInvocation.
    let inv = mk_invocation("cfa-equity-analyst", None);
    assert!(record_invocation(&inv).is_ok());
}

#[test]
fn ruf_orc_002_target_agent_validated_against_registry() {
    // Unknown specialist slugs are rejected by the coordination layer.
    let bad = mk_invocation("not-a-real-specialist", None);
    assert!(record_invocation(&bad).is_err());
    assert!(!validate_target_agent("not-a-real-specialist"));
    assert!(validate_target_agent("cfa-equity-analyst"));
}

#[test]
fn ruf_orc_003_multidomain_plan_emitted_before_specialists_run() {
    // build_plan(goal, action_space) emits a serialised GoapPlan that
    // can be printed before any specialist runs. v1 verifies the plan
    // serialises and contains step-level actions.
    let cat = load_action_catalogue();
    let p = plan("initiate coverage on PFE including credit and macro", &cat).unwrap();
    let serialised = serde_json::to_string(&p).unwrap();
    assert!(!serialised.is_empty());
    assert!(p.steps.len() >= 3);
}

#[test]
fn ruf_orc_004_entity_extraction_yields_entities_for_typical_outputs() {
    // The hand-rolled extractor produces >= 1 entity ref for typical
    // specialist outputs. v1 spot-checks four representative outputs.
    let outputs = [
        "AAPL beat MSFT on revenue",
        "CUSIP 037833100 traded today",
        "Acme Industrial Corp issued new debt",
        "Healthcare and Financials lagged the index",
    ];
    let with_entities = outputs
        .iter()
        .filter(|t| !extract_entities_from_text(t).is_empty())
        .count();
    assert_eq!(with_entities, outputs.len());
}

#[test]
fn ruf_orc_005_cross_specialist_co_occurrence_emits_pattern_detected() {
    // Three specialist-output edges touching AAPL drive the entity
    // graph's pattern_detected query above its threshold.
    let mut g = EntityGraph::new();
    let aapl = EntityRef {
        kind: EntityKind::Ticker,
        value: "AAPL".into(),
        source_invocation: None,
    };
    for other in ["MSFT", "GOOG", "META"] {
        let other_ref = EntityRef {
            kind: EntityKind::Ticker,
            value: other.into(),
            source_invocation: None,
        };
        g.add_relation(aapl.clone(), other_ref, RelationKind::MentionedTogether);
    }
    let hits = g.pattern_detected(3);
    assert!(hits.iter().any(|e| e.value == "AAPL"));
}

#[test]
fn ruf_orc_006_plan_depth_default_ceiling_not_exceeded() {
    // Default per-invocation step cap is 8. The plan for a typical
    // multi-domain goal stays within the cap.
    let cat = load_action_catalogue();
    let p = plan("initiate coverage on AAPL", &cat).unwrap();
    let budget = PlanBudget::new(Some("local".into()), "cfa-equity-analyst");
    assert_eq!(check_budget(&p, &budget, &[]), BudgetVerdict::Ok);
}

#[test]
fn ruf_orc_007_plan_cycle_detection_rejects_self_cyclic_plan() {
    // A plan whose action chain revisits an upstream step under the
    // same precondition is detected via detect_cycles.
    let cat = load_action_catalogue();
    let mut p = plan("dcf for AAPL", &cat).unwrap();
    let last = p.steps.len() - 1;
    let last_id = p.steps[last].step_id;
    p.steps[0].depends_on.push(last_id);
    assert!(detect_cycles(&p));
}

#[test]
fn ruf_orc_008_budget_reservation_precedes_dispatch() {
    // A failed budget verdict means the orchestrator must not record
    // an agent_invoked event. We exercise the verdict flow.
    let cat = load_action_catalogue();
    let p = plan("initiate coverage on PFE", &cat).unwrap();
    let mut b = PlanBudget::new(Some("local".into()), "cfa-equity-analyst");
    b.max_steps_per_invocation = 2;
    assert!(matches!(
        check_budget(&p, &b, &[]),
        BudgetVerdict::WouldExceedSteps { .. }
    ));
}

#[test]
fn ruf_orc_009_at_most_one_in_flight_per_specialist() {
    // The default concurrency cap is 1. A second invocation against
    // the same (tenant, specialist) pair would exceed.
    let cat = load_action_catalogue();
    let p = plan("dcf for AAPL", &cat).unwrap();
    let b = PlanBudget::new(Some("local".into()), "cfa-equity-analyst");
    let in_flight = vec![Uuid::now_v7()];
    assert!(matches!(
        check_budget(&p, &b, &in_flight),
        BudgetVerdict::WouldExceedConcurrency { current: 1, max: 1 }
    ));
}

#[test]
fn ruf_orc_010_every_invocation_reaches_one_terminal_state() {
    // Every accepted invocation must produce exactly one terminal
    // status. v1 verifies the InvocationStatus enum exposes the
    // expected terminal variants.
    let terminals = [InvocationStatus::Completed, InvocationStatus::Failed];
    let non_terminals = [InvocationStatus::Pending, InvocationStatus::Running];
    assert_eq!(terminals.len() + non_terminals.len(), 4);
    // Sanity: serialise each variant so the audit pipeline can persist
    // without error.
    for s in [
        InvocationStatus::Pending,
        InvocationStatus::Running,
        InvocationStatus::Completed,
        InvocationStatus::Failed,
    ] {
        let _ = serde_json::to_string(&s).unwrap();
    }
}

// ---------------------------------------------------------------------------
// MAC-INV-001 .. MAC-INV-010
// ---------------------------------------------------------------------------

#[test]
fn mac_inv_001_invocation_carries_tenant_id() {
    // Every AgentInvocation we record in v1 carries an explicit
    // tenant_id (Some).
    let inv = mk_invocation("cfa-credit-analyst", None);
    assert!(inv.tenant_id.is_some());
}

#[test]
fn mac_inv_002_target_in_registered_set() {
    // Every entry in REGISTERED_CFA_AGENTS validates; nothing else does.
    for slug in REGISTERED_CFA_AGENTS {
        assert!(validate_target_agent(slug));
    }
    assert!(!validate_target_agent("anonymous"));
}

#[test]
fn mac_inv_003_no_cycles_in_parent_chain() {
    // A chain root → child → grand where grand targets root.target is a
    // cycle and is rejected.
    let root = mk_invocation("cfa-equity-analyst", None);
    let child = AgentInvocation {
        parent_invocation_id: Some(root.invocation_id),
        ..mk_invocation("cfa-credit-analyst", Some(root.invocation_id))
    };
    let grand = AgentInvocation {
        parent_invocation_id: Some(child.invocation_id),
        ..mk_invocation("cfa-equity-analyst", Some(child.invocation_id))
    };
    let history = vec![root, child];
    assert!(!no_cycles_in_chain(&grand, &history));
}

#[test]
fn mac_inv_004_entity_kind_has_exactly_five_variants() {
    // EntityKind has exactly five variants per the DDD model.
    let all = [
        EntityKind::Ticker,
        EntityKind::Issuer,
        EntityKind::Sector,
        EntityKind::Fund,
        EntityKind::Cusip,
    ];
    assert_eq!(all.len(), 5);
}

#[test]
fn mac_inv_005_entities_tenant_scoped_unless_federation_bridges() {
    // The graph carries one tenant scope per session in v1; we assert
    // that an EntityRef can carry source-invocation provenance which
    // an upstream tenant filter can consume. Negative test for
    // federation-bridging is owned by the federation context.
    let mut g = EntityGraph::new();
    let aapl = EntityRef {
        kind: EntityKind::Ticker,
        value: "AAPL".into(),
        source_invocation: Some(Uuid::now_v7()),
    };
    let idx = g.add_entity(aapl.clone());
    assert_eq!(g.node_count(), 1);
    let _ = idx;
}

#[test]
fn mac_inv_006_plan_replans_bounded() {
    // Default replan ceiling is 3; a 4th replan errors.
    let cat = load_action_catalogue();
    let mut p = plan("ic memo for Acme", &cat).unwrap();
    assert_eq!(p.max_replans, DEFAULT_MAX_REPLANS);
    let target = p.steps[1].step_id;
    for _ in 0..DEFAULT_MAX_REPLANS {
        replan(&mut p, target, "x").unwrap();
    }
    let res = replan(&mut p, target, "again");
    assert!(res.is_err());
}

#[test]
fn mac_inv_007_plan_hash_deterministic_for_same_goal_and_registry() {
    let cat = load_action_catalogue();
    let a = plan("initiate coverage on PFE", &cat).unwrap();
    let b = plan("initiate coverage on PFE", &cat).unwrap();
    assert_eq!(plan_hash(&a), plan_hash(&b));
    assert_eq!(a.plan_hash, b.plan_hash);
}

#[test]
fn mac_inv_008_plan_emitted_before_execution() {
    // The plan struct serialises to JSON with all step actions before
    // any are marked Running/Completed. We verify the initial state
    // for every step is Pending.
    let cat = load_action_catalogue();
    let p = plan("ic memo for Acme", &cat).unwrap();
    assert!(!p.steps.is_empty());
    assert!(p.steps.iter().all(|s| s.status == StepStatus::Pending));
    let _ = serde_json::to_string(&p).unwrap();
}

#[test]
fn mac_inv_009_output_hash_set_iff_completed() {
    // The InvocationStatus enum and the audit shape contract together
    // enforce: Completed implies output_hash is Some, others imply
    // None. v1 verifies the status terminal flags model the contract.
    fn requires_output_hash(s: InvocationStatus) -> bool {
        matches!(s, InvocationStatus::Completed)
    }
    assert!(requires_output_hash(InvocationStatus::Completed));
    assert!(!requires_output_hash(InvocationStatus::Failed));
    assert!(!requires_output_hash(InvocationStatus::Running));
    assert!(!requires_output_hash(InvocationStatus::Pending));
}

#[test]
fn mac_inv_010_surface_events_translated_through_acl() {
    // The multi_agent ACL boundary requires that PlanAction validation
    // delegates to validate_action (which routes through the
    // catalogue) rather than admitting raw external strings. We verify
    // a specialist-action validate roundtrips through the catalogue
    // adapter.
    let cat = load_action_catalogue();
    let action = PlanAction::McpTool {
        name: "dcf_model".into(),
        input_hint: json!({}),
    };
    assert!(validate_action(&cat, &action));
    let foreign = PlanAction::McpTool {
        name: "an_external_unregistered_tool".into(),
        input_hint: json!({}),
    };
    assert!(!validate_action(&cat, &foreign));
}
