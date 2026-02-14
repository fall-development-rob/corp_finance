---
name: "FMP SEC & Compliance"
description: "Use the fmp-mcp-server SEC filing, insider trading, and institutional ownership tools for regulatory compliance analysis, insider activity monitoring, and 13F ownership tracking. Invoke when researching SEC filings, monitoring insider buying/selling, tracking institutional positions, or performing compliance due diligence."
---

# FMP SEC & Compliance Skill

This skill covers 26 tools from the `fmp-mcp-server` organized into three categories: SEC Filings, Insider Trading, and Institutional Ownership / 13F. Use these tools to perform regulatory compliance analysis, monitor insider activity, track institutional positions, and conduct due diligence on public companies.

---

## SEC Filings (12 tools)

These tools query SEC EDGAR data for company filings, registrant lookups, and industry classification codes.

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_sec_filings_8k` | Latest 8-K material event filings | from, to, page, limit |
| `fmp_sec_filings_financials` | Latest financial filings (10-K, 10-Q) | from, to, page, limit |
| `fmp_sec_filings_by_form` | Search filings by form type | formType, from, to |
| `fmp_sec_filings_by_symbol` | SEC filings for a company | symbol, from, to |
| `fmp_sec_filings_by_cik` | SEC filings by CIK number | cik, from, to |
| `fmp_sec_company_search_name` | Search SEC-registered companies by name | company |
| `fmp_sec_company_search_symbol` | SEC company lookup by ticker | symbol |
| `fmp_sec_company_search_cik` | SEC company lookup by CIK | cik |
| `fmp_sec_profile` | Full SEC company profile | symbol |
| `fmp_sic_list` | SIC industry classification codes | (none) |
| `fmp_sic_search` | Search SIC classifications | (none) |
| `fmp_all_sic` | Complete SIC classification list | (none) |

### SEC Filings — Tool Details

#### fmp_sec_filings_8k

Retrieves the latest 8-K filings, which disclose material events such as executive changes, M&A activity, earnings pre-releases, and other significant corporate developments.

- **Parameters:** `from` (date), `to` (date), `page` (int), `limit` (int)
- **Use when:** Scanning for recent material events across the market or within a date window.

#### fmp_sec_filings_financials

Retrieves the latest financial statement filings including 10-K annual reports and 10-Q quarterly reports.

- **Parameters:** `from` (date), `to` (date), `page` (int), `limit` (int)
- **Use when:** Monitoring newly filed annual or quarterly financial statements.

#### fmp_sec_filings_by_form

Searches SEC filings filtered by a specific form type (e.g., 8-K, 10-K, 10-Q, DEF 14A, S-1, 13F-HR).

- **Parameters:** `formType` (string), `from` (date), `to` (date)
- **Use when:** Looking for a specific type of filing across all registrants within a date range.

#### fmp_sec_filings_by_symbol

Retrieves all SEC filings for a given company by its ticker symbol.

- **Parameters:** `symbol` (string), `from` (date), `to` (date)
- **Use when:** Reviewing the full filing history of a specific company.

#### fmp_sec_filings_by_cik

Retrieves SEC filings using the company's Central Index Key (CIK) number.

- **Parameters:** `cik` (string), `from` (date), `to` (date)
- **Use when:** Querying filings for entities where the CIK is known but the ticker may be ambiguous or unavailable.

#### fmp_sec_company_search_name

Searches for SEC-registered companies by name.

- **Parameters:** `company` (string)
- **Use when:** Finding the CIK or ticker for a company when only the corporate name is known.

#### fmp_sec_company_search_symbol

Looks up an SEC-registered company by its ticker symbol.

- **Parameters:** `symbol` (string)
- **Use when:** Confirming SEC registration details and CIK for a known ticker.

#### fmp_sec_company_search_cik

Looks up an SEC-registered company by its CIK number.

- **Parameters:** `cik` (string)
- **Use when:** Resolving a CIK to a company name and ticker.

#### fmp_sec_profile

Returns the full SEC company profile including SIC code, state of incorporation, fiscal year end, business address, and filing metadata.

- **Parameters:** `symbol` (string)
- **Use when:** Performing due diligence on a company's SEC registration and corporate details.

#### fmp_sic_list

Returns available SIC (Standard Industrial Classification) industry codes.

- **Parameters:** (none)
- **Use when:** Referencing industry classification codes for sector-based analysis.

#### fmp_sic_search

Searches SIC classifications by keyword or code.

- **Parameters:** (none)
- **Use when:** Looking up the SIC code for a specific industry or business type.

#### fmp_all_sic

Returns the complete list of all SIC classification codes and their descriptions.

- **Parameters:** (none)
- **Use when:** Needing a comprehensive reference of all industry classification codes.

---

## Insider Trading (6 tools)

These tools track Form 4 insider transactions, beneficial ownership filings, and aggregate insider trading statistics.

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_insider_latest` | Latest insider trades across all companies | page, limit |
| `fmp_insider_search` | Search insider trades by symbol | symbol, page, limit |
| `fmp_insider_by_name` | Insider trades by person name | name |
| `fmp_insider_transaction_types` | All insider transaction type codes | (none) |
| `fmp_insider_stats` | Insider trading statistics for a stock | symbol |
| `fmp_beneficial_ownership` | Beneficial ownership filings | symbol |

### Insider Trading — Tool Details

#### fmp_insider_latest

Returns the most recent insider trades filed across all public companies.

- **Parameters:** `page` (int), `limit` (int)
- **Use when:** Screening for the latest insider buying and selling activity market-wide.

#### fmp_insider_search

Searches insider trades for a specific company by ticker symbol.

- **Parameters:** `symbol` (string), `page` (int), `limit` (int)
- **Use when:** Investigating insider activity within a specific company (e.g., cluster buys before earnings).

#### fmp_insider_by_name

Looks up insider trades filed by a specific person's name.

- **Parameters:** `name` (string)
- **Use when:** Tracking a specific executive's or director's trading history across companies.

#### fmp_insider_transaction_types

Returns all insider transaction type codes (e.g., P-Purchase, S-Sale, A-Award) and their definitions.

- **Parameters:** (none)
- **Use when:** Decoding transaction type codes returned by other insider trading tools.

#### fmp_insider_stats

Returns aggregate insider trading statistics for a given stock, including net buy/sell ratios and total transaction volumes.

- **Parameters:** `symbol` (string)
- **Use when:** Gauging overall insider sentiment for a stock — net buying suggests confidence, net selling may warrant caution.

#### fmp_beneficial_ownership

Retrieves beneficial ownership filings (Schedule 13D/13G) for a given stock, showing entities with 5%+ ownership stakes.

- **Parameters:** `symbol` (string)
- **Use when:** Identifying major shareholders and activist positions that may influence corporate governance.

---

## Institutional Ownership / 13F (8 tools)

These tools analyze 13F institutional holdings filings, providing insight into how large asset managers allocate capital.

| MCP Tool | Purpose | Key Inputs |
|----------|---------|------------|
| `fmp_institutional_latest` | Latest 13F filings | page, limit |
| `fmp_institutional_extract` | Extract holdings from a 13F filing | cik, year, quarter |
| `fmp_institutional_dates` | Available 13F filing dates for a holder | cik |
| `fmp_institutional_analytics_holder` | Holder analytics for a stock | symbol, year, quarter |
| `fmp_holder_performance` | Institutional holder performance summary | cik |
| `fmp_holder_industry_breakdown` | Holder's industry allocation | cik, year, quarter |
| `fmp_positions_summary` | All institutional positions in a stock | symbol, year, quarter |
| `fmp_industry_ownership_summary` | Industry-level institutional ownership | year, quarter |

### Institutional Ownership — Tool Details

#### fmp_institutional_latest

Returns the most recently filed 13F reports across all institutional investment managers.

- **Parameters:** `page` (int), `limit` (int)
- **Use when:** Discovering which institutions have recently filed updated holdings disclosures.

#### fmp_institutional_extract

Extracts the full list of holdings from a specific 13F filing, identified by the filer's CIK, year, and quarter.

- **Parameters:** `cik` (string), `year` (int), `quarter` (int)
- **Use when:** Analyzing the complete portfolio of a specific institutional holder for a given quarter.

#### fmp_institutional_dates

Returns all available 13F filing dates for a given institutional holder by CIK.

- **Parameters:** `cik` (string)
- **Use when:** Determining which quarters are available before extracting holdings data.

#### fmp_institutional_analytics_holder

Provides analytics on institutional holders of a specific stock, including position sizes and changes.

- **Parameters:** `symbol` (string), `year` (int), `quarter` (int)
- **Use when:** Understanding which institutions own a stock and how their positions have changed.

#### fmp_holder_performance

Returns a performance summary for an institutional holder's portfolio.

- **Parameters:** `cik` (string)
- **Use when:** Evaluating an institution's track record before following their trades.

#### fmp_holder_industry_breakdown

Shows the industry allocation breakdown for an institutional holder's portfolio.

- **Parameters:** `cik` (string), `year` (int), `quarter` (int)
- **Use when:** Understanding a holder's sector concentration and diversification.

#### fmp_positions_summary

Returns a summary of all institutional positions in a specific stock for a given quarter.

- **Parameters:** `symbol` (string), `year` (int), `quarter` (int)
- **Use when:** Gauging total institutional interest in a stock and identifying top holders.

#### fmp_industry_ownership_summary

Provides industry-level aggregation of institutional ownership data.

- **Parameters:** `year` (int), `quarter` (int)
- **Use when:** Analyzing macro-level institutional capital flows across sectors.

---

## Usage Patterns

### 1. Insider Activity Monitor

Detect and interpret insider trading patterns for a specific stock.

```
Step 1: fmp_insider_search(symbol="AAPL")
        Retrieve recent insider transactions for the target company.

Step 2: fmp_insider_stats(symbol="AAPL")
        Pull aggregate insider trading statistics to see net buy/sell ratios.

Step 3: Analyze the results.
        - Cluster buys (multiple insiders buying within a short window) signal
          collective confidence and are often a stronger indicator than isolated
          transactions.
        - Compare insider purchase sizes relative to their total holdings.
        - Cross-reference transaction dates with 8-K filings or earnings dates
          for context.
```

### 2. 13F Deep Dive

Analyze a major institutional holder's full portfolio, performance, and sector allocation.

```
Step 1: fmp_institutional_extract(cik="0001067983", year=2025, quarter=4)
        Extract Berkshire Hathaway's full 13F holdings for the target quarter.

Step 2: fmp_holder_performance(cik="0001067983")
        Review the holder's portfolio performance summary to assess track record.

Step 3: fmp_holder_industry_breakdown(cik="0001067983", year=2025, quarter=4)
        Examine sector allocation to understand concentration risk and
        investment themes (e.g., heavy financials or energy tilt).
```

### 3. Regulatory Due Diligence

Conduct a comprehensive regulatory review of a company's SEC filing history and profile.

```
Step 1: fmp_sec_filings_by_symbol(symbol="TSLA")
        Pull the company's complete SEC filing history to identify all
        disclosure types and filing frequency.

Step 2: fmp_sec_profile(symbol="TSLA")
        Retrieve the full SEC profile including SIC code, state of
        incorporation, fiscal year end, and business address.

Step 3: fmp_sec_filings_by_form(formType="DEF 14A")
        Search for proxy statement filings to review executive compensation,
        board composition, and shareholder proposals.
```

### 4. Smart Money Tracking

Monitor quarter-over-quarter changes in institutional positions to detect accumulation or distribution.

```
Step 1: fmp_positions_summary(symbol="NVDA", year=2025, quarter=3)
        Retrieve all institutional positions for the stock in Q3.

Step 2: fmp_positions_summary(symbol="NVDA", year=2025, quarter=4)
        Retrieve the same data for Q4.

Step 3: Compare Q-over-Q changes.
        - Identify institutions that initiated new positions (accumulation).
        - Flag institutions that exited entirely (distribution).
        - Calculate the net change in total institutional shares held.
        - Large coordinated increases across multiple holders can confirm
          a bullish institutional consensus.
```

---

## Tips and Best Practices

- **CIK lookup first:** When working with 13F or CIK-based tools, use `fmp_sec_company_search_name` or `fmp_sec_company_search_symbol` to resolve the CIK before calling filing-specific endpoints.
- **Date ranges matter:** Always specify `from` and `to` dates when available to narrow results and avoid hitting rate limits or returning excessive data.
- **Decode transaction types:** Use `fmp_insider_transaction_types` to interpret transaction codes before drawing conclusions from insider trading data.
- **Quarter lag awareness:** 13F filings are due 45 days after quarter-end, so data reflects positions as of the end of the prior quarter, not current holdings.
- **Combine across categories:** The most powerful analyses combine tools from multiple categories — e.g., pairing insider buying signals with increasing institutional ownership and recent 8-K filings for a holistic view.
