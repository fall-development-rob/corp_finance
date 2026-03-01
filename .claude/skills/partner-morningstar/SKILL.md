---
name: partner-morningstar
description: "Morningstar -- fund ratings, investment research, ESG data, and portfolio analytics via Morningstar MCP"
---

# Morningstar (Partner Integration)

Morningstar provides fund ratings, investment research, ESG data, and portfolio analytics. This is an OPTIONAL premium integration -- users must have their own Morningstar subscription and API credentials.

## MCP Server

**Package**: `@corp-finance/partner-morningstar`
**Authentication**: Bearer token
**Environment Variables**:
- `MORNINGSTAR_API_KEY` -- API key for Bearer token authentication

## Tool Catalogue (14 tools)

| Tool | Purpose |
|------|---------|
| `ms_fund_rating` | Morningstar star ratings, analyst ratings (Gold/Silver/Bronze), and category rankings |
| `ms_fund_holdings` | Fund top holdings, sector allocation, geographic breakdown, and style box |
| `ms_fund_performance` | Fund performance data: returns, risk-adjusted metrics, category percentile rankings |
| `ms_historical_nav` | Historical net asset values, total returns, and dividend distributions |
| `ms_expense_analysis` | Expense ratios, fee breakdowns, fee-level assessments, and fee impact projections |
| `ms_etf_analytics` | ETF cost analysis, tracking error, premium/discount, flows, and replication method |
| `ms_fair_value` | Morningstar fair value estimates, uncertainty ratings, and margin of safety |
| `ms_moat_rating` | Economic moat ratings (Wide/Narrow/None), moat trend, and moat sources |
| `ms_esg_risk` | Sustainalytics ESG Risk Rating, carbon metrics, controversy flags, and portfolio ESG scoring |
| `ms_analyst_report` | Morningstar analyst research reports, investment theses, and bull/bear cases |
| `ms_company_profile` | Company profiles: business description, key metrics, sector classification |
| `ms_portfolio_xray` | Portfolio X-ray analysis: overlap detection, hidden exposures, and fee aggregation |
| `ms_asset_allocation` | Asset allocation analysis, style drift detection, and allocation optimisation inputs |
| `ms_peer_comparison` | Peer group comparison: relative performance, risk, fees, and holdings overlap |

## Key Capabilities

- **Fund Ratings**: Morningstar star ratings, analyst ratings, category rankings, risk-adjusted returns via `ms_fund_rating` and `ms_fund_performance`
- **Fund Data**: NAV, expense ratios, holdings, sector/geography allocation, style box, performance attribution via `ms_fund_holdings`, `ms_historical_nav`, and `ms_expense_analysis`
- **ETF Analytics**: ETF cost analysis, tracking error, premium/discount, flows, replication method via `ms_etf_analytics`
- **Investment Research**: Analyst reports, fair value estimates, economic moat ratings, uncertainty ratings via `ms_fair_value`, `ms_moat_rating`, and `ms_analyst_report`
- **ESG**: Sustainalytics ESG Risk Rating, carbon metrics, controversy flags, portfolio-level ESG scoring via `ms_esg_risk`
- **Portfolio Analytics**: X-ray analysis, overlap detection, asset allocation optimisation, fee impact analysis via `ms_portfolio_xray` and `ms_asset_allocation`

## Data Note

Morningstar subscription required. Data subject to Morningstar licence terms, including redistribution and usage restrictions. Morningstar is particularly valuable for mutual fund and ETF analysis, ESG scoring (via Sustainalytics), and fair value research.
