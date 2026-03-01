---
name: data-edgar
description: "SEC EDGAR -- structured XBRL company financial facts, SEC filings (10-K, 10-Q, 8-K), full-text filing search, and CIK/ticker resolution from the SEC directly"
---

# SEC EDGAR Data

You have access to 20 EDGAR MCP tools for retrieving structured financial data, SEC filings, and company identifiers directly from the SEC. No API key needed -- uses a User-Agent header for identification.

## Tool Reference

### XBRL / Company Facts (5 tools)

| MCP Tool | Description |
|----------|-------------|
| `edgar_company_facts` | Get ALL XBRL facts for a company by CIK -- every financial concept across all filings. Comprehensive structured financial data. |
| `edgar_company_concept` | Get a single XBRL concept for one company over time (e.g., Revenue history for AAPL). Ideal for time-series of a specific metric. |
| `edgar_frames` | Cross-sectional XBRL frame: one concept across ALL companies for a single period. Great for peer comparison and screening. |
| `edgar_concept` | Aggregated data for a single XBRL concept across all companies for a year (alias for frames with USD unit). |
| `edgar_xbrl_tags` | List available XBRL tags/concepts for a company within a taxonomy. Discover what a company reports before querying. |

### Filings and Submissions (6 tools)

| MCP Tool | Description |
|----------|-------------|
| `edgar_submissions` | Full submission history for a company: metadata (name, SIC, tickers, exchanges) plus all recent filings. |
| `edgar_filings` | Get filings filtered by form type (10-K, 10-Q, 8-K, etc.). Returns dates, accession numbers, document links. |
| `edgar_filing_by_accession` | Get a specific filing by CIK and accession number. Returns the filing index with all documents. |
| `edgar_recent_filings` | Get the most recent N filings for a company regardless of form type. |
| `edgar_company_tickers` | Complete SEC ticker-to-CIK mapping for all companies. |
| `edgar_cik_lookup` | Look up a CIK number from a ticker symbol. |

### Full-Text Search (3 tools)

| MCP Tool | Description |
|----------|-------------|
| `edgar_full_text_search` | Search across all SEC filings by keyword. Filter by form type and date range. Returns matching filings with excerpts. |
| `edgar_efts_search` | Advanced full-text search with exact phrase matching, date ranges, and form type filtering. |
| `edgar_document_content` | Fetch the raw content of a specific SEC filing document by URL. |

### Identifier Resolution (6 tools)

| MCP Tool | Description |
|----------|-------------|
| `edgar_cik_from_ticker` | Resolve a stock ticker to its SEC CIK number. Essential for using other EDGAR tools. |
| `edgar_ticker_from_cik` | Reverse lookup: CIK to ticker symbol. |
| `edgar_company_search` | Search for SEC-registered companies by name with partial matching. |
| `edgar_sic_lookup` | Find companies by SIC (Standard Industrial Classification) code. |
| `edgar_mutual_fund_search` | Search for mutual funds by name or ticker in SEC filings. |
| `edgar_series_search` | Search for investment company series and classes (mutual funds, ETFs). |

## Common XBRL Concepts

| Taxonomy | Concept | Description |
|----------|---------|-------------|
| `us-gaap` | `Revenues` | Total revenue |
| `us-gaap` | `NetIncomeLoss` | Net income |
| `us-gaap` | `Assets` | Total assets |
| `us-gaap` | `EarningsPerShareBasic` | Basic EPS |
| `us-gaap` | `OperatingIncomeLoss` | Operating income |
| `us-gaap` | `LongTermDebt` | Long-term debt |

## Usage Notes

- Most EDGAR tools require a CIK number. Use `edgar_cik_from_ticker` to resolve tickers first.
- CIK numbers are zero-padded to 10 digits (e.g., `0000320193` for Apple).
- XBRL company facts (`edgar_company_facts`) returns a large payload with all financial data -- use `edgar_company_concept` for targeted queries.
- `edgar_frames` is uniquely powerful for screening: get one metric for every public company in one call.
- No API key required. The server uses a User-Agent header per SEC fair-access guidelines.
- Full-text search (EFTS) covers the actual text content of filed documents, not just metadata.
