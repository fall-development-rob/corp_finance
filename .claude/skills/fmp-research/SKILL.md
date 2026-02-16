---
name: "FMP Research"
description: "Use the fmp-mcp-server tools for market research, screening, sector analysis, economic data, and index composition. Invoke when performing stock screening, sector/industry performance analysis, economic indicator analysis, treasury rate lookups, economic calendar review, market index constituent analysis, and macro environment assessment. Complements FMP Market Data with research-oriented tools."
---

# FMP Research

You have access to research-oriented FMP tools for market screening, economic analysis, sector/industry intelligence, and index composition. These tools provide macro and market-level data to complement the security-level data from FMP Market Data tools.

## Tool Reference

### Search & Discovery (14 tools)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_search_symbol` | Find stocks by ticker pattern | query, limit, exchange |
| `fmp_search_name` | Find stocks by company name | query, limit, exchange |
| `fmp_search_cik` | Look up company by SEC CIK number | cik |
| `fmp_search_cusip` | Look up security by CUSIP identifier | cusip |
| `fmp_search_isin` | Look up security by ISIN identifier | isin |
| `fmp_exchange_variants` | Get all exchange-listed variants of a symbol | symbol |
| `fmp_stock_screener` | Screen stocks by market cap, sector, industry, exchange, country | market_cap_more_than/less_than, sector, industry, exchange, country, limit |
| `fmp_stock_list` | Full list of all traded stocks | — |
| `fmp_financial_statement_symbols` | Symbols that have financial statements available | — |
| `fmp_cik_list` | Full CIK-to-symbol mapping list | — |
| `fmp_symbol_changes` | Recent ticker symbol changes (mergers, renames) | — |
| `fmp_etf_list` | Full list of all traded ETFs | — |
| `fmp_actively_trading` | Symbols actively trading right now | — |
| `fmp_available_exchanges` | List all supported exchanges | — |
| `fmp_available_sectors` | List all sector classifications | — |
| `fmp_available_industries` | List all industry classifications | — |
| `fmp_available_countries` | List all country classifications | — |

### Sector & Industry Analysis (10 tools)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_sector_performance` | All sector returns snapshot | date (optional) |
| `fmp_industry_performance` | Granular industry-level returns | date (optional) |
| `fmp_historical_sector_performance` | Sector returns over a date range | from, to |
| `fmp_historical_industry_performance` | Industry returns over a date range | from, to |
| `fmp_sector_pe` | Current P/E ratio by sector | date (optional) |
| `fmp_industry_pe` | Current P/E ratio by industry | date (optional) |
| `fmp_historical_sector_pe` | Historical sector P/E ratios | from, to |
| `fmp_historical_industry_pe` | Historical industry P/E ratios | from, to |
| `fmp_biggest_gainers` | Top gaining stocks today | — |
| `fmp_biggest_losers` | Top losing stocks today | — |
| `fmp_most_active` | Most actively traded stocks today | — |

### Market Indexes (7 tools)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_index_constituents` | S&P 500 / Nasdaq / Dow Jones member companies | index (sp500, nasdaq, dowjones) |
| `fmp_index_list` | Full list of available market indexes | — |
| `fmp_historical_sp500_constituent` | Historical S&P 500 additions and removals | from, to |
| `fmp_historical_nasdaq_constituent` | Historical Nasdaq additions and removals | from, to |
| `fmp_historical_dowjones_constituent` | Historical Dow Jones additions and removals | from, to |
| `fmp_batch_index_quotes` | Batch real-time quotes for index symbols | symbols (comma-separated) |
| `fmp_commodities_list` | List of available commodity symbols | — |

### Economic Data (4 tools)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_treasury_rates` | US Treasury rates across maturities (1M–30Y) | from, to |
| `fmp_economic_indicators` | GDP, CPI, unemployment, and other macro indicators | name, from, to |
| `fmp_economic_calendar` | Upcoming economic events (FOMC, CPI, employment) | from, to |
| `fmp_market_risk_premium` | Equity risk premium by country | — |

### Market Hours (3 tools)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_exchange_hours` | Trading hours for a specific exchange | exchange |
| `fmp_exchange_holidays` | Holiday schedule for a specific exchange | exchange |
| `fmp_all_exchange_hours` | Trading hours for all exchanges at once | — |

## Usage Patterns

### WACC Input Gathering
1. `fmp_treasury_rates` → Get current risk-free rate (use 10Y Treasury yield).
2. `fmp_market_risk_premium` → Get country-specific equity risk premium.
3. `fmp_company_profile` (from FMP Market Data) → Get beta.
4. `fmp_sector_pe` → Cross-check implied cost of equity against sector averages.
5. Feed real inputs into `wacc_calculator` from corp-finance-mcp.

### Peer Set Construction
1. `fmp_available_sectors` / `fmp_available_industries` → Confirm exact sector and industry labels for screening.
2. `fmp_stock_screener` → Filter by sector + market cap range + country to build a candidate peer list.
3. `fmp_industry_pe` → Validate that peer set P/E ratios are reasonable relative to the industry median.
4. `fmp_batch_quote` (from FMP Market Data) → Pull current multiples for the filtered set.
5. Feed into `comps_analysis` from corp-finance-mcp.

### Macro Environment Assessment
1. `fmp_economic_indicators` (GDP) → Growth trend.
2. `fmp_economic_indicators` (CPI) → Inflation trend.
3. `fmp_treasury_rates` → Yield curve shape (compare 2Y vs 10Y for inversion signal).
4. `fmp_market_risk_premium` → Current equity risk premium across markets.
5. `fmp_economic_calendar` → Upcoming catalysts (FOMC, payrolls, CPI releases).
6. `fmp_sector_performance` → Relative sector strength as a sentiment check.

### Sector Rotation Analysis
1. `fmp_historical_sector_performance` → Compare sector returns over trailing 1M, 3M, 6M, 12M windows.
2. `fmp_historical_sector_pe` → Track valuation expansion or compression by sector over time.
3. `fmp_sector_pe` → Current snapshot of relative sector valuations.
4. `fmp_biggest_gainers` / `fmp_biggest_losers` / `fmp_most_active` → Gauge near-term momentum and market breadth.
5. `fmp_treasury_rates` → Overlay rate environment to identify rate-sensitive rotation (e.g., growth vs. value).
6. `fmp_index_constituents` → Drill into specific index to see which sectors are over- or under-represented.

### Index Analysis
1. `fmp_index_list` → Discover available indexes.
2. `fmp_index_constituents` → Get all members of S&P 500, Nasdaq, or Dow Jones.
3. `fmp_historical_sp500_constituent` / `fmp_historical_nasdaq_constituent` / `fmp_historical_dowjones_constituent` → Track index turnover (additions/removals) over time.
4. `fmp_batch_index_quotes` → Get real-time quotes for a batch of index symbols.
5. `fmp_commodities_list` → Identify commodity tickers for macro overlay analysis.

### Security Identifier Resolution
1. `fmp_search_symbol` / `fmp_search_name` → Standard ticker or name lookup. Supports fuzzy matching — returns results ranked by name similarity and exchange preference.
2. **CLI**: `fmp-cli search "company name" --json` → Machine-readable JSON output with symbol, name, exchange, and currency. Prefers primary exchange listings (NYSE, NASDAQ, TSX, TSXV, LSE) over OTC.
3. `fmp_search_cik` / `fmp_search_cusip` / `fmp_search_isin` → Resolve institutional identifiers to symbols.
4. `fmp_exchange_variants` → Find all exchange listings for a given symbol (useful for ADRs and dual-listed securities).
5. `fmp_symbol_changes` → Check whether a ticker has been renamed or merged recently.

## Important Notes

- **Treasury Rates for WACC**: Always use `fmp_treasury_rates` to get the current 10-year yield for the risk-free rate input to `wacc_calculator`.
- **Market Risk Premium**: Use `fmp_market_risk_premium` alongside the risk-free rate for a complete cost-of-equity estimate.
- **Screener for Comps**: Use `fmp_stock_screener` to build peer sets by sector + market cap, then feed into `comps_analysis`.
- **Enumeration Helpers**: Call `fmp_available_sectors`, `fmp_available_industries`, `fmp_available_countries`, and `fmp_available_exchanges` to get exact valid values before passing them to the screener.
- **Date Filtering**: Most tools accept `from` and `to` date parameters for historical ranges.
- **Market Hours**: Check `fmp_exchange_hours` and `fmp_exchange_holidays` before making assumptions about trading availability, especially for international exchanges.
- **Historical Index Constituents**: Use the historical constituent tools to detect survivorship bias when backtesting—they show which companies were added or removed and when.
