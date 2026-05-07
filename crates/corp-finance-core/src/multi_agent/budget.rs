//! Per-`(tenant, specialist)` plan budget verdicts.
//!
//! v1 enforces two budget bounds on a [`GoapPlan`] before the
//! orchestrator dispatches any specialist:
//!
//! - `max_steps_per_invocation` (default 8) — the plan cannot have more
//!   than this many steps for a single specialist invocation.
//! - `max_concurrent_per_specialist` (default 1) — only one in-flight
//!   invocation per `(tenant, specialist)` pair, per `RUF-ORC-009`.
//!
//! The module is pure: it reports verdicts and lets the caller decide
//! whether to dispatch, queue, or reject.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::multi_agent::types::GoapPlan;

/// Default per-invocation step cap. The 8-step ceiling matches the
/// PRD-27 hard plan-tree depth; it is conservative and intentionally low
/// so chief-analyst plans stay legible.
pub const DEFAULT_MAX_STEPS_PER_INVOCATION: u8 = 8;

/// Default per-`(tenant, specialist)` concurrency cap. v1 holds this at
/// 1 per `RUF-ORC-009`.
pub const DEFAULT_MAX_CONCURRENT_PER_SPECIALIST: u8 = 1;

/// Budget envelope for a single `(tenant, specialist)` slot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanBudget {
    pub tenant_id: Option<String>,
    pub specialist: String,
    pub max_steps_per_invocation: u8,
    pub max_concurrent_per_specialist: u8,
}

impl PlanBudget {
    /// Convenience constructor with the default caps.
    pub fn new(tenant_id: Option<String>, specialist: impl Into<String>) -> Self {
        Self {
            tenant_id,
            specialist: specialist.into(),
            max_steps_per_invocation: DEFAULT_MAX_STEPS_PER_INVOCATION,
            max_concurrent_per_specialist: DEFAULT_MAX_CONCURRENT_PER_SPECIALIST,
        }
    }
}

/// Verdict returned by [`check_budget`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum BudgetVerdict {
    /// Dispatch is allowed.
    Ok,
    /// Plan has more steps than the per-invocation cap.
    WouldExceedSteps { current: u8, max: u8 },
    /// `(tenant, specialist)` already has an in-flight invocation at
    /// or above the concurrency cap.
    WouldExceedConcurrency { current: u8, max: u8 },
}

/// Evaluate `plan` against `budget` given a slice of currently in-flight
/// invocation ids for the same `(tenant, specialist)` pair.
///
/// The caller is responsible for filtering `in_flight` to the relevant
/// pair; this function does not look at invocation metadata.
pub fn check_budget(plan: &GoapPlan, budget: &PlanBudget, in_flight: &[Uuid]) -> BudgetVerdict {
    let step_count = plan.steps.len();
    let step_count_u8 = u8::try_from(step_count).unwrap_or(u8::MAX);
    if step_count_u8 > budget.max_steps_per_invocation {
        return BudgetVerdict::WouldExceedSteps {
            current: step_count_u8,
            max: budget.max_steps_per_invocation,
        };
    }
    let concurrency = u8::try_from(in_flight.len()).unwrap_or(u8::MAX);
    if concurrency >= budget.max_concurrent_per_specialist {
        return BudgetVerdict::WouldExceedConcurrency {
            current: concurrency,
            max: budget.max_concurrent_per_specialist,
        };
    }
    BudgetVerdict::Ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_agent::goap_adapter::load_action_catalogue;
    use crate::multi_agent::planner::plan;

    #[test]
    fn defaults_match_constants() {
        let b = PlanBudget::new(Some("local".into()), "cfa-equity-analyst");
        assert_eq!(b.max_steps_per_invocation, DEFAULT_MAX_STEPS_PER_INVOCATION);
        assert_eq!(
            b.max_concurrent_per_specialist,
            DEFAULT_MAX_CONCURRENT_PER_SPECIALIST
        );
    }

    #[test]
    fn ok_when_within_budget() {
        let cat = load_action_catalogue();
        let p = plan("dcf for AAPL", &cat).unwrap();
        let b = PlanBudget::new(Some("local".into()), "cfa-equity-analyst");
        assert_eq!(check_budget(&p, &b, &[]), BudgetVerdict::Ok);
    }

    #[test]
    fn rejects_when_steps_exceed_cap() {
        let cat = load_action_catalogue();
        let p = plan("initiate coverage on PFE", &cat).unwrap();
        let mut b = PlanBudget::new(Some("local".into()), "cfa-equity-analyst");
        b.max_steps_per_invocation = 2;
        match check_budget(&p, &b, &[]) {
            BudgetVerdict::WouldExceedSteps { current, max } => {
                assert!(current > max);
                assert_eq!(max, 2);
            }
            _ => panic!("expected WouldExceedSteps verdict"),
        }
    }

    #[test]
    fn rejects_when_concurrency_at_cap() {
        let cat = load_action_catalogue();
        let p = plan("dcf for AAPL", &cat).unwrap();
        let b = PlanBudget::new(Some("local".into()), "cfa-equity-analyst");
        let in_flight = vec![Uuid::now_v7()];
        assert!(matches!(
            check_budget(&p, &b, &in_flight),
            BudgetVerdict::WouldExceedConcurrency { .. }
        ));
    }
}
