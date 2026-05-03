# cfa-core streaming roadmap

## v0.2

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

## v0.3 (this release) — first streaming tool: `run_monte_carlo_streaming`

True MCP streaming requires Rust-side progress callbacks. v0.3 ships:

- `corp_finance_core::progress::ProgressSink` trait (+ `NullSink`)
- `run_monte_carlo_simulation_with_progress(input, &dyn ProgressSink)` —
  RNG-identical to `run_monte_carlo_simulation`
- A WASM streaming binding macro and `run_monte_carlo_streaming` export
  that bridges a JS callback to `ProgressSink`
- An MCP tool `run_monte_carlo_streaming` that turns each callback into a
  `notifications/progress` JSON-RPC message tagged with the request's
  `_meta.progressToken`
- An end-to-end integration test
  (`plugins/cfa-core/mcp/tests/streaming.test.mjs`)

The 244-tool non-streaming surface is **unchanged** — `run_monte_carlo`
keeps working exactly as before, so existing skills and pipelines need no
update.

### How it fits together

```
                                  ┌────────────────────────────────────────┐
                                  │ corp-finance-core                       │
LLM/Claude                        │                                         │
  │                               │  trait ProgressSink {                   │
  │ tools/call                    │      fn report(&self, f64, &str);       │
  │ {_meta:                       │  }                                      │
  │   progressToken: "x"}         │                                         │
  ▼                               │  fn run_monte_carlo_simulation_with_    │
┌────────────────────┐            │      progress(input, &dyn ProgressSink) │
│ cfa-core MCP       │            │                                         │
│ server (server.ts) │            └────────────────────────────────────────┘
│  └─ streaming.ts   │                                ▲
│     handler:       │                                │
│                    │            ┌───────────────────┴────────────────────┐
│  ┌──────────────┐  │            │ corp-finance-wasm                       │
│  │ build cb,    │  │            │                                         │
│  │ buffer evts  │──┼────────────►  wasm_tool_streaming!(                  │
│  │              │  │   call wit │      run_monte_carlo_streaming, …)      │
│  │ on result:   │  │   JS cb    │                                         │
│  │ flush evts   │  │            │  struct JsCallbackSink wrapping         │
│  │ as           │  │            │      js_sys::Function                   │
│  │ notifications│  │            │                                         │
│  │ /progress    │  │            └─────────────────────────────────────────┘
│  └──────────────┘  │
└────────────────────┘
```

### Adding more streaming tools

Three steps, no codegen run needed:

1. Add `<name>_with_progress(input, &dyn ProgressSink)` in corp-finance-core.
   Pattern: keep the existing entry point as a one-line shim that delegates
   with `&NullSink`. See `crates/corp-finance-core/src/monte_carlo/simulation.rs`.

2. Add `wasm_tool_streaming!(<name>_streaming, …)` in
   `crates/corp-finance-wasm/src/streaming.rs`.

3. Append a `StreamingToolSpec` to the `STREAMING_TOOLS` array in
   `plugins/cfa-core/mcp/src/streaming.ts`.

Then `bash plugins/cfa-core/scripts/build-wasm.sh && (cd plugins/cfa-core/mcp && npx tsc)`.

The codegen-parity script (`scripts/verify-codegen-parity.sh`) is not affected
because `tools.ts`, `lib.rs`, and the schemas all stay untouched.

### Suggested next streaming targets

Order them by user-visible latency × usage frequency:

- `run_mc_dcf` — same Monte Carlo loop, distinct entry point (low effort,
  high value)
- `optimize_mean_variance` — quadratic programming, can take seconds for
  large covariance matrices
- `build_implied_vol_surface` — calibrates per-tenor smiles
- `calibrate_sabr` — iterative non-linear least squares

### Known limitation: notifications buffer until the call returns

The current implementation **buffers** progress events on the JS side and
flushes them right before the `tools/call` response. The reason: the WASM
function is a synchronous Rust loop, and we can't `await
sendNotification(...)` from inside a `Closure<dyn Fn(f64, &str)>` without
crossing the sync/async boundary (which would either deadlock or require
splitting each path into a separate event-loop tick).

**Practical effect**: for a 60-second simulation the user still sees a
60-second silent wait, then a burst of 20 progress events plus the result.
The events still let the client *render* a progress bar after the fact, but
they don't drive a *live* progress bar during the simulation.

To get true real-time emission we need one of:

- Rust async loop with `wasm_bindgen_futures::JsFuture` await between
  checkpoints — yields the event loop so each notification flushes before
  the next batch of paths runs. Adds a wasm-bindgen-futures dep.
- Worker-thread Rust loop with `postMessage` events — full pipeline async,
  no buffering. Higher complexity (need to also handle cancellation).

Both are tracked under v0.4 below.

## v0.4 (planned)

- **Real-time event flushing**: convert the streaming loop to async so each
  `notifications/progress` arrives at the client *while* the simulation is
  running, not in a final burst.
- **Cancellation**: respect MCP `$/cancelRequest` to abort long-running
  Rust loops mid-flight. Needs the same async refactor as above plus an
  `AtomicBool` "should_cancel" inspected at each checkpoint.
- **True chunked output**: emit partial Monte Carlo paths as JSON-Lines
  events instead of a single batch. Lets the LLM make decisions on
  intermediate distributions without waiting for all 10k paths.
- **Concurrent tool invocations**: refactor to allow N tools running in
  parallel within one MCP server (currently each call blocks the server).

## Why not just timeout?

MCP requests can timeout from the client side, but the Rust call keeps
running in the WASM module. Without cooperative cancellation we can't
actually stop a runaway Monte Carlo. v0.4 cancellation work needs to be
done in coordination with the streaming work (same callback infrastructure).
