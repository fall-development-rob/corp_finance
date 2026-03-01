---
name: partner-factset
description: "FactSet -- multi-asset financial data, analytics, and portfolio tools via FactSet MCP"
---

# FactSet (Partner Integration)

FactSet provides multi-asset financial data, analytics, and portfolio tools via their MCP server. This is an OPTIONAL premium integration -- users must have their own FactSet subscription and API credentials.

**MCP Endpoint**: `mcp.factset.com/mcp`
**Requires**: `FACTSET_API_KEY` environment variable

## Capabilities

| Domain | Description |
|--------|-------------|
| Company Fundamentals | Standardised financial statements, ratios, estimates, and actuals across 70,000+ public companies |
| Estimates & Consensus | Broker estimates, consensus forecasts, estimate revisions, earnings surprise history |
| Ownership | Institutional ownership, 13F holdings, activist positions, insider transactions |
| Pricing & Reference | Global equity/fixed income/FX pricing, corporate actions, security master, index constituents |
| Portfolio Analytics | Attribution analysis, risk decomposition, factor exposure, compliance monitoring |
| Quantitative | Factor libraries, screening, time-series analytics, cross-sectional regression tools |
| Fixed Income | Bond pricing, spread analytics, yield curve data, credit analytics |
| Private Markets | PE fund performance, deal data, fundraising, portfolio company information |

## Configuration

Add to your Claude Code MCP configuration:

```json
{
  "mcpServers": {
    "factset": {
      "url": "https://mcp.factset.com/mcp",
      "headers": {
        "Authorization": "Bearer ${FACTSET_API_KEY}"
      }
    }
  }
}
```

## Important Notes

- This is a **partner integration**. Tools are provided by the FactSet MCP server, not this codebase.
- You must have an active FactSet subscription to access this data.
- The FactSet MCP server defines its own tool names and schemas. Refer to FactSet developer documentation for the full tool catalogue.
- Data is subject to FactSet licence terms, including redistribution and usage restrictions.
- FactSet excels at consensus estimates, institutional ownership, and multi-asset portfolio analytics.
