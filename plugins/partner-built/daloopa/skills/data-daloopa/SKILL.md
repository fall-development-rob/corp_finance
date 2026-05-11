<!-- Source: plugins/vertical-plugins/financial-analysis/.mcp.json (anthropics/financial-services) -->
---
name: data-daloopa
description: "[PAID — vendor subscription required] Daloopa -- normalized financial data, consensus estimates, actuals via Daloopa MCP. Use when fetching model-ready financials, broker consensus, or fundamental time series."
---

# Daloopa Financial Data

You have access to the Daloopa MCP server for retrieving normalized, model-ready financial data including actuals, broker consensus estimates, and KPI time series. Daloopa maintains a continuously updated database of structured fundamental data sourced from company filings and sell-side research.

**Connector URL**: `https://mcp.daloopa.com/server/mcp`
**Transport**: `http` (streamable HTTP as defined in MCP spec)
**Requires**: `DALOOPA_API_KEY` environment variable. Obtain an API key at https://www.daloopa.com -- contact sales or sign up for a developer account.

> **Status**: Wired. Run `./scripts/register-data-mcp.sh --apply` once `DALOOPA_API_KEY` is set in your environment to register the remote Daloopa MCP server with Claude Code.

## What Daloopa Provides

Daloopa specialises in eliminating the manual work of financial modelling by delivering pre-normalised data that drops directly into Excel or LLM contexts:

- **Actuals**: Reported income statement, balance sheet, and cash flow line items, normalised across companies and periods.
- **Consensus estimates**: Aggregated sell-side forecasts for revenue, EBITDA, EPS, and other KPIs; mean, median, high, low, and number of estimates.
- **KPI time series**: Non-GAAP and segment-level metrics (e.g., same-store sales, unit economics, ARR, bookings) as reported by management.
- **Model-ready output**: Data is pre-mapped to a consistent taxonomy so it can be inserted into financial models without manual reformatting.

## Tool Reference

The Daloopa MCP server is a remote HTTP server; its tool manifest is only discoverable via a live MCP client connection. Based on Daloopa's documented capabilities, the server is expected to expose tools in the following categories. Verify exact tool names by calling `mcp list-tools` after registration.

### Financials

| Expected MCP Tool | Description |
|-------------------|-------------|
| `daloopa_get_financials` | Retrieve normalised actuals (income statement, balance sheet, cash flow) for a company by ticker or identifier, optionally filtered by period and line item. |
| `daloopa_get_kpis` | Fetch non-GAAP and segment KPIs for a company. Useful for SaaS metrics (ARR, NRR, CAC), retail (SSS, comp growth), or any company-reported operating metric. |

### Consensus Estimates

| Expected MCP Tool | Description |
|-------------------|-------------|
| `daloopa_get_consensus` | Retrieve aggregated sell-side consensus estimates (mean, median, high, low, count) for a company and metric across forward periods. |
| `daloopa_get_estimate_revisions` | Get the history of estimate revisions for a metric, useful for tracking earnings revision momentum. |

### Search and Discovery

| Expected MCP Tool | Description |
|-------------------|-------------|
| `daloopa_search_companies` | Search for companies by name, ticker, or sector to resolve identifiers before fetching data. |
| `daloopa_list_metrics` | List available financial metrics and KPIs for a specific company to understand what data Daloopa carries. |

## Example Queries

```
# Retrieve last 8 quarters of revenue and EBITDA actuals for MSFT
daloopa_get_financials(ticker="MSFT", metrics=["revenue", "ebitda"], periods=8)

# Get current consensus EPS estimates for the next 3 fiscal years
daloopa_get_consensus(ticker="AAPL", metric="eps", forward_periods=3)

# Fetch ARR and NRR KPIs for a SaaS company
daloopa_get_kpis(ticker="CRM", kpis=["arr", "nrr"])

# Check estimate revision trend heading into earnings
daloopa_get_estimate_revisions(ticker="NVDA", metric="revenue", lookback_days=60)
```

## Integration Notes

### Equity Research (cfa-equity-analyst + workflow-equity-research)

Daloopa is the primary source for model-ready financial time series in equity research workflows:

- **Initiating coverage** (`/initiate-coverage`): Use `daloopa_get_financials` to pre-populate the historical period table (3-5 years of actuals) before building forward estimates.
- **Earnings analysis** (`/earnings`): Compare reported actuals from Daloopa against the consensus from `daloopa_get_consensus` to compute surprise and revision delta.
- **Model updates** (`workflow-equity-research` model update workflow): Pull the latest actuals after each earnings release to refresh the base year without manual data entry.
- **Thesis tracking** (`/thesis`): Use KPI time series from `daloopa_get_kpis` as observable signals to confirm or refute the investment thesis over time.

### Combining with Other Data Sources

| Use Case | Combine With |
|----------|-------------|
| Validate actuals against filings | `data-edgar` (`edgar_company_concept`) |
| Add market prices and ratios | `fmp-market-data` |
| Overlay macro assumptions | `data-fred` (GDP, rates, CPI) |
| Cross-reference sell-side ratings | `vendor-sp-global` (transcript context) |

### Data Freshness

Daloopa updates actuals typically within hours of a company reporting. Consensus estimates refresh intraday as sell-side models are revised. For time-sensitive workflows (same-day earnings), confirm data availability before citing.

## Usage Notes

- Authentication uses an API key passed as a bearer token. The MCP client sends `Authorization: Bearer <DALOOPA_API_KEY>` on each HTTP request. No env block is defined in `.mcp.json` — the key is injected by the MCP client configuration (see `scripts/register-data-mcp.sh`).
- The server URL `https://mcp.daloopa.com/server/mcp` is confirmed from `plugins/vertical-plugins/financial-analysis/.mcp.json` in the `anthropics/financial-services` repository.
- Daloopa coverage is weighted toward large- and mid-cap US equities; verify coverage for small-cap or international names before relying on it in a workflow.
- Data is normalised but not audited — always cross-reference against primary sources (SEC filings via EDGAR) for numbers used in published research.
