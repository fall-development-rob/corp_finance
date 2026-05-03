//! Progress reporting for long-running computations.
//!
//! ## Why this module exists
//!
//! Many corp-finance-core operations (Monte Carlo, vol surface, optimization)
//! run for seconds or minutes on realistic inputs. When invoked through the
//! cfa-core MCP server, the LLM sees no output until the call returns — a
//! 60-second `run_monte_carlo` looks like a frozen conversation.
//!
//! [`ProgressSink`] is the abstraction that lets a long-running function emit
//! intermediate signals without coupling to any particular transport. The
//! WASM bindings wire it to a JS callback; the MCP server then turns those
//! callbacks into `notifications/progress` messages on the JSON-RPC stream.
//!
//! ## API design
//!
//! Every existing entry point keeps its current signature unchanged. The
//! pattern is:
//!
//! ```text
//! pub fn run_xxx(input: &XxxInput) -> Result<...> {
//!     run_xxx_with_progress(input, &NullSink)
//! }
//!
//! pub fn run_xxx_with_progress(
//!     input: &XxxInput,
//!     sink: &dyn ProgressSink,
//! ) -> Result<...> { /* real impl, calls sink.report(...) inside loops */ }
//! ```
//!
//! That keeps the NAPI / native CLI / WASM (non-streaming) bindings buildable
//! without changes — only the new streaming WASM macro and direct integrators
//! who want progress need to know `ProgressSink` exists.
//!
//! ## Threading
//!
//! [`ProgressSink`] is `Send + Sync` so future parallelized loops can call
//! `report` from worker threads. The current Monte Carlo loop is single-
//! threaded so this is forward-looking, but pinning the trait to `Send + Sync`
//! now avoids a breaking change later.

/// A handle for emitting progress signals out of a long-running computation.
///
/// Implementations should be cheap to invoke — `report` is called many times
/// per second from inner loops. The default [`NullSink`] is a no-op so
/// callers that don't care about progress pay zero overhead.
///
/// # Conventions
///
/// - `fraction` is in `[0.0, 1.0]`. Values outside this range should be
///   clamped by the consumer rather than the producer (so partial-progress
///   over-shoots like `iter / batch_size` don't cause panics).
/// - `label` is a short, human-readable hint such as `"5000/10000 paths"`.
///   Producers should keep it under ~80 characters.
/// - Producers should call `report` at coarse checkpoints (every ~5% or every
///   N hundred iterations), **not** on every iteration of a tight inner loop —
///   the WASM/JS round-trip dominates simulation cost otherwise.
pub trait ProgressSink: Send + Sync {
    /// Report incremental progress.
    ///
    /// `fraction` is the completion ratio in `[0.0, 1.0]`. `label` is a
    /// short human-readable description of what just finished.
    fn report(&self, fraction: f64, label: &str);
}

/// A no-op [`ProgressSink`] used as the default when callers don't care about
/// progress. All existing entry points pass `&NullSink` to maintain the
/// pre-streaming behaviour.
///
/// `report` here compiles to nothing in release builds, so wrapping the
/// non-streaming codepath through `_with_progress` has no measurable cost.
pub struct NullSink;

impl ProgressSink for NullSink {
    #[inline(always)]
    fn report(&self, _fraction: f64, _label: &str) {
        // Intentional no-op.
    }
}

/// Decide whether the inner loop at iteration `i` of `n` should emit a
/// progress checkpoint.
///
/// Heuristic:
/// - Always emit at the very first iteration so the consumer learns the
///   computation has actually started.
/// - Otherwise emit every `max(1, n/20)` iterations (≈ 5% increments).
///
/// Producers are not required to use this — it's a convenience for the
/// common case. Inlined so the branch is cheap inside hot loops.
#[inline]
pub fn should_report(i: usize, n: usize) -> bool {
    if n == 0 {
        return false;
    }
    if i == 0 {
        return true;
    }
    let step = (n / 20).max(1);
    i % step == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Capture-everything sink for unit testing producers.
    struct VecSink {
        events: Mutex<Vec<(f64, String)>>,
    }

    impl VecSink {
        fn new() -> Self {
            Self { events: Mutex::new(Vec::new()) }
        }
        fn snapshot(&self) -> Vec<(f64, String)> {
            self.events.lock().unwrap().clone()
        }
    }

    impl ProgressSink for VecSink {
        fn report(&self, fraction: f64, label: &str) {
            self.events
                .lock()
                .unwrap()
                .push((fraction, label.to_string()));
        }
    }

    #[test]
    fn null_sink_is_silent() {
        let sink = NullSink;
        for i in 0..100 {
            sink.report(i as f64 / 100.0, "tick");
        }
        // Nothing to assert beyond "didn't panic / didn't compile to anything
        // observable" — that's the whole point of NullSink.
    }

    #[test]
    fn vec_sink_captures_in_order() {
        let sink = VecSink::new();
        sink.report(0.1, "a");
        sink.report(0.5, "b");
        sink.report(1.0, "c");
        let events = sink.snapshot();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], (0.1, "a".to_string()));
        assert_eq!(events[2], (1.0, "c".to_string()));
    }

    #[test]
    fn should_report_first_iteration() {
        assert!(should_report(0, 1000));
    }

    #[test]
    fn should_report_handles_zero_total() {
        assert!(!should_report(0, 0));
        assert!(!should_report(5, 0));
    }

    #[test]
    fn should_report_emits_about_twenty_checkpoints() {
        let n = 10_000;
        let count = (0..n).filter(|i| should_report(*i, n)).count();
        // Expect ~20 checkpoints (one every 5% — n/20 = 500, so iterations
        // 0, 500, 1000, ..., 9500 = 20 events).
        assert!(
            (19..=21).contains(&count),
            "expected ~20 checkpoints, got {count}"
        );
    }

    #[test]
    fn should_report_handles_small_n() {
        // n=1 should still let i=0 fire so consumers see a "started" event.
        assert!(should_report(0, 1));
    }

    #[test]
    fn dyn_dispatch_works() {
        // Type-check that &dyn ProgressSink is the right shape for the
        // pattern documented in the module doc.
        fn take_sink(sink: &dyn ProgressSink) {
            sink.report(0.5, "halfway");
        }
        take_sink(&NullSink);
        take_sink(&VecSink::new());
    }
}
