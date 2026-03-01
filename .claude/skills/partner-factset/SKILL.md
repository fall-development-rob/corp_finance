---
name: partner-factset
description: "FactSet -- multi-asset financial data, analytics, and portfolio tools via FactSet MCP"
---

# FactSet (Partner Integration)

FactSet provides multi-asset financial data, analytics, and portfolio tools. This is an OPTIONAL premium integration -- users must have their own FactSet subscription and API credentials.

## MCP Server

**Package**: `@corp-finance/partner-factset`
**Authentication**: Basic auth (username + API key)
**Environment Variables**:
- `FACTSET_USERNAME` -- FactSet account username
- `FACTSET_API_KEY` -- FactSet API key

## Tool Catalogue (16 tools)

| Tool | Purpose |
|------|---------|
| `factset_fundamentals` | Standardised financial statements, ratios, and key metrics across 70,000+ companies |
| `factset_estimates` | Broker estimates, consensus forecasts, estimate revisions, and earnings surprise history |
| `factset_company_search` | Search for companies by name, ticker, ISIN, CUSIP, or SEDOL |
| `factset_prices` | Global equity, fixed income, and FX pricing -- end-of-day and intraday |
| `factset_bond_pricing` | Bond pricing, yield analytics, spread decomposition, and duration/convexity |
| `factset_ownership` | Institutional ownership, 13F holdings, and ownership concentration |
| `factset_institutional` | Institutional investor profiles, AUM, allocation, and investment style |
| `factset_portfolio_analytics` | Attribution analysis, risk decomposition, and compliance monitoring |
| `factset_risk_model` | Multi-factor risk model exposures, covariance matrices, and risk forecasts |
| `factset_factor_exposure` | Factor library exposures: value, momentum, quality, size, volatility |
| `factset_supply_chain` | Company supply chain relationships, revenue exposure, and supplier/customer links |
| `factset_geo_revenue` | Geographic revenue breakdown by country and region |
| `factset_events` | Corporate events calendar: earnings dates, ex-dividend dates, conferences |
| `factset_people` | Executive and board member profiles, compensation, and biographical data |
| `factset_ma_deals` | M&A transaction data: deal terms, multiples, advisors, and structure |
| `factset_batch_request` | Batch multiple FactSet API requests into a single call for efficiency |

## Key Capabilities

- **Company Fundamentals**: Standardised financial statements, ratios, estimates, and actuals via `factset_fundamentals` and `factset_estimates`
- **Ownership & Institutional**: Institutional ownership, 13F holdings, activist positions, investor profiles via `factset_ownership` and `factset_institutional`
- **Portfolio Analytics**: Attribution analysis, risk decomposition, factor exposure, compliance monitoring via `factset_portfolio_analytics`, `factset_risk_model`, and `factset_factor_exposure`
- **Supply Chain & Revenue**: Company supply chain mapping and geographic revenue breakdowns via `factset_supply_chain` and `factset_geo_revenue`
- **Fixed Income**: Bond pricing, spread analytics, yield curve data via `factset_bond_pricing` and `factset_prices`
- **Batch Operations**: Combine multiple requests into a single call for high-throughput workflows via `factset_batch_request`

## Data Note

FactSet subscription required. Data subject to FactSet licence terms, including redistribution and usage restrictions. FactSet excels at consensus estimates, institutional ownership, and multi-asset portfolio analytics.
