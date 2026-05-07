<!-- Phase 29 Wave 5 — local connector skeleton at packages/data-mcp-server/src/chronograph/ -->
---
name: data-chronograph
description: "[PAID — vendor subscription required] Chronograph -- PE / VC portfolio monitoring, capital account tracking, portco KPI roll-up. Use when monitoring a fund's portfolio companies, generating LP capital account statements, or pulling KPI time series for portco diligence."
---

# Chronograph Portfolio Monitoring

You have access to the Chronograph MCP tools (registered locally in our data-mcp-server) for PE / VC portfolio monitoring. Chronograph aggregates portfolio company financials and operational KPIs and exposes them as structured data for LP reporting and portfolio analytics.

**Connector**: Local connector at `packages/data-mcp-server/src/chronograph/`
**Authentication**: REST API at `https://api.chronograph.pe/v1` with bearer-token auth
**Requires**: `CHRONOGRAPH_API_KEY` environment variable. Sign up or request access at https://chronograph.pe.

> **Graceful degradation**: When `CHRONOGRAPH_API_KEY` is unset, all tools return a structured `credentials_required` response — they don't crash, and they're discoverable via `mcp list-tools`. Set the env var to enable real API calls.

## Tool Reference

| Tool | Description |
|------|-------------|
| `chronograph_list_portfolios` | List portfolios / funds under management. Filter by status (active / realized / all) and fund id. |
| `chronograph_get_company` | Fetch portco metadata and latest metrics by company id. |
| `chronograph_capital_account` | LP capital account statement (commitments, contributions, distributions, NAV) — fund or per-LP. |
| `chronograph_kpi_pull` | Pull KPI time series for a portco (e.g. ARR, CAC, gross margin) over a period range. |

## Example Queries

```
# List active portfolio companies across all funds
chronograph_list_portfolios(status="active", limit=50)

# Get full profile and metrics for a specific portco
chronograph_get_company(company_id="portco-abc123", include_metrics=true)

# LP capital account statement as of Q4 2025 for a specific LP
chronograph_capital_account(fund_id="fund-xyz789", lp_id="lp-001", as_of_date="2025-12-31")

# Pull ARR and gross margin time series for a portco over 2025
chronograph_kpi_pull(company_id="portco-abc123", metric_codes=["arr","gross_margin"], period_start="2025-01-01", period_end="2025-12-31")
```

## When to use

- LP capital account roll-up at quarter-end
- Portfolio company KPI surveillance (ARR, CAC, NRR, headcount)
- Cross-fund portfolio reporting and fund-level analytics
- Portco diligence — pulling historical KPI trends before add-on acquisition

## When NOT to use

- Public market equity research → use FMP / Yahoo Finance / EDGAR connectors
- Fixed income / derivatives research → use vendor MCPs (LSEG, S&P)
- Macro / economic data → use FRED or World Bank connectors
- Earnings call transcripts → use Aiera connector
