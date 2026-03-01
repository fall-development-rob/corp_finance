---
name: data-wb
description: "World Bank Open Data -- 16,000+ international economic indicators for sovereign analysis, country risk, emerging markets, GDP, debt ratios, trade, and FDI"
---

# World Bank Open Data

You have access to 14 World Bank MCP tools for retrieving international economic indicators across 200+ countries. Covers 16,000+ indicators spanning GDP, inflation, debt, trade, demographics, health, education, and development. Essential for sovereign analysis, country risk assessment, and emerging market research.

No authentication required.

## Tool Reference

### Indicators (5 tools)

| MCP Tool | Description |
|----------|-------------|
| `wb_indicator` | Get indicator data for a country. Returns time series of yearly values. |
| `wb_indicator_search` | Search indicators by keyword. Discover available indicators before querying. |
| `wb_indicator_info` | Get metadata for a specific indicator: name, description, source, unit, topic. |
| `wb_indicator_sources` | List all World Bank data sources (WDI, IDS, Doing Business, etc.). |
| `wb_topics` | List all topic categories (Agriculture, Education, Health, Trade, etc.). |

### Countries (4 tools)

| MCP Tool | Description |
|----------|-------------|
| `wb_country` | Get detailed info: name, region, income level, capital city, coordinates, lending type. |
| `wb_countries` | List all countries with codes, regions, income levels, and capital cities. |
| `wb_country_indicators` | Get 10 popular macro indicators for a country in one call: GDP, growth, inflation, unemployment, population, current account, government debt, exports, real rate, FX rate. |
| `wb_income_levels` | List World Bank income level classifications (High, Upper middle, Lower middle, Low). |

### Data Queries (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `wb_data_series` | Get a time series for one country and one indicator over a date range. |
| `wb_multi_country` | Compare one indicator across multiple countries. Semicolon-separated codes (e.g., `US;GB;CN`). |
| `wb_time_series` | Get multiple indicators for one country. Semicolon-separated indicator codes. |

### Sources (2 tools)

| MCP Tool | Description |
|----------|-------------|
| `wb_sources` | List all data sources with IDs, names, descriptions, and URLs. |
| `wb_source_indicators` | List indicators available within a specific data source by source ID. |

## Key Indicator Codes

| Indicator Code | Description |
|----------------|-------------|
| `NY.GDP.MKTP.CD` | GDP (current USD) |
| `NY.GDP.MKTP.KD.ZG` | GDP growth (annual %) |
| `FP.CPI.TOTL.ZG` | Inflation, consumer prices (annual %) |
| `SL.UEM.TOTL.ZS` | Unemployment (% of labour force) |
| `DT.DOD.DECT.GD.ZS` | External debt stocks (% of GNI) |
| `GC.DOD.TOTL.GD.ZS` | Central government debt (% of GDP) |
| `CM.MKT.LCAP.GD.ZS` | Market capitalisation of listed companies (% of GDP) |
| `BN.CAB.XOKA.GD.ZS` | Current account balance (% of GDP) |
| `BX.KLT.DINV.WD.GD.ZS` | Foreign direct investment, net inflows (% of GDP) |
| `NE.EXP.GNFS.ZS` | Exports of goods and services (% of GDP) |

## Usage Notes

- Use `wb_country_indicators` for a quick macro snapshot of any country -- it fetches 10 key indicators in parallel.
- Use `wb_multi_country` to compare a single indicator across countries (e.g., GDP growth for G7 nations).
- Use `wb_time_series` to pull multiple indicators for one country in a single call.
- The `date` parameter accepts year ranges like `2015:2024` or single years like `2023`.
- Country codes use ISO 3166-1 alpha-2 (e.g., `US`, `GB`, `CN`, `BR`, `IN`) or World Bank group codes (`WLD` for World, `EUU` for EU).
- Use `wb_indicator_search` to discover indicators you do not already know the code for.
- Data is typically annual with a 6-12 month lag for the most recent year.
