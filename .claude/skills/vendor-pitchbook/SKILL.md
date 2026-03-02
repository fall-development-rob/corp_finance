---
name: vendor-pitchbook
description: "PitchBook -- private equity deal data, VC transactions, fund performance, and company profiles for private market research via PitchBook MCP"
---

# PitchBook (Vendor Integration)

PitchBook provides private equity deal data, VC transactions, fund performance, and company profiles. This is an OPTIONAL premium integration -- users must have their own PitchBook subscription and API credentials.

## MCP Server

**Package**: `@corp-finance/vendor-pitchbook`
**Authentication**: Bearer token
**Environment Variables**:
- `PITCHBOOK_API_KEY` -- API key for Bearer token authentication

## Tool Catalogue (14 tools)

| Tool | Purpose |
|------|---------|
| `pb_company_search` | Search for private and public companies by name, sector, geography, or funding stage |
| `pb_company_profile` | Comprehensive company profiles: financials, ownership history, board, employee count |
| `pb_deal_search` | Search PE/VC transactions by date range, deal size, sector, geography, or investor |
| `pb_deal_details` | Detailed deal information: valuation multiples, leverage, structure, and advisor roles |
| `pb_comparable_deals` | Find comparable transactions for valuation benchmarking and precedent analysis |
| `pb_investor_profile` | Investor profiles: fund family, strategy, AUM, track record, and team stability |
| `pb_fund_search` | Search PE/VC funds by vintage, strategy, geography, size, and performance quartile |
| `pb_fund_performance` | Fund performance metrics: IRR, TVPI, DPI, RVPI, PME, and vintage year benchmarks |
| `pb_lp_commitments` | Limited partner commitment data, allocation targets, and pacing models |
| `pb_vc_exits` | VC exit data: IPOs, M&A exits, secondary sales, exit valuations, and holding periods |
| `pb_fundraising` | Fundraising activity: funds in market, target sizes, closes, and dry powder |
| `pb_market_stats` | Market-level statistics: deal flow trends, sector valuations, and exit activity |
| `pb_people_search` | Search for executives, board members, and investors by name or affiliation |
| `pb_service_providers` | Service provider data: legal advisors, auditors, placement agents, and prime brokers |

## Key Capabilities

- **PE Deals**: Private equity transaction data including deal size, valuation multiples, leverage, deal structure, advisor roles via `pb_deal_search`, `pb_deal_details`, and `pb_comparable_deals`
- **VC Transactions**: Venture capital funding rounds with pre/post money valuations, investor participation, round terms via `pb_deal_search` and `pb_vc_exits`
- **Fund Performance**: PE/VC fund IRR, TVPI, DPI, RVPI, vintage year performance, quartile rankings via `pb_fund_performance` and `pb_fund_search`
- **Company Profiles**: Private company profiles with financials, ownership history, board composition via `pb_company_search` and `pb_company_profile`
- **LP Data**: Limited partner commitments, allocation targets, pacing models, co-investment activity via `pb_lp_commitments`
- **Market Intelligence**: Deal flow trends, dry powder analysis, sector valuations, exit activity, fundraising environment via `pb_fundraising` and `pb_market_stats`

## Data Note

PitchBook subscription required. Data subject to PitchBook licence terms, including redistribution and usage restrictions. PitchBook is the primary source for private market deal data and fund performance. Complements the corp-finance-mcp PE (LBO, waterfall, fund economics) and fund-of-funds modules.
