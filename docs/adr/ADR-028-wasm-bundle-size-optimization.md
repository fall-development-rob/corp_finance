# ADR-028: WASM Bundle Size Optimization (Wave 17a)

## Status: Accepted

## Date: 2026-05-08

## Deciders

- Robert Fall
- Chief Analyst (Claude Opus 4.7)

## Tags

`wasm`, `bundle-size`, `release-profile`, `wasm-opt`, `ci-gate`

## Context

Phase 29 Wave 16 completed the bulk port of pure-math corp-finance modules from NAPI-only to WASM-also. The cfa-core plugin's `corp_finance_wasm_bg.wasm` grew from 318 KiB (Wave 16a pilot, 4 tools) to 4.34 MiB (Wave 16α, 222 tools) — approaching ADR-027's 5 MiB hard ceiling above which the wave guidance demands evaluating per-domain bundle splitting.

Without an optimization pass, the next handful of ports (the 3 `wasm_blocker` carve-outs and the 2 partial-module residuals — see Wave 16α commit 47ddf93) would push the bundle past the gate. Splitting bundles is structural work; size optimization is mechanical and reversible.

## Decision

Apply two stacked optimization passes — one cargo-side, one wasm-pack-side — and gate the resulting bundle in CI.

**Cargo `[profile.release]` (workspace root `Cargo.toml`):**

- `opt-level = "z"` — already in place; favours size over speed
- `lto = "fat"` (was `lto = true` aka thin) — cross-crate inlining and dead-code elimination across the whole compile unit
- `codegen-units = 1` — already in place; required for `lto = "fat"` to reach maximum DCE
- `panic = "abort"` — drops the panic-unwinding code path (~30-50 KiB on a binary this size)
- `strip = true` — strips debug symbols from the final artefact

**`crates/corp-finance-wasm/Cargo.toml` `[package.metadata.wasm-pack.profile.release]`:**

- `wasm-opt = ["-Oz", "--all-features"]` — `-Oz` is wasm-opt's most aggressive size pass; `--all-features` enables every wasm proposal so the validator accepts whatever rustc emits (bulk-memory, sign-ext, mutable-globals, nontrapping-float-to-int, reference-types). Without `--all-features` the bundled wasm-opt rejects bulk-memory ops that rustc 1.82+ emits by default.

**CI gate (`.github/workflows/wasm-bundle-size.yml`):**

- Hard ceiling: 5 MiB (matches ADR-027). Bundle larger than this fails CI.
- Soft target: 4 MiB. Bundle larger than this raises a CI warning so wave authors can evaluate splitting before merging.
- Plus a plugin-boot smoke test that asserts the wasm loads without initialisation panics (catches getrandom-without-js misconfigurations of the kind Wave 16α already shimmed).

## Consequences

**Positive:**

- Bundle size after the optimization pass: 4.34 MiB → 3.86 MiB on the Wave 16α tool surface. ~11% reduction without any code changes.
- Future ports buy headroom: each remaining `wasm_blocker` resolution can land without immediately blowing the gate.
- CI gate makes regressions impossible-to-miss; size is now a first-class invariant alongside the surface-parity gate from Wave 15.
- `panic = "abort"` propagates: every cargo release build (CLI, NAPI bindings, WASM) is now smaller and faster to compile.

**Negative:**

- `panic = "abort"` removes stack unwinding — a Rust panic now aborts the process instead of unwinding. Acceptable here because: (a) the WASM context already aborts on panic by default, (b) the NAPI binding catches panics at the JS boundary via `napi::Result`, and (c) the CLI binary's panic handlers print the error before exit either way. Test runs use `[profile.test]` which retains unwinding.
- `strip = true` removes symbol names: stack traces in production are line-only. Debug builds (`cargo build` without `--release`) retain full symbols.
- `lto = "fat"` extends release-build wall time by 30-90s vs. the previous `lto = true` (thin). Acceptable for a release-only profile.
- `--all-features` for wasm-opt locks the validator to accepting future wasm proposals as rustc enables them — a small surface for future-version drift if a new feature changes semantics. Tracked: revisit if a wasm-validator regression surfaces.

## Alternatives Considered

- **`-Z build-std=std,panic_abort -Z build-std-features=panic_immediate_abort`** (rebuild std without panic strings). Saves 100s of KiB but requires nightly Rust. Deferred — re-evaluate if the bundle ever needs to drop below 2 MiB.
- **Split into per-domain `.wasm` files** loaded lazily by the plugin server. The eventual answer if the bundle keeps growing past 5 MiB. Not necessary at 3.86 MiB.
- **Replace `rust_decimal` with a thinner numeric type** (e.g., `bigdecimal` or hand-rolled fixed-point). Rejected: would invalidate the workspace's decimal-precision invariant and require auditing every numeric path.
- **Drop wasm-opt entirely** and rely on rustc's `opt-level = "z"`. Rejected: wasm-opt's peephole and stack-frame analyses provide ~5-10% beyond rustc-only.
- **Bundle the plugin without size optimization** and let consumers run wasm-opt themselves. Rejected: shipping non-optimized binaries to plugin consumers is a footgun.

## Links

- ADR-027: Wave-16 WASM Port Strategy — sets the 5 MiB ceiling and "split-evaluation" trigger this ADR is responding to.
- ADR-026: Plugin/packages Dual-Mode Architecture — establishes the WASM plugin surface that size optimization protects.
- `.github/workflows/wasm-bundle-size.yml` — implements the CI gate.
- `Cargo.toml` workspace `[profile.release]` — release profile changes.
- `crates/corp-finance-wasm/Cargo.toml` `[package.metadata.wasm-pack.profile.release]` — wasm-opt configuration.
- Wave 16α commit `47ddf93` — the wave that pushed bundle size to 4.34 MiB and triggered this work.
