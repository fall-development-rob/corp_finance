//! Hand-maintained streaming WASM bindings.
//!
//! ## Why this file exists
//!
//! `crates/corp-finance-wasm/src/lib.rs` is auto-generated from the NAPI
//! bindings (see `crates/cfa-codegen` and `scripts/gen-wasm-bindings.py`).
//! Streaming tools don't have an NAPI counterpart — they exist only at the
//! WASM/MCP boundary, where progress notifications make sense. Living in a
//! separate hand-maintained file keeps the codegen-parity check simple:
//! `lib.rs` stays byte-identical to whatever the generator produces, while
//! streaming functions live here under a stable `_streaming` naming
//! convention.
//!
//! ## What's exported
//!
//! Today: `run_monte_carlo_streaming(input_json, callback)`.
//!
//! The JS callback signature is `(progress: number, message: string) => void`.
//! It's invoked roughly every 5% of total work, plus a `(0.0, "starting…")`
//! event at the top and a `(1.0, "complete")` event at the bottom. The MCP
//! server (`plugins/cfa-core/mcp/src/streaming.ts`) wraps each callback into
//! a `notifications/progress` JSON-RPC notification using the request's
//! `_meta.progressToken`.
//!
//! ## Adding more streaming tools
//!
//! 1. Make sure the underlying corp-finance-core function has a
//!    `*_with_progress(input, &dyn ProgressSink)` variant. Pattern in
//!    `crates/corp-finance-core/src/monte_carlo/simulation.rs`.
//! 2. Add a `wasm_tool_streaming!(name, InputPath, fn_path);` invocation
//!    below.
//! 3. Register the tool name in
//!    `plugins/cfa-core/mcp/src/streaming.ts::STREAMING_TOOLS`.
//!
//! That's it — no codegen rerun needed.

use corp_finance_core::progress::ProgressSink;
use js_sys::Function;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

/// Bridge from a JS function to the Rust [`ProgressSink`] trait.
///
/// Each `report` invocation calls the JS callback with `(progress, message)`.
/// Errors thrown by the JS side are silently swallowed — progress is purely
/// observational and we don't want a flaky callback to abort the whole
/// computation. (If we ever need to surface callback errors, we can wire
/// them through a `Cell<Option<JsValue>>` similar to wasm-bindgen futures.)
struct JsCallbackSink {
    callback: Function,
}

// SAFETY: js_sys::Function is `!Send + !Sync` because JS values aren't
// thread-safe in general, but wasm32-unknown-unknown is single-threaded so
// the markers are vacuously safe to assert here. If we ever target wasm32
// with threading (`-Zthreads`) this needs revisiting.
unsafe impl Send for JsCallbackSink {}
unsafe impl Sync for JsCallbackSink {}

impl ProgressSink for JsCallbackSink {
    fn report(&self, fraction: f64, label: &str) {
        let this = JsValue::NULL;
        let p = JsValue::from_f64(fraction);
        let m = JsValue::from_str(label);
        // Discard the call result and any thrown error — the consumer
        // doesn't get to abort the simulation through a misbehaving sink.
        let _ = self.callback.call2(&this, &p, &m);
    }
}

/// Re-implementation of the auto-generated `err_to_js` helper. The macro
/// in `lib.rs` references the parent module's `err_to_js`, but since
/// `streaming` is a sub-module we can't reuse it directly without making
/// the parent function `pub(crate)`. Inlining keeps streaming.rs
/// self-contained and decoupled from the generated file's internals.
fn err_to_js(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}

/// Helper macro for streaming bindings. Mirrors the shape of the
/// non-streaming `wasm_tool!` macro in the generated `lib.rs`, but threads
/// a JS callback through to the `*_with_progress` variant.
///
/// Generated signature on the JS side:
/// `<name>(input_json: string, callback: (p: number, m: string) => void) => string`
macro_rules! wasm_tool_streaming {
    ($name:ident, $input_path:path, $fn_path:path) => {
        #[wasm_bindgen]
        pub fn $name(input_json: &str, callback: Function) -> Result<String, JsValue> {
            let input: $input_path = serde_json::from_str(input_json).map_err(err_to_js)?;
            let sink = JsCallbackSink { callback };
            let output = $fn_path(&input, &sink).map_err(err_to_js)?;
            serde_json::to_string(&output).map_err(err_to_js)
        }
    };
}

// ---------------------------------------------------------------------------
// Streaming tool registry
// ---------------------------------------------------------------------------

#[cfg(feature = "monte_carlo")]
wasm_tool_streaming!(
    run_monte_carlo_streaming,
    corp_finance_core::monte_carlo::simulation::MonteCarloInput,
    corp_finance_core::monte_carlo::simulation::run_monte_carlo_simulation_with_progress
);

// Future additions go here. Suggested next targets (per docs/STREAMING.md
// v0.4 plan):
// - optimize_mean_variance (portfolio_optimization)
// - build_implied_vol_surface (volatility_surface)
// - calibrate_sabr (volatility_surface)
// - run_mc_dcf (monte_carlo)
