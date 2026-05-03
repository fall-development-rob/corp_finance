# cfa-core streaming roadmap

## v0.2 (this release)

`cfa_estimate_runtime` lets the LLM (or a user) check expected wall time
**before** invoking a slow tool. It combines:

- **Hand-curated baselines** for ~10 known-slow tools (Monte Carlo,
  optimization, vol surface, SABR calibration)
- **Live measurements** from `cfa_profile` data once a tool's been called
  3+ times in this session

It returns `typical_us`, `max_us`, and a recommendation:

| Condition | Recommendation |
|---|---|
| `total_max_us > 30s` | split into multiple invocations (MCP timeout risk) |
| `total_typical_us > 5s` | run in background; v0.3 streaming planned |
| `total_typical_us > 500ms` | OK but slow; cache across iterations |
| else | run normally |

This is enough to prevent the worst case ("LLM iterates `run_monte_carlo`
50 times in a sensitivity grid and hangs the conversation for 10 minutes").

## v0.3 (planned)

True MCP streaming requires Rust-side progress callbacks. The work breaks
into three coordinated changes:

### 1. Rust: progress callback type

```rust
// crates/corp-finance-core/src/progress.rs (new)
pub trait ProgressSink {
    fn report(&self, fraction: f64, label: &str);
}

pub struct NullSink;
impl ProgressSink for NullSink { fn report(&self, _: f64, _: &str) {} }
```

Slow functions (`run_monte_carlo`, `optimize_mean_variance`,
`build_implied_vol_surface`, `calibrate_sabr`) take an `&dyn ProgressSink`
and call `sink.report(0.5, "5000/10000 paths complete")` at sensible
checkpoints.

### 2. WASM: marshal callbacks across the boundary

`wasm-bindgen` supports `Closure<dyn Fn(f64, &str)>`. Add a variant of
`wasm_tool!` (call it `wasm_tool_streaming!`) that takes a JS callback
and wires it to a `ProgressSink` impl.

### 3. MCP server: emit progress notifications

The MCP SDK supports `progressNotification`. Each progress callback
becomes a notification with the original request's `progressToken`. Claude
Code displays this as a streaming UI.

## v0.4 (further out)

- **True chunked output**: emit partial Monte Carlo paths as JSON-Lines
  events instead of a single batch. Lets the LLM make decisions on
  intermediate distributions without waiting for all 10k paths.
- **Cancellation**: respect MCP `$/cancelRequest` to abort long-running
  Rust loops mid-flight.
- **Concurrent tool invocations**: refactor to allow N tools running in
  parallel within one MCP server (currently each call blocks the server).

## Why not just timeout?

MCP requests can timeout from the client side, but the Rust call keeps
running in the WASM module. Without cooperative cancellation we can't
actually stop a runaway Monte Carlo. v0.4 cancellation work needs to be
done in coordination with the streaming work (same callback infrastructure).
