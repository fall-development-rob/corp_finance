# cfa-core

Institutional-grade corporate finance toolkit for Claude Code. Pure-compute, decimal-precision, **zero API keys required**.

## What's included

- **80+ MCP tools** (WASM-compiled Rust): WACC, DCF, comps, LBO, credit metrics, debt capacity, covenant tests, bond pricing/yield/duration, yield-curve bootstrapping, options/forwards/swaps, three-statement modelling, portfolio analytics
- **14 skills**: 8 corp-finance domain skills + 6 workflow skills (equity research, IB, PE, wealth management, financial analysis, deal documents)
- **9 specialist agents**: chief-analyst, equity, credit, fixed-income, derivatives, macro, quant-risk, esg-regulatory, private-markets
- **Slash commands**: `/cfa:bond-analysis`, `/cfa:credit-analysis`, `/cfa:derivatives-valuation`, `/cfa:property-valuation`, `/cfa:acquisition-model`, `/cfa:jurisdiction-comparison`, `/cfa:fund-migration`, `/cfa:financial-plan`, plus document workflows (`/cfa:cim`, `/cfa:ic-memo`, `/cfa:dd-checklist`, `/cfa:value-creation`, `/cfa:teaser`, `/cfa:pitch-deck`, `/cfa:buyer-list`)

## Why WASM, why decimal

- **Single artifact** — same `.wasm` runs in Claude Code, Desktop, Web. No per-platform CI matrix.
- **`rust_decimal` everywhere** — no f64 precision drift in NPVs, IRRs, or compounding. Audit-grade.
- **Offline by design** — no network calls, no API keys. Inputs in, calculations out.

## Companion plugins (optional)

- `cfa-data` — free public data feeds (EDGAR, FRED, FIGI, Yahoo Finance, World Bank, geopolitical APIs). Mostly keyless.
- `cfa-pro` — premium feeds (FMP, LSEG, S&P, FactSet, Morningstar, Moody's, PitchBook). BYO API keys.

## Build

```bash
# from repo root — full 244-tool variant (4.4 MB)
./plugins/cfa-core/scripts/build-wasm.sh
npm --prefix plugins/cfa-core/mcp install
npm --prefix plugins/cfa-core/mcp run build
```

The WASM artifact lands in `mcp/wasm/` and is loaded by `mcp/dist/server.js` at runtime.

### Slim WASM builds (per-domain feature gates)

Every `wasm_tool!` binding in `crates/corp-finance-wasm/src/lib.rs` is wrapped in
`#[cfg(feature = "<domain>")]`, where `<domain>` is one of the per-domain Cargo
features defined in `crates/corp-finance-wasm/Cargo.toml` (mirroring the same
features on `corp-finance-core`). This lets you ship a much smaller `.wasm`
when you only need a subset of the toolkit.

```bash
# Built-in "smallest useful" preset — valuation + credit (~255 KB).
cargo build -p corp-finance-wasm --release \
    --no-default-features --features=slim

# Custom mix — fixed income, derivatives, and credit (~389 KB).
cargo build -p corp-finance-wasm --release \
    --no-default-features --features="fixed_income,derivatives,credit"

# Full surface (244 tools, ~4.4 MB) — same as the default build.
cargo build -p corp-finance-wasm --release --features=full
```

To wrap a slim build into a publishable npm package, swap the `cargo build`
in `scripts/build-wasm.sh` for the equivalent `wasm-pack` invocation:

```bash
wasm-pack build crates/corp-finance-wasm \
    --release --target nodejs \
    --out-dir plugins/cfa-core/mcp/wasm \
    --out-name corp_finance_wasm \
    -- --no-default-features --features=slim
```

#### Measured artifact sizes

Numbers below are the optimized `wasm-pack --release --target nodejs` output
of `corp_finance_wasm_bg.wasm` (post wasm-opt). Tool counts include the
streaming and `version()` exports.

| Variant      | Features                                  | `.wasm` size | Tools |
| ------------ | ----------------------------------------- | -----------: | ----: |
| `full`       | `full` (default)                          | 4 551 KB     | 246   |
| `slim`       | `valuation`, `credit`                     |   255 KB     | 8     |
| custom       | `fixed_income`, `derivatives`, `credit`   |   389 KB     | 19    |
| valuation    | `valuation` only                          |   185 KB     | 4     |

Rule of thumb: a fresh `wasm-bindgen` baseline costs ~150 KB, then each
domain adds roughly 5-50 KB depending on transitive math (Monte Carlo and
quant risk are the largest, simple decimal-arithmetic domains the smallest).
For the full list of domain feature names, see the `[features]` section in
`crates/corp-finance-wasm/Cargo.toml`.

## Install as a Claude Code MCP server

```bash
claude mcp add cfa-core -- node /absolute/path/to/plugins/cfa-core/mcp/dist/server.js
```

After install, all 6 tools (`wacc_calculator`, `dcf_model`, `comps_analysis`, `credit_metrics`, `debt_capacity`, `covenant_compliance`) are available in any Claude Code conversation. Skills, agents, and slash commands load from this directory automatically when discovered as a plugin.

## License

MIT
