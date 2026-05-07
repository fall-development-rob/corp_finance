//! Span builders for the four CFA surfaces.
//!
//! Each builder returns a `tracing::Span` already populated with the
//! mandatory ADR-017 §4 attributes for that surface. The returned span has
//! NOT yet been entered — the caller is responsible for `.in_scope(|| ...)`
//! or `.entered()` so the span tree topology matches the actual control flow.
//!
//! Usage pattern from `crates/corp-finance-cli/src/main.rs` (illustrative):
//!
//! ```ignore
//! let span = corp_finance_core::observability::span_helpers::cli_span(&subcmd);
//! let _enter = span.entered();
//! dispatch_subcommand(subcmd, args)?;
//! // Span closes when `_enter` drops.
//! ```
//!
//! Every helper generates a fresh `Uuid::now_v7()` for `cfa.surface_event_id`.
//! This is the *event-instance* id (one per surface fire); it is distinct
//! from `SpanContext::run_id`, which is the *correlation* id assembled across
//! all surface events that share a single CFA run.

use crate::observability::types::{attr, Surface};
use tracing::{field::Empty, info_span, Span};
use uuid::Uuid;

/// Create the root span for a CLI subcommand invocation.
///
/// Attributes set: `cfa.surface=cli`, `cfa.cli.subcommand=<subcommand>`,
/// `cfa.surface_event_id=<uuid-v7>`. The `cfa.tenant_id` and `cfa.run_id`
/// fields are declared but left empty so [`with_tenant`] / [`with_run_id`]
/// can populate them later in the same span without re-creating it.
pub fn cli_span(subcommand: &str) -> Span {
    let event_id = Uuid::now_v7().to_string();
    info_span!(
        "cfa.cli.invocation",
        { attr::SURFACE } = Surface::Cli.as_str(),
        { attr::SURFACE_EVENT_ID } = %event_id,
        { attr::CLI_SUBCOMMAND } = subcommand,
        { attr::TENANT_ID } = Empty,
        { attr::RUN_ID } = Empty,
    )
}

/// Create the root span for an MCP `server.tool(...)` handler invocation.
///
/// Attributes set: `cfa.surface=mcp`, `cfa.mcp.tool=<tool_name>`,
/// `cfa.surface_event_id=<uuid-v7>`.
pub fn mcp_span(tool_name: &str) -> Span {
    let event_id = Uuid::now_v7().to_string();
    info_span!(
        "cfa.mcp.tool",
        { attr::SURFACE } = Surface::Mcp.as_str(),
        { attr::SURFACE_EVENT_ID } = %event_id,
        { attr::MCP_TOOL } = tool_name,
        { attr::TENANT_ID } = Empty,
        { attr::RUN_ID } = Empty,
    )
}

/// Create the root span for a plugin hook fire (Write / Edit / PreToolUse /
/// PostToolUse / PreMemoryWrite).
///
/// Attributes set: `cfa.surface=plugin`, `cfa.plugin.hook=<hook_name>`,
/// `cfa.surface_event_id=<uuid-v7>`.
pub fn plugin_hook_span(hook_name: &str) -> Span {
    let event_id = Uuid::now_v7().to_string();
    info_span!(
        "cfa.plugin.hook",
        { attr::SURFACE } = Surface::Plugin.as_str(),
        { attr::SURFACE_EVENT_ID } = %event_id,
        { attr::PLUGIN_HOOK } = hook_name,
        { attr::TENANT_ID } = Empty,
        { attr::RUN_ID } = Empty,
    )
}

/// Create the root span for a slash-command-driven skill invocation.
///
/// Attributes set: `cfa.surface=skill`, `cfa.skill.name=<skill_name>`,
/// `cfa.surface_event_id=<uuid-v7>`.
pub fn skill_span(skill_name: &str) -> Span {
    let event_id = Uuid::now_v7().to_string();
    info_span!(
        "cfa.skill.invocation",
        { attr::SURFACE } = Surface::Skill.as_str(),
        { attr::SURFACE_EVENT_ID } = %event_id,
        { attr::SKILL_NAME } = skill_name,
        { attr::TENANT_ID } = Empty,
        { attr::RUN_ID } = Empty,
    )
}

/// Attach `cfa.tenant_id` to an existing span. Returns the same span (the
/// `tracing` API mutates in place; the return is only for chaining
/// ergonomics).
///
/// ADR-017 §4 / RUF-OBS-003 require this value be redacted to
/// first-initial-last-name format BEFORE this helper is called. The helper
/// itself performs no transformation — redaction is the caller's
/// responsibility because the canonical "username -> redacted form"
/// algorithm depends on environment shape (USER vs FULL_NAME vs an LDAP
/// lookup) and does not belong in the span layer.
pub fn with_tenant(span: Span, tenant_id: &str) -> Span {
    span.record(attr::TENANT_ID, tenant_id);
    span
}

/// Attach `cfa.run_id` to an existing span. The run id is a uuid-v7 minted
/// at the root of a CFA run that may span multiple surface events (e.g., a
/// CLI subcommand that fans out to four MCP tool calls — all five spans
/// share one run id).
pub fn with_run_id(span: Span, run_id: &Uuid) -> Span {
    span.record(attr::RUN_ID, run_id.to_string().as_str());
    span
}
