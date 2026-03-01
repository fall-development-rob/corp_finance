---
name: data-fred
description: "FRED (Federal Reserve Economic Data) -- macro economic indicators, yield curves, interest rates, CPI, GDP, unemployment, and credit spreads via the St. Louis Fed API"
---

# FRED Economic Data

You have access to 18 FRED MCP tools for retrieving macroeconomic data from the Federal Reserve Bank of St. Louis. Covers 800,000+ time series including interest rates, inflation, employment, GDP, yield curves, and credit spreads.

**Requires**: `FRED_API_KEY` environment variable (free from https://fred.stlouisfed.org/docs/api/api_key.html).

## Tool Reference

### Series (6 tools)

| MCP Tool | Description |
|----------|-------------|
| `fred_series` | Get time-series observations (date/value pairs) for any FRED series |
| `fred_series_info` | Get metadata: title, units, frequency, seasonal adjustment, date range |
| `fred_series_search` | Search FRED by keyword to discover available data series |
| `fred_series_categories` | Get category classification for a series |
| `fred_series_tags` | Get tags describing a series (geography, source, frequency, topic) |
| `fred_series_vintage` | Get vintage/revision dates for real-time data analysis |

### Releases (4 tools)

| MCP Tool | Description |
|----------|-------------|
| `fred_releases` | List all FRED data releases with IDs and names |
| `fred_release` | Get details for a single release by ID |
| `fred_release_dates` | Get historical and upcoming publication dates for a release |
| `fred_release_series` | Get all data series associated with a specific release |

### Categories (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `fred_category` | Get info for a FRED category by ID (0 = root) |
| `fred_category_children` | Navigate child categories to discover data by topic |
| `fred_category_series` | Get all data series within a category |

### Tags (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `fred_tags` | List FRED tags with series counts and group IDs |
| `fred_related_tags` | Discover tags that co-occur with given tag names |
| `fred_series_match_tags` | Find all series matching specific tag names |

### Yield Curve and Spreads (2 tools)

| MCP Tool | Description |
|----------|-------------|
| `fred_yield_curve` | Fetch the full US Treasury yield curve (1M to 30Y, 11 tenors in parallel). Returns structured curve with date, labels, and rates. Essential for fixed income analysis, term structure modelling, and WACC risk-free rate selection. |
| `fred_spread` | Compute spread between any two FRED series (long minus short). Use for yield curve slope (DGS10 - DGS2), credit spreads (BAMLH0A0HYM2 - DGS10), or any rate differential. Returns date-aligned spread time series. |

## Key Series IDs

| Series ID | Description |
|-----------|-------------|
| `DGS10` | 10-Year Treasury Constant Maturity Rate |
| `DGS2` | 2-Year Treasury Constant Maturity Rate |
| `T10Y2Y` | 10Y-2Y Treasury spread (yield curve slope) |
| `FEDFUNDS` | Federal Funds Effective Rate |
| `CPIAUCSL` | Consumer Price Index (All Urban, SA) |
| `GDP` | Gross Domestic Product (nominal) |
| `UNRATE` | Civilian Unemployment Rate |
| `BAMLH0A0HYM2` | ICE BofA US High Yield OAS |

## Usage Notes

- The `fred_yield_curve` tool fetches all 11 standard tenors (DGS1MO through DGS30) in parallel and returns a structured curve object. Use this for WACC risk-free rate selection and term structure analysis.
- The `fred_spread` tool accepts any two series IDs and computes the differential. Common use: `series_id_long: "DGS10", series_id_short: "DGS2"` for the 10Y-2Y slope.
- Use `fred_series_search` to discover series you do not already know the ID for.
- Observations with value `"."` represent missing data.
- All tools support date range filtering via `observation_start` and `observation_end`.
