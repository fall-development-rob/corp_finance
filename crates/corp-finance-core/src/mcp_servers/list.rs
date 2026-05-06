//! MCP-server discovery by cost tier.
//!
//! Pure function: walks the static `MCP_SERVER_REGISTRY` (no I/O) and emits
//! a grouped list. Lets users find MCP servers they can wire in without
//! paying for a vendor data subscription:
//!
//! ```text
//! cfa mcp list                                # all servers, grouped by tier
//! cfa mcp list --tier=free-native             # offline only (cfa-core)
//! cfa mcp list --tier=free-public-with-api-key # FRED/EDGAR/FIGI/YF/WB + geopol
//! cfa mcp list --tier=freemium                # FMP (free tier)
//! cfa mcp list --tier=paid-vendor             # LSEG/S&P/FactSet/...
//! ```

use crate::mcp_servers::types::{
    McpListEntry, McpListInput, McpListOutput, McpServerTier, MCP_SERVER_REGISTRY,
};
use crate::CorpFinanceResult;

/// Enumerate MCP servers, optionally filtered by cost tier.
pub fn list_servers(input: &McpListInput) -> CorpFinanceResult<McpListOutput> {
    let entries: Vec<McpListEntry> = MCP_SERVER_REGISTRY
        .iter()
        .filter(|e| match input.tier {
            None => true,
            Some(t) => e.tier == t,
        })
        .map(|e| McpListEntry {
            name: e.name.to_string(),
            tier: e.tier,
            description: e.description.to_string(),
            setup: e.setup.to_string(),
            package_path: e.package_path.to_string(),
        })
        .collect();

    let free_native = MCP_SERVER_REGISTRY
        .iter()
        .filter(|e| e.tier == McpServerTier::FreeNative)
        .count();
    let free_public_with_api_key = MCP_SERVER_REGISTRY
        .iter()
        .filter(|e| e.tier == McpServerTier::FreePublicWithApiKey)
        .count();
    let freemium = MCP_SERVER_REGISTRY
        .iter()
        .filter(|e| e.tier == McpServerTier::Freemium)
        .count();
    let paid_vendor = MCP_SERVER_REGISTRY
        .iter()
        .filter(|e| e.tier == McpServerTier::PaidVendor)
        .count();

    Ok(McpListOutput {
        total: entries.len(),
        free_native,
        free_public_with_api_key,
        freemium,
        paid_vendor,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_no_filter_returns_all_four() {
        let out = list_servers(&McpListInput { tier: None }).unwrap();
        assert_eq!(out.total, 4);
        assert_eq!(out.entries.len(), 4);
        // Counts add up to the total.
        assert_eq!(
            out.free_native + out.free_public_with_api_key + out.freemium + out.paid_vendor,
            out.total
        );
        // Each tier has exactly one server in our registry.
        assert_eq!(out.free_native, 1);
        assert_eq!(out.free_public_with_api_key, 1);
        assert_eq!(out.freemium, 1);
        assert_eq!(out.paid_vendor, 1);
    }

    #[test]
    fn list_filter_free_native_returns_cfa_core_only() {
        let out = list_servers(&McpListInput {
            tier: Some(McpServerTier::FreeNative),
        })
        .unwrap();
        assert_eq!(out.total, 1);
        assert_eq!(out.entries.len(), 1);
        assert_eq!(out.entries[0].name, "cfa-core");
        assert!(out
            .entries
            .iter()
            .all(|e| e.tier == McpServerTier::FreeNative));
    }

    #[test]
    fn list_filter_paid_vendor_excludes_free() {
        let out = list_servers(&McpListInput {
            tier: Some(McpServerTier::PaidVendor),
        })
        .unwrap();
        assert!(out
            .entries
            .iter()
            .all(|e| e.tier == McpServerTier::PaidVendor));
        // None of the free servers should leak in.
        let names: Vec<&str> = out.entries.iter().map(|e| e.name.as_str()).collect();
        assert!(!names.contains(&"cfa-core"));
        assert!(!names.contains(&"data"));
        assert!(!names.contains(&"fmp"));
        assert!(names.contains(&"vendor"));
    }

    #[test]
    fn list_filter_freemium_returns_fmp() {
        let out = list_servers(&McpListInput {
            tier: Some(McpServerTier::Freemium),
        })
        .unwrap();
        assert_eq!(out.total, 1);
        assert_eq!(out.entries[0].name, "fmp");
    }

    #[test]
    fn list_filter_free_public_with_api_key_returns_data() {
        let out = list_servers(&McpListInput {
            tier: Some(McpServerTier::FreePublicWithApiKey),
        })
        .unwrap();
        assert_eq!(out.total, 1);
        assert_eq!(out.entries[0].name, "data");
        assert_eq!(out.entries[0].package_path, "packages/data-mcp-server");
    }

    #[test]
    fn list_output_serialises_to_json() {
        let out = list_servers(&McpListInput { tier: None }).unwrap();
        let v = serde_json::to_value(&out).unwrap();
        assert_eq!(v["total"], 4);
        assert!(v["entries"].is_array());
    }
}
