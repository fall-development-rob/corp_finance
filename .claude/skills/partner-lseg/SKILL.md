---
name: partner-lseg
description: "LSEG (London Stock Exchange Group) -- bond pricing, yield curve analysis, FX carry trade evaluation, options valuation, macro dashboard via LSEG Financial Analytics MCP"
---

# LSEG Financial Analytics (Partner Integration)

LSEG (London Stock Exchange Group) provides institutional-grade financial data and analytics. This is an OPTIONAL premium integration -- users must have their own LSEG subscription and API credentials.

## MCP Server

**Package**: `@corp-finance/partner-lseg`
**Authentication**: OAuth2 (client credentials flow)
**Environment Variables**:
- `LSEG_CLIENT_ID` -- OAuth2 client ID
- `LSEG_CLIENT_SECRET` -- OAuth2 client secret

## Tool Catalogue (15 tools)

| Tool | Purpose |
|------|---------|
| `lseg_historical_prices` | Historical end-of-day pricing for equities, bonds, FX, and commodities |
| `lseg_intraday_prices` | Intraday tick and bar data for real-time market analysis |
| `lseg_bond_pricing` | Bond pricing, yield-to-maturity, duration, convexity, and spread analytics |
| `lseg_fx_rates` | Spot and forward FX rates, cross-currency pairs, carry trade inputs |
| `lseg_company_search` | Search for companies by name, ticker, ISIN, or SEDOL |
| `lseg_fundamentals` | Standardised financial statements, ratios, and key metrics |
| `lseg_esg_scores` | ESG scores, carbon metrics, controversy flags, and sustainability ratings |
| `lseg_news` | Real-time and historical news, filtered by company, sector, or topic |
| `lseg_options_chain` | Options chains with strikes, expiries, Greeks, and implied volatility |
| `lseg_economic_indicators` | Macro economic indicators: GDP, CPI, PMI, employment, central bank rates |
| `lseg_yield_curve` | Government and swap yield curves, term structure construction and interpolation |
| `lseg_credit_spreads` | Corporate credit spreads by rating, sector, tenor, and benchmark decomposition |
| `lseg_reference_data` | Security master, instrument identifiers, index constituents, and classifications |
| `lseg_corporate_actions` | Dividends, splits, mergers, spin-offs, and other corporate event data |
| `lseg_ownership` | Institutional and insider ownership, 13F holdings, activist positions |

## Key Capabilities

- **Fixed Income**: Bond pricing, credit spread analysis, yield-to-maturity calculations, duration/convexity, benchmark spread decomposition via `lseg_bond_pricing`, `lseg_credit_spreads`, and `lseg_yield_curve`
- **Yield Curves**: Government and swap curve construction, curve interpolation, term structure analysis, historical curve comparison via `lseg_yield_curve`
- **FX Analytics**: Carry trade evaluation, forward rate implied yield differentials, cross-currency basis swap analysis via `lseg_fx_rates`
- **Options**: Equity and FX options valuation, implied volatility surfaces, Greeks calculation via `lseg_options_chain`
- **Macro Dashboard**: Economic indicator tracking, central bank rate expectations, inflation breakevens, PMI aggregation via `lseg_economic_indicators`
- **Reference Data**: Security master, corporate actions, dividend forecasts, index constituents via `lseg_reference_data` and `lseg_corporate_actions`

## Data Note

LSEG subscription required. Data subject to LSEG license terms, including redistribution and usage restrictions. For fixed income and FX analytics, LSEG provides institutional-quality pricing that complements the corp-finance-mcp calculation engine.
