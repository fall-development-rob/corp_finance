---
name: data-figi
description: "OpenFIGI -- financial instrument identifier cross-referencing (ISIN, CUSIP, SEDOL, ticker to FIGI mapping) for multi-source data normalisation"
---

# OpenFIGI Identifier Mapping

You have access to 8 FIGI MCP tools for cross-referencing financial instrument identifiers. The Financial Instrument Global Identifier (FIGI) system maps between ISINs, CUSIPs, SEDOLs, tickers, and exchange codes.

**Auth**: `OPENFIGI_API_KEY` environment variable is optional. Without a key, rate limits are lower (20 requests/min vs 250/min).

## Tool Reference

### Identifier Mapping (4 tools)

| MCP Tool | Description |
|----------|-------------|
| `figi_map` | Map a single identifier (ISIN, CUSIP, SEDOL, TICKER, etc.) to its FIGI and instrument data. Specify `idType` and `idValue`. Optionally filter by exchange/MIC/currency. |
| `figi_bulk_map` | Batch-map up to 100 identifiers to FIGIs in one request. Each job specifies idType, idValue, and optional filters. Returns array of result sets. |
| `figi_isin_to_ticker` | Convenience: map an ISIN directly to its FIGI, ticker, and exchange details. |
| `figi_cusip_to_ticker` | Convenience: map a CUSIP directly to its FIGI, ticker, and exchange details. |

### Search and Discovery (4 tools)

| MCP Tool | Description |
|----------|-------------|
| `figi_search` | Search by keyword (company name, partial ticker). Returns matching FIGI records with instrument details. |
| `figi_filter` | Filter securities by attributes: exchange code, MIC code, currency, security type, market sector, or ticker. |
| `figi_enumerations` | Get valid values for mapping attributes (exchCode, micCode, securityType, securityType2, marketSector, currency). |
| `figi_security_info` | Look up a specific FIGI identifier to get full instrument details. |

## Supported Identifier Types

| `idType` Value | Description |
|----------------|-------------|
| `ID_ISIN` | International Securities Identification Number |
| `ID_CUSIP` | Committee on Uniform Securities Identification Procedures |
| `ID_SEDOL` | Stock Exchange Daily Official List |
| `TICKER` | Exchange ticker symbol |
| `ID_BB_GLOBAL` | Bloomberg Global Identifier (FIGI) |
| `ID_WERTPAPIER` | German securities identifier |
| `COMPOSITE_FIGI` | Composite Financial Instrument Global Identifier |

## Usage Notes

- **Primary use case**: resolving identifiers across data sources. For example, converting an ISIN from a fixed income dataset into a ticker for FMP or Yahoo Finance lookup.
- Use `figi_bulk_map` for batch lookups (up to 100 per request) to minimise API calls.
- The `figi_isin_to_ticker` and `figi_cusip_to_ticker` tools are convenience wrappers for the most common conversion patterns.
- Use `figi_enumerations` to discover valid filter values before calling `figi_filter`.
- Results include the FIGI ID, ticker, exchange code, security type, market sector, and instrument name.
- Without an API key, the server is limited to 20 mapping requests per minute.
