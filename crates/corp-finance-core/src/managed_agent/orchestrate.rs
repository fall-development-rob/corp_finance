//! Event-loop routing for managed-agent orchestration.
//!
//! Pure function: takes a parsed `OrchestrateEvent` and an optional allowlist,
//! validates the event type and payload schema, and returns a `OrchestrateOutput`
//! with a routing decision. No network I/O.
//!
//! Designed so HTTP dispatch can be added by the caller later without changing
//! the core routing logic.
//!
//! Pattern mirrors `crates/corp-finance-core/src/workflows/` module structure.

use crate::managed_agent::types::{
    DispatchDecision, OrchestrateInput, OrchestrateOutput, ALLOWED_SLUGS,
};
use crate::CorpFinanceResult;

/// Route an incoming event to the appropriate agent.
///
/// Validation rules:
/// 1. `event.event_type` must be `"handoff_request"`.
/// 2. `event.target` must be in the effective allowlist.
/// 3. `event.payload` must be a JSON object (not null, array, or scalar).
///
/// On success returns `OrchestrateOutput { accepted: true, dispatch: Some(...) }`.
/// On failure returns `OrchestrateOutput { accepted: false, dispatch: None }`.
pub fn route_event(input: &OrchestrateInput) -> CorpFinanceResult<OrchestrateOutput> {
    let event = &input.event;

    // 1. Event type check
    if event.event_type != "handoff_request" {
        return Ok(OrchestrateOutput {
            accepted: false,
            target: event.target.clone(),
            reason: format!(
                "Unknown event_type '{}'; only 'handoff_request' is accepted",
                event.event_type
            ),
            dispatch: None,
        });
    }

    // 2. Allowlist check
    let effective_allowlist: Vec<&str> = input
        .allowlist
        .as_ref()
        .map(|v| v.iter().map(String::as_str).collect())
        .unwrap_or_else(|| ALLOWED_SLUGS.to_vec());

    if !effective_allowlist.contains(&event.target.as_str()) {
        return Ok(OrchestrateOutput {
            accepted: false,
            target: event.target.clone(),
            reason: format!(
                "Target '{}' is not in the allowlist: {:?}",
                event.target, effective_allowlist
            ),
            dispatch: None,
        });
    }

    // 3. Payload must be a JSON object
    if !event.payload.is_object() {
        return Ok(OrchestrateOutput {
            accepted: false,
            target: event.target.clone(),
            reason: "payload must be a JSON object".to_string(),
            dispatch: None,
        });
    }

    Ok(OrchestrateOutput {
        accepted: true,
        target: event.target.clone(),
        reason: "event validated and routed".to_string(),
        dispatch: Some(DispatchDecision {
            agent_slug: event.target.clone(),
            event_type: event.event_type.clone(),
            payload: event.payload.clone(),
        }),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managed_agent::types::OrchestrateEvent;
    use serde_json::json;

    fn make_input(
        event_type: &str,
        target: &str,
        payload: serde_json::Value,
        allowlist: Option<Vec<String>>,
    ) -> OrchestrateInput {
        OrchestrateInput {
            event: OrchestrateEvent {
                event_type: event_type.to_string(),
                target: target.to_string(),
                payload,
            },
            allowlist,
        }
    }

    #[test]
    fn valid_event_is_accepted() {
        let input = make_input(
            "handoff_request",
            "equity-analyst",
            json!({"ticker": "AAPL"}),
            None,
        );
        let out = route_event(&input).unwrap();
        assert!(out.accepted);
        assert_eq!(out.target, "equity-analyst");
        assert!(out.dispatch.is_some());
        let d = out.dispatch.unwrap();
        assert_eq!(d.agent_slug, "equity-analyst");
    }

    #[test]
    fn wrong_event_type_is_rejected() {
        let input = make_input(
            "unknown_event",
            "equity-analyst",
            json!({"ticker": "AAPL"}),
            None,
        );
        let out = route_event(&input).unwrap();
        assert!(!out.accepted);
        assert!(out.reason.contains("Unknown event_type"));
    }

    #[test]
    fn unknown_target_is_rejected() {
        let input = make_input(
            "handoff_request",
            "rogue-agent",
            json!({"ticker": "AAPL"}),
            None,
        );
        let out = route_event(&input).unwrap();
        assert!(!out.accepted);
        assert!(out.reason.contains("not in the allowlist"));
    }

    #[test]
    fn non_object_payload_is_rejected() {
        let input = make_input(
            "handoff_request",
            "equity-analyst",
            json!("just a string"),
            None,
        );
        let out = route_event(&input).unwrap();
        assert!(!out.accepted);
        assert!(out.reason.contains("JSON object"));
    }

    #[test]
    fn custom_allowlist_restricts_targets() {
        let input = make_input(
            "handoff_request",
            "equity-analyst",
            json!({"ticker": "AAPL"}),
            Some(vec!["credit-analyst".to_string()]),
        );
        let out = route_event(&input).unwrap();
        assert!(!out.accepted);
    }

    #[test]
    fn custom_allowlist_permits_target() {
        let input = make_input(
            "handoff_request",
            "credit-analyst",
            json!({"issuer": "MSFT"}),
            Some(vec!["credit-analyst".to_string()]),
        );
        let out = route_event(&input).unwrap();
        assert!(out.accepted);
    }

    #[test]
    fn dispatch_contains_full_payload() {
        let payload = json!({"ticker": "NVDA", "period": "Q4-2025"});
        let input = make_input("handoff_request", "equity-analyst", payload.clone(), None);
        let out = route_event(&input).unwrap();
        let d = out.dispatch.unwrap();
        assert_eq!(d.payload, payload);
    }

    #[test]
    fn null_payload_is_rejected() {
        let input = make_input(
            "handoff_request",
            "equity-analyst",
            serde_json::Value::Null,
            None,
        );
        let out = route_event(&input).unwrap();
        assert!(!out.accepted);
    }
}
