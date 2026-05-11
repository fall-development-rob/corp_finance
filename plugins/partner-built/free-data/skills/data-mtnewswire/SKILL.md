<!-- Source: plugins/vertical-plugins/financial-analysis/.mcp.json (anthropics/financial-services) -->
---
name: data-mtnewswire
description: "[FREE — no API key] Midnight Trader Newswire (mtnewswire) -- real-time financial news headlines, ticker-tagged news, market-moving stories via the public mtnewswires MCP at vast-mcp.blueskyapi.com. No auth required."
---

# mtnewswire (Midnight Trader Newswire)

You have access to the mtnewswire MCP server for real-time financial news monitoring. mtnewswire aggregates breaking financial news, market-moving headlines, and ticker-tagged news items, exposed as a remote MCP server.

**Connector URL**: `https://vast-mcp.blueskyapi.com/mtnewswires`
**Transport**: `http` (streamable HTTP as defined in MCP spec)
**Requires**: none — public MCP, no API key or env var required.

This skill follows the same direct-connector pattern as `data-aiera`, `vendor-lseg`, and the other remote-MCP skills: cookbooks declare the URL in their `mcp_servers` list and the runtime connects over MCP JSON-RPC. There is no local proxy in `packages/data-mcp-server/`; the upstream service speaks MCP natively.

## How to Wire Into a Cookbook

Add the connector to the cookbook's `agent.json` (orchestrator or subagent that needs news access):

```json
"mcp_servers": [
  { "type": "http", "name": "mtnewswire", "url": "https://vast-mcp.blueskyapi.com/mtnewswires" }
]
```

Then enable the MCP toolset on the consuming subagent:

```json
{
  "type": "mcp_toolset",
  "mcp_server_name": "mtnewswire",
  "default_config": { "enabled": true }
}
```

Or gate per-tool with `default_config: { enabled: false }` plus an explicit `configs[]` allowlist (see `managed-agent-cookbooks/equity-analyst/subagents/analyst.json` for the gating pattern).

## Tool Reference

The mtnewswire MCP server is remote; the authoritative tool manifest is discoverable via a live MCP client connection. Tool names below reflect the upstream's published surface and conform to MCP naming conventions (`mtnewswire_*`). Verify exact names with `mcp list-tools` after registration.

| Expected MCP Tool | Description |
|-------------------|-------------|
| `mtnewswire_latest` | Recent headlines feed. Optional category filter (earnings, macro, M&A, regulatory). Returns timestamp, headline, body, tickers, source. |
| `mtnewswire_search` | Keyword/phrase search across news bodies and headlines. Use for thematic monitoring (e.g. "rate cut", "guidance raise") and thesis tracking. |
| `mtnewswire_by_ticker` | Recent news tagged to a specific equity ticker. Use for single-name coverage, earnings drift, and event-driven analysis. |

## When to Invoke

- **Real-time news monitoring**: `mtnewswire_latest` for the morning-note feed and intraday breaking-news scan.
- **Thematic / thesis tracking**: `mtnewswire_search` over recurring terms (supply-chain commentary, FX-headwind callouts, buyback authorisations).
- **Ticker-specific event coverage**: `mtnewswire_by_ticker` when initiating coverage, drafting an earnings preview, or watching a position around a catalyst.
- **Sentiment overlay**: pair with `fmp-news-intelligence` for cross-source confirmation and `data-fred` for macro context.

## Cookbooks That Should Wire This Connector

- `earnings-reviewer` — pre/post-call colour, breaking guidance changes
- `sector-research` — sector-level news flow during the morning-note window
- `pitch-deck-builder` — recent target-company news for the situational slide
- `lseg-rates-monitor` — macro and central-bank headlines alongside curve moves
- `sp-credit-research` — credit-event news flow on covered issuers

## Combining With Other Sources

| Use Case | Combine With |
|----------|--------------|
| Cross-confirm a breaking story | `fmp-news-intelligence` |
| Map news to price reaction | `fmp-market-data` (intraday quotes) |
| Validate against filings | `data-edgar` (8-K, S-1) |
| Earnings-call colour | `data-aiera` (transcripts) |
| Macro overlay | `data-fred` (rates, CPI) |

## Operational Notes

- No auth, no env var. Cookbooks deploy without secrets injection.
- Latency: mtnewswire is a low-latency real-time feed. Don't cache aggressively in the consuming subagent.
- Failure mode: if the upstream is unreachable the MCP connection itself will fail; the runtime surfaces this to the orchestrator as a tool-call error, not a silent stale read.
