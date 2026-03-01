---
name: partner-moodys
description: "Moody's -- credit ratings, fixed income analytics, structured finance data, and economic research via Moody's GenAI-Ready Data MCP"
---

# Moody's GenAI-Ready Data (Partner Integration)

Moody's provides credit ratings, fixed income analytics, structured finance data, and economic research via their GenAI-Ready Data MCP server. This is an OPTIONAL premium integration -- users must have their own Moody's subscription and API credentials.

**MCP Endpoint**: `api.moodys.com/genai-ready-data/m1/mcp`
**Requires**: `MOODYS_API_KEY` environment variable

## Capabilities

| Domain | Description |
|--------|-------------|
| Credit Ratings | Moody's issuer and instrument ratings, outlooks, rating histories, watchlist status, rating actions |
| Default Research | Historical default rates, recovery rates, transition matrices, loss-given-default studies |
| Fixed Income Analytics | Bond pricing, spread analytics, relative value, curve analysis, credit spread decomposition |
| Structured Finance | CMBS/RMBS/ABS/CLO performance data, deal structures, tranche analytics, collateral analysis |
| Economic Research | Moody's Analytics macro forecasts, country risk assessments, scenario-based economic projections |
| ESG & Climate | ESG credit impact scores, carbon transition assessments, physical risk scores, net-zero alignment |
| Financial Metrics | Moody's-adjusted financial ratios, standardised creditworthiness metrics, peer comparison |

## Configuration

Add to your Claude Code MCP configuration:

```json
{
  "mcpServers": {
    "moodys": {
      "url": "https://api.moodys.com/genai-ready-data/m1/mcp",
      "headers": {
        "Authorization": "Bearer ${MOODYS_API_KEY}"
      }
    }
  }
}
```

## Important Notes

- This is a **partner integration**. Tools are provided by the Moody's MCP server, not this codebase.
- You must have an active Moody's subscription to access this data.
- The Moody's MCP server defines its own tool names and schemas. Refer to Moody's developer documentation for the full tool catalogue.
- Data is subject to Moody's licence terms, including redistribution and usage restrictions.
- Moody's is the authoritative source for credit ratings, default studies, and structured finance analytics. Complements the corp-finance-mcp credit scoring and CLO analytics modules.
