# cfa-pro

Premium financial data vendors for institutional research. **267+ MCP tools across 7 vendors.** BYO API keys.

Companion to **cfa-core** (compute) and **cfa-data** (free public data).

## Vendors

| Vendor | Tools | Auth | What it covers |
|---|---|---|---|
| **FMP** (Financial Modeling Prep) | 180 | `FMP_API_KEY` | 70k+ securities, financials, ratios, estimates, earnings, IPOs, M&A, executive comp, technicals |
| **LSEG** (London Stock Exchange Group) | 15 | OAuth2 client credentials | Bond pricing, yield curves, FX carry, options valuation, macro dashboard |
| **S&P Global** (Kensho) | 14 | Bearer token | Capital IQ financials, earnings call transcripts, company tearsheets, funding digests |
| **FactSet** | 16 | Basic auth | Multi-asset financials, analytics, portfolio tools |
| **Morningstar** | 14 | Bearer token | Fund ratings, ESG data, portfolio analytics |
| **Moody's** | 14 | OAuth2 client credentials | Credit ratings, fixed income analytics, structured finance |
| **PitchBook** | 14 | Bearer token | PE deal data, VC transactions, fund performance |

## Install

```bash
claude plugin install cfa-pro
# or for local dev:
claude mcp add cfa-pro -- node /path/to/packages/fmp-mcp-server/dist/index.js
```

## API keys

Each vendor needs its own credentials. Set what you have — missing keys disable that vendor's tools but don't fail the whole plugin.

```bash
# FMP — most users start here ($14/mo Starter, $39/mo Premium)
export FMP_API_KEY=...

# LSEG (institutional)
export LSEG_CLIENT_ID=...
export LSEG_CLIENT_SECRET=...

# S&P Global Kensho
export SP_GLOBAL_API_KEY=...

# FactSet
export FACTSET_USERNAME=...
export FACTSET_API_KEY=...

# Morningstar
export MORNINGSTAR_API_KEY=...

# Moody's
export MOODYS_CLIENT_ID=...
export MOODYS_CLIENT_SECRET=...

# PitchBook
export PITCHBOOK_API_KEY=...
```

## When to use cfa-pro vs cfa-data

- **cfa-data** for free sources: SEC EDGAR, FRED, Yahoo Finance, World Bank, geopolitical APIs. Sufficient for academic research, hobbyist analysis, basic equity screens.
- **cfa-pro** for institutional research: real-time, point-in-time-clean, ratings agencies, vendor-quality fundamentals. Required for sell-side, IB, PE, hedge-fund workflows.

## Companion plugins

- **cfa-core** — corporate finance compute (244 tools, no keys, fully offline)
- **cfa-data** — free public data (121 tools, mostly keyless)

## License

MIT
