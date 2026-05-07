//! Multi-tenant federation bounded context (Phase 27, ADR-019).
//!
//! v1 is **simple tenancy only** — per-tenant output path scoping, per-tenant
//! memory partitions, PII redaction policies. v2 will layer the
//! cross-installation handshake (mTLS via `rustls`, ed25519 signatures via
//! `ed25519-dalek`) on top; the module structure is identical so v2 is a
//! focused add-on rather than a rewrite.
//!
//! ## Module layout
//!
//! - [`types`] — domain value objects: `Tenant`, `TenantContext`,
//!   `TrustTier`, `PIIRedactionPolicy`, `RedactionAction`, `TrustScore`,
//!   `FederatedSession`.
//! - [`tenant`] — tenant provisioning, surface-resolution helpers (CLI,
//!   MCP, plugin), per-tenant path composition, and isolation enforcement.
//! - [`pii_redaction`] — outbound PII policy application; reuses the
//!   security module's 14-category scanner.
//! - [`trust_score`] — behavioral trust scoring per peer; SQLite
//!   persistence helpers.
//! - [`session`] — federated session lifecycle (v2 stub).
//!
//! ## Feature gating
//!
//! The whole module is gated behind the `federation` cargo feature at the
//! crate root. The feature pulls in `rusqlite` (trust score persistence),
//! `sha2` (the `Hash` redaction action), `uuid` (session IDs), and
//! transitively `security` (the PII scanner).
//!
//! ## Tier-to-trust mapping
//!
//! `cookbook_tier_to_trust_tier` and `mcp_server_tier_to_trust_tier` are the
//! single sources of truth for translating an upstream tier label into a
//! federation `TrustTier` (RUF-FED-009 / FED-INV-007).

pub mod pii_redaction;
pub mod session;
pub mod tenant;
pub mod trust_score;
pub mod types;

#[cfg(test)]
mod tests;

pub use pii_redaction::{apply_policy, default_policy_for_tier, redact_text, RedactionResult};
pub use session::{close_session, open_session, record_payload};
pub use tenant::{
    enforce_isolation, provision_tenant, resolve_tenant_for_cli, resolve_tenant_for_mcp,
    resolve_tenant_for_plugin, tenant_scoped_path, ResourceKind,
};
pub use trust_score::{
    compute_trust_score, instant_downgrade_on_threat, upgrade_eligibility, InteractionRecord,
    ThreatSeverity,
};
pub use types::{
    FederatedSession, PIIRedactionPolicy, RedactionAction, Tenant, TenantContext, TrustScore,
    TrustTier,
};

use crate::managed_agent::types::CookbookTier;
use crate::mcp_servers::types::McpServerTier;

/// Single source of truth for `CookbookTier` -> `TrustTier` mapping
/// (RUF-FED-009 / FED-INV-007). Cookbook deploys are CLI invocations and
/// inherit a deploy-time tier; this function maps that tier into the
/// federation trust posture.
pub fn cookbook_tier_to_trust_tier(tier: CookbookTier) -> TrustTier {
    match tier {
        CookbookTier::CoreOnly => TrustTier::Open,
        CookbookTier::Freemium => TrustTier::Verified,
        CookbookTier::PaidVendor => TrustTier::Trusted,
    }
}

/// Single source of truth for `McpServerTier` -> `TrustTier` mapping
/// (RUF-FED-009). Runtime CLI invocations and MCP tool calls take their
/// trust posture from the consumed MCP server tier.
pub fn mcp_server_tier_to_trust_tier(tier: McpServerTier) -> TrustTier {
    match tier {
        // Free native + free public both map to Open: no client-data inputs,
        // outputs are computation results that may be shared subject to PII.
        McpServerTier::FreeNative => TrustTier::Open,
        McpServerTier::FreePublicWithApiKey => TrustTier::Open,
        McpServerTier::Freemium => TrustTier::Verified,
        McpServerTier::PaidVendor => TrustTier::Trusted,
    }
}
