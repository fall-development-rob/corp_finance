# Domain Model: Alpha Vantage Data Source

## Bounded Context: Market Data (Extended)

Alpha Vantage extends the Market Data bounded context within `packages/data-mcp-server/` by adding a supplementary financial data source that provides stock quotes, time series, fundamentals, forex, crypto, commodities, economic indicators, technical analysis, and AI news sentiment. It follows the same ACL pattern established by FRED, EDGAR, FIGI, Yahoo Finance, and World Bank integrations.

### Domain Language (Ubiquitous Language)

| Term | Definition |
|------|-----------|
| **Global Quote** | A snapshot of a security's latest trading data: price, change, volume, latest trading day, previous close |
| **Company Overview** | A comprehensive fundamental profile with 50+ metrics: PE, EPS, market cap, dividend yield, book value, profit margins, analyst target price, sector, industry |
| **Time Series** | Chronologically ordered OHLCV (open, high, low, close, volume) data points at a specific interval (intraday, daily, weekly, monthly) |
| **Technical Indicator** | A server-computed mathematical transformation of price/volume data (SMA, EMA, RSI, MACD, Bollinger Bands, etc.) with configurable period, interval, and series type |
| **News Sentiment** | An AI-scored news article with overall sentiment score (-1 to 1), relevance score per ticker, and topic classifications |
| **Forex Rate** | A real-time currency exchange rate with bid/ask prices for a source/destination currency pair |
| **Crypto Rate** | A cryptocurrency exchange rate in a specified market currency with bid/ask and volume |
| **Economic Indicator** | A US macroeconomic time series (GDP, CPI, inflation, federal funds rate, treasury yield, unemployment, payroll, retail sales) |
| **Commodity Price** | A spot/benchmark price for a physical commodity (WTI crude, Brent crude, natural gas, copper) |

### Aggregates

#### Equity Data Aggregate
- Root: `AlphaVantageEquityRepository`
- Value Objects:
  - `GlobalQuote { symbol: String, price: Decimal, change: Decimal, change_pct: Decimal, volume: u64, latest_day: Date }`
  - `CompanyOverview { symbol: String, name: String, sector: String, industry: String, market_cap: Decimal, pe: Decimal, eps: Decimal, dividend_yield: Decimal, book_value: Decimal, analyst_target: Decimal, ... }`
  - `TimeSeries { symbol: String, interval: Interval, data_points: Vec<OhlcvPoint> }`
  - `OhlcvPoint { date: DateTime, open: Decimal, high: Decimal, low: Decimal, close: Decimal, volume: u64 }`
- Invariants:
  - Volume must be >= 0
  - Price values must be > 0
  - Interval restricted to { 1min, 5min, 15min, 30min, 60min, daily, weekly, monthly }

#### Fundamental Data Aggregate
- Root: `AlphaVantageFundamentalRepository`
- Value Objects:
  - `IncomeStatement { fiscal_date: Date, revenue: Decimal, gross_profit: Decimal, operating_income: Decimal, ebitda: Decimal, net_income: Decimal, eps: Decimal }`
  - `BalanceSheet { fiscal_date: Date, total_assets: Decimal, total_liabilities: Decimal, total_equity: Decimal, cash: Decimal, total_debt: Decimal }`
  - `CashFlow { fiscal_date: Date, operating: Decimal, investing: Decimal, financing: Decimal, capex: Decimal, free_cash_flow: Decimal }`
  - `EarningsRecord { fiscal_date: Date, reported_eps: Decimal, estimated_eps: Decimal, surprise: Decimal, surprise_pct: Decimal }`
- Invariants:
  - Fiscal dates must be valid calendar dates
  - Financial statement arrays ordered by fiscal date descending

#### Forex and Crypto Aggregate
- Root: `AlphaVantageFxCryptoRepository`
- Value Objects:
  - `ExchangeRate { from: String, to: String, rate: Decimal, bid: Decimal, ask: Decimal, timestamp: DateTime }`
  - `CryptoTimeSeries { symbol: String, market: String, interval: Interval, data_points: Vec<CryptoOhlcv> }`
- Invariants:
  - Currency codes must be valid ISO 4217 (fiat) or standard crypto symbols
  - Exchange rates must be > 0
  - Bid <= Ask

#### Economic and Commodity Aggregate
- Root: `AlphaVantageEconRepository`
- Value Objects:
  - `EconomicDataPoint { indicator: String, date: Date, value: Decimal, unit: String }`
  - `TreasuryYield { maturity: String, date: Date, value: Decimal }`
  - `CommodityPrice { commodity: String, date: Date, value: Decimal, unit: String }`
- Invariants:
  - Treasury maturity restricted to { 3month, 2year, 5year, 7year, 10year, 30year }
  - GDP values in billions USD
  - CPI as index value (not percentage)
  - Commodity prices must be > 0

#### Technical Analysis Aggregate
- Root: `AlphaVantageTechnicalRepository`
- Value Objects:
  - `TechnicalDataPoint { date: DateTime, indicator: String, values: Record<String, Decimal> }`
- Invariants:
  - Time period must be 1–500
  - Series type restricted to { close, open, high, low }
  - RSI values bounded 0–100
  - MACD returns three values (MACD, signal, histogram)
  - Bollinger Bands returns three values (upper, middle, lower)

#### News Intelligence Aggregate
- Root: `AlphaVantageNewsRepository`
- Value Objects:
  - `SentimentArticle { title: String, url: String, source: String, summary: String, overall_sentiment: Decimal, sentiment_label: String, relevance_scores: Vec<TickerRelevance>, topics: Vec<String>, published_at: DateTime }`
  - `TickerRelevance { ticker: String, relevance_score: Decimal, sentiment_score: Decimal, sentiment_label: String }`
- Invariants:
  - Overall sentiment score bounded -1.0 to 1.0
  - Relevance scores bounded 0.0 to 1.0
  - Sentiment labels restricted to { Bearish, Somewhat-Bearish, Neutral, Somewhat-Bullish, Bullish }
  - Topics restricted to Alpha Vantage canonical set (blockchain, earnings, ipo, mergers_and_acquisitions, financial_markets, economy_fiscal, economy_monetary, economy_macro, energy_transportation, finance, life_sciences, manufacturing, real_estate, retail_wholesale, technology)

### Anti-Corruption Layer

| External Endpoint | ACL Client | Key Responsibilities | Domain Value Object |
|-------------------|------------|---------------------|---------------------|
| `GLOBAL_QUOTE` | avFetch | Validate symbol, parse nested response | GlobalQuote |
| `SYMBOL_SEARCH` | avFetch | Normalize results | SearchResult |
| `TIME_SERIES_*` | avFetch | Parse interval-keyed JSON, validate OHLCV | TimeSeries |
| `COMPANY_OVERVIEW` | avFetch | Validate 50+ fields, handle nulls | CompanyOverview |
| `INCOME_STATEMENT` | avFetch | Separate annual/quarterly arrays | IncomeStatement[] |
| `BALANCE_SHEET` | avFetch | Separate annual/quarterly arrays | BalanceSheet[] |
| `CASH_FLOW` | avFetch | Separate annual/quarterly arrays | CashFlow[] |
| `EARNINGS` | avFetch | Parse surprise calculations | EarningsRecord[] |
| `CURRENCY_EXCHANGE_RATE` | avFetch | Validate currency codes | ExchangeRate |
| `FX_DAILY/MONTHLY` | avFetch | Parse time series | FxTimeSeries |
| `DIGITAL_CURRENCY_*` | avFetch | Parse multi-currency fields | CryptoTimeSeries |
| `REAL_GDP`, `CPI`, etc. | avFetch | Validate indicator format | EconomicDataPoint[] |
| `WTI`, `BRENT`, etc. | avFetch | Parse commodity series | CommodityPrice[] |
| `SMA`, `EMA`, `RSI`, etc. | avFetch | Validate indicator bounds | TechnicalDataPoint[] |
| `NEWS_SENTIMENT` | avFetch | Validate sentiment bounds, parse nested ticker scores | SentimentArticle[] |

The ACL client (`alphavantage/client.ts`) handles:
- API key injection via `ALPHA_VANTAGE_API_KEY` environment variable
- Inline error detection (Alpha Vantage returns errors as JSON fields, not HTTP status codes)
- In-memory cache with configurable TTL per data type
- Rate limiting (per-minute window with configurable ceiling)
- 15-second request timeout via AbortController

### Context Map Integration

```
Alpha Vantage API (external)
        |
        v
  avFetch (ACL) ── cache + rate limit + error detection
        |
        v
  packages/data-mcp-server/src/alphavantage/
  ├── tools/quotes.ts        → av_quote, av_search, av_market_status, av_top_gainers_losers
  ├── tools/time-series.ts   → av_intraday, av_daily, av_weekly, av_monthly
  ├── tools/fundamentals.ts  → av_company_overview, av_income_statement, av_balance_sheet, ...
  ├── tools/forex-crypto.ts  → av_fx_rate, av_fx_daily, av_crypto_rate, av_crypto_daily, ...
  ├── tools/economics.ts     → av_real_gdp, av_cpi, av_treasury_yield, av_wti_oil, ...
  ├── tools/technicals.ts    → av_sma, av_ema, av_rsi, av_macd, av_bbands, ...
  └── tools/intelligence.ts  → av_news_sentiment
        |
        v
  CFA Agent Specialists
  ├── cfa-equity-analyst     → av_company_overview, av_earnings, av_news_sentiment
  ├── cfa-macro-analyst      → av_real_gdp, av_cpi, av_treasury_yield, av_federal_funds_rate
  ├── cfa-quant-risk-analyst → av_sma, av_ema, av_rsi, av_macd, av_bbands
  └── cfa-credit-analyst     → av_company_overview, av_income_statement, av_balance_sheet
        |
        v
  corp-finance-mcp (computation tools)
```

### Relationship to Existing Data Sources

| Capability | Primary Source | Alpha Vantage Role |
|------------|---------------|-------------------|
| Stock quotes | FMP (fmp_quote) | Cross-check (av_quote) |
| Financial statements | FMP (fmp_income_statement, etc.) | Supplementary (av_income_statement, etc.) |
| Economic indicators | FRED (fred_series) | Quick lookup (av_real_gdp, av_cpi, etc.) |
| Technical indicators | FMP (fmp-technicals) | Extended set (av_sma, av_bbands, av_vwap, etc.) |
| Options chains | Yahoo Finance (yf_options_chain) | Not available |
| SEC filings | EDGAR (edgar_filings) | Not available |
| **AI news sentiment** | **None** | **Primary (av_news_sentiment)** |
| **Commodity spot prices** | EIA (production only) | **Primary (av_wti_oil, av_brent_oil, etc.)** |
| Forex | FMP (partial) | Extended coverage (av_fx_rate, 150+ pairs) |
| Crypto | CoinGecko (sentiment only) | Price data (av_crypto_rate, av_crypto_daily) |
