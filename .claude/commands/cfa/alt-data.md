# Alternative Data Dashboard

Alternative and non-traditional data analysis using Polymarket, CoinGecko, UNHCR, and Open-Meteo from `geopolitical-alternative`.

## What It Does
Aggregates forward-looking signals: prediction market odds for geopolitical events, crypto sentiment indicators, displacement statistics as instability proxies, and climate anomaly data. Produces a non-traditional risk dashboard.

## Agent
Routes to `cfa-quant-risk-analyst`.

## Key Tools
`polymarket_geopolitical`, `coingecko_fear_greed`, `unhcr_country`, `openmeteo_climate_anomaly`, `coingecko_stablecoin_health`

## Usage
Run without arguments for a global alternative data dashboard. Specify a topic (e.g., "election odds", "crypto sentiment", "Syria displacement") for a focused view.
