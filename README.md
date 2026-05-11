# corp-finance-mcp

Institutional-grade corporate finance calculations exposed as an MCP (Model Context Protocol) server, with a multi-agent AI analyst system for CFA-level financial analysis.

All financial math runs in 128-bit decimal precision via Rust, with Node.js bindings, a TypeScript MCP interface, and 9 specialist AI agents that route, coordinate, and synthesise across 200+ tools.

**Runs end-to-end with zero paid vendor subscriptions.** The `cfa-core` MCP server (~206 tools), the CLI, the 49 skills, and the 9 specialist agents all work offline on user-supplied JSON. Free public data feeds (FRED, EDGAR, FIGI, YF, WB) and the FMP free tier plug in additively. Paid vendors (LSEG, S&P, FactSet, Morningstar, Moody's, PitchBook) are opt-in. See [`docs/VENDOR_FREE_PATH.md`](docs/VENDOR_FREE_PATH.md).

> **[Wiki](https://github.com/fall-development-rob/corp_finance/wiki)** — Full technical documentation, module reference, data source catalogue, and architecture details.

## Architecture

```
crates/corp-finance-core    Rust library — 72 domain modules, all in Decimal
crates/corp-finance-cli     Rust CLI — 72 subcommands
packages/mcp-server         206 corp-finance MCP tools (Zod-validated, offline)
packages/data-mcp-server    121 data tools — free public sources (FRED, EDGAR, FIGI, Yahoo Finance, World Bank, geopolitical)
packages/fmp-mcp-server     ~180 FMP tools (free tier with API key)
packages/vendor-mcp-server  87 vendor tools — paid (LSEG, S&P, FactSet, Morningstar, Moody's, PitchBook)
packages/agents             9-analyst pipeline with HNSW routing and swarm coordination
```

### Plugin layout (three-tier, Phase 40)

```
plugins/
  agent-plugins/    24 plugins — one per deployable agent (9 specialists + 15 cookbooks)
  vertical-plugins/ 11 plugins — one per business domain (equity-research, ib, pe, fund-admin,
                      operations, wealth-management, financial-analysis, foundations, macro,
                      derivatives stub, fixed-income stub)
  partner-built/    10 plugins — one per data vendor (lseg, sp-global, factset, moodys,
                      morningstar, pitchbook, fmp, free-data, aiera stub, daloopa stub)
  cfa-core/         compute backbone — WASM/NAPI MCP server, 4 corp-finance-tools-* skills,
                      specflow, cfa-managed-agent, security/audit hooks
```

Each plugin carries a `.claude-plugin/plugin.json` manifest; partner-built plugins also carry
a `.mcp.json` for MCP server endpoint registration. See `docs/adr/ADR-043-three-tier-plugin-architecture.md`
for the full decision record.

## Quick Start

### As an MCP Server

```json
{
  "mcpServers": {
    "corp-finance": {
      "command": "node",
      "args": ["/path/to/packages/mcp-server/dist/index.js"]
    }
  }
}
```

### Build & Run

```bash
npm install && npm run build     # Turborepo — builds all 6 packages
cargo test --workspace --all-features   # ~6,100 Rust tests
npm run test:contracts                  # 406 agent contract tests
```

### Agent CLI

```bash
export ANTHROPIC_API_KEY=sk-ant-...

# Pipeline mode — routes to best specialist(s), coordinates, synthesises
cfa analyze "Calculate WACC for beta 1.2, risk-free 4%, ERP 6%"

# Single agent
cfa analyze --agent cfa-equity-analyst "Run a 3-stage DCF"

# Interactive REPL
cfa analyze -i
```

## What's Inside

| Area | Coverage |
|------|----------|
| **Valuation & Modelling** | DCF, WACC, comps, three-statement, LBO, merger model, SOTP |
| **Fixed Income** | Bond pricing, curves, duration, MBS, TIPS, repo, rate models |
| **Derivatives** | Options (BS/CRR), Greeks, vol surface, SABR, forwards, swaps |
| **Credit** | Ratings, Altman Z, CDS, CVA, CLO waterfall, CECL, migration |
| **Risk & Quant** | Factor models, BL, VaR/CVaR, risk parity, pairs trading, momentum |
| **Real Estate** | Rent roll, comparable sales, HBU, replacement cost, NCREIF benchmarking, acquisition model |
| **PE & VC** | LBO, waterfall, fund returns, SAFEs, J-curve, commitment pacing |
| **Regulatory** | Basel III, AIFMD, MiFID II, GIPS, KYC/AML, FATCA/CRS, BEPS |
| **ESG & Climate** | ESG scoring, carbon markets, CBAM, green bonds, SLL |
| **Geopolitical** | Conflict (ACLED/UCDP/GDELT), disasters (GDACS/USGS), trade (WTO/EIA), alt data (Polymarket) |
| **Fund Structures** | Onshore (US/UK/EU), offshore (Cayman/BVI/Lux/Ireland), transfer pricing, tax treaty |

> See the **[Modules](https://github.com/fall-development-rob/corp_finance/wiki/Modules)** wiki page for the full 71-module reference with feature flags and tool counts.

## Multi-Agent Pipeline

9 specialist analysts orchestrated by a chief analyst, with HNSW semantic routing and flash-attention swarm coordination.

| Agent | Domain |
|-------|--------|
| Equity Analyst | DCF, comps, earnings quality, target price |
| Credit Analyst | Ratings, spreads, default risk, credit scoring |
| Fixed Income Analyst | Bonds, curves, duration, MBS |
| Derivatives Analyst | Options, Greeks, vol surface, structured products |
| Quant Risk Analyst | VaR, factor models, portfolio optimisation |
| Macro Analyst | Monetary policy, FX, sovereign, trade |
| ESG Analyst | ESG scoring, carbon, climate risk |
| Private Markets Analyst | PE, VC, real assets, restructuring |

25 slash commands available in Claude Code (`/cfa:initiate-coverage`, `/cfa:ic-memo`, `/cfa:property-valuation`, `/cfa:acquisition-model`, etc.).

> See the **[Multi-Agent Pipeline](https://github.com/fall-development-rob/corp_finance/wiki/Multi-Agent-Pipeline)** wiki page for routing details, workflow skills, and slash command reference.

## Data Sources

| Package | Tools | Cost | Sources |
|---------|-------|------|---------|
| mcp-server (cfa-core) | 206 | Free, offline | Pure Rust compute, no network calls |
| data-mcp-server | 121 | Free (3 free signups) | FRED, EDGAR, FIGI, Yahoo Finance, World Bank, UCDP, GDELT, GDACS, USGS, WTO, Polymarket, CoinGecko, UNHCR, Open-Meteo (no key); ACLED, NASA FIRMS, EIA (free signup) |
| fmp-mcp-server | 180+ | Freemium | Financial Modeling Prep (quotes, financials, technicals, news) |
| vendor-mcp-server | 87 | Paid | LSEG, S&P Global, FactSet, Morningstar, Moody's, PitchBook |

> See [`docs/VENDOR_FREE_PATH.md`](docs/VENDOR_FREE_PATH.md) for what runs without paid feeds, and the **[Data Sources](https://github.com/fall-development-rob/corp_finance/wiki/Data-Sources)** wiki page for the full tool breakdown and authentication requirements.

## Deployment examples

`managed-agent-cookbooks/` ships 15 deployment templates for the [Anthropic Managed Agents API](https://docs.anthropic.com/) (`/v1/agents`) — equity research, IC memo, GL recon, KYC, model audit, etc. Cookbooks are **examples**, not the primary product; the local plugin / CLI / skills / MCP servers above already cover every workflow. 13 of 15 cookbooks are CoreOnly or Freemium tier (no paid subscription); 2 require LSEG or S&P. List by tier with `cfa managed-agent list --tier=core-only`. See [`managed-agent-cookbooks/README.md`](managed-agent-cookbooks/README.md).

## Documentation

| Resource | Description |
|----------|-------------|
| [`docs/VENDOR_FREE_PATH.md`](docs/VENDOR_FREE_PATH.md) | What runs without paid feeds, and how to add free / paid layers |
| **[Wiki](https://github.com/fall-development-rob/corp_finance/wiki)** | Full technical reference |
| `docs/adr/` | Architecture Decision Records (ADR-015 to ADR-043) |
| `docs/prd/` | Product Requirements Documents |
| `docs/ddd/` | Domain-Driven Design documents |
| `docs/contracts/` | Executable specification contracts |

## License

MIT
