---
name: vendor-sp-global
description: "S&P Global (Kensho) -- Capital IQ financials, earnings call transcripts, company tearsheets, and funding digests via S&P Global MCP"
---

# S&P Global / Kensho (Vendor Integration)

S&P Global provides institutional financial data via their Kensho-powered MCP server. Access Capital IQ financials, earnings call transcripts, company tearsheets, and funding digests. This is an OPTIONAL premium integration -- users must have their own S&P Global subscription.

## MCP Server

**Package**: `@corp-finance/vendor-sp-global`
**Authentication**: Bearer token
**Environment Variables**:
- `SP_GLOBAL_API_KEY` -- API key for Bearer token authentication

## Tool Catalogue (14 tools)

| Tool | Purpose |
|------|---------|
| `sp_company_search` | Search for companies by name, ticker, CIK, or ISIN across Capital IQ universe |
| `sp_company_tearsheet` | Comprehensive company profile: business description, key metrics, ownership, ratings |
| `sp_capital_structure` | Debt maturity profile, leverage ratios, credit facility details, and capital stack |
| `sp_ownership` | Institutional ownership, insider holdings, activist positions, and ownership changes |
| `sp_financials` | Standardised financial statements (income, balance sheet, cash flow) with Capital IQ quality |
| `sp_estimates` | Broker estimates, consensus forecasts, estimate revisions, and earnings surprises |
| `sp_segment_data` | Business and geographic segment breakdowns, segment-level financials |
| `sp_earnings_transcript` | Full-text earnings call transcripts with speaker attribution and Q&A sections |
| `sp_credit_rating` | S&P credit ratings, outlooks, rating histories, and watchlist status |
| `sp_peer_analysis` | Peer group construction, comparative financial metrics, and relative valuation |
| `sp_key_developments` | Material corporate events: M&A announcements, management changes, regulatory actions |
| `sp_industry_benchmark` | Industry-level financial benchmarks, market sizing, and competitive landscape |
| `sp_ma_deals` | M&A transaction data: deal terms, valuations, advisors, and deal structure |
| `sp_funding_digest` | Private funding rounds, IPO data, and investment activity tracking |

## Key Capabilities

- **Capital IQ Financials**: Detailed financial statements, standardised metrics, segment data, historical financials via `sp_financials` and `sp_segment_data`
- **Earnings Transcripts**: Full-text earnings call transcripts with speaker attribution, Q&A sections, management guidance extraction via `sp_earnings_transcript`
- **Company Tearsheets**: Comprehensive company profiles with business description, key metrics, ownership, credit ratings via `sp_company_tearsheet`
- **Credit Ratings**: S&P credit ratings, outlooks, and rating histories for corporates and sovereigns via `sp_credit_rating`
- **M&A and Funding**: Transaction data, deal terms, private funding rounds, and IPO tracking via `sp_ma_deals` and `sp_funding_digest`
- **Industry Analysis**: Industry-level financial benchmarks, market sizing, competitive landscape via `sp_industry_benchmark`

## Data Note

S&P Global/Kensho subscription required. Data subject to S&P Global licence terms, including redistribution and usage restrictions. Particularly valuable for earnings transcript analysis and standardised financial data that complements SEC EDGAR raw filings.
