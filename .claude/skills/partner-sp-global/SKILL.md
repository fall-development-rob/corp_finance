---
name: partner-sp-global
description: "S&P Global (Kensho) -- Capital IQ financials, earnings call transcripts, company tearsheets, and funding digests via S&P Global MCP"
---

# S&P Global / Kensho (Partner Integration)

S&P Global provides institutional financial data via their Kensho-powered MCP server. Access Capital IQ financials, earnings call transcripts, company tearsheets, and funding digests. This is an OPTIONAL premium integration -- users must have their own S&P Global subscription.

**MCP Endpoint**: `kfinance.kensho.com/integrations/mcp`
**Requires**: `SP_GLOBAL_API_KEY` environment variable

## Capabilities

| Domain | Description |
|--------|-------------|
| Capital IQ Financials | Detailed financial statements, standardised metrics, segment data, historical financials with Capital IQ quality |
| Earnings Transcripts | Full-text earnings call transcripts with speaker attribution, Q&A sections, management guidance extraction |
| Company Tearsheets | Comprehensive company profiles: business description, key metrics, ownership, credit ratings, ESG scores |
| Funding Digests | Private funding rounds, M&A transactions, IPO data, investment activity tracking |
| Credit Ratings | S&P credit ratings, outlooks, and rating histories for corporates and sovereigns |
| Industry Analysis | Industry-level financial benchmarks, market sizing, competitive landscape data |

## Configuration

Add to your Claude Code MCP configuration:

```json
{
  "mcpServers": {
    "sp-global": {
      "url": "https://kfinance.kensho.com/integrations/mcp",
      "headers": {
        "Authorization": "Bearer ${SP_GLOBAL_API_KEY}"
      }
    }
  }
}
```

## Important Notes

- This is a **partner integration**. Tools are provided by the S&P Global MCP server, not this codebase.
- You must have an active S&P Global / Capital IQ subscription to access this data.
- The S&P Global MCP server defines its own tool names and schemas. Refer to S&P Global documentation for the full tool catalogue.
- Data is subject to S&P Global licence terms, including redistribution and usage restrictions.
- Particularly valuable for earnings transcript analysis and standardised financial data that complements SEC EDGAR raw filings.
