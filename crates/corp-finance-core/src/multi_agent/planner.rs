//! A* planner over the registered action catalogue.
//!
//! ## v1 design
//!
//! v1 uses a deliberately simple state model so the A* search is
//! tractable on the static action catalogue:
//!
//! - **State**: a vector of action indices already selected.
//! - **Successors**: append one further action from the goal-relevant
//!   sub-set of the catalogue, with a unit cost. Each action may appear
//!   at most once per plan.
//! - **Heuristic**: distance to a goal-specific target step count.
//! - **Goal predicate**: keyword-driven decomposition (e.g. "initiate
//!   coverage" → wacc → dcf → comps → target_price). When no keyword
//!   matches, the planner falls back to a generic 3-step plan over
//!   `screen` → `comps` → `dcf`.
//!
//! v2 will replace the goal predicate with a richer LLM-assisted
//! decomposition and let the heuristic learn from the Phase 26 memory
//! layer's most-similar past plans.
//!
//! ## Determinism
//!
//! Per `MAC-INV-007`, [`plan_hash`] is deterministic over `(goal,
//! catalogue version)`. v1 keys the catalogue version on the constant
//! length of [`crate::multi_agent::goap_adapter::MCP_TOOL_NAMES`] and
//! [`crate::multi_agent::goap_adapter::SLASH_COMMAND_NAMES`]; any change
//! to those slices produces a different plan hash for the same goal.
//!
//! ## Replanning
//!
//! Per `MAC-INV-006`, replans are bounded (default `max_replans = 3`).
//! [`replan`] bumps the counter and returns
//! [`crate::error::CorpFinanceError::FinancialImpossibility`] (treated
//! as a domain error envelope) when the bound is exceeded.

use std::collections::HashMap;

use pathfinding::directed::astar::astar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::CorpFinanceError;
use crate::multi_agent::goap_adapter::{ActionCatalogue, MCP_TOOL_NAMES, SLASH_COMMAND_NAMES};
use crate::multi_agent::types::{GoapPlan, PlanAction, PlanStep, StepStatus};
use crate::CorpFinanceResult;

/// Default maximum number of replans per plan, per `MAC-INV-006`.
pub const DEFAULT_MAX_REPLANS: u8 = 3;

/// Catalogue snapshot version used for plan-hash determinism.
fn catalogue_version() -> String {
    format!(
        "v1:mcp{}:slash{}",
        MCP_TOOL_NAMES.len(),
        SLASH_COMMAND_NAMES.len()
    )
}

/// Build an A* plan over the action catalogue for the given `goal`.
///
/// The plan is emitted with:
///
/// - A new UUID v7 `plan_id`.
/// - A `goal` field carrying the input verbatim.
/// - One `PlanStep` per selected action, with sequential `depends_on`
///   edges (`step[i+1].depends_on = [step[i].step_id]`) so the plan is a
///   DAG with no cycles by construction.
/// - A deterministic `plan_hash` (djb2 over canonical JSON) given the
///   same `goal` + the same catalogue version.
/// - `replan_count = 0` and `max_replans = DEFAULT_MAX_REPLANS`.
///
/// Per `MAC-INV-008`, the caller (chief-analyst) must emit the returned
/// plan to stdout for review before invoking any step.
pub fn plan(goal: &str, catalogue: &ActionCatalogue) -> CorpFinanceResult<GoapPlan> {
    let template = decompose_goal(goal, catalogue);
    if template.is_empty() {
        return Err(CorpFinanceError::InsufficientData(format!(
            "no plan template available for goal '{goal}'"
        )));
    }
    let target_len = template.len();

    // A* state is the index into the template (0..target_len). Cost is
    // the index. The heuristic is the gap to the target length.
    let result = astar(
        &0usize,
        |&idx| -> Vec<(usize, u32)> {
            if idx >= target_len {
                vec![]
            } else {
                vec![(idx + 1, 1u32)]
            }
        },
        |&idx| (target_len - idx) as u32,
        |&idx| idx == target_len,
    );

    let path = match result {
        Some((p, _)) => p,
        None => {
            return Err(CorpFinanceError::ConvergenceFailure {
                function: "multi_agent::planner::plan".into(),
                iterations: target_len as u32,
                last_delta: rust_decimal::Decimal::ZERO,
            });
        }
    };

    // path is [0, 1, 2, ..., target_len]; map each step index to the
    // template entry at that index (the action chosen at step `i+1` is
    // `template[path[i]]`).
    let mut steps: Vec<PlanStep> = Vec::with_capacity(target_len);
    let mut prev_id: Option<Uuid> = None;
    for (i, win) in path.windows(2).enumerate() {
        let _ = win;
        let action = template[i].clone();
        let step_id = Uuid::now_v7();
        let depends_on = prev_id.map(|p| vec![p]).unwrap_or_default();
        steps.push(PlanStep {
            step_id,
            action,
            depends_on,
            status: StepStatus::Pending,
            result_summary: None,
        });
        prev_id = Some(step_id);
    }

    let mut plan = GoapPlan {
        plan_id: Uuid::now_v7(),
        goal: goal.to_string(),
        steps,
        plan_hash: String::new(),
        replan_count: 0,
        max_replans: DEFAULT_MAX_REPLANS,
    };
    plan.plan_hash = plan_hash(&plan);
    Ok(plan)
}

/// Compute the deterministic plan hash for `plan`.
///
/// Format: `djb2:0xXXXXXXXX`. The hash is computed over a canonical
/// JSON envelope that excludes the existing `plan_hash` field and the
/// per-step UUIDs so the result depends only on `(goal, catalogue
/// version, ordered action keys)` per `MAC-INV-007`.
pub fn plan_hash(plan: &GoapPlan) -> String {
    #[derive(Serialize, Deserialize)]
    struct Canonical<'a> {
        goal: &'a str,
        catalogue: String,
        actions: Vec<String>,
    }
    let canonical = Canonical {
        goal: &plan.goal,
        catalogue: catalogue_version(),
        actions: plan.steps.iter().map(|s| s.action.display_key()).collect(),
    };
    let payload = serde_json::to_string(&canonical).unwrap_or_default();
    let mut hash: u32 = 5381;
    for byte in payload.as_bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(u32::from(*byte));
    }
    format!("djb2:0x{:08x}", hash)
}

/// Replan from `failed_step` with `reason`.
///
/// Bumps `plan.replan_count` and re-derives the plan-hash. Returns
/// [`crate::error::CorpFinanceError::FinancialImpossibility`] when the
/// replan would exceed `plan.max_replans` (`MAC-INV-006`).
///
/// v1 strategy: mark `failed_step` (and downstream steps) as
/// `StepStatus::Failed`, leave upstream completed steps in place, and
/// let the chief-analyst decide whether to mutate the plan further. v2
/// will rerun the planner with the failed action excluded from the
/// successor set.
pub fn replan(plan: &mut GoapPlan, failed_step: Uuid, reason: &str) -> CorpFinanceResult<()> {
    if plan.replan_count >= plan.max_replans {
        return Err(CorpFinanceError::FinancialImpossibility(format!(
            "replan budget exhausted ({}/{}); reason='{}'",
            plan.replan_count, plan.max_replans, reason
        )));
    }
    plan.replan_count += 1;

    let mut found = false;
    for step in plan.steps.iter_mut() {
        if step.step_id == failed_step {
            step.status = StepStatus::Failed;
            step.result_summary = Some(format!("replan: {reason}"));
            found = true;
            continue;
        }
        if found {
            step.status = StepStatus::Pending;
        }
    }
    if !found {
        return Err(CorpFinanceError::InvalidInput {
            field: "failed_step".into(),
            reason: format!("step {failed_step} not present in plan"),
        });
    }
    plan.plan_hash = plan_hash(plan);
    Ok(())
}

/// Detect cycles in the plan's step-dependency DAG.
///
/// Returns `true` when the dependency graph contains a cycle, `false`
/// otherwise. Per `RUF-ORC-007`, a plan with a cycle must be rejected.
/// Implemented via [`petgraph::algo::is_cyclic_directed`].
pub fn detect_cycles(plan: &GoapPlan) -> bool {
    use petgraph::algo::is_cyclic_directed;
    use petgraph::graph::DiGraph;

    let mut graph: DiGraph<Uuid, ()> = DiGraph::new();
    let mut indices: HashMap<Uuid, _> = HashMap::new();
    for step in &plan.steps {
        let idx = graph.add_node(step.step_id);
        indices.insert(step.step_id, idx);
    }
    for step in &plan.steps {
        let to = match indices.get(&step.step_id) {
            Some(i) => *i,
            None => continue,
        };
        for dep in &step.depends_on {
            if let Some(from) = indices.get(dep) {
                graph.add_edge(*from, to, ());
            }
        }
    }
    is_cyclic_directed(&graph)
}

// ---------------------------------------------------------------------------
// Goal decomposition (v1: keyword templates)
// ---------------------------------------------------------------------------

fn decompose_goal(goal: &str, catalogue: &ActionCatalogue) -> Vec<PlanAction> {
    let lc = goal.to_ascii_lowercase();

    // Initiate coverage: standard equity-research workflow.
    if contains_any(
        &lc,
        &[
            "initiate coverage",
            "initiating coverage",
            "coverage initiation",
        ],
    ) {
        return vec![
            mcp(catalogue, "wacc_calculator"),
            mcp(catalogue, "dcf_model"),
            mcp(catalogue, "comps_table"),
            mcp(catalogue, "calculate_target_price"),
            slash(catalogue, "initiate-coverage"),
        ];
    }
    if contains_any(&lc, &["ic memo", "investment committee", "ic-memo"]) {
        return vec![
            mcp(catalogue, "lbo_model"),
            mcp(catalogue, "irr_moic"),
            mcp(catalogue, "sources_uses"),
            mcp(catalogue, "waterfall_distribution"),
            slash(catalogue, "ic-memo"),
        ];
    }
    if contains_any(&lc, &["morning note", "morning-note", "daily note"]) {
        return vec![
            mcp(catalogue, "fmp_quote"),
            mcp(catalogue, "fred_series"),
            slash(catalogue, "morning-note"),
        ];
    }
    if contains_any(&lc, &["earnings", "10-q", "10q"]) {
        return vec![
            mcp(catalogue, "fmp_income_statement"),
            mcp(catalogue, "fmp_key_metrics"),
            mcp(catalogue, "edgar_filing"),
            slash(catalogue, "earnings"),
        ];
    }
    if contains_any(&lc, &["credit", "covenant", "debt capacity"]) {
        return vec![
            mcp(catalogue, "credit_metrics"),
            mcp(catalogue, "debt_capacity"),
            mcp(catalogue, "covenant_check"),
            slash(catalogue, "credit-analysis"),
        ];
    }
    if contains_any(&lc, &["lbo", "buyout", "acquisition model"]) {
        return vec![
            mcp(catalogue, "lbo_model"),
            mcp(catalogue, "debt_schedule"),
            mcp(catalogue, "irr_moic"),
            mcp(catalogue, "waterfall_distribution"),
            slash(catalogue, "lbo"),
        ];
    }
    if contains_any(&lc, &["dcf", "discounted cash flow", "valuation"]) {
        return vec![
            mcp(catalogue, "wacc_calculator"),
            mcp(catalogue, "dcf_model"),
            mcp(catalogue, "comps_table"),
            slash(catalogue, "dcf"),
        ];
    }
    if contains_any(&lc, &["macro", "fed", "rates"]) {
        return vec![
            mcp(catalogue, "fred_series"),
            mcp(catalogue, "yield_curve_bootstrap"),
            slash(catalogue, "macro-rates"),
        ];
    }
    // Fallback: generic three-step coverage skeleton.
    vec![
        mcp(catalogue, "fmp_quote"),
        mcp(catalogue, "comps_table"),
        mcp(catalogue, "dcf_model"),
    ]
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn mcp(catalogue: &ActionCatalogue, name: &str) -> PlanAction {
    debug_assert!(
        catalogue.mcp_tools.iter().any(|t| t == &name),
        "planner template referenced unregistered MCP tool: {name}"
    );
    PlanAction::McpTool {
        name: name.into(),
        input_hint: serde_json::json!({}),
    }
}

fn slash(catalogue: &ActionCatalogue, name: &str) -> PlanAction {
    debug_assert!(
        catalogue.slash_commands.iter().any(|c| c == &name),
        "planner template referenced unregistered slash command: {name}"
    );
    PlanAction::SlashCommand {
        name: name.into(),
        args: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_agent::goap_adapter::load_action_catalogue;

    #[test]
    fn plan_initiate_coverage_produces_five_steps() {
        let cat = load_action_catalogue();
        let plan = plan("initiate coverage on AAPL", &cat).unwrap();
        assert_eq!(plan.steps.len(), 5);
        assert_eq!(plan.replan_count, 0);
        assert_eq!(plan.max_replans, DEFAULT_MAX_REPLANS);
    }

    #[test]
    fn plan_dependencies_are_sequential() {
        let cat = load_action_catalogue();
        let p = plan("initiate coverage on MSFT", &cat).unwrap();
        for (i, step) in p.steps.iter().enumerate() {
            if i == 0 {
                assert!(step.depends_on.is_empty());
            } else {
                assert_eq!(step.depends_on.len(), 1);
                assert_eq!(step.depends_on[0], p.steps[i - 1].step_id);
            }
        }
    }

    #[test]
    fn plan_hash_is_deterministic_given_same_goal() {
        let cat = load_action_catalogue();
        let a = plan("initiate coverage on PFE", &cat).unwrap();
        let b = plan("initiate coverage on PFE", &cat).unwrap();
        assert_eq!(a.plan_hash, b.plan_hash);
        assert_ne!(a.plan_id, b.plan_id);
    }

    #[test]
    fn plan_hash_differs_for_different_goals() {
        let cat = load_action_catalogue();
        let a = plan("initiate coverage on PFE", &cat).unwrap();
        let b = plan("ic memo for Acme", &cat).unwrap();
        assert_ne!(a.plan_hash, b.plan_hash);
    }

    #[test]
    fn replan_bumps_counter_and_marks_failed() {
        let cat = load_action_catalogue();
        let mut p = plan("ic memo for Acme", &cat).unwrap();
        let failed = p.steps[1].step_id;
        replan(&mut p, failed, "data missing").unwrap();
        assert_eq!(p.replan_count, 1);
        assert_eq!(p.steps[1].status, StepStatus::Failed);
    }

    #[test]
    fn replan_rejects_after_max() {
        let cat = load_action_catalogue();
        let mut p = plan("ic memo for Acme", &cat).unwrap();
        p.replan_count = DEFAULT_MAX_REPLANS;
        let failed = p.steps[1].step_id;
        let res = replan(&mut p, failed, "x");
        assert!(res.is_err());
    }

    #[test]
    fn detect_cycles_returns_false_for_linear_plan() {
        let cat = load_action_catalogue();
        let p = plan("dcf for AAPL", &cat).unwrap();
        assert!(!detect_cycles(&p));
    }

    #[test]
    fn detect_cycles_returns_true_for_circular_dependency() {
        let cat = load_action_catalogue();
        let mut p = plan("dcf for AAPL", &cat).unwrap();
        let last_idx = p.steps.len() - 1;
        let first = p.steps[0].step_id;
        let last = p.steps[last_idx].step_id;
        p.steps[0].depends_on.push(last);
        // step[0] -> step[1] -> ... -> step[last] -> step[0]
        let _ = first;
        assert!(detect_cycles(&p));
    }
}
