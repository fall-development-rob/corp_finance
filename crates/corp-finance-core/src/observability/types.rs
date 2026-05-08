//! Shared value types for the Observability bounded context.
//!
//! Per `docs/ddd/domain-audit-observability.md`, the `Surface` enum is shared
//! kernel between Memory, Audit, Cost, and Observability contexts. As of the
//! Phase-27 cleanup, the canonical definition lives in
//! `corp_finance_core::surface::Surface` and this module re-exports it for
//! backward compatibility.
//!
//! The canonical attribute names below follow ADR-017 §4 (Structured Traces).
//! Every span emitted from a CFA surface event MUST set `cfa.surface` and
//! `cfa.surface_event_id`. Other attributes are surface-specific.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Origin surface of a CFA event.
///
/// Re-exported from the shared kernel `corp_finance_core::surface::Surface`
/// (Phase 27 cleanup). The local path
/// `corp_finance_core::observability::types::Surface` (and the
/// `observability::Surface` re-export via `mod.rs`) is preserved for
/// backward compatibility with span helpers and the MCP/CLI wrappers.
pub use crate::surface::Surface;

/// Lightweight context propagated alongside a span. Spans themselves are
/// owned by the `tracing` runtime; `SpanContext` is the value-type that
/// observability callers and downstream surfaces (cost ledger, audit
/// manifest, memory) can read without taking a tracing dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema_gen", derive(schemars::JsonSchema))]
pub struct SpanContext {
    /// Originating surface of the event.
    pub surface: Surface,
    /// Per-event identifier — CLI subcommand name, MCP tool name, hook id,
    /// or slash-command id. NOT a uuid; this is the *kind* of event.
    pub surface_event_id: String,
    /// Optional tenant/customer id; redacted to "j.smith"-style on the wire
    /// when populated from the OS user identity (ADR-017 §4 / RUF-OBS-003).
    pub tenant_id: Option<String>,
    /// Run id (uuid-v7) generated at the root of a CFA surface event. The
    /// audit manifest, run summary, cost ledger row, and span tree all share
    /// this id for cross-surface correlation.
    pub run_id: Option<Uuid>,
}

impl SpanContext {
    /// Construct a minimal context for a surface event with no tenant or
    /// pre-existing run id. Callers that have already minted a run id at a
    /// higher layer (e.g., the CLI binary entry point) should use the
    /// `with_run_id` builder instead.
    pub fn new(surface: Surface, surface_event_id: impl Into<String>) -> Self {
        Self {
            surface,
            surface_event_id: surface_event_id.into(),
            tenant_id: None,
            run_id: None,
        }
    }

    /// Attach an externally-minted run id (e.g., from the CLI entry point).
    pub fn with_run_id(mut self, run_id: Uuid) -> Self {
        self.run_id = Some(run_id);
        self
    }

    /// Attach a tenant id. Caller is responsible for any redaction policy;
    /// the span helpers do not inspect or transform this value.
    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }
}

/// Canonical span attribute names. Public so the MCP and CLI wrappers can
/// reference them by symbol (avoiding string typos at instrumentation
/// points). Order is informational; readers should not depend on it.
///
/// See ADR-017 §4 for the per-surface attribute matrix.
pub const SPAN_ATTRIBUTES: &[&str] = &[
    "cfa.surface",
    "cfa.surface_event_id",
    "cfa.cli.subcommand",
    "cfa.mcp.tool",
    "cfa.plugin.hook",
    "cfa.skill.name",
    "cfa.tenant_id",
    "cfa.run_id",
];

/// Per-attribute string constants. Prefer these over inline string literals
/// when emitting span fields, to keep the call sites refactor-safe.
pub mod attr {
    pub const SURFACE: &str = "cfa.surface";
    pub const SURFACE_EVENT_ID: &str = "cfa.surface_event_id";
    pub const CLI_SUBCOMMAND: &str = "cfa.cli.subcommand";
    pub const MCP_TOOL: &str = "cfa.mcp.tool";
    pub const PLUGIN_HOOK: &str = "cfa.plugin.hook";
    pub const SKILL_NAME: &str = "cfa.skill.name";
    pub const TENANT_ID: &str = "cfa.tenant_id";
    pub const RUN_ID: &str = "cfa.run_id";
}
