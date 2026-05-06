//! Type definitions for the MCP-server tier registry.
//!
//! Follows the same shape as `crate::managed_agent::types`:
//! a `*Tier` enum + a static `&[*RegistryEntry]` slice + typed
//! `*Input`/`*Output` structs with `serde::{Serialize, Deserialize}`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Tier enum + registry entry
// ---------------------------------------------------------------------------

/// Cost tier for an upstream MCP server.
///
/// Lets users discover which MCP servers they can wire into a CFA managed
/// agent without paying for any vendor data subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpServerTier {
    /// Bundled with cfa-core. Pure Rust computation, no network, no API key.
    /// Always free to run.
    FreeNative,
    /// Free public data sources. Some endpoints need a free API key (ACLED,
    /// NASA FIRMS, EIA); the rest are open. No paid subscription required.
    FreePublicWithApiKey,
    /// Vendor offers a free tier (e.g. FMP free tier — 250 requests/day).
    /// Sign-up is free; paid plans unlock higher rate limits.
    Freemium,
    /// Requires a paid vendor subscription. User must supply vendor API
    /// credentials (Bearer / Basic / OAuth2) at deploy time.
    PaidVendor,
}

/// One row of the canonical MCP-server registry: name + tier + description +
/// setup hint + relative `package_path` from the repo root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerRegistryEntry {
    pub name: &'static str,
    pub tier: McpServerTier,
    pub description: &'static str,
    pub setup: &'static str,
    pub package_path: &'static str,
}

/// Canonical MCP-server registry for the cfa-agent repo.
///
/// Order: FreeNative, then FreePublicWithApiKey, then Freemium, then PaidVendor.
pub const MCP_SERVER_REGISTRY: &[McpServerRegistryEntry] = &[
    McpServerRegistryEntry {
        name: "cfa-core",
        tier: McpServerTier::FreeNative,
        description: "212 corp-finance Rust tools, decimal precision, 100% offline. WACC/DCF/LBO/credit/derivatives/fixed-income/cookbook tooling.",
        setup: "No setup. Bundled in the cfa-core plugin.",
        package_path: "packages/mcp-server",
    },
    McpServerRegistryEntry {
        name: "data",
        tier: McpServerTier::FreePublicWithApiKey,
        description: "121 free public-data tools: FRED, EDGAR, FIGI, Yahoo Finance, World Bank, geopolitical data (ACLED conflict, NASA FIRMS fire, EIA energy).",
        setup: "FREE. Three sources need a free API key (ACLED_ACCESS_TOKEN, NASA_FIRMS_API_KEY, EIA_API_KEY). Other 11 sources need no auth.",
        package_path: "packages/data-mcp-server",
    },
    McpServerRegistryEntry {
        name: "fmp",
        tier: McpServerTier::Freemium,
        description: "180 Financial Modeling Prep market-data tools: quotes, statements, ratios, estimates, ownership, screening.",
        setup: "Requires FMP_API_KEY. FMP has a free tier (250 requests/day) — sign up at financialmodelingprep.com.",
        package_path: "packages/fmp-mcp-server",
    },
    McpServerRegistryEntry {
        name: "vendor",
        tier: McpServerTier::PaidVendor,
        description: "87 paid-vendor tools across LSEG, S&P Global (Kensho), FactSet, Morningstar, Moody's, PitchBook. Three auth patterns: Bearer, Basic, OAuth2.",
        setup: "Requires paid vendor subscription per source. Set per-vendor env vars (LSEG_*, SP_GLOBAL_*, FACTSET_*, MORNINGSTAR_*, MOODYS_*, PITCHBOOK_*).",
        package_path: "packages/vendor-mcp-server",
    },
];

/// Look up the registry entry for a server name, if any.
pub fn mcp_server_registry_entry(name: &str) -> Option<&'static McpServerRegistryEntry> {
    MCP_SERVER_REGISTRY.iter().find(|e| e.name == name)
}

// ---------------------------------------------------------------------------
// List input/output (the public API for `list::list_servers`)
// ---------------------------------------------------------------------------

/// Input for `list::list_servers`. If `tier` is `None`, every server is
/// returned. If set, only that tier's servers are returned.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpListInput {
    /// Optional filter by cost tier.
    pub tier: Option<McpServerTier>,
}

/// One row in `McpListOutput`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpListEntry {
    pub name: String,
    pub tier: McpServerTier,
    pub description: String,
    pub setup: String,
    pub package_path: String,
}

/// Output of `list::list_servers`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpListOutput {
    pub total: usize,
    pub free_native: usize,
    pub free_public_with_api_key: usize,
    pub freemium: usize,
    pub paid_vendor: usize,
    pub entries: Vec<McpListEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_four_known_servers() {
        assert_eq!(MCP_SERVER_REGISTRY.len(), 4);
        let names: Vec<&str> = MCP_SERVER_REGISTRY.iter().map(|e| e.name).collect();
        assert!(names.contains(&"cfa-core"));
        assert!(names.contains(&"data"));
        assert!(names.contains(&"fmp"));
        assert!(names.contains(&"vendor"));
    }

    #[test]
    fn registry_entry_lookup_works() {
        let entry = mcp_server_registry_entry("cfa-core").expect("cfa-core present");
        assert_eq!(entry.tier, McpServerTier::FreeNative);
        assert!(mcp_server_registry_entry("nonexistent").is_none());
    }

    #[test]
    fn tier_serialises_snake_case() {
        let t = McpServerTier::FreePublicWithApiKey;
        let v = serde_json::to_value(t).unwrap();
        assert_eq!(v.as_str().unwrap(), "free_public_with_api_key");
    }
}
