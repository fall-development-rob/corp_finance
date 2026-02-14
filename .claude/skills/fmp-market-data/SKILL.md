---
name: "FMP Market Data"
description: "Use the fmp-mcp-server's 181 market data MCP tools for real-time and historical data retrieval. Invoke when looking up stock quotes, company profiles, financial statements (income, balance sheet, cash flow), key metrics, financial ratios, earnings data, analyst estimates, price targets, grades, ratings, dividends, splits, IPOs, M&A activity, executive compensation, shares float, and historical prices across 70,000+ securities including equities, ETFs, mutual funds, crypto, forex, indices, and commodities."
---

# FMP Market Data

You have access to 181 market data MCP tools from the FMP (Financial Modeling Prep) server for retrieving real-time and historical financial data. These tools fetch live data from FMP's API — they do NOT perform calculations. Use corp-finance-mcp tools for computation and fmp-mcp-server tools for data retrieval.

## Tool Reference

### Stock Quotes (17 tools)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_quote` | Real-time stock quote (price, change, volume, market cap, PE, 52-week range) | symbol |
| `fmp_batch_quote` | Multiple stock quotes in a single request | symbols (comma-separated) |
| `fmp_quote_short` | Abbreviated quote (price and volume only) | symbol |
| `fmp_aftermarket_trade` | After-hours last trade data for a single symbol | symbol |
| `fmp_aftermarket_quote` | After-hours bid/ask quote for a single symbol | symbol |
| `fmp_price_change` | Price change summary (1D, 5D, 1M, 3M, 6M, YTD, 1Y, 3Y, 5Y, 10Y, max) | symbol |
| `fmp_batch_quote_short` | Abbreviated quotes for multiple symbols at once | symbols (comma-separated) |
| `fmp_batch_aftermarket_trade` | Batch after-hours last trade data | symbols (comma-separated) |
| `fmp_batch_aftermarket_quote` | Batch after-hours bid/ask quotes | symbols (comma-separated) |
| `fmp_exchange_quotes` | All real-time quotes for an entire exchange | exchange |
| `fmp_batch_mutualfund_quotes` | Bulk quotes for all tracked mutual funds | — |
| `fmp_batch_etf_quotes` | Bulk quotes for all tracked ETFs | — |
| `fmp_batch_crypto_quotes` | Bulk quotes for all tracked cryptocurrencies | — |
| `fmp_batch_forex_quotes` | Bulk quotes for all tracked forex pairs | — |
| `fmp_batch_index_quotes` | Bulk quotes for all tracked market indices | — |
| `fmp_batch_commodity_quotes` | Bulk quotes for all tracked commodities | — |
| `fmp_historical_price` | End-of-day OHLCV with adjusted prices and VWAP | symbol, from, to |

**Historical Price Sub-tools (included in the 17 above and below):**

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_intraday_chart` | Intraday candles (1min to 4hour intervals) | symbol, interval, from, to |
| `fmp_historical_price_light` | Lightweight EOD prices (close, volume only — faster response) | symbol, from, to |
| `fmp_historical_price_unadjusted` | Raw unadjusted EOD OHLCV (no split/dividend adjustment) | symbol, from, to |
| `fmp_historical_price_div_adjusted` | EOD prices adjusted for dividends only (not splits) | symbol, from, to |

### Company Profiles (17 tools)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_company_profile` | Full company profile (description, sector, industry, CEO, employees, beta) | symbol |
| `fmp_profile_by_cik` | Look up company profile using SEC CIK number | cik |
| `fmp_stock_peers` | Peer companies for comparable analysis | symbol |
| `fmp_key_executives` | Company executives and their roles | symbol |
| `fmp_market_cap` | Current market capitalisation | symbol |
| `fmp_batch_market_cap` | Market cap for multiple symbols at once | symbols (comma-separated) |
| `fmp_historical_market_cap` | Historical market cap time series | symbol, from, to |
| `fmp_company_notes` | Company-issued notes and filings annotations | symbol |
| `fmp_employee_count` | Current employee headcount | symbol |
| `fmp_historical_employee_count` | Historical employee headcount over time | symbol |
| `fmp_shares_float` | Current shares float and outstanding shares | symbol |
| `fmp_shares_float_all` | Shares float data for all available companies | — |
| `fmp_executive_compensation` | Detailed executive compensation (salary, bonus, stock awards, total) | symbol |
| `fmp_compensation_benchmark` | Compensation benchmarking data across industry/role | symbol |
| `fmp_delisted_companies` | List of companies that have been delisted | — |
| `fmp_ma_latest` | Latest mergers & acquisitions activity | — |
| `fmp_ma_search` | Search M&A deals by company, date, or deal type | name, from, to |

### Financial Statements (26 tools)

#### Core Statements

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_income_statement` | Income statement (revenue, EBITDA, net income, EPS) | symbol, period (annual/quarter), limit |
| `fmp_balance_sheet` | Balance sheet (assets, liabilities, equity, debt, cash) | symbol, period, limit |
| `fmp_cash_flow` | Cash flow statement (operating, investing, financing, FCF) | symbol, period, limit |

#### Trailing Twelve Months (TTM)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_income_ttm` | TTM income statement | symbol |
| `fmp_balance_sheet_ttm` | TTM balance sheet | symbol |
| `fmp_cash_flow_ttm` | TTM cash flow statement | symbol |

#### Metrics, Ratios & Scoring

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_key_metrics` | 50+ financial metrics (PE, PB, EV/EBITDA, D/E, ROE, ROA) | symbol, period, limit |
| `fmp_key_metrics_ttm` | TTM key metrics snapshot | symbol |
| `fmp_financial_ratios` | Comprehensive ratio analysis (profitability, liquidity, leverage) | symbol, period, limit |
| `fmp_ratios_ttm` | TTM financial ratios snapshot | symbol |
| `fmp_financial_scores` | Piotroski F-Score, Altman Z-Score, and other scoring models | symbol |
| `fmp_owner_earnings` | Buffett-style owner earnings (net income + D&A − maintenance capex) | symbol |
| `fmp_enterprise_values` | Enterprise value breakdown (market cap + debt − cash) | symbol, period, limit |

#### Growth Rates

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_income_growth` | Year-over-year income statement growth rates | symbol, period, limit |
| `fmp_balance_sheet_growth` | Year-over-year balance sheet growth rates | symbol, period, limit |
| `fmp_cash_flow_growth` | Year-over-year cash flow growth rates | symbol, period, limit |
| `fmp_financial_growth` | Aggregated financial growth rates (revenue, EPS, FCF, etc.) | symbol, period, limit |

#### Segment & Reporting Data

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_financial_reports_dates` | Available SEC filing dates for a company | symbol |
| `fmp_financial_reports_json` | Full SEC filing data in structured JSON format | symbol, year, period |
| `fmp_revenue_product_segments` | Revenue broken down by product/business segment | symbol, period |
| `fmp_revenue_geo_segments` | Revenue broken down by geographic region | symbol, period |
| `fmp_latest_financial_statements` | Most recent financial statements across all companies | — |

#### As-Reported Statements (GAAP, unmodified)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_income_as_reported` | Income statement exactly as reported in SEC filings | symbol, period, limit |
| `fmp_balance_as_reported` | Balance sheet exactly as reported in SEC filings | symbol, period, limit |
| `fmp_cash_flow_as_reported` | Cash flow statement exactly as reported in SEC filings | symbol, period, limit |
| `fmp_full_statement_as_reported` | Complete financial statement package as filed with the SEC | symbol, period, limit |

### Earnings & Analyst Data (16 tools)

#### Earnings

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_earnings` | Historical EPS actual vs estimated, revenue, surprise % | symbol, limit |
| `fmp_earnings_calendar` | Upcoming earnings dates across all companies | from, to |
| `fmp_earnings_transcript` | Full earnings call transcript (management commentary, Q&A) | symbol, year, quarter |
| `fmp_earnings_transcript_latest` | Most recent earnings call transcript for a company | symbol |
| `fmp_earnings_transcript_dates` | All available transcript dates for a company | symbol |
| `fmp_earnings_transcript_list` | List of all available earnings transcripts (all companies) | — |

#### Analyst Estimates & Targets

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_analyst_estimates` | Consensus revenue, EBITDA, EPS, net income estimates | symbol, period, limit |
| `fmp_price_target` | Individual analyst price targets with analyst name and firm | symbol |
| `fmp_price_target_consensus` | Aggregated consensus price target (average, median, high, low) | symbol |

#### Analyst Grades & Ratings

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_grades` | Recent analyst grade changes (upgrades, downgrades, reiterations) | symbol, limit |
| `fmp_grades_historical` | Full history of analyst grade changes | symbol, limit |
| `fmp_grades_consensus` | Aggregated consensus grade (Strong Buy → Strong Sell) | symbol |
| `fmp_ratings_snapshot` | Current composite rating snapshot (overall score + sub-scores) | symbol |
| `fmp_ratings_historical` | Historical composite rating scores over time | symbol, limit |

#### Dividends

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_dividends` | Dividend payment history (amount, ex-date, pay date, frequency) | symbol |
| `fmp_dividends_calendar` | Upcoming dividend ex-dates across all companies | from, to |

### Splits & IPOs (5 tools)

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_splits` | Stock split history for a company | symbol |
| `fmp_splits_calendar` | Upcoming and recent stock splits across all companies | from, to |
| `fmp_ipo_calendar` | Upcoming and recent IPO dates, price ranges, and exchanges | from, to |
| `fmp_ipo_disclosure` | Detailed IPO disclosure filings and registration details | from, to |
| `fmp_ipo_prospectus` | IPO prospectus data (shares offered, price range, underwriters) | from, to |

---

## Usage Patterns & CFA Workflows

### Full Equity Research Report
1. **Company overview**: `fmp_company_profile` + `fmp_key_executives` + `fmp_executive_compensation`
2. **Financial statements**: `fmp_income_statement` + `fmp_balance_sheet` + `fmp_cash_flow` (5 years annual)
3. **TTM snapshot**: `fmp_income_ttm` + `fmp_balance_sheet_ttm` + `fmp_cash_flow_ttm`
4. **Valuation metrics**: `fmp_key_metrics` + `fmp_enterprise_values` + `fmp_financial_ratios`
5. **Growth analysis**: `fmp_income_growth` + `fmp_cash_flow_growth` + `fmp_financial_growth`
6. **Segment breakdown**: `fmp_revenue_product_segments` + `fmp_revenue_geo_segments`
7. **Peer comparison**: `fmp_stock_peers` → then batch pull metrics for peers
8. **Analyst consensus**: `fmp_analyst_estimates` + `fmp_price_target_consensus` + `fmp_grades_consensus`
9. **Earnings quality**: `fmp_earnings` + `fmp_earnings_transcript_latest` + `fmp_financial_scores`
10. Then use `dcf_model`, `comps_analysis`, `credit_metrics` from corp-finance-mcp for computation

### Quick Valuation Check
1. `fmp_quote` → Current price and market cap
2. `fmp_key_metrics_ttm` → PE, EV/EBITDA, P/B ratios (TTM)
3. `fmp_price_target_consensus` → Analyst consensus target
4. `fmp_ratings_snapshot` → Composite analyst rating
5. Compare current price vs analyst targets and historical multiples

### Credit Analysis Workflow (CFA Level II/III)
1. `fmp_income_statement` → Revenue, EBITDA, interest expense (5 years)
2. `fmp_balance_sheet` → Total debt, cash, total assets, current ratio components
3. `fmp_cash_flow` → Operating cash flow, FCF, debt repayment
4. `fmp_financial_scores` → Altman Z-Score, Piotroski F-Score
5. `fmp_financial_ratios` → Debt/equity, interest coverage, current ratio
6. `fmp_owner_earnings` → Sustainable earnings power
7. Then use `credit_metrics`, `altman_zscore`, `debt_capacity` from corp-finance-mcp

### Earnings Season Monitoring
1. `fmp_earnings_calendar` → Identify upcoming reporters this week/month
2. `fmp_analyst_estimates` → Pre-earnings consensus expectations
3. `fmp_earnings` → Post-report: actual vs estimated, surprise %
4. `fmp_earnings_transcript` → Management commentary, guidance, Q&A tone
5. `fmp_grades` → Track analyst upgrades/downgrades after report
6. `fmp_price_change` → Measure market reaction across timeframes

### Dividend & Income Analysis
1. `fmp_dividends` → Full payment history, frequency, growth trend
2. `fmp_dividends_calendar` → Upcoming ex-dates for portfolio holdings
3. `fmp_income_statement` → Payout ratio calculation (dividends / net income)
4. `fmp_cash_flow` → FCF payout ratio (dividends / free cash flow)
5. `fmp_financial_growth` → Earnings growth to assess dividend sustainability

### IPO & Corporate Actions Analysis
1. `fmp_ipo_calendar` → Upcoming IPOs with pricing details
2. `fmp_ipo_prospectus` → Underwriter details, shares offered, use of proceeds
3. `fmp_ipo_disclosure` → Registration and disclosure filings
4. `fmp_splits_calendar` → Upcoming stock splits
5. `fmp_ma_latest` + `fmp_ma_search` → M&A deal flow and targets

### After-Hours & Extended Trading
1. `fmp_aftermarket_trade` → Last after-hours trade price for a symbol
2. `fmp_aftermarket_quote` → Current after-hours bid/ask spread
3. `fmp_batch_aftermarket_trade` → Monitor multiple positions after close
4. `fmp_batch_aftermarket_quote` → Portfolio-wide after-hours snapshot

### Multi-Asset Screening
1. `fmp_batch_etf_quotes` → Full ETF universe snapshot
2. `fmp_batch_crypto_quotes` → Full crypto market snapshot
3. `fmp_batch_forex_quotes` → All forex pair rates
4. `fmp_batch_commodity_quotes` → All commodity prices
5. `fmp_batch_index_quotes` → All market index levels
6. `fmp_exchange_quotes` → All quotes for a specific exchange

### Historical & As-Reported Analysis (SEC Filing Deep Dive)
1. `fmp_financial_reports_dates` → List available SEC filings
2. `fmp_financial_reports_json` → Pull structured filing data
3. `fmp_income_as_reported` + `fmp_balance_as_reported` + `fmp_cash_flow_as_reported` → GAAP as-filed
4. `fmp_full_statement_as_reported` → Complete package as submitted to the SEC
5. Compare standardised (`fmp_income_statement`) vs as-reported to identify data provider adjustments

---

## Important Notes

- **Data vs Computation**: FMP tools retrieve data; corp-finance-mcp tools compute. Always use both together.
- **Batch where possible**: Use `fmp_batch_quote`, `fmp_batch_market_cap`, and the bulk asset-class endpoints to minimise API calls.
- **Rate Limits**: FMP has rate limits (5 req/min free, 300 req/min paid). Batch aggressively to stay within limits.
- **Caching**: Quotes are cached 30s, financials 1 hour, profiles 24 hours.
- **TTM tools**: Use `_ttm` variants for the most current trailing-twelve-month view without manually summing quarters.
- **As-Reported vs Standardised**: Use `_as_reported` tools when you need exact SEC filing figures; use standard tools for cross-company comparability.
- **Requires API Key**: `FMP_API_KEY` must be set in environment.
