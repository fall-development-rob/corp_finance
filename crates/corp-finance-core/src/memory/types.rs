//! Memory bounded-context value objects and aggregates (DDD).
//!
//! Mirrors `docs/ddd/domain-memory.md` and the canonical schema defined in
//! ADR-016. The shapes here are intentionally minimal for the Phase 26
//! Rust core — additional fields from the JSON schema (token_usage,
//! recommendation, asset_class, etc.) are passed through opaquely on the
//! integration side and are not required by the indexing path.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Runtime surface that produced a `RunSummary`.
///
/// Re-exported from the shared kernel `corp_finance_core::surface::Surface`
/// (Phase 27 cleanup). Matches ADR-015's four-surface model: every CFA
/// runtime event lands on exactly one of these four surfaces. The local
/// path `corp_finance_core::memory::Surface` is preserved for backward
/// compatibility with existing call sites.
pub use crate::surface::Surface;

/// Canonical entity kinds extracted from `RunSummary` text.
///
/// Shared kernel with the multi-agent coordination context (Phase 27); the
/// `EntityRef` value object also lives there once that context lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Ticker,
    Issuer,
    Sector,
    Fund,
    Cusip,
}

/// A single named entity associated with a `RunSummary`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct EntityRef {
    pub kind: EntityKind,
    pub value: String,
}

/// The canonical record of a single surface invocation.
///
/// This is the aggregate root of the Memory bounded context. Every CLI
/// subcommand, MCP tool handler, plugin hook fire, and skill execution
/// that produces an output emits exactly one `RunSummary`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct RunSummary {
    /// Unique invocation id (UUID v7 — sorts lexicographically by time).
    pub run_id: Uuid,
    /// Runtime surface that produced this record.
    pub surface: Surface,
    /// CLI subcommand path / MCP tool name / slash command / hook id.
    pub surface_event_id: String,
    /// `djb2:0x...` hash over the surface configuration (ADR-009 / ADR-017).
    pub surface_audit_hash: String,
    /// Free-form summary text indexed by BM25 and embedded for HNSW.
    pub summary_text: String,
    /// Embedding vector (length = `HnswMemoryIndex::embedding_dim`).
    pub embedding: Vec<f32>,
    /// Invocation start timestamp.
    pub ts: DateTime<Utc>,
    /// Optional tenant id for multi-tenant deployments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

impl RunSummary {
    /// Build a new `RunSummary` with a freshly-allocated UUID v7 and the
    /// current wall-clock timestamp.
    pub fn new(
        surface: Surface,
        surface_event_id: impl Into<String>,
        surface_audit_hash: impl Into<String>,
        summary_text: impl Into<String>,
        embedding: Vec<f32>,
    ) -> Self {
        Self {
            run_id: Uuid::now_v7(),
            surface,
            surface_event_id: surface_event_id.into(),
            surface_audit_hash: surface_audit_hash.into(),
            summary_text: summary_text.into(),
            embedding,
            ts: Utc::now(),
            tenant_id: None,
        }
    }

    /// Builder-style setter for the optional tenant id.
    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }
}

/// A persisted, restorable analyst workspace.
///
/// Aggregates a sequence of `RunSummary` records under a single
/// `session_id`. Backed on disk by the `.cfa-session` portable archive
/// format (gzip-compressed JSON).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct CfaSession {
    pub session_id: Uuid,
    pub surface: Surface,
    pub run_summaries: Vec<RunSummary>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CfaSession {
    /// Create an empty session bound to a single owning surface.
    pub fn new(surface: Surface) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::now_v7(),
            surface,
            run_summaries: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Append a `RunSummary` and bump `updated_at` to wall-clock now.
    pub fn append(&mut self, summary: RunSummary) {
        self.run_summaries.push(summary);
        self.updated_at = Utc::now();
    }
}

/// A query submitted to the hybrid retriever.
///
/// At least one of `embedding` or `text` must be supplied; both are used
/// when both are present (the retriever fuses the two candidate lists).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct MemoryQuery {
    /// Vector query, used by the HNSW index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Free-form text query, used by the BM25 index.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Optional surface filter (AND-combined with other filters).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<Surface>,
    /// Number of results to return; the retriever caps `results.len() <= limit`.
    pub limit: usize,
    /// Optional tenant scope (AND-combined).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}

impl MemoryQuery {
    /// Default-construct an empty query with `limit = 5`.
    pub fn new() -> Self {
        Self {
            embedding: None,
            text: None,
            surface: None,
            limit: 5,
            tenant_id: None,
        }
    }
}

impl Default for MemoryQuery {
    fn default() -> Self {
        Self::new()
    }
}
