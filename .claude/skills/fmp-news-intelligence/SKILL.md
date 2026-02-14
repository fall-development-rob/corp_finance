---
name: "FMP News Intelligence"
description: "Use the fmp-mcp-server news tools for real-time financial news monitoring, press release tracking, and sentiment analysis across stocks, crypto, and forex. Invoke when monitoring breaking news, tracking press releases, or gathering news sentiment for investment decisions."
---

# FMP News Intelligence

## Overview

This skill provides access to 10 news-oriented tools from the `fmp-mcp-server` MCP server. These tools enable real-time financial news monitoring, corporate press release tracking, and cross-asset news intelligence spanning stocks, cryptocurrency, and forex markets. Use them to stay informed on breaking developments, gauge sentiment ahead of key events, and surface market-moving headlines.

---

## Available Tools

### General News & Editorial

#### `fmp_fmp_articles`
- **Purpose:** Retrieve FMP editorial articles and in-depth analysis pieces.
- **Key Inputs:**
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of articles to return per page.
- **When to use:** When you need curated analytical content or editorial commentary on market themes.

#### `fmp_news_general`
- **Purpose:** Fetch general financial news headlines across all markets.
- **Key Inputs:**
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of headlines to return per page.
- **When to use:** For a broad market pulse — top headlines spanning equities, macro, commodities, and more.

#### `fmp_news_press_releases`
- **Purpose:** Retrieve the latest corporate press releases across all companies.
- **Key Inputs:**
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of press releases to return per page.
- **When to use:** When scanning for new corporate announcements (earnings, guidance, M&A, leadership changes) without filtering by symbol.

---

### Asset-Class News Feeds

#### `fmp_news_stock`
- **Purpose:** Fetch the latest stock-specific news headlines.
- **Key Inputs:**
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of articles to return per page.
- **When to use:** When monitoring the equity news stream for broad stock market developments.

#### `fmp_news_crypto`
- **Purpose:** Fetch the latest cryptocurrency news headlines.
- **Key Inputs:**
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of articles to return per page.
- **When to use:** When tracking crypto-specific developments — token launches, regulation, exchange news, on-chain events.

#### `fmp_news_forex`
- **Purpose:** Fetch the latest forex market news headlines.
- **Key Inputs:**
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of articles to return per page.
- **When to use:** When monitoring currency pair movements, central bank policy signals, or geopolitical events affecting FX.

---

### Symbol-Specific Search

#### `fmp_search_press_releases`
- **Purpose:** Search corporate press releases filtered by one or more ticker symbols.
- **Key Inputs:**
  - `symbols` (string) — Comma-separated ticker symbols (e.g., `"AAPL,MSFT"`).
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of results to return per page.
- **When to use:** When you need official company announcements for specific tickers — earnings releases, 8-K filings, product launches.

#### `fmp_search_stock_news`
- **Purpose:** Search stock news articles filtered by one or more ticker symbols.
- **Key Inputs:**
  - `symbols` (string) — Comma-separated ticker symbols (e.g., `"TSLA,NVDA"`).
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of results to return per page.
- **When to use:** When you need media coverage and analyst commentary specific to individual stocks.

#### `fmp_search_crypto_news`
- **Purpose:** Search cryptocurrency news filtered by one or more crypto symbols.
- **Key Inputs:**
  - `symbols` (string) — Comma-separated crypto symbols (e.g., `"BTCUSD,ETHUSD"`).
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of results to return per page.
- **When to use:** When tracking news for specific tokens or trading pairs.

#### `fmp_search_forex_news`
- **Purpose:** Search forex news filtered by one or more currency pairs.
- **Key Inputs:**
  - `symbols` (string) — Comma-separated forex pairs (e.g., `"EURUSD,GBPUSD"`).
  - `page` (integer) — Page number for pagination (default: 0).
  - `limit` (integer) — Number of results to return per page.
- **When to use:** When tracking news affecting specific currency pairs or regional FX flows.

---

## Usage Patterns

### 1. Pre-Earnings News Scan

Build a comprehensive sentiment picture before an earnings announcement.

```
Step 1: fmp_search_stock_news  →  symbols="AAPL", limit=20
           Gather recent media coverage and analyst commentary.

Step 2: fmp_search_press_releases  →  symbols="AAPL", limit=10
           Pull official company press releases (guidance updates, pre-announcements).

Step 3: Review sentiment
           Synthesize headlines and press releases to assess whether market
           expectations lean bullish, bearish, or neutral heading into earnings.
```

**Why this order matters:** Third-party news provides the market narrative, while press releases reveal what the company itself has communicated. Comparing the two surfaces disconnects between market expectations and corporate signals.

### 2. Breaking News Monitor

Rapidly identify and validate market-moving events.

```
Step 1: fmp_news_general  →  limit=25
           Scan top financial headlines for breaking stories.

Step 2: fmp_news_stock  →  limit=25
           Check equity-specific news for impacted tickers.

Step 3: Cross-reference with quotes
           Use FMP quote tools (e.g., fmp_quote) to verify whether price action
           confirms the news impact. Look for unusual volume or sharp moves.
```

**Why this order matters:** General news catches macro events first; stock news narrows down to affected equities. Cross-referencing with live quotes validates whether the news has already been priced in or is still developing.

### 3. Crypto/Forex Intelligence

Identify market-moving events across digital assets and currency markets.

```
Step 1: fmp_news_crypto  →  limit=20
           Pull the latest cryptocurrency headlines — regulation, exchange events,
           protocol upgrades, whale movements.

Step 2: fmp_news_forex  →  limit=20
           Pull the latest forex headlines — central bank decisions, economic data
           releases, geopolitical developments.

Step 3: Identify market-moving events
           Look for correlated themes (e.g., USD strength affecting both crypto and
           FX). Flag headlines with potential for outsized price impact. Use
           fmp_search_crypto_news or fmp_search_forex_news to drill into specific
           symbols when a theme emerges.
```

**Why this order matters:** Crypto and forex markets are highly interconnected through the US dollar. Scanning both feeds together reveals cross-asset themes — a hawkish Fed signal, for example, may simultaneously pressure crypto valuations and strengthen USD pairs.

---

## Tips & Best Practices

- **Pagination:** Start with `page=0` and a reasonable `limit` (10–25). Increase the page number to load older results when building a historical news timeline.
- **Symbol formatting:** Use standard ticker symbols for stocks (`AAPL`, `MSFT`), crypto pairs (`BTCUSD`, `ETHUSD`), and forex pairs (`EURUSD`, `GBPUSD`). Separate multiple symbols with commas.
- **Combine with other FMP skills:** Pair news intelligence with quote data, financial statements, and technical indicators for a complete research workflow. News provides the "why" behind price moves that quantitative data alone cannot explain.
- **Rate awareness:** When making multiple calls in sequence, be mindful of API rate limits. Batch your symbol searches rather than making one call per ticker.
