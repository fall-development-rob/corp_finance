---
name: geopolitical-trade
description: "Trade policy and energy supply data -- EIA petroleum/electricity, WTO tariffs and trade barriers, USASpending federal contract awards"
---

# Trade Policy & Energy Supply Data

You have access to 8 trade MCP tools across 3 sources for tracking energy supply/demand, trade policy barriers, tariff rates, and US federal spending. Essential for commodity analysis, supply chain risk, trade policy impact, and macro research.

EIA requires `EIA_API_KEY` (free). WTO and USASpending require no authentication.

## Tool Reference

### EIA (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `eia_petroleum` | US petroleum supply & demand: crude production, inventory, refinery throughput, imports/exports, SPR levels. Weekly or monthly frequency. |
| `eia_electricity` | US electricity generation by fuel type (coal, natural gas, nuclear, solar, wind, hydro). Monthly or annual. |
| `eia_capacity` | US power plant operating generator capacity by energy source. Returns plant-level nameplate capacity data. |

### WTO (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `wto_tariffs` | Tariff rates by country and product. Returns MFN applied rates and bound rates. Use HS codes to filter by product category. |
| `wto_barriers` | SPS/TBT trade barrier notifications. Filter by notifying country or keyword. Returns notification details including products covered. |
| `wto_trade_stats` | Bilateral trade flows between two countries. Returns merchandise exports, imports, and trade balance values. |

### USASpending (2 tools)

| MCP Tool | Description |
|----------|-------------|
| `usaspending_contracts` | Search US federal contract awards. Filter by keyword, agency, amount range, date range. Returns award details including recipient and obligation amount. |
| `usaspending_agencies` | Federal agency spending summaries: total budgetary resources and obligations by top-tier agency for a fiscal year. |

## Usage Notes

- Use `eia_petroleum` for weekly crude oil inventory data -- key input for commodity trading and energy sector analysis.
- Use `wto_tariffs` + `wto_barriers` together to assess trade policy risk for a country or sector.
- Use `wto_trade_stats` to map bilateral trade dependence (e.g., US-China trade flows).
- USASpending is valuable for defence sector equity analysis -- track contract awards to specific companies.
- USASpending API can be slow (>5s) -- the circuit breaker has a generous timeout.
- EIA data covers US only. For global energy data, combine with World Bank `wb_trade` indicators.
