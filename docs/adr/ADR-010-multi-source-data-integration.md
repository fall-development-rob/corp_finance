# ADR-010: Multi-Source Financial Data Integration

## Status: Accepted

## Context

The CFA agent relied solely on FMP (Financial Modeling Prep) for external market data (180 tools). Users need access to macro economic data (yield curves, CPI, GDP), structured SEC filings (XBRL), options chains, identifier cross-referencing, and international indicators that FMP doesn't cover. Additionally, institutional users may have subscriptions to premium data providers (LSEG, S&P Global, FactSet, Morningstar, Moody's, PitchBook).

## Decision

Add 5 free/open MCP server packages and 6 partner skill documentation files:

| Source | Package | Tools | Auth | Key Data |
|--------|---------|-------|------|----------|
| FRED | packages/fred-mcp-server/ | 18 | Free API key | Yields, CPI, GDP, spreads |
| SEC EDGAR | packages/edgar-mcp-server/ | 20 | User-Agent only | XBRL facts, filings, search |
| OpenFIGI | packages/figi-mcp-server/ | 8 | Optional key | Identifier mapping |
| Yahoo Finance | packages/yf-mcp-server/ | 15 | None (unofficial) | Options chains, prices |
| World Bank | packages/wb-mcp-server/ | 14 | None | Sovereign/EM indicators |

Partner integrations are config-only — users connect their own MCP endpoints.

Each free source follows the established FMP pattern: separate MCP server package, agent bridge, skill file, and pipeline routing.

## Consequences

- Total MCP tool count: 200 (corp-finance) + 180 (FMP) + 75 (new free sources) = 455 tools
- 5 new data skills + 6 partner skills = 11 new skill files
- Each source is independently deployable — users only enable sources they need
- Yahoo Finance is unofficial and may break; marked as fragile in docs
- Alpha Vantage excluded (25 req/day free tier insufficient)
