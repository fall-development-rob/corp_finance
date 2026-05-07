//! Action-catalogue adapter exposing the registered MCP tools and CFA
//! slash commands as the planner's action space.
//!
//! ## v1 vs future versions
//!
//! v1 declares the action space as `&'static [&'static str]` constants
//! covering the most common ~40 MCP tools and ~50 CFA slash commands.
//! This is deliberately a snapshot, not an exhaustive registry — the full
//! `~594 MCP tools + ~57 slash commands` action space described in
//! ADR-018 is the eventual target. Phase 28 (per the same ADR) plans to
//! read the action catalogue dynamically from the MCP server registries
//! and the `.claude/commands/cfa/` directory at startup.
//!
//! For v1 the planner has enough action vocabulary to emit non-trivial
//! plans for the most common chief-analyst goals (initiate-coverage, IC
//! memos, morning notes, etc.) without breaking the build on every new
//! tool added downstream.

use crate::multi_agent::types::PlanAction;

/// Snapshot of MCP tool names exposed across the four CFA MCP servers
/// (`cfa-core`, `fmp-mcp-server`, `data-mcp-server`, `vendor-mcp-server`).
///
/// v1: ~40 high-frequency tools. See module docs for the long-tail plan.
pub const MCP_TOOL_NAMES: &[&str] = &[
    // cfa-core: valuation
    "wacc_calculator",
    "dcf_model",
    "comps_table",
    "comparable_companies",
    "calculate_target_price",
    "sotp_valuation",
    // cfa-core: credit
    "credit_metrics",
    "debt_capacity",
    "covenant_check",
    "altman_z_score",
    "cds_pricing",
    // cfa-core: PE / M&A
    "lbo_model",
    "irr_moic",
    "debt_schedule",
    "sources_uses",
    "waterfall_distribution",
    "merger_model",
    "accretion_dilution",
    // cfa-core: portfolio + risk
    "sharpe_ratio",
    "value_at_risk",
    "kelly_criterion",
    "factor_attribution",
    "black_litterman",
    "risk_parity",
    "stress_test",
    "monte_carlo_dcf",
    // cfa-core: fixed income / derivatives
    "bond_pricing",
    "duration_convexity",
    "yield_curve_bootstrap",
    "option_pricing",
    "implied_volatility",
    "swap_valuation",
    // fmp-mcp-server: market data
    "fmp_quote",
    "fmp_company_profile",
    "fmp_income_statement",
    "fmp_balance_sheet",
    "fmp_cash_flow",
    "fmp_key_metrics",
    "fmp_ratios",
    // data-mcp-server: macro + filings
    "fred_series",
    "edgar_filing",
    "figi_lookup",
    "yf_history",
    "wb_indicator",
    // vendor-mcp-server: paid feeds
    "lseg_curve",
    "sp_credit_rating",
    "factset_factor_exposure",
    "moodys_rating",
];

/// Snapshot of CFA slash commands under `.claude/commands/cfa/`.
///
/// v1: ~50 commands matching the on-disk set as of Phase 27. See module
/// docs for the dynamic-read plan.
pub const SLASH_COMMAND_NAMES: &[&str] = &[
    "initiate-coverage",
    "earnings",
    "earnings-preview",
    "morning-note",
    "thesis",
    "screen",
    "screen-deal",
    "sector",
    "model-update",
    "catalysts",
    "one-pager",
    "dcf",
    "comps",
    "lbo",
    "merger-model",
    "3-statement-model",
    "ic-memo",
    "dd-checklist",
    "dd-prep",
    "value-creation",
    "unit-economics",
    "returns",
    "portfolio",
    "source",
    "deal-tracker",
    "cim",
    "teaser",
    "pitch-deck",
    "buyer-list",
    "process-letter",
    "client-review",
    "client-report",
    "financial-plan",
    "rebalance",
    "tlh",
    "proposal",
    "competitive-analysis",
    "debug-model",
    "macro-rates",
    "fx-carry",
    "swap-curve",
    "bond-analysis",
    "bond-rv",
    "fi-portfolio",
    "credit-analysis",
    "derivatives-valuation",
    "option-vol",
    "property-valuation",
    "acquisition-model",
    "jurisdiction-comparison",
    "fund-migration",
    "fund-ops",
];

/// The catalogue of actions available to the planner.
///
/// `mcp_tools` and `slash_commands` are the registered names; the planner
/// composes [`PlanAction`] instances around them at plan-generation time.
#[derive(Debug, Clone)]
pub struct ActionCatalogue {
    pub mcp_tools: Vec<&'static str>,
    pub slash_commands: Vec<&'static str>,
}

impl ActionCatalogue {
    /// Total number of actions in the catalogue.
    pub fn total_actions(&self) -> usize {
        self.mcp_tools.len() + self.slash_commands.len()
    }
}

/// Load the v1 action catalogue.
///
/// Pure function over the static [`MCP_TOOL_NAMES`] / [`SLASH_COMMAND_NAMES`]
/// slices; no I/O. Phase 28 will replace the body with a dynamic
/// directory walk plus an MCP server-registry crawl.
pub fn load_action_catalogue() -> ActionCatalogue {
    ActionCatalogue {
        mcp_tools: MCP_TOOL_NAMES.to_vec(),
        slash_commands: SLASH_COMMAND_NAMES.to_vec(),
    }
}

/// Validate that `action` references a registered tool / command per
/// `MAC-INV-002`.
///
/// `SpecialistAgent` actions are validated by
/// [`crate::multi_agent::agent_invoke::validate_target_agent`]; this
/// function delegates so callers have a single entry point for any
/// `PlanAction` shape.
pub fn validate_action(catalogue: &ActionCatalogue, action: &PlanAction) -> bool {
    match action {
        PlanAction::McpTool { name, .. } => catalogue.mcp_tools.iter().any(|t| t == name),
        PlanAction::SlashCommand { name, .. } => catalogue.slash_commands.iter().any(|c| c == name),
        PlanAction::SpecialistAgent { slug, .. } => {
            crate::multi_agent::agent_invoke::validate_target_agent(slug)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn catalogue_loads_with_actions() {
        let cat = load_action_catalogue();
        assert!(!cat.mcp_tools.is_empty());
        assert!(!cat.slash_commands.is_empty());
        assert!(cat.total_actions() >= 80);
    }

    #[test]
    fn validate_known_mcp_tool() {
        let cat = load_action_catalogue();
        let action = PlanAction::McpTool {
            name: "dcf_model".into(),
            input_hint: json!({}),
        };
        assert!(validate_action(&cat, &action));
    }

    #[test]
    fn validate_unknown_mcp_tool_rejected() {
        let cat = load_action_catalogue();
        let action = PlanAction::McpTool {
            name: "ghost_tool".into(),
            input_hint: json!({}),
        };
        assert!(!validate_action(&cat, &action));
    }

    #[test]
    fn validate_known_slash_command() {
        let cat = load_action_catalogue();
        let action = PlanAction::SlashCommand {
            name: "initiate-coverage".into(),
            args: vec![],
        };
        assert!(validate_action(&cat, &action));
    }

    #[test]
    fn validate_specialist_delegates_to_agent_invoke() {
        let cat = load_action_catalogue();
        let action = PlanAction::SpecialistAgent {
            slug: "cfa-equity-analyst".into(),
            brief: "x".into(),
        };
        assert!(validate_action(&cat, &action));
        let bad = PlanAction::SpecialistAgent {
            slug: "ghost-analyst".into(),
            brief: "x".into(),
        };
        assert!(!validate_action(&cat, &bad));
    }
}
