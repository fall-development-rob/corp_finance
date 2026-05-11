<!-- Source: plugins/vertical-plugins/financial-analysis/.mcp.json (anthropics/financial-services) -->
---
name: data-aiera
description: "[PAID — vendor subscription required] Aiera -- earnings call transcripts, event intelligence, company event monitoring via Aiera MCP. Use when researching earnings call commentary, management guidance, or scheduled corporate events."
---

# Aiera Event Intelligence

You have access to the Aiera MCP server for retrieving earnings call transcripts, corporate event schedules, and real-time event intelligence. Aiera specialises in aggregating and processing spoken and written corporate communications, turning live calls and events into searchable, structured text.

**Connector URL**: `https://mcp-pub.aiera.com`
**Transport**: `http` (streamable HTTP as defined in MCP spec)
**Requires**: `AIERA_API_KEY` environment variable. Obtain an API key via the Aiera dashboard at https://dashboard.aiera.com or by contacting support@aiera.com.

> **Status**: Wired. Run `./scripts/register-data-mcp.sh --apply` once `AIERA_API_KEY` is set in your environment to register the remote Aiera MCP server with Claude Code.

## What Aiera Provides

Aiera processes corporate communications -- earnings calls, investor days, conferences, and management roadshows -- and exposes them as structured, searchable data:

- **Earnings call transcripts**: Full verbatim or processed transcripts of quarterly earnings calls, with speaker attribution (CFO, CEO, analyst names).
- **Event scheduling**: Upcoming and historical corporate events (earnings releases, investor days, analyst days, conferences) with date, time, and company metadata.
- **Event monitoring**: Real-time or near-real-time access to live call content as events unfold.
- **Management guidance**: Parsed forward-looking statements and guidance commentary extracted from call transcripts.
- **Sentiment and tone**: Event-level sentiment signals derived from call content (where available via the API).

## Tool Reference

The Aiera MCP server is a remote HTTP server; its full tool manifest is only discoverable via a live MCP client connection. Based on Aiera's documented platform capabilities (REST API at `rest.aiera.com`), the server is expected to expose tools in the following categories. Verify exact tool names by calling `mcp list-tools` after registration.

### Transcripts

| Expected MCP Tool | Description |
|-------------------|-------------|
| `aiera_get_transcript` | Retrieve the full transcript for a specific earnings call or corporate event by event ID. Returns speaker-attributed text blocks. |
| `aiera_search_transcripts` | Full-text search across all transcripts for a company or query string. Returns matching excerpts with event metadata. |
| `aiera_get_transcript_segments` | Retrieve specific segments (e.g., Q&A section, prepared remarks) of a transcript rather than the full document. |

### Events

| Expected MCP Tool | Description |
|-------------------|-------------|
| `aiera_list_events` | List upcoming or historical corporate events for a company or set of companies. Filterable by event type (earnings, investor day, conference) and date range. |
| `aiera_get_event` | Get metadata and status for a single event by ID: scheduled time, actual start, participants, webcast link. |
| `aiera_get_event_audio` | Retrieve audio stream or recording reference for an event (where available and licensed). |

### Search and Discovery

| Expected MCP Tool | Description |
|-------------------|-------------|
| `aiera_search_companies` | Resolve a company name or ticker to an Aiera company ID for use in other tool calls. |
| `aiera_get_watchlist` | Retrieve events for a pre-configured watchlist of companies. Useful for monitoring a coverage universe. |

## Example Queries

```
# Get the most recent earnings call transcript for Microsoft
aiera_list_events(ticker="MSFT", event_type="earnings", limit=1)
aiera_get_transcript(event_id="<id from above>")

# Search all NVDA transcripts for mentions of "data center" guidance
aiera_search_transcripts(ticker="NVDA", query="data center guidance", date_from="2024-01-01")

# List all earnings events scheduled for the next 2 weeks across a coverage list
aiera_list_events(tickers=["AAPL","MSFT","GOOGL","AMZN"], event_type="earnings", date_from="today", date_to="+14d")

# Retrieve just the Q&A section of a specific call
aiera_get_transcript_segments(event_id="<id>", segment="qa")
```

## Integration Notes

### Equity Research (cfa-equity-analyst + workflow-equity-research)

Aiera is the primary source for qualitative management communication context in equity research workflows:

- **Earnings analysis** (`/earnings`): Pull the earnings call transcript immediately after results are released. Extract management commentary on guidance, margin outlook, and demand signals to augment the quantitative earnings surprise calculation.
- **Initiating coverage** (`/initiate-coverage`): Review the last 4-8 quarters of earnings transcripts to build a picture of management credibility, guidance track record, and strategic narrative consistency.
- **Morning note** (`/morning-note`): Use `aiera_list_events` to populate the day's earnings calendar and flag pre-market or after-close calls for monitoring.
- **Thesis tracking** (`/thesis`): Monitor transcript search results for thesis-confirming or thesis-threatening quotes over time (e.g., track mentions of pricing power, competitive dynamics, or capex language quarter over quarter).
- **Model updates** (`workflow-equity-research` model update workflow): Cross-reference management guidance language in the transcript against the revised consensus figures pulled from `data-daloopa`.

### Combining with Other Data Sources

| Use Case | Combine With |
|----------|-------------|
| Correlate commentary with reported numbers | `data-daloopa` (actuals + consensus) |
| Validate guidance against SEC filings | `data-edgar` (10-Q MD&A section) |
| Map sentiment to price reaction | `fmp-market-data` (historical prices) |
| Check for concurrent institutional moves | `fmp-sec-compliance` (13F filings) |
| Overlay macro commentary | `data-fred` (rates, CPI context) |

### Conference and Investor Day Coverage

Aiera covers events beyond quarterly earnings, including investor days, analyst days, and sector conferences. Use `aiera_list_events` with `event_type="investor_day"` or `event_type="conference"` to monitor non-earnings corporate communications that often contain the most candid forward guidance.

## Usage Notes

- Authentication uses an API key passed as a bearer token. The MCP client sends `Authorization: Bearer <AIERA_API_KEY>` on each HTTP request. No env block is defined in `.mcp.json` -- the key is injected by the MCP client configuration (see `scripts/register-data-mcp.sh`).
- The server URL `https://mcp-pub.aiera.com` is confirmed from `plugins/vertical-plugins/financial-analysis/.mcp.json` in the `anthropics/financial-services` repository.
- Transcript availability depends on Aiera's coverage universe and licensing. Large-cap US equities have near-complete coverage; small-cap and international names may have gaps.
- Live event monitoring (real-time call access) may require a higher-tier subscription than standard API access. Confirm entitlements with Aiera before building real-time workflows.
- Speaker attribution quality varies by call audio quality; always verify attributed quotes against the original webcast if using for published research.
