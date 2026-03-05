# ADR-014: Alpha Vantage Data Source Integration

## Status: Accepted

## Context

ADR-003 originally rejected Alpha Vantage as the primary market data provider in favour of FMP (70,000+ symbols, 300 req/min). ADR-010 excluded it from the free data source set, noting the 25 req/day free tier as insufficient.

However, Alpha Vantage provides several capabilities that complement existing data sources:

1. **Company Overview**: A single endpoint returning 50+ fundamental metrics (PE, EPS, book value, dividend yield, analyst target, 52-week range) — more comprehensive than any single FMP endpoint.
2. **Economic Indicators**: GDP, CPI, inflation, federal funds rate, treasury yields, unemployment, retail sales, nonfarm payroll — bundled in a single API, complementing FRED's 800K+ series for quick lookups.
3. **Commodities**: WTI, Brent, natural gas, copper — direct commodity price feeds.
4. **AI News Sentiment**: Unique sentiment scoring with topic classification on financial news articles — not available from FMP, FRED, or Yahoo Finance.
5. **Server-Side Technical Indicators**: 50+ technical indicators (SMA, EMA, RSI, MACD, Bollinger Bands, VWAP, etc.) computed server-side — reduces local computation burden.
6. **Forex & Crypto**: 150+ currency pairs and crypto assets with daily/monthly time series.

The free tier limitation (25 req/day) is mitigated by aggressive caching (TTL: 60s–7d by data type) and the fact that Alpha Vantage serves as a supplementary cross-check source, not the primary data pipeline.

## Decision

Add Alpha Vantage as a supplementary data source within the existing `packages/data-mcp-server/`, following the established per-source directory pattern (ADR-010, ADR-011).

### Package Structure

```
packages/data-mcp-server/src/alphavantage/
  client.ts                    -- API client with caching, rate limiting
  schemas/common.ts            -- Zod schemas for all parameter types
  tools/
    quotes.ts                  -- 4 tools: av_quote, av_search, av_market_status, av_top_gainers_losers
    time-series.ts             -- 4 tools: av_intraday, av_daily, av_weekly, av_monthly
    fundamentals.ts            -- 7 tools: overview, income, balance, cash flow, earnings, calendars
    forex-crypto.ts            -- 6 tools: FX rate/daily/monthly, crypto rate/daily/monthly
    economics.ts               -- 12 tools: GDP, CPI, inflation, fed rate, treasury, unemployment, payroll, retail, oil, gas, copper
    technicals.ts              -- 9 tools: SMA, EMA, RSI, MACD, BBANDS, STOCH, ADX, OBV, VWAP
    intelligence.ts            -- 1 tool: av_news_sentiment
```

### Source Details

| Field | Value |
|-------|-------|
| Base URL | `https://www.alphavantage.co/query` |
| Auth | API key via `ALPHA_VANTAGE_API_KEY` env var (free at alphavantage.co) |
| Free Tier | 25 requests/day |
| Premium | 600–1,200 requests/min |
| Cache TTL | REALTIME (60s), SHORT (300s), MEDIUM (1h), LONG (24h), STATIC (7d) |
| Total Tools | 43 |

### Tool Naming

All tools use `av_` prefix to distinguish from FMP (`fmp_`), FRED (`fred_`), and Yahoo Finance (`yf_`) tools.

### Integration

Registered in `packages/data-mcp-server/src/index.ts` alongside FRED, EDGAR, FIGI, Yahoo Finance, World Bank, and geopolitical sources. Skill file at `.claude/skills/data-av/SKILL.md`.

## Consequences

### Positive
- Fills the AI news sentiment gap — no other source provides this
- Company overview endpoint is the most comprehensive single-call fundamental snapshot
- Server-side technicals reduce agent computation overhead
- Commodity prices (WTI, Brent, natgas, copper) complement EIA production data from ADR-011
- Treasury yields provide an alternative to FRED for quick risk-free rate lookups
- Total data MCP tool count increases from ~121 to ~164

### Negative
- Free tier limit (25 req/day) makes Alpha Vantage unsuitable as a primary source
- Some endpoints (intraday, daily adjusted, options) require premium subscription
- Adds 1 new environment variable (`ALPHA_VANTAGE_API_KEY`)
- Overlaps with existing sources: economic indicators (FRED), fundamentals (FMP), prices (FMP/YF)

### Mitigation
- Aggressive caching: economic indicators cached 24h, fundamentals 24h, quotes 60s
- Client returns clear error when rate limited, directing users to premium plans
- Skill documentation explicitly states overlap and recommends primary sources per use case

## Related Decisions

- ADR-003: FMP Integration (Alpha Vantage rejected as primary; now added as supplementary)
- ADR-010: Multi-Source Data Integration (established the per-source pattern in data-mcp-server)
- ADR-011: Geopolitical Data Integration (same pattern, same package)

## References

- [Alpha Vantage Documentation](https://www.alphavantage.co/documentation/)
- [Alpha Vantage Free API Key](https://www.alphavantage.co/support/#api-key)
