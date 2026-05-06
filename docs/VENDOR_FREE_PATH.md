# Running CFA Without Paid Vendor Subscriptions

## TL;DR

Install the `cfa-core` plugin (or run `cargo build` and register the MCP server) and the entire CFA system runs end-to-end on your machine. No paid Bloomberg, LSEG, S&P, FactSet, Morningstar, Moody's, or PitchBook subscription is required. Decimal-precision Rust math, ~6,500 unit tests, 200+ MCP tools, 49 skills, 9 specialist agents, and 25 slash commands — all free. Optional layers (free public data feeds, FMP free tier, paid vendor feeds) plug in additively when you have credentials.

## The four free product surfaces

The CFA system ships four user-facing surfaces. All of them work with the `cfa-core` MCP server alone — no API keys, no network calls.

- **CLI** — `cfa analyze`, `cfa managed-agent ...`, `cfa workflow ...`. 72 Rust subcommands; pure compute over user-supplied JSON.
- **Skills** — 49 curated `.claude/skills/` markdown files for analyst workflows (DCF, LBO, IC memo, KYC, GL recon, model audit, etc.).
- **MCP servers** — 4 packages, the most important of which (`cfa-core`, ~206 tools) is 100% offline Rust.
- **Plugin** — `plugins/cfa-core/` packages the cfa-core MCP server, skills, slash commands, and agents as a single Claude Code plugin.

## What works without any vendor API

Everything that does not strictly require a live data feed runs on `cfa-core` alone. You bring the inputs as JSON; the Rust core returns audit-grade numbers.

| Workflow | Surface required | Vendor feed required? |
|----------|------------------|-----------------------|
| DCF / WACC / comps | `cfa-core` | No |
| LBO / waterfall / IRR / MOIC | `cfa-core` | No |
| Three-statement model | `cfa-core` | No |
| Credit metrics, debt capacity, covenant tests | `cfa-core` | No |
| Altman Z, Beneish, Piotroski, accrual quality | `cfa-core` | No |
| Bond pricing, yield, duration, convexity | `cfa-core` | No |
| Options (BS/CRR), Greeks, vol surface | `cfa-core` | No |
| Portfolio analytics (Sharpe, VaR, factor models) | `cfa-core` | No |
| Monte Carlo DCF / generic | `cfa-core` | No |
| KYC scoring, sanctions screening, AML | `cfa-core` | No |
| GL reconciliation, month-end close | `cfa-core` | No |
| LP statement audit, fund admin, NAV, GP/LP splits | `cfa-core` | No |
| Property valuation, acquisition model, rent roll | `cfa-core` | No |
| Jurisdiction comparison, fund migration, BEPS, treaty | `cfa-core` | No |
| Carbon markets, ETS, CBAM, ESG scoring | `cfa-core` | No |
| ESG carbon shadow price, climate stress | `cfa-core` | No |

The corollary: any time a workflow asks "what are the inputs", you can paste them in. You only need a data feed when you want the system to fetch live numbers itself.

## Free public data integrations

The `data-mcp-server` package ships 121 tools wired to free public sources. 11 of 14 sources need no auth at all; 3 require a free-signup API key.

**No API key required:**

- **FRED** (St. Louis Fed) — yield curves, CPI, GDP, unemployment, credit spreads
- **EDGAR** (SEC) — 10-K/10-Q/8-K, XBRL company facts, full-text filing search
- **OpenFIGI** — instrument identifier mapping (ISIN/CUSIP/SEDOL/ticker → FIGI)
- **Yahoo Finance** (unofficial) — quotes, options chains, fundamentals
- **World Bank** — 16,000+ development indicators, WGI governance scores
- **UCDP, GDELT, GDACS, USGS, WTO, USASpending, Polymarket, CoinGecko, UNHCR, Open-Meteo** — geopolitical, disasters, trade, alternative data

**Free-tier API key (one-line signup):**

- **ACLED** — armed-conflict events. Sign up: https://acleddata.com/data/
- **NASA FIRMS** — fire detection. Sign up: https://firms.modaps.eosdis.nasa.gov/api/
- **EIA** — petroleum and electricity supply. Sign up: https://www.eia.gov/opendata/

Set the corresponding env vars (`ACLED_ACCESS_TOKEN`, `NASA_FIRMS_API_KEY`, `EIA_API_KEY`) and the relevant tools start returning data.

## Optional freemium step: FMP

`fmp-mcp-server` exposes ~180 Financial Modeling Prep tools (quotes, financials, technicals, news, screening, SEC filings, ETFs, etc.). FMP has a free tier — sign up at https://financialmodelingprep.com, set `FMP_API_KEY`, and the FMP toolset becomes available to every analyst and managed-agent.

Free-tier limits at signup time: limited daily call quota and a smaller fundamentals history window. Adequate for single-name analysis, model building, and most managed-agent cookbook workflows. Upgrade only if you hit the cap.

## What's behind a paywall

The `vendor-mcp-server` package and two managed-agent cookbooks require commercial subscriptions. The system runs without them.

| Vendor | What it adds | Required for |
|--------|--------------|--------------|
| LSEG (Refinitiv) | Bond pricing, yield curves, FX, options analytics | `lseg-rates-monitor` cookbook |
| S&P Global (Capital IQ / Kensho) | Capital IQ financials, transcripts, tearsheets | `sp-credit-research` cookbook |
| FactSet | Multi-asset analytics, portfolio tools | none — vendor-mcp tools only |
| Morningstar | Fund ratings, ESG, portfolio analytics | none — vendor-mcp tools only |
| Moody's | Ratings, fixed-income analytics | none — vendor-mcp tools only |
| PitchBook | PE deal data, VC transactions, fund performance | none — vendor-mcp tools only |
| Aiera (data plugin) | Earnings call transcripts (live) | optional plugin |
| Daloopa (data plugin) | Standardised fundamentals | optional plugin |

To enable: set the relevant credentials env vars (e.g. `LSEG_CLIENT_ID` / `LSEG_CLIENT_SECRET`, `SP_API_KEY`, `FACTSET_USERNAME` / `FACTSET_API_KEY`) and rebuild. Auth patterns: Bearer (S&P / Morningstar / PitchBook), Basic (FactSet), OAuth2 client_credentials (LSEG / Moody's).

## Discovery

```bash
# List which MCP servers are wired up and which need credentials
cfa mcp list

# List managed-agent cookbooks by cost tier
cfa managed-agent list --tier=core-only     # 5 cookbooks, no feeds at all
cfa managed-agent list --tier=freemium      # 8 cookbooks, free public + FMP free tier
cfa managed-agent list --tier=paid-vendor   # 2 cookbooks, vendor subscription required
cfa managed-agent list                      # all 15, grouped
```

The `managed_agent_list` MCP tool returns the same data programmatically.

## Cookbooks are deployment examples, not the product

The 15 directories under `managed-agent-cookbooks/` are deployment templates for the Anthropic Managed Agents API (`/v1/agents`). They take the same skills, agents, and MCP servers used locally and package them as remote managed agents. **The CFA system is fully usable without ever deploying a cookbook.** See `managed-agent-cookbooks/README.md` for cookbook deploy mechanics.

### Cookbook tier breakdown

**CoreOnly (5)** — `cfa-core` only, user supplies inputs as JSON:
- `gl-reconciler`
- `kyc-screener`
- `lp-statement-auditor`
- `model-builder`
- `month-end-closer`

**Freemium (8)** — `cfa-core` + free public data and/or FMP free tier:
- `credit-analyst`
- `earnings-reviewer`
- `equity-analyst`
- `pitch-deck-builder`
- `private-markets-analyst`
- `sector-research`
- `valuation-reviewer`
- `wealth-meeting-prep`

**PaidVendor (2)** — requires LSEG or S&P Global subscription:
- `lseg-rates-monitor` (LSEG)
- `sp-credit-research` (S&P Global, FMP fallback available)

13 of 15 cookbooks are deployable without any paid subscription.
