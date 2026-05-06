//! Type definitions for managed-agent deployment tooling.
//!
//! Follows the pattern established in `crates/corp-finance-core/src/workflows/types.rs`:
//! typed `*Input`/`*Output` structs with `serde::{Serialize, Deserialize}`, pure functions
//! returning `CorpFinanceResult<T>`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Allowlisted slugs
// ---------------------------------------------------------------------------

/// Canonical agent slugs that may be deployed.
pub const ALLOWED_SLUGS: &[&str] = &[
    // Phase 23 (original 3 analyst orchestrators)
    "equity-analyst",
    "private-markets-analyst",
    "credit-analyst",
    // Phase 24 — adopted from anthropics/financial-services upstream
    "pitch-deck-builder",
    "sector-research",
    "earnings-reviewer",
    "model-builder",
    "wealth-meeting-prep",
    "valuation-reviewer",
    "lp-statement-auditor",
    // Phase 24 — adapted from upstream skip-list to fit our deterministic pattern
    "gl-reconciler",
    "month-end-closer",
    "kyc-screener",
    "lseg-rates-monitor",
    "sp-credit-research",
];

// ---------------------------------------------------------------------------
// Manifest types  (deserialise agent.json)
// ---------------------------------------------------------------------------

/// The `system` block inside an `agent.json` manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSystem {
    /// Path to a `.md` system-prompt file (relative to the cookbook directory).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Inline system prompt text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Text appended after the resolved system prompt (headless suffix).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
}

/// A single tool entry inside an `agent.json` manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// An MCP server reference inside an `agent.json` manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestMcpServer {
    #[serde(rename = "type")]
    pub server_type: String,
    pub name: String,
    pub url: String,
}

/// A skill reference inside an `agent.json` manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSkill {
    pub from_skill: String,
}

/// A callable-agent reference inside an `agent.json` manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestCallableAgent {
    pub manifest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

/// Top-level `agent.json` manifest for a CFA managed agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentManifest {
    pub name: String,
    pub model: String,
    pub system: ManifestSystem,
    #[serde(default)]
    pub tools: Vec<ManifestTool>,
    #[serde(default)]
    pub mcp_servers: Vec<ManifestMcpServer>,
    #[serde(default)]
    pub skills: Vec<ManifestSkill>,
    #[serde(default)]
    pub callable_agents: Vec<ManifestCallableAgent>,
}

// ---------------------------------------------------------------------------
// Validate types
// ---------------------------------------------------------------------------

/// Input for `validate::validate_manifest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateInput {
    /// Agent slug — must be in `ALLOWED_SLUGS`.
    pub slug: String,
    /// Absolute path to the cookbooks root (e.g. `<repo>/managed-agent-cookbooks`).
    pub cookbooks_root: String,
    /// Absolute path to the skills root (e.g. `<repo>/.claude/skills`).
    pub skills_root: String,
    /// Absolute path to the agents directory (e.g. `<repo>/.claude/agents/cfa`).
    pub agents_root: String,
}

/// A single check result inside `ValidateOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub detail: String,
}

/// Output of `validate::validate_manifest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateOutput {
    pub slug: String,
    pub ok: bool,
    pub checks: Vec<CheckResult>,
}

// ---------------------------------------------------------------------------
// Deploy types
// ---------------------------------------------------------------------------

/// Input for `deploy::build_deploy_payload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployInput {
    /// Agent slug.
    pub slug: String,
    /// Absolute path to the cookbooks root.
    pub cookbooks_root: String,
    /// Absolute path to the skills root.
    pub skills_root: String,
    /// Absolute path to the agents directory.
    pub agents_root: String,
    /// Environment variable substitutions (from `std::env::vars()`).
    pub env_vars: HashMap<String, String>,
}

/// A single subagent payload inside `DeployOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentPayload {
    pub manifest_path: String,
    pub payload: serde_json::Value,
}

/// A single skill payload inside `DeployOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillPayload {
    pub from_skill: String,
    pub skill_path: String,
    pub content_length: usize,
}

/// Output of `deploy::build_deploy_payload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployOutput {
    pub slug: String,
    pub orchestrator_payload: serde_json::Value,
    pub subagent_payloads: Vec<SubagentPayload>,
    pub skill_payloads: Vec<SkillPayload>,
    pub dry_run: bool,
}

// ---------------------------------------------------------------------------
// CheckAll types — walk every cookbook in `cookbooks_root` and validate it.
// ---------------------------------------------------------------------------

/// Input for `validate::validate_all`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckAllInput {
    /// Absolute path to the cookbooks root.
    pub cookbooks_root: String,
    /// Absolute path to the skills root.
    pub skills_root: String,
    /// Absolute path to the agents directory.
    pub agents_root: String,
}

/// Per-cookbook outcome inside `CheckAllOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookbookOutcome {
    pub slug: String,
    pub ok: bool,
    pub failed_checks: Vec<String>,
}

/// Output of `validate::validate_all`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckAllOutput {
    pub cookbooks_root: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub outcomes: Vec<CookbookOutcome>,
}

// ---------------------------------------------------------------------------
// Orchestrate types
// ---------------------------------------------------------------------------

/// An event received by the orchestrator event loop (one JSON line on stdin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrateEvent {
    /// Must be `"handoff_request"`.
    pub event_type: String,
    /// Target agent slug to route to.
    pub target: String,
    /// Arbitrary event payload (must be a JSON object).
    pub payload: serde_json::Value,
}

/// Input for `orchestrate::route_event`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrateInput {
    /// The incoming event.
    pub event: OrchestrateEvent,
    /// Allowed target slugs; if `None` defaults to `ALLOWED_SLUGS`.
    pub allowlist: Option<Vec<String>>,
}

/// Output of `orchestrate::route_event`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrateOutput {
    pub accepted: bool,
    pub target: String,
    pub reason: String,
    pub dispatch: Option<DispatchDecision>,
}

/// Routing decision for an accepted event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchDecision {
    pub agent_slug: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_slugs_contains_known_agents() {
        assert!(ALLOWED_SLUGS.contains(&"equity-analyst"));
        assert!(ALLOWED_SLUGS.contains(&"private-markets-analyst"));
        assert!(ALLOWED_SLUGS.contains(&"credit-analyst"));
    }

    #[test]
    fn agent_manifest_round_trips_via_json() {
        let json = r#"{
            "name": "cfa-equity-analyst",
            "model": "claude-opus-4-7",
            "system": { "text": "Hello" },
            "tools": [],
            "mcp_servers": [],
            "skills": [{"from_skill": "corp-finance-analyst-core"}],
            "callable_agents": []
        }"#;
        let manifest: AgentManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "cfa-equity-analyst");
        assert_eq!(manifest.skills.len(), 1);
        let back = serde_json::to_string(&manifest).unwrap();
        assert!(back.contains("cfa-equity-analyst"));
    }

    #[test]
    fn orchestrate_event_deserialises() {
        let json = r#"{
            "event_type": "handoff_request",
            "target": "equity-analyst",
            "payload": {"ticker": "AAPL"}
        }"#;
        let event: OrchestrateEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, "handoff_request");
        assert_eq!(event.target, "equity-analyst");
    }

    #[test]
    fn validate_input_serialises() {
        let input = ValidateInput {
            slug: "equity-analyst".to_string(),
            cookbooks_root: "/repo/managed-agent-cookbooks".to_string(),
            skills_root: "/repo/.claude/skills".to_string(),
            agents_root: "/repo/.claude/agents/cfa".to_string(),
        };
        let v = serde_json::to_value(&input).unwrap();
        assert_eq!(v["slug"], "equity-analyst");
    }

    #[test]
    fn check_result_ok_and_fail() {
        let pass = CheckResult {
            name: "slug_allowlist".to_string(),
            passed: true,
            detail: "slug in allowlist".to_string(),
        };
        let fail = CheckResult {
            name: "system_prompt".to_string(),
            passed: false,
            detail: "file not found".to_string(),
        };
        assert!(pass.passed);
        assert!(!fail.passed);
    }
}
