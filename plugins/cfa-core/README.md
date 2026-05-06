# cfa-core

Institutional-grade corporate finance toolkit for Claude Code. Pure-compute, decimal-precision, **zero API keys required**.

## What's included

- **80+ MCP tools** (WASM-compiled Rust): WACC, DCF, comps, LBO, credit metrics, debt capacity, covenant tests, bond pricing/yield/duration, yield-curve bootstrapping, options/forwards/swaps, three-statement modelling, portfolio analytics
- **38 skills**: 8 corp-finance domain skills + 12 workflow skills (equity research, IB, PE, wealth management, financial analysis, deal documents, fund admin, KYC, model audit, clean-data, PPTX/XLSX authoring) + 6 free public data skills (EDGAR, FRED, FIGI, World Bank, Yahoo Finance, mtnewswire) + 6 FMP freemium skills + 4 geopolitical data skills + cfa-managed-agent + specflow
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
# from repo root
./plugins/cfa-core/scripts/build-wasm.sh
npm --prefix plugins/cfa-core/mcp install
npm --prefix plugins/cfa-core/mcp run build
```

The WASM artifact lands in `mcp/wasm/` and is loaded by `mcp/dist/server.js` at runtime.

## Install as a Claude Code MCP server

```bash
claude mcp add cfa-core -- node /absolute/path/to/plugins/cfa-core/mcp/dist/server.js
```

After install, all 6 tools (`wacc_calculator`, `dcf_model`, `comps_analysis`, `credit_metrics`, `debt_capacity`, `covenant_compliance`) are available in any Claude Code conversation. Skills, agents, and slash commands load from this directory automatically when discovered as a plugin.

## License

MIT
