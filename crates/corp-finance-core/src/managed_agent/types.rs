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

/// Cost tier for a managed-agent cookbook.
///
/// Lets users discover which cookbooks they can run without paying for
/// any vendor data subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CookbookTier {
    /// Runs against `cfa-core` only. No data feeds, no vendor APIs, no FMP.
    /// User supplies inputs as JSON. Always free to run.
    CoreOnly,
    /// Runs against `cfa-core` + free public data sources (FRED, EDGAR, FIGI,
    /// YF, WB, geopolitical) and/or FMP (which has a free tier). No paid
    /// vendor subscription required.
    Freemium,
    /// Requires a paid vendor subscription (LSEG, S&P Global, FactSet,
    /// Morningstar, Moody's, PitchBook, Aiera, Daloopa). User must supply
    /// vendor API credentials at deploy time.
    PaidVendor,
}

/// One row of the canonical cookbook registry: slug + cost tier + the
/// upstream MCP servers it expects (for documentation; runtime config
/// lives in each cookbook's `agent.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookbookRegistryEntry {
    pub slug: &'static str,
    pub tier: CookbookTier,
    /// Short tag for the vendor / data dependency, e.g. "cfa-core only",
    /// "cfa-core + FMP", "cfa-core + LSEG (paid)".
    pub dependencies: &'static str,
}

/// Canonical cookbook registry — slug, tier, dependency tag.
///
/// Keep alphabetically grouped by tier so the `list` CLI output is stable.
pub const COOKBOOK_REGISTRY: &[CookbookRegistryEntry] = &[
    // ---- CoreOnly: cfa-core only, user supplies inputs ----
    CookbookRegistryEntry {
        slug: "gl-reconciler",
        tier: CookbookTier::CoreOnly,
        dependencies: "cfa-core only",
    },
    CookbookRegistryEntry {
        slug: "kyc-screener",
        tier: CookbookTier::CoreOnly,
        dependencies: "cfa-core only",
    },
    CookbookRegistryEntry {
        slug: "lp-statement-auditor",
        tier: CookbookTier::CoreOnly,
        dependencies: "cfa-core only",
    },
    CookbookRegistryEntry {
        slug: "model-builder",
        tier: CookbookTier::CoreOnly,
        dependencies: "cfa-core (user supplies fundamentals JSON)",
    },
    CookbookRegistryEntry {
        slug: "month-end-closer",
        tier: CookbookTier::CoreOnly,
        dependencies: "cfa-core only",
    },
    // ---- Freemium: cfa-core + FMP and/or free public data ----
    CookbookRegistryEntry {
        slug: "credit-analyst",
        tier: CookbookTier::Freemium,
        dependencies: "cfa-core + FMP (free tier)",
    },
    CookbookRegistryEntry {
        slug: "earnings-reviewer",
        tier: CookbookTier::Freemium,
        dependencies: "cfa-core + FMP + data (FRED/EDGAR free)",
    },
    CookbookRegistryEntry {
        slug: "equity-analyst",
        tier: CookbookTier::Freemium,
        dependencies: "cfa-core + FMP (free tier)",
    },
    CookbookRegistryEntry {
        slug: "pitch-deck-builder",
        tier: CookbookTier::Freemium,
        dependencies: "cfa-core + FMP (free tier)",
    },
    CookbookRegistryEntry {
        slug: "private-markets-analyst",
        tier: CookbookTier::Freemium,
        dependencies: "cfa-core + FMP (free tier)",
    },
    CookbookRegistryEntry {
        slug: "sector-research",
        tier: CookbookTier::Freemium,
        dependencies: "cfa-core + FMP + data (FRED free)",
    },
    CookbookRegistryEntry {
        slug: "valuation-reviewer",
        tier: CookbookTier::Freemium,
        dependencies: "cfa-core + FMP (free tier)",
    },
    CookbookRegistryEntry {
        slug: "wealth-meeting-prep",
        tier: CookbookTier::Freemium,
        dependencies: "cfa-core + FMP (free tier)",
    },
    // ---- PaidVendor: requires vendor subscription ----
    CookbookRegistryEntry {
        slug: "lseg-rates-monitor",
        tier: CookbookTier::PaidVendor,
        dependencies: "cfa-core + LSEG (paid OAuth2)",
    },
    CookbookRegistryEntry {
        slug: "sp-credit-research",
        tier: CookbookTier::PaidVendor,
        dependencies: "cfa-core + S&P Global (paid Bearer) + FMP fallback",
    },
];

/// Canonical agent slugs that may be deployed. Derived from `COOKBOOK_REGISTRY`
/// so the two are kept in sync. Order: CoreOnly, then Freemium, then PaidVendor.
pub const ALLOWED_SLUGS: &[&str] = &[
    // CoreOnly
    "gl-reconciler",
    "kyc-screener",
    "lp-statement-auditor",
    "model-builder",
    "month-end-closer",
    // Freemium
    "credit-analyst",
    "earnings-reviewer",
    "equity-analyst",
    "pitch-deck-builder",
    "private-markets-analyst",
    "sector-research",
    "valuation-reviewer",
    "wealth-meeting-prep",
    // PaidVendor
    "lseg-rates-monitor",
    "sp-credit-research",
];

/// Look up the registry entry for a slug, if any.
pub fn cookbook_registry_entry(slug: &str) -> Option<&'static CookbookRegistryEntry> {
    COOKBOOK_REGISTRY.iter().find(|e| e.slug == slug)
}

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
// Sync types — coverage/freshness audit between cookbooks and skills.
//
// Pattern note: in this repo, cookbooks reference skills by slug; the skill
// content lives in `.claude/skills/<slug>/SKILL.md` and is resolved at deploy
// time (no bundled copies). `sync_skills` therefore does NOT copy files —
// it audits which skills are referenced by which cookbooks, flags references
// that don't resolve to a real `SKILL.md`, and flags skills on disk that no
// cookbook references (orphans).
// ---------------------------------------------------------------------------

/// Input for `sync::sync_skills`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncInput {
    /// Absolute path to the cookbooks root.
    pub cookbooks_root: String,
    /// Absolute path to the skills root.
    pub skills_root: String,
}

/// A single missing-skill record (referenced by a cookbook but not on disk).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUsage {
    pub cookbook: String,
    pub skill: String,
}

/// Output of `sync::sync_skills`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOutput {
    pub cookbooks_root: String,
    pub skills_root: String,
    pub total_cookbooks: usize,
    pub total_skills: usize,
    /// Map of `skill_slug` → list of cookbook slugs that reference it.
    /// Sorted alphabetically by skill, then by cookbook within each list.
    pub skill_usage: HashMap<String, Vec<String>>,
    /// Skills referenced by cookbooks that don't resolve to `<skills_root>/<slug>/SKILL.md`.
    pub missing_skills: Vec<SkillUsage>,
    /// Skills present on disk that no cookbook references.
    pub orphan_skills: Vec<String>,
}

// ---------------------------------------------------------------------------
// List types — enumerate cookbooks by cost tier (no I/O, just registry).
// ---------------------------------------------------------------------------

/// Input for `list::list_cookbooks`. If `tier` is `None`, all cookbooks are
/// returned grouped by tier. If set, only that tier's cookbooks are returned.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ListInput {
    /// Optional filter by cost tier.
    pub tier: Option<CookbookTier>,
}

/// One row in `ListOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEntry {
    pub slug: String,
    pub tier: CookbookTier,
    pub dependencies: String,
}

/// Output of `list::list_cookbooks`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListOutput {
    pub total: usize,
    pub core_only: usize,
    pub freemium: usize,
    pub paid_vendor: usize,
    pub entries: Vec<ListEntry>,
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
