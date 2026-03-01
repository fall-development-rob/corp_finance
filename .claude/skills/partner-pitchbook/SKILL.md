---
name: partner-pitchbook
description: "PitchBook -- private equity deal data, VC transactions, fund performance, and company profiles for private market research via PitchBook MCP"
---

# PitchBook (Partner Integration)

PitchBook provides private equity deal data, VC transactions, fund performance, and company profiles via their MCP server. This is an OPTIONAL premium integration -- users must have their own PitchBook subscription and API credentials.

**MCP Endpoint**: `premium.mcp.pitchbook.com/mcp`
**Requires**: `PITCHBOOK_API_KEY` environment variable

## Capabilities

| Domain | Description |
|--------|-------------|
| PE Deals | Private equity transaction data: deal size, valuation multiples, leverage, deal structure, advisor roles |
| VC Transactions | Venture capital funding rounds: pre/post money valuations, investor participation, round terms, stage progression |
| Fund Performance | PE/VC fund IRR, TVPI, DPI, RVPI, vintage year performance, quartile rankings, J-curve analysis |
| Company Profiles | Private company profiles: financials, ownership history, board composition, employee count, comparable transactions |
| LP Data | Limited partner commitments, allocation targets, pacing models, co-investment activity |
| Fund Manager Research | GP track records, fund family performance, team stability, strategy evolution, AUM history |
| Market Intelligence | Deal flow trends, dry powder analysis, sector valuations, exit activity, fundraising environment |

## Configuration

Add to your Claude Code MCP configuration:

```json
{
  "mcpServers": {
    "pitchbook": {
      "url": "https://premium.mcp.pitchbook.com/mcp",
      "headers": {
        "Authorization": "Bearer ${PITCHBOOK_API_KEY}"
      }
    }
  }
}
```

## Important Notes

- This is a **partner integration**. Tools are provided by the PitchBook MCP server, not this codebase.
- You must have an active PitchBook subscription to access this data.
- The PitchBook MCP server defines its own tool names and schemas. Refer to PitchBook developer documentation for the full tool catalogue.
- Data is subject to PitchBook licence terms, including redistribution and usage restrictions.
- PitchBook is the primary source for private market deal data and fund performance. Complements the corp-finance-mcp PE (LBO, waterfall, fund economics) and fund-of-funds modules.
