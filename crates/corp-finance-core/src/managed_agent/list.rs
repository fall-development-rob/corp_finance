//! Cookbook discovery by cost tier.
//!
//! Pure function: walks the static `COOKBOOK_REGISTRY` (no I/O) and emits a
//! grouped list. Lets users find cookbooks they can run without paying for a
//! vendor data subscription:
//!
//! ```text
//! cfa managed-agent list                  # all cookbooks, grouped by tier
//! cfa managed-agent list --tier=core_only # only zero-dependency cookbooks
//! cfa managed-agent list --tier=freemium  # cfa-core + free public data
//! cfa managed-agent list --tier=paid_vendor # requires paid subscription
//! ```

use crate::managed_agent::types::{
    CookbookTier, ListEntry, ListInput, ListOutput, COOKBOOK_REGISTRY,
};
use crate::CorpFinanceResult;

/// Enumerate cookbooks, optionally filtered by cost tier.
pub fn list_cookbooks(input: &ListInput) -> CorpFinanceResult<ListOutput> {
    let entries: Vec<ListEntry> = COOKBOOK_REGISTRY
        .iter()
        .filter(|e| match input.tier {
            None => true,
            Some(t) => e.tier == t,
        })
        .map(|e| ListEntry {
            slug: e.slug.to_string(),
            tier: e.tier,
            dependencies: e.dependencies.to_string(),
        })
        .collect();

    let core_only = COOKBOOK_REGISTRY
        .iter()
        .filter(|e| e.tier == CookbookTier::CoreOnly)
        .count();
    let freemium = COOKBOOK_REGISTRY
        .iter()
        .filter(|e| e.tier == CookbookTier::Freemium)
        .count();
    let paid_vendor = COOKBOOK_REGISTRY
        .iter()
        .filter(|e| e.tier == CookbookTier::PaidVendor)
        .count();

    Ok(ListOutput {
        total: entries.len(),
        core_only,
        freemium,
        paid_vendor,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_no_filter_returns_all() {
        let out = list_cookbooks(&ListInput { tier: None }).unwrap();
        assert_eq!(out.total, COOKBOOK_REGISTRY.len());
        assert!(out.core_only > 0);
        assert!(out.freemium > 0);
        assert!(out.paid_vendor > 0);
    }

    #[test]
    fn list_filter_core_only_excludes_paid() {
        let out = list_cookbooks(&ListInput {
            tier: Some(CookbookTier::CoreOnly),
        })
        .unwrap();
        assert!(out.entries.iter().all(|e| e.tier == CookbookTier::CoreOnly));
        assert!(!out.entries.iter().any(|e| e.slug == "lseg-rates-monitor"));
    }

    #[test]
    fn list_filter_paid_vendor_only_returns_two_known() {
        let out = list_cookbooks(&ListInput {
            tier: Some(CookbookTier::PaidVendor),
        })
        .unwrap();
        let slugs: Vec<String> = out.entries.iter().map(|e| e.slug.clone()).collect();
        assert!(slugs.contains(&"lseg-rates-monitor".to_string()));
        assert!(slugs.contains(&"sp-credit-research".to_string()));
        assert!(out.paid_vendor >= 2);
    }

    #[test]
    fn registry_and_allowed_slugs_in_sync() {
        // Every cookbook in the registry must be in the allowlist.
        for entry in COOKBOOK_REGISTRY {
            assert!(
                crate::managed_agent::types::ALLOWED_SLUGS.contains(&entry.slug),
                "Registry has '{}' but ALLOWED_SLUGS does not",
                entry.slug
            );
        }
        // And every allowlisted slug must be in the registry.
        for slug in crate::managed_agent::types::ALLOWED_SLUGS {
            assert!(
                COOKBOOK_REGISTRY.iter().any(|e| e.slug == *slug),
                "ALLOWED_SLUGS has '{}' but registry does not",
                slug
            );
        }
    }
}
