---
name: partner-moodys
description: "Moody's -- credit ratings, fixed income analytics, structured finance data, and economic research via Moody's GenAI-Ready Data MCP"
---

# Moody's GenAI-Ready Data (Partner Integration)

Moody's provides credit ratings, fixed income analytics, structured finance data, and economic research. This is an OPTIONAL premium integration -- users must have their own Moody's subscription and API credentials.

## MCP Server

**Package**: `@corp-finance/partner-moodys`
**Authentication**: OAuth2 (client credentials flow)
**Environment Variables**:
- `MOODYS_CLIENT_ID` -- OAuth2 client ID
- `MOODYS_CLIENT_SECRET` -- OAuth2 client secret

## Tool Catalogue (14 tools)

| Tool | Purpose |
|------|---------|
| `moodys_credit_rating` | Moody's issuer and instrument credit ratings, outlooks, and watchlist status |
| `moodys_rating_history` | Historical rating actions, upgrades, downgrades, and rating trajectories |
| `moodys_issuer_profile` | Issuer profiles: business description, sector, geography, and key credit metrics |
| `moodys_default_rates` | Historical default rates by rating, cohort, and time horizon |
| `moodys_recovery_rates` | Recovery rate data by seniority, collateral type, and industry |
| `moodys_transition_matrix` | Rating transition matrices for credit migration analysis |
| `moodys_economic_forecast` | Moody's Analytics macro forecasts, baseline and alternative scenarios |
| `moodys_country_risk` | Sovereign risk assessments, country ceilings, and political risk scores |
| `moodys_industry_outlook` | Sector-level credit outlooks, rating distribution, and trend analysis |
| `moodys_esg_score` | ESG credit impact scores, carbon transition assessments, and net-zero alignment |
| `moodys_climate_risk` | Physical and transition climate risk scores, scenario-based projections |
| `moodys_structured_finance` | CMBS/RMBS/ABS/CLO performance data, deal structures, and tranche analytics |
| `moodys_municipal_score` | Municipal credit scores, GO/revenue bond analysis, and state-level assessments |
| `moodys_company_financials` | Moody's-adjusted financial ratios, standardised creditworthiness metrics |

## Key Capabilities

- **Credit Ratings**: Moody's issuer and instrument ratings, outlooks, watchlist status, rating actions via `moodys_credit_rating` and `moodys_rating_history`
- **Default Research**: Historical default rates, recovery rates, transition matrices, loss-given-default studies via `moodys_default_rates`, `moodys_recovery_rates`, and `moodys_transition_matrix`
- **Structured Finance**: CMBS/RMBS/ABS/CLO performance data, deal structures, tranche analytics via `moodys_structured_finance`
- **Economic Research**: Moody's Analytics macro forecasts, country risk assessments, scenario-based economic projections via `moodys_economic_forecast` and `moodys_country_risk`
- **ESG and Climate**: ESG credit impact scores, carbon transition assessments, physical risk scores, net-zero alignment via `moodys_esg_score` and `moodys_climate_risk`
- **Municipal Credit**: Municipal credit scores, GO/revenue bond analysis via `moodys_municipal_score`

## Data Note

Moody's subscription required. Data subject to Moody's licence terms, including redistribution and usage restrictions. Moody's is the authoritative source for credit ratings, default studies, and structured finance analytics. Complements the corp-finance-mcp credit scoring and CLO analytics modules.
