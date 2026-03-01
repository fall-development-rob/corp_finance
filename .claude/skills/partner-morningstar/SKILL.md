---
name: partner-morningstar
description: "Morningstar -- fund ratings, investment research, ESG data, and portfolio analytics via Morningstar MCP"
---

# Morningstar (Partner Integration)

Morningstar provides fund ratings, investment research, ESG data, and portfolio analytics via their MCP server. This is an OPTIONAL premium integration -- users must have their own Morningstar subscription and API credentials.

**MCP Endpoint**: `mcp.morningstar.com/mcp`
**Requires**: `MORNINGSTAR_API_KEY` environment variable

## Capabilities

| Domain | Description |
|--------|-------------|
| Fund Ratings | Morningstar star ratings, analyst ratings (Gold/Silver/Bronze), category rankings, risk-adjusted returns |
| Fund Data | NAV, expense ratios, holdings, sector/geography allocation, style box, performance attribution |
| ETF Analytics | ETF cost analysis, tracking error, premium/discount, flows, replication method assessment |
| Investment Research | Analyst reports, fair value estimates, economic moat ratings, uncertainty ratings, capital allocation scores |
| ESG | Sustainalytics ESG Risk Rating, carbon metrics, controversy flags, portfolio-level ESG scoring |
| Portfolio Analytics | X-ray analysis, overlap detection, asset allocation optimisation, fee impact analysis |
| Manager Research | Fund manager tenure, track record analysis, stewardship ratings |

## Configuration

Add to your Claude Code MCP configuration:

```json
{
  "mcpServers": {
    "morningstar": {
      "url": "https://mcp.morningstar.com/mcp",
      "headers": {
        "Authorization": "Bearer ${MORNINGSTAR_API_KEY}"
      }
    }
  }
}
```

## Important Notes

- This is a **partner integration**. Tools are provided by the Morningstar MCP server, not this codebase.
- You must have an active Morningstar subscription to access this data.
- The Morningstar MCP server defines its own tool names and schemas. Refer to Morningstar developer documentation for the full tool catalogue.
- Data is subject to Morningstar licence terms, including redistribution and usage restrictions.
- Morningstar is particularly valuable for mutual fund and ETF analysis, ESG scoring (via Sustainalytics), and fair value research.
