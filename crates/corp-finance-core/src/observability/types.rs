//! Shared value types for the Observability bounded context.
//!
//! Per `docs/ddd/domain-audit-observability.md`, the `Surface` enum is shared
//! kernel between Memory, Audit, Cost, and Observability contexts. This module
//! defines a local copy used by span helpers; once the audit/memory modules
//! are wired into `lib.rs` post-Phase-26, the duplicates collapse into a
//! single shared kernel re-export.
//!
//! The canonical attribute names below follow ADR-017 §4 (Structured Traces).
//! Every span emitted from a CFA surface event MUST set `cfa.surface` and
//! `cfa.surface_event_id`. Other attributes are surface-specific.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Origin surface of a CFA event. Mirrors the enum used in
/// `corp_finance_core::audit::surface_audit::Surface` and (forthcoming)
/// `corp_finance_core::memory::types::Surface` — kept locally to avoid a
/// circular feature dependency before lib.rs wiring lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Surface {
    /// `cfa <subcommand>` — invoked by `crates/corp-finance-cli/src/main.rs`.
    Cli,
    /// MCP `server.tool(...)` handler — invoked by any of the four
    /// `packages/*-mcp-server/` crates.
    Mcp,
    /// Slash-command emission resolved against `.claude/skills/*` skill body.
    Skill,
    /// Plugin hook fire — `plugins/cfa-core/hooks/hooks.json` (`PreToolUse`,
    /// `PostToolUse`, `PreMemoryWrite`, `Write`, `Edit`).
    Plugin,
}

impl Surface {
    /// Stable, lowercase string used as the `cfa.surface` span attribute value.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Surface::Cli => "cli",
            Surface::Mcp => "mcp",
            Surface::Skill => "skill",
            Surface::Plugin => "plugin",
        }
    }
}

/// Lightweight context propagated alongside a span. Spans themselves are
/// owned by the `tracing` runtime; `SpanContext` is the value-type that
/// observability callers and downstream surfaces (cost ledger, audit
/// manifest, memory) can read without taking a tracing dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
