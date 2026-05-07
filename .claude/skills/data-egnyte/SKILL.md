<!-- Phase 29 Wave 5 — local connector skeleton at packages/data-mcp-server/src/egnyte/ -->
---
name: data-egnyte
description: "[PAID — vendor subscription required] Egnyte -- enterprise content management with finserv-grade audit trails. Use when accessing deal rooms, due-diligence document repositories, or LP-investor data rooms hosted on Egnyte."
---

# Egnyte Content Repository

You have access to Egnyte MCP tools (registered locally in our data-mcp-server) for accessing content repositories used in finserv workflows: deal rooms, DD packages, LP investor portals, board package archives.

**Connector**: Local connector at `packages/data-mcp-server/src/egnyte/`
**Authentication**: REST API at `https://<domain>.egnyte.com/pubapi/v1` with bearer-token auth (domain-scoped per tenant)
**Requires**: BOTH `EGNYTE_DOMAIN` (e.g. `acme`) AND `EGNYTE_API_KEY`. Provision via the Egnyte admin panel; see https://developers.egnyte.com.

> **Graceful degradation**: When either env var is unset, all tools return a structured `credentials_required` response — discoverable via `mcp list-tools` but not network-active.

## Tool Reference

| Tool | Description |
|------|-------------|
| `egnyte_list_files` | List files in a folder. Optional recursion. Returns name, size, modified time, content hash. |
| `egnyte_get_file_metadata` | Fetch metadata for a single file: type, size, sha-1, version count, sharing status. |
| `egnyte_search` | Full-text search across content with optional folder-scope and file-type filter. |
| `egnyte_download_file` | Get file bytes (base64) or a temporary download URL. |

## Example Queries

```json
// List a deal room folder
{ "tool": "egnyte_list_files", "params": { "folder_path": "/Shared/DealRoom/AAPL", "limit": 50 } }

// Fetch metadata for a specific CIM
{ "tool": "egnyte_get_file_metadata", "params": { "file_path": "/Shared/DealRoom/AAPL/cim.pdf" } }

// Search for EBITDA analysis documents in the DD folder
{ "tool": "egnyte_search", "params": { "query": "EBITDA bridge", "folder_scope": "/Shared/DealRoom", "file_type": "document", "limit": 10 } }

// Get a download URL for a board package (no content fetch)
{ "tool": "egnyte_download_file", "params": { "file_path": "/Shared/BoardPacks/Q1-2026.pdf", "return_content": false } }
```

## When to use

- Pulling deal-room documents for an M&A or PE workflow
- Surfacing DD package contents for an investment committee memo
- Locating a specific filing or board pack from history
- Auditing document access trails (Egnyte provides server-side audit logs)

## When NOT to use

- Public regulatory filings → use EDGAR (`data-edgar`)
- Vendor research → use vendor MCPs (S&P, LSEG, Morningstar)
- Live earnings call transcripts → use Aiera (`data-aiera`)
- Market data, prices, financials → use FMP (`fmp-market-data`)

## Required env vars

```bash
export EGNYTE_DOMAIN="acme"      # your Egnyte subdomain (acme in acme.egnyte.com)
export EGNYTE_API_KEY="<token>"  # bearer token from admin panel
```

Optional tuning:

```bash
export EGNYTE_RATE_LIMIT="30"    # requests per second (Egnyte default: 30/s per token)
export EGNYTE_CACHE_TTL="60"     # default cache TTL in seconds
```
