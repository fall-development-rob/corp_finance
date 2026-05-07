//! Observability subsystem for CFA surface events (Phase 26 / ADR-017).
//!
//! Three responsibilities:
//!
//! - [`tracing_setup`] — install a global `tracing_subscriber` (idempotent).
//!   Three convenience initialisers wrap [`tracing_setup::init_tracing`] for
//!   the three runtime hosts (CLI binary, MCP subprocess, test harness).
//! - [`span_helpers`] — root-span builders for the four CFA surfaces (CLI,
//!   MCP, Plugin, Skill). Each builder generates a fresh `uuid-v7` for the
//!   `cfa.surface_event_id` attribute and pre-populates the ADR-017 §4
//!   attribute set.
//! - [`types`] — the shared `Surface` enum, `SpanContext` value object, and
//!   canonical attribute-name constants.
//!
//! All public entry points are pure (no I/O outside of `tracing` write-side
//! emit, which is owned by the installed subscriber). No global state aside
//! from the one-shot subscriber install. No async.
//!
//! Wiring into the binding crates happens post-build:
//!
//! - `crates/corp-finance-cli/src/main.rs` — call [`tracing_setup::init_for_cli`]
//!   at process start; wrap the subcommand dispatch in [`span_helpers::cli_span`].
//! - `packages/*-mcp-server/src/` — call [`tracing_setup::init_for_mcp`] in
//!   the NAPI module init; wrap each `server.tool` handler in
//!   [`span_helpers::mcp_span`].
//! - `plugins/cfa-core/hooks/hooks.json` — emit a [`span_helpers::plugin_hook_span`]
//!   from the hook bridge before delegating to the underlying tool.
//!
//! Feature gate: this whole module is exposed only when the `observability`
//! Cargo feature is enabled. The feature pulls in `tracing`, `tracing-subscriber`
//! (with `env-filter` and `json` features), and `uuid`.

pub mod span_helpers;
pub mod tracing_setup;
pub mod types;

pub use span_helpers::{
    cli_span, mcp_span, plugin_hook_span, skill_span, with_run_id, with_tenant,
};
pub use tracing_setup::{init_for_cli, init_for_mcp, init_for_test, init_tracing};
pub use types::{attr, SpanContext, Surface, SPAN_ATTRIBUTES};

#[cfg(test)]
mod tests;
