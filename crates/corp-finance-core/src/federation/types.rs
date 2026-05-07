//! Domain types for the federation bounded context (ADR-019, DDD).
//!
//! These are pure value objects / aggregate roots:
//!
//! - [`Tenant`] — root of the Tenant aggregate
//! - [`TenantContext`] — value object passed to every CLI / MCP / plugin
//!   surface invocation
//! - [`TrustTier`] — the three-variant federation trust posture
//! - [`PIIRedactionPolicy`] — root of the PIIRedactionPolicy aggregate
//! - [`RedactionAction`] — Block / Redact / Hash / Pass per ADR-019
//! - [`TrustScore`] — root of the TrustScore aggregate (per peer)
//! - [`FederatedSession`] — root of the FederatedSession aggregate (v2 only)

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Trust attestation types (Phase 29 Wave 4, ADR-019 v2)
// ---------------------------------------------------------------------------

/// A signed claim that the issuer tenant grants the subject tenant a set of
/// capabilities for a fixed time window. Verifiable offline by anyone with
/// the issuer's ed25519 verifying key.
///
/// Canonical signing payload (must match exactly across issue/verify):
///
/// ```text
/// v1|<issuer_tenant>|<subject_tenant>|<capabilities_csv_sorted>|<issued_at_rfc3339>|<expires_at_rfc3339>
/// ```
///
/// The `capabilities` vector is sorted lexicographically before being CSV-joined
/// for the canonical payload, but the field on the wire keeps the caller's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustAttestation {
    pub attestation_id: Uuid,
    pub issuer_tenant: String,
    pub subject_tenant: String,
    pub capabilities: Vec<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// hex-encoded ed25519 signature (128 hex chars / 64 bytes raw)
    pub signature: String,
    /// hex-encoded ed25519 verifying key (64 hex chars / 32 bytes raw)
    pub public_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttestationStatus {
    /// signature valid, current time is within [issued_at, expires_at], not revoked
    Valid,
    /// signature valid, current time > expires_at
    Expired,
    /// listed in attestation_revocations
    Revoked,
    /// signature does not verify against public_key on canonical payload
    BadSignature,
    /// canonical-payload-internal mismatch (e.g. expected_issuer mismatches stored issuer)
    IssuerMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttestationRevocation {
    pub attestation_id: Uuid,
    pub revoked_at: DateTime<Utc>,
    pub revoked_by: String,
    pub reason: String,
}

use crate::security::types::PiiCategory;

// ---------------------------------------------------------------------------
// TrustTier
// ---------------------------------------------------------------------------

/// Federation trust posture (FED-INV-005: exactly three variants).
///
/// Mapped from `CookbookTier` and `McpServerTier` via the single-source-of-
/// truth functions in [`super::cookbook_tier_to_trust_tier`] and
/// [`super::mcp_server_tier_to_trust_tier`]. Cross-installation behaviour:
///
/// - `Open` — always permitted to federate. No vendor credentials, no
///   client-data inputs.
/// - `Verified` — receiving installation must present a valid signed
///   federation identity.
/// - `Trusted` — both installations sign a session-scoped data
///   redistribution agreement (v2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    Open,
    Verified,
    Trusted,
}

impl TrustTier {
    /// Stable lowercase identifier for logs and audit fields.
    pub const fn as_str(&self) -> &'static str {
        match self {
            TrustTier::Open => "open",
            TrustTier::Verified => "verified",
            TrustTier::Trusted => "trusted",
        }
    }
}

// ---------------------------------------------------------------------------
// Tenant aggregate
// ---------------------------------------------------------------------------

/// Tenant aggregate root. A distinct legal entity (LP, family, fund, deal
/// team) whose data, audit trail, and outputs must be isolated from other
/// tenants on the same installation.
///
/// Invariants (enforced at construction / provisioning, see
/// `tenant::provision_tenant`):
///
/// - `tenant_id` is non-empty, lowercase, kebab-case
/// - `tenant_id` is unique within the registry
/// - `output_root` is an absolute path under `<base>/<tenant_id>/`
/// - The reserved id `local` cannot be re-registered (FED-INV-002)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tenant {
    pub tenant_id: String,
    pub display_name: String,
    pub output_root: PathBuf,
    pub env_namespace: String,
    pub created_at: DateTime<Utc>,
    pub trust_tier: TrustTier,
}

/// Value object passed to every surface invocation. The surface boundary
/// (CLI argv parser, MCP wrapper, plugin hook entry) materialises a
/// `TenantContext` and threads it through to the dispatched handler.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TenantContext {
    pub tenant_id: String,
    pub output_root: PathBuf,
    pub env_namespace: String,
}

// ---------------------------------------------------------------------------
// PII redaction policy
// ---------------------------------------------------------------------------

/// Action taken on each detected PII finding for outbound messages
/// (ADR-019 §5).
///
/// - `Block` — the entire message is rejected; no bytes leave.
/// - `Redact` — the finding is replaced with a structure-preserving
///   placeholder (e.g. `XXX-XX-XXXX` for SSN).
/// - `Hash` — the finding is replaced with `sha256:<hex[:12]>` so
///   downstream peers can verify equality without recovering the value.
/// - `Pass` — the finding passes through untouched. Used only for
///   high-trust peers under explicit operator policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionAction {
    Block,
    Redact,
    Hash,
    Pass,
}

impl RedactionAction {
    pub const fn as_str(&self) -> &'static str {
        match self {
            RedactionAction::Block => "block",
            RedactionAction::Redact => "redact",
            RedactionAction::Hash => "hash",
            RedactionAction::Pass => "pass",
        }
    }
}

/// Root of the PIIRedactionPolicy aggregate. One policy per tenant; binds
/// a [`TrustTier`] to a single `RedactionAction` over a set of PII
/// categories.
///
/// Default policy semantics (per ADR-019 default policy table):
///
/// - `Open`   -> `Block`  over all 14 categories
/// - `Verified` -> `Redact` over all 14 categories
/// - `Trusted` -> `Hash`   over the four highly-sensitive categories
///   (SSN, EIN, CreditCard, Iban); `Pass` is implicit for the rest
///   (categories not listed receive `Pass`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PIIRedactionPolicy {
    pub tier: TrustTier,
    pub action: RedactionAction,
    pub categories: Vec<PiiCategory>,
}

// ---------------------------------------------------------------------------
// Trust score aggregate
// ---------------------------------------------------------------------------

/// Root of the TrustScore aggregate. One row per peer.
///
/// Invariants:
///
/// - Each component is in `[0.0, 1.0]`
/// - `composite = 0.4*success_rate + 0.2*uptime + 0.2*(1.0 - threat_score)
///   + 0.2*integrity_score`
/// - `composite` is recomputed by the constructor; callers should treat it
///   as read-only.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrustScore {
    pub peer_id: String,
    pub success_rate: f32,
    pub uptime: f32,
    pub threat_score: f32,
    pub integrity_score: f32,
    pub composite: f32,
    pub last_updated: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Federated session aggregate (v2)
// ---------------------------------------------------------------------------

/// Root of the FederatedSession aggregate. Declared in v1 for type-shape
/// completeness; the live state machine (Initiated -> Authenticated ->
/// Authorised -> Active, plus Hold / Failed transitions) lands in v2 along
/// with the rustls / ed25519-dalek handshake stack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FederatedSession {
    pub session_id: Uuid,
    pub peer_id: String,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub payload_count: u64,
}
