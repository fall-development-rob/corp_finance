# PRD: Alpha Vantage Data Source Integration

## Overview

Add Alpha Vantage as a supplementary financial data source in `packages/data-mcp-server/`, providing 43 MCP tools for stock quotes, time series, fundamentals, forex, crypto, commodities, economic indicators, server-side technical analysis, and AI-powered news sentiment.

## Problem Statement

The platform has comprehensive market data via FMP (180 tools), macro data via FRED (18 tools), and filings via EDGAR (15 tools), but lacks:

1. **AI news sentiment** — No source provides article-level sentiment scoring with topic classification
2. **Single-call fundamental snapshot** — FMP requires multiple calls to assemble a comprehensive company overview; Alpha Vantage returns 50+ metrics in one call
3. **Server-side technicals** — FMP technicals exist but Alpha Vantage supports 50+ indicators computed server-side (SMA, EMA, RSI, MACD, Bollinger Bands, Stochastic, ADX, OBV, VWAP, and more)
4. **Commodity spot prices** — Direct WTI, Brent, natural gas, and copper feeds complement EIA production data

## User Stories

1. As an equity analyst, I want AI-scored news sentiment for a stock so that I can quickly gauge media tone before diving into fundamental analysis.
2. As a macro analyst, I want quick GDP, CPI, and treasury yield lookups so that I don't have to search FRED's 800K series when I just need a headline number.
3. As a quant analyst, I want server-computed RSI, MACD, and Bollinger Bands so that I can screen for technical signals without local computation.
4. As a commodity analyst, I want WTI and Brent crude spot prices alongside EIA production data so that I can correlate supply fundamentals with price action.
5. As a credit analyst, I want a company overview with 50+ metrics in one call so that I can quickly assess a name before running detailed credit models.
6. As an FX analyst, I want real-time exchange rates for 150+ currency pairs so that I can cross-check FMP and FRED FX data.

## Features

### Quotes and Search (4 tools)

| Tool | Description |
|------|-------------|
| `av_quote` | Real-time stock quote: price, change, volume, latest trading day |
| `av_search` | Search for ticker symbols by company name or keyword |
| `av_market_status` | Global exchange open/closed status |
| `av_top_gainers_losers` | Top US market movers: gainers, losers, most active |

### Time Series (4 tools)

| Tool | Description |
|------|-------------|
| `av_intraday` | Intraday OHLCV (1min–60min intervals). Premium may be required. |
| `av_daily` | Daily OHLCV (compact: 100 days, full: 20+ years) |
| `av_weekly` | Weekly OHLCV with full history |
| `av_monthly` | Monthly OHLCV with full history |

### Fundamentals (7 tools)

| Tool | Description |
|------|-------------|
| `av_company_overview` | 50+ metrics: PE, EPS, market cap, dividend yield, book value, analyst target, sector, industry |
| `av_income_statement` | Annual + quarterly income statements (5 years / 20 quarters) |
| `av_balance_sheet` | Annual + quarterly balance sheets |
| `av_cash_flow` | Annual + quarterly cash flow statements |
| `av_earnings` | EPS history: reported, estimated, surprise % |
| `av_earnings_calendar` | Upcoming earnings dates (3/6/12 month horizon) |
| `av_ipo_calendar` | Upcoming IPOs with price range and exchange |

### Forex and Cryptocurrency (6 tools)

| Tool | Description |
|------|-------------|
| `av_fx_rate` | Real-time FX rate for any currency pair (150+ currencies) |
| `av_fx_daily` | Daily OHLC for FX pair |
| `av_fx_monthly` | Monthly OHLC for FX pair |
| `av_crypto_rate` | Real-time crypto exchange rate |
| `av_crypto_daily` | Daily OHLCV for crypto |
| `av_crypto_monthly` | Monthly OHLCV for crypto |

### Economic Indicators and Commodities (12 tools)

| Tool | Description |
|------|-------------|
| `av_real_gdp` | US Real GDP (quarterly/annual) |
| `av_cpi` | US Consumer Price Index |
| `av_inflation` | US annual inflation rate |
| `av_federal_funds_rate` | Effective federal funds rate |
| `av_treasury_yield` | US Treasury yield by maturity (3mo–30y) |
| `av_unemployment` | US unemployment rate |
| `av_nonfarm_payroll` | US nonfarm payroll |
| `av_retail_sales` | US advance retail sales |
| `av_wti_oil` | WTI crude oil prices |
| `av_brent_oil` | Brent crude oil prices |
| `av_natural_gas` | Henry Hub natural gas prices |
| `av_copper` | Global copper prices |

### Technical Indicators (9 tools)

| Tool | Description |
|------|-------------|
| `av_sma` | Simple Moving Average |
| `av_ema` | Exponential Moving Average |
| `av_rsi` | Relative Strength Index |
| `av_macd` | MACD with signal and histogram |
| `av_bbands` | Bollinger Bands |
| `av_stoch` | Stochastic Oscillator |
| `av_adx` | Average Directional Index |
| `av_obv` | On Balance Volume |
| `av_vwap` | Volume Weighted Average Price |

### Intelligence (1 tool)

| Tool | Description |
|------|-------------|
| `av_news_sentiment` | AI-scored news articles with sentiment, relevance, and topic classification |

## Architecture

Alpha Vantage is added as a new source directory within `packages/data-mcp-server/src/alphavantage/`, following the same pattern as `fred/`, `edgar/`, `yf/`, `wb/`:

- `client.ts` — API client with `ALPHA_VANTAGE_API_KEY` env var, in-memory cache (TTL per data type), rate limiting, timeout, and inline error detection
- `schemas/common.ts` — Zod schemas for all tool input types
- `tools/*.ts` — 7 tool registration modules

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `ALPHA_VANTAGE_API_KEY` | Yes | — | Free key from alphavantage.co |
| `AV_RATE_LIMIT` | No | `25` | Requests per minute ceiling |
| `AV_CACHE_TTL` | No | `300` | Default cache TTL override (seconds) |

### Cache Strategy

| Data Type | TTL | Examples |
|-----------|-----|---------|
| Real-time | 60s | Quotes, FX rates, crypto rates |
| Short | 5 min | Daily prices, technicals |
| Medium | 1 hour | News, search results |
| Long | 24 hours | Fundamentals, income/balance/cash flow |
| Static | 7 days | Market status, listing data |

## Success Metrics

- 43 new MCP tools registered in data-mcp-server
- All tools validate inputs via Zod schemas (MCP-001 compliance)
- All tool names use `av_` prefix with snake_case (MCP-003 compliance)
- Skill file at `.claude/skills/data-av/SKILL.md` with tool reference table
- TypeScript builds cleanly (`tsc --noEmit` passes)
- Graceful error when `ALPHA_VANTAGE_API_KEY` is not set
- No regression in existing data-mcp-server tools

## Out of Scope

- CLI wrapper (data access is via MCP tools only, consistent with other data sources)
- Premium-only endpoints (tools that require premium will return clear error messages)
- Replacing FMP or FRED as primary sources (Alpha Vantage is supplementary)
- New Rust computation modules (tools provide data inputs to existing modules)
