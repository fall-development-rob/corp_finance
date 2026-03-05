# ADR-010: Multi-Source Financial Data Integration

## Status: Accepted

## Context

The CFA agent relied solely on FMP (Financial Modeling Prep) for external market data (180 tools). Users need access to macro economic data (yield curves, CPI, GDP), structured SEC filings (XBRL), options chains, identifier cross-referencing, and international indicators that FMP doesn't cover. Additionally, institutional users may have subscriptions to premium data providers (LSEG, S&P Global, FactSet, Morningstar, Moody's, PitchBook).

## Decision

Add 6 free/open MCP server packages and 6 partner skill documentation files:

| Source | Package | Tools | Auth | Key Data |
|--------|---------|-------|------|----------|
| FRED | packages/fred-mcp-server/ | 18 | Free API key | Yields, CPI, GDP, spreads |
| SEC EDGAR | packages/edgar-mcp-server/ | 20 | User-Agent only | XBRL facts, filings, search |
| OpenFIGI | packages/figi-mcp-server/ | 8 | Optional key | Identifier mapping |
| Yahoo Finance | packages/yf-mcp-server/ | 15 | None (unofficial) | Options chains, prices |
| World Bank | packages/wb-mcp-server/ | 14 | None | Sovereign/EM indicators |
| Alpha Vantage | packages/data-mcp-server/src/alphavantage/ | 43 | Free API key | Quotes, fundamentals, FX, crypto, economics, technicals, AI news sentiment |

Partner integrations are config-only — users connect their own MCP endpoints.

Each free source follows the established FMP pattern: separate MCP server package, agent bridge, skill file, and pipeline routing.

## Consequences

- Total MCP tool count: 200 (corp-finance) + 180 (FMP) + 75 (original free sources) + 43 (Alpha Vantage) = 498 tools
- 6 new data skills + 6 partner skills = 12 new skill files
- Each source is independently deployable — users only enable sources they need
- Yahoo Finance is unofficial and may break; marked as fragile in docs
- Alpha Vantage added as supplementary source (ADR-014) — aggressive caching mitigates 25 req/day free tier limit
