# cfa-data

Free public financial and macro data feeds for Claude Code. **121 MCP tools** across 11 sources, mostly **keyless** — works out of the box.

## What's included

### Financial data (no API keys)
- **SEC EDGAR** (15 tools) — XBRL company facts, 10-K/10-Q/8-K filings, full-text filing search, CIK/ticker resolution
- **FRED** (18 tools) — Federal Reserve macro indicators, yield curves, CPI, GDP, unemployment, credit spreads
- **Yahoo Finance** (17 tools) — equity quotes, historical prices, options chains with Greeks, financial statements *(unofficial API — may be unstable)*
- **OpenFIGI** (7 tools) — instrument identifier cross-referencing (ISIN/CUSIP/SEDOL/ticker → FIGI)
- **World Bank** (30 tools) — 16,000+ development indicators, governance scores, sovereign data

### Geopolitical data (mostly keyless)
- **ACLED** (3 tools) — armed conflict events *(needs `ACLED_ACCESS_TOKEN`)*
- **UCDP** (3 tools) — battle deaths, country profiles
- **GDELT** (3 tools) — news tone, bilateral tensions
- **GDACS** (2 tools) — disaster alerts, country exposure
- **USGS** (2 tools) — significant earthquakes
- **NASA FIRMS / EONET** (2 tools) — fire detection, natural events *(FIRMS needs `NASA_FIRMS_API_KEY`)*
- **EIA** (3 tools) — petroleum, electricity *(needs `EIA_API_KEY`)*
- **WTO** (3 tools) — tariffs, trade barriers
- **USASpending** (2 tools) — federal contract awards
- **Polymarket** (1 tool) — prediction markets
- **CoinGecko** (3 tools) — Fear & Greed, stablecoin health
- **Open-Meteo** (1 tool) — climate anomalies
- **UNHCR** (1 tool) — displacement statistics

## Why a separate plugin

cfa-core is **pure compute** — give it numbers, get numbers. cfa-data **fetches** the numbers from public sources. Splitting them lets you:
- Use cfa-core fully offline
- Pick and choose data sources via env vars (ACLED/NASA/EIA optional)
- Swap in cfa-pro for premium feeds when needed

## Install

```bash
claude plugin install cfa-data
# or for local dev:
claude mcp add cfa-data -- node /path/to/packages/data-mcp-server/dist/index.js
```

## Optional API keys

Three of the 14 sources need free API keys:

```bash
export ACLED_ACCESS_TOKEN=...   # https://acleddata.com/register/
export NASA_FIRMS_API_KEY=...   # https://firms.modaps.eosdis.nasa.gov/api/
export EIA_API_KEY=...          # https://www.eia.gov/opendata/register.php
```

The other 11 sources work without authentication.

## Companion plugins

- **cfa-core** — corporate finance compute (WACC, DCF, LBO, credit, derivatives). 244 MCP tools, no keys.
- **cfa-pro** — premium feeds (FMP, LSEG, S&P, FactSet, Morningstar, Moody's, PitchBook). BYO API keys.

## License

MIT
