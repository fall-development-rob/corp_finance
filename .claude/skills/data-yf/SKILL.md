---
name: data-yf
description: "Yahoo Finance (unofficial) -- equity quotes, historical prices, options chains with Greeks, financial statements, and analyst data. WARNING -- unofficial API, may be unstable"
---

# Yahoo Finance Data (Unofficial)

You have access to 15 Yahoo Finance MCP tools for equity quotes, historical prices, options chains, financial statements, and analyst data.

**WARNING**: This is an UNOFFICIAL Yahoo Finance API. It reverse-engineers Yahoo's internal endpoints and may break without notice at any time. Do not rely on this for production or time-critical workflows. Use FMP as the primary data source and Yahoo Finance as a supplementary/fallback source.

No API key required.

## Tool Reference

### Quotes and Prices (5 tools)

| MCP Tool | Description |
|----------|-------------|
| `yf_quote` | Real-time quote: price, change, volume, market cap, PE, 52-week range |
| `yf_historical` | Historical OHLCV price data. Periods: 1d to max. Intervals: 1m to 3mo |
| `yf_summary_detail` | Summary details: dividend yield, PE, market cap, beta, 52-week range, volume averages |
| `yf_fast_info` | Lightweight price snapshot: current price, change, change percent |
| `yf_batch_quotes` | Multi-symbol quotes in a single request. Comma-separated symbols. |

### Options (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `yf_options_expirations` | Get available options expiration dates for a symbol (epoch timestamps) |
| `yf_options_chain` | Full options chain (calls + puts) for a specific expiration date |
| `yf_options_all` | Options chains for ALL expiration dates. Fetches expirations then chains in parallel. |

### Financial Statements (4 tools)

| MCP Tool | Description |
|----------|-------------|
| `yf_income_statement` | Income statement (annual + quarterly): revenue, gross profit, EBITDA, net income, EPS |
| `yf_balance_sheet` | Balance sheet (annual + quarterly): assets, liabilities, equity, cash, debt |
| `yf_cash_flow` | Cash flow statement (annual + quarterly): operating, investing, financing, capex, FCF |
| `yf_earnings` | Earnings history and trends: actual vs estimate EPS, surprise %, forward estimates |

### Company Info and Analyst Data (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `yf_info` | Full company profile: sector, industry, employees, description, website, key statistics |
| `yf_analyst_targets` | Analyst price targets: mean, high, low, recommendation, number of analysts |
| `yf_upgrades_downgrades` | Analyst upgrade/downgrade history: firm, action, from/to grade, date |

## Key Differentiator: Options Chains

The options tools are the primary reason to use Yahoo Finance over FMP. Yahoo provides full options chain data including calls, puts, strikes, last price, bid/ask, volume, open interest, and implied volatility. FMP does not provide this data.

## Usage Notes

- **Prefer FMP** for equity data, financial statements, and analyst data. FMP is a stable, authenticated API.
- **Use Yahoo Finance** for options chains (which FMP lacks) and as a cross-check or fallback source.
- All tools are prefixed with `[UNOFFICIAL Yahoo Finance]` in their descriptions as a reminder of instability.
- Historical data supports intervals from 1-minute (intraday, limited lookback) to 3-month (long-term).
- Financial statements include both annual and quarterly data in a single response.
- Batch quotes (`yf_batch_quotes`) accept comma-separated symbols for efficient multi-stock comparison.
- Options expiration dates are returned as Unix epoch timestamps. Pass these to `yf_options_chain`.
