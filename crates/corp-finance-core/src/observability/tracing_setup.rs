//! Global tracing subscriber setup for CFA surface wrappers.
//!
//! All initialisers are **idempotent**: the first successful call installs a
//! global subscriber; every subsequent call is a no-op. This is enforced by a
//! `std::sync::Once` so concurrent callers from the CLI binary, NAPI bindings,
//! and the test harness do not race or panic on duplicate-subscriber errors.
//!
//! Three convenience entry points are provided for the three runtime hosts:
//!
//! | Caller | Function | Format | Level source |
//! |---|---|---|---|
//! | `crates/corp-finance-cli/src/main.rs` | [`init_for_cli`] | env-driven | `CFA_LOG_LEVEL` |
//! | `packages/*-mcp-server/src/` (NAPI) | [`init_for_mcp`] | JSON / stderr | `CFA_LOG_LEVEL` |
//! | unit + integration tests | [`init_for_test`] | text / NoWriter | `trace` |
//!
//! Per ADR-017 §4, the JSON formatter is the institutional default for MCP
//! subprocesses (parent process can ingest stderr as a structured event
//! stream); CLI defaults to a human-readable format.

use crate::CorpFinanceResult;
use std::sync::Once;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// One-shot guard around the global subscriber install. Once any
/// `init_*` function succeeds, every subsequent call is a no-op
/// (irrespective of which init function called it). This satisfies
/// RUF-OBS-INV-001 (idempotency).
static SUBSCRIBER_INIT: Once = Once::new();

/// Install the global `tracing_subscriber` registry with a `fmt` layer.
///
/// * `json` — when `true`, the layer emits one JSON object per event to
///   stderr; when `false`, a human-readable colourised text format is used.
/// * `level` — string passed to `EnvFilter::new`. Accepts the standard
///   `tracing` filter syntax (e.g., `"info"`, `"corp_finance_core=debug,info"`).
///
/// Idempotent: only the first call installs a subscriber. Subsequent calls
/// silently succeed without altering global state. This is required by
/// RUF-OBS-INV-001 — many CFA surfaces (NAPI bindings, test harnesses,
/// CLI subcommands invoked via shell wrappers) may all attempt initialisation
/// in the same process.
pub fn init_tracing(json: bool, level: &str) -> CorpFinanceResult<()> {
    let level = level.to_string();
    SUBSCRIBER_INIT.call_once(|| {
        // Prefer the `RUST_LOG` env var when set; fall back to the
        // explicitly-passed level. This matches the convention used by
        // every other tracing-subscriber consumer in the Rust ecosystem.
        let filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.as_str()));

        if json {
            let layer = fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(false)
                .with_writer(std::io::stderr);
            // `try_init` returns Err if a subscriber is already set; we treat
            // that as success because idempotency is the contract.
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(layer)
                .try_init();
        } else {
            let layer = fmt::layer().with_target(true).with_writer(std::io::stderr);
            let _ = tracing_subscriber::registry()
                .with(filter)
                .with(layer)
                .try_init();
        }
    });
    Ok(())
}

/// CLI default: read `CFA_LOG_LEVEL` (default `"info"`) and `CFA_LOG_FORMAT`
/// (default `"text"`; set to `"json"` to enable structured output).
pub fn init_for_cli() -> CorpFinanceResult<()> {
    let level = std::env::var("CFA_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    let format = std::env::var("CFA_LOG_FORMAT").unwrap_or_else(|_| "text".to_string());
    let json = format.eq_ignore_ascii_case("json");
    init_tracing(json, &level)
}

/// MCP-subprocess default: JSON-only over stderr. Parent processes ingest the
/// event stream as structured records. Level overridable via `CFA_LOG_LEVEL`
/// (default `"info"`).
pub fn init_for_mcp() -> CorpFinanceResult<()> {
    let level = std::env::var("CFA_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    init_tracing(true, &level)
}

/// Test default: text-format subscriber at `trace` level, written to stderr.
/// Idempotent — safe to call from every `#[test]` body. The first test that
/// runs installs the subscriber; later tests reuse it.
pub fn init_for_test() -> CorpFinanceResult<()> {
    init_tracing(false, "trace")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_for_test_is_idempotent() {
        init_for_test().expect("first init");
        init_for_test().expect("second init");
        init_for_test().expect("third init");
    }

    #[test]
    fn init_tracing_with_explicit_level() {
        // Whatever the first test installed wins; this should not panic
        // regardless of order.
        init_tracing(false, "debug").expect("init_tracing");
        init_tracing(true, "info").expect("second init_tracing");
    }
}
