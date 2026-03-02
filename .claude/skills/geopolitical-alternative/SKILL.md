---
name: geopolitical-alternative
description: "Alternative data -- Polymarket prediction markets, CoinGecko Fear & Greed and stablecoin health, UNHCR displacement statistics, Open-Meteo climate anomalies"
---

# Alternative Data

You have access to 8 alternative data MCP tools across 4 sources providing forward-looking probability signals, sentiment proxies, displacement indicators, and climate anomaly data. Essential for macro analysis, geopolitical risk, ESG scoring, and quantitative strategies.

No authentication required for any source. CoinGecko free tier has tight rate limits (10-30 req/min).

## Tool Reference

### Polymarket (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `polymarket_events` | Search active prediction markets. Returns events with market odds, volume, and liquidity. |
| `polymarket_odds` | Current odds for a specific event by event_id or slug. Returns outcomes with prices, volume, liquidity. |
| `polymarket_geopolitical` | Pre-filtered geopolitical markets (politics, elections, conflict, sanctions). Returns curated list with odds and volume. |

### CoinGecko (2 tools)

| MCP Tool | Description |
|----------|-------------|
| `coingecko_fear_greed` | Crypto Fear & Greed index (0-100). Returns current value, classification (extreme_fear to extreme_greed), and history. |
| `coingecko_stablecoin_health` | Stablecoin peg monitoring. Checks price vs $1.00 peg, flags depegging (>0.5% deviation). Returns deviation and 24h change. |

### UNHCR (2 tools)

| MCP Tool | Description |
|----------|-------------|
| `unhcr_displacement` | Global displacement statistics: refugee, asylum seeker, IDP, and stateless populations by country of origin and asylum. |
| `unhcr_country` | Country displacement profile: total displaced, refugees from/hosted, IDPs, asylum seekers, trend vs prior year. |

### Open-Meteo (1 tool)

| MCP Tool | Description |
|----------|-------------|
| `openmeteo_climate_anomaly` | ERA5 climate anomaly analysis for a lat/lon location. Compares observation period vs baseline climatology. Returns temperature/precipitation deltas and severity (normal/moderate/severe/extreme). |

## Usage Notes

- Use `polymarket_geopolitical` for forward-looking probability signals on elections, conflicts, sanctions -- unavailable from traditional data sources.
- Polymarket odds are crowd-sourced probabilities. Higher volume = more reliable signal.
- Polymarket is an unofficial API with no stability guarantees -- treat as best-effort.
- Use `coingecko_fear_greed` as a risk appetite proxy -- extreme fear often correlates with broader risk-off sentiment.
- Use `coingecko_stablecoin_health` as a DeFi/crypto systemic risk indicator.
- UNHCR data is essential for sovereign risk and ESG analysis -- displacement is a leading indicator of instability.
- Open-Meteo ERA5 reanalysis data has a ~5-day lag -- not suitable for real-time weather signals.
- CoinGecko has a 2-second polite delay between requests to respect free-tier rate limits.
