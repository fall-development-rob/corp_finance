---
name: partner-lseg
description: "LSEG (London Stock Exchange Group) -- bond pricing, yield curve analysis, FX carry trade evaluation, options valuation, macro dashboard via LSEG Financial Analytics MCP"
---

# LSEG Financial Analytics (Partner Integration)

LSEG (London Stock Exchange Group), the successor to Refinitiv Eikon, provides institutional-grade financial data and analytics via their Financial Analytics MCP server. This is an OPTIONAL premium integration -- users must have their own LSEG subscription and API credentials.

**MCP Endpoint**: `api.analytics.lseg.com/lfa/mcp`
**Requires**: `LSEG_API_KEY` environment variable

## Capabilities

| Domain | Description |
|--------|-------------|
| Fixed Income | Bond pricing, credit spread analysis, yield-to-maturity calculations, duration/convexity, benchmark spread decomposition |
| Yield Curves | Government and swap curve construction, curve interpolation, term structure analysis, historical curve comparison |
| FX Analytics | Carry trade evaluation, forward rate implied yield differentials, cross-currency basis swap analysis, REER modelling |
| Options | Equity and FX options valuation, implied volatility surfaces, Greeks calculation, strategy payoff analysis |
| Macro Dashboard | Economic indicator tracking, central bank rate expectations, inflation breakevens, PMI aggregation |
| Reference Data | Security master, corporate actions, dividend forecasts, index constituents |

## Configuration

Add to your Claude Code MCP configuration:

```json
{
  "mcpServers": {
    "lseg": {
      "url": "https://api.analytics.lseg.com/lfa/mcp",
      "headers": {
        "Authorization": "Bearer ${LSEG_API_KEY}"
      }
    }
  }
}
```

## Important Notes

- This is a **partner integration**. Tools are provided by the LSEG MCP server, not this codebase.
- You must have an active LSEG subscription to access this data.
- The LSEG MCP server defines its own tool names and schemas. Refer to LSEG documentation for the full tool catalogue.
- LSEG data is subject to the terms of your LSEG licence agreement, including redistribution restrictions.
- For fixed income and FX analytics, LSEG provides institutional-quality pricing that complements the corp-finance-mcp calculation engine.
