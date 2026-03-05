---
name: data-av
description: "Alpha Vantage -- stock quotes, time series, fundamentals, forex, crypto, commodities, economic indicators, technical indicators, and AI news sentiment via the Alpha Vantage API"
---

# Alpha Vantage Data

You have access to 36 Alpha Vantage MCP tools for retrieving financial market data including equities, forex, crypto, commodities, economic indicators, technical analysis, and AI-powered news sentiment.

**Requires**: `ALPHA_VANTAGE_API_KEY` environment variable (free from https://www.alphavantage.co/support/#api-key).

**Rate limits**: Free tier allows 25 requests/day. Premium plans offer 600-1200 requests/min. Aggressive caching is enabled to minimize API calls.

## Tool Reference

### Quotes and Search (4 tools)

| MCP Tool | Description |
|----------|-------------|
| `av_quote` | Real-time stock quote: price, change, volume, latest trading day |
| `av_search` | Search for ticker symbols by company name or keyword |
| `av_market_status` | Current open/closed status for major global exchanges |
| `av_top_gainers_losers` | Top gainers, losers, and most actively traded US tickers |

### Time Series (4 tools)

| MCP Tool | Description |
|----------|-------------|
| `av_intraday` | Intraday OHLCV (1min, 5min, 15min, 30min, 60min). May require premium. |
| `av_daily` | Daily OHLCV. compact = 100 days, full = 20+ years |
| `av_weekly` | Weekly OHLCV with full 20+ year history |
| `av_monthly` | Monthly OHLCV with full 20+ year history |

### Fundamentals (7 tools)

| MCP Tool | Description |
|----------|-------------|
| `av_company_overview` | Company profile with 50+ fundamental metrics: PE, EPS, market cap, dividend yield, book value, analyst target |
| `av_income_statement` | Annual and quarterly income statements (up to 5 years / 20 quarters) |
| `av_balance_sheet` | Annual and quarterly balance sheets |
| `av_cash_flow` | Annual and quarterly cash flow statements |
| `av_earnings` | Earnings history: reported EPS, estimated EPS, surprise % |
| `av_earnings_calendar` | Upcoming earnings dates and EPS estimates (3/6/12 month horizon) |
| `av_ipo_calendar` | Upcoming IPO dates with price range and exchange |

### Forex (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `av_fx_rate` | Real-time exchange rate for any currency pair (150+ currencies) |
| `av_fx_daily` | Daily OHLC time series for a forex pair |
| `av_fx_monthly` | Monthly OHLC time series for a forex pair |

### Cryptocurrency (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `av_crypto_rate` | Real-time crypto exchange rate with bid/ask |
| `av_crypto_daily` | Daily OHLCV for crypto with volume and market cap |
| `av_crypto_monthly` | Monthly OHLCV for crypto with full history |

### Economic Indicators and Commodities (12 tools)

| MCP Tool | Description |
|----------|-------------|
| `av_real_gdp` | US Real GDP (quarterly or annual) |
| `av_cpi` | US Consumer Price Index (monthly or semiannual) |
| `av_inflation` | US annual inflation rate |
| `av_federal_funds_rate` | Effective federal funds rate (daily/weekly/monthly) |
| `av_treasury_yield` | US Treasury yield by maturity (3mo, 2y, 5y, 7y, 10y, 30y) |
| `av_unemployment` | US unemployment rate (monthly) |
| `av_nonfarm_payroll` | US nonfarm payroll (monthly) |
| `av_retail_sales` | US advance retail sales (monthly) |
| `av_wti_oil` | WTI crude oil prices |
| `av_brent_oil` | Brent crude oil prices |
| `av_natural_gas` | Henry Hub natural gas prices |
| `av_copper` | Global copper prices |

### Technical Indicators (9 tools)

| MCP Tool | Description |
|----------|-------------|
| `av_sma` | Simple Moving Average |
| `av_ema` | Exponential Moving Average |
| `av_rsi` | Relative Strength Index (overbought >70, oversold <30) |
| `av_macd` | MACD with signal line and histogram |
| `av_bbands` | Bollinger Bands (upper, middle, lower) |
| `av_stoch` | Stochastic Oscillator (SlowK, SlowD) |
| `av_adx` | Average Directional Index (trend strength) |
| `av_obv` | On Balance Volume |
| `av_vwap` | Volume Weighted Average Price (intraday only) |

### Intelligence (1 tool)

| MCP Tool | Description |
|----------|-------------|
| `av_news_sentiment` | Market news with AI-powered sentiment analysis. Filter by tickers and/or topics. |

## CLI Usage

The `av-cli` command provides direct terminal access to Alpha Vantage data:

```bash
av-cli quote AAPL                           # Real-time quote
av-cli daily MSFT --outputsize full         # Full daily history
av-cli overview TSLA                        # Company fundamentals
av-cli fx EUR USD                           # Forex rate
av-cli crypto BTC USD                       # Crypto rate
av-cli treasury --maturity 10year           # Treasury yield
av-cli sma AAPL --period 50 --interval daily  # 50-day SMA
av-cli news --tickers AAPL,MSFT            # News with sentiment
av-cli search "Apple"                       # Symbol search
av-cli --help                               # Full command list
```

## Key Differentiators vs Other Data Sources

- **Company Overview**: Returns 50+ fundamental metrics in a single call — more comprehensive than FMP's profile
- **Economic Indicators**: Direct access to GDP, CPI, unemployment, treasury yields, commodities (similar to FRED but bundled)
- **AI News Sentiment**: Unique sentiment scoring and topic classification on news articles
- **Technical Indicators**: 50+ indicators computed server-side — no local calculation needed
- **Global Coverage**: Forex (150+ currencies), crypto, international equities

## Usage Notes

- Free tier is limited to **25 requests/day**. Use caching aggressively.
- Premium endpoints (intraday, daily adjusted, bulk quotes, options) may return errors on free tier.
- All tools are prefixed with `av_` to distinguish from FMP and Yahoo Finance tools.
- Use `av_company_overview` as a quick fundamental snapshot before diving into detailed statements.
- Economic indicator tools overlap with FRED. Use Alpha Vantage for quick lookups; use FRED for deeper analysis with 800K+ series.
- Technical indicator tools overlap with FMP technicals. Alpha Vantage computes server-side with more indicator variety.
