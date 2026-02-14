---
name: "FMP ETF & Funds"
description: "Use the fmp-mcp-server ETF and mutual fund tools for fund analysis, holdings decomposition, sector/country allocation, and institutional disclosure tracking. Invoke when analysing ETF composition, fund exposure to specific stocks, sector weightings, or mutual fund disclosure filings."
---

# FMP ETF & Funds Skill

## Overview

This skill covers the 9 ETF and mutual fund tools available through the `fmp-mcp-server` MCP integration. These tools enable comprehensive fund analysis including holdings decomposition, geographic and sector allocation breakdowns, passive ownership mapping, and institutional disclosure tracking.

---

## Tools Reference

### ETF Analysis Tools

#### `fmp_etf_holdings`

Retrieves the full list of constituent holdings and their portfolio weights for an ETF.

- **Key Inputs**: `symbol` (ETF ticker, e.g. `SPY`, `QQQ`, `VWO`)
- **Returns**: Array of holdings with ticker, name, weight percentage, and share count
- **Use when**: You need to see exactly what an ETF owns and in what proportions

```
fmp_etf_holdings(symbol="SPY")
```

#### `fmp_etf_info`

Returns ETF or fund metadata including expense ratio, AUM (assets under management), inception date, issuer, and investment strategy description.

- **Key Inputs**: `symbol` (ETF ticker)
- **Returns**: Fund profile with expense ratio, AUM, inception date, issuer, description
- **Use when**: Evaluating fund costs, size, or comparing similar ETFs

```
fmp_etf_info(symbol="QQQ")
```

#### `fmp_etf_country_weightings`

Breaks down an ETF's geographic allocation by country, showing what percentage of the fund is invested in each country.

- **Key Inputs**: `symbol` (ETF ticker)
- **Returns**: Country-level allocation percentages
- **Use when**: Assessing geographic risk concentration or international diversification

```
fmp_etf_country_weightings(symbol="VWO")
```

#### `fmp_etf_asset_exposure`

Performs a reverse lookup to find which ETFs hold a specific stock and at what weight. This is the inverse of `fmp_etf_holdings`.

- **Key Inputs**: `symbol` (stock ticker, e.g. `AAPL`, `MSFT`)
- **Returns**: List of ETFs that hold the specified stock, with weight in each fund
- **Use when**: Mapping passive ownership of a stock, or finding ETFs with exposure to a specific name

```
fmp_etf_asset_exposure(symbol="AAPL")
```

#### `fmp_etf_sector_weightings`

Breaks down an ETF's sector allocation, showing what percentage of the fund is invested in each GICS sector.

- **Key Inputs**: `symbol` (ETF ticker)
- **Returns**: Sector-level allocation percentages (Technology, Healthcare, Financials, etc.)
- **Use when**: Evaluating sector concentration risk or comparing sector tilts across funds

```
fmp_etf_sector_weightings(symbol="SPY")
```

---

### Fund Disclosure Tools

#### `fmp_fund_disclosure_holders`

Returns the latest institutional fund holders for a given stock, showing which mutual funds and institutional investors hold positions.

- **Key Inputs**: `symbol` (stock ticker)
- **Returns**: List of institutional holders with fund name, shares held, market value, and weight
- **Use when**: Identifying the largest institutional owners of a stock

```
fmp_fund_disclosure_holders(symbol="TSLA")
```

#### `fmp_fund_disclosure`

Retrieves a specific quarterly disclosure filing (13F) for a fund, showing all positions held at the end of that quarter.

- **Key Inputs**: `symbol` (fund CIK or ticker), `year` (filing year), `quarter` (Q1, Q2, Q3, or Q4)
- **Returns**: Complete position list with shares, market value, and changes from prior quarter
- **Use when**: Examining a fund's full portfolio at a specific point in time, or comparing quarter-over-quarter changes

```
fmp_fund_disclosure(symbol="VWO", year=2025, quarter="Q4")
```

#### `fmp_fund_disclosure_search`

Searches fund disclosure filings by fund name. Useful when you know the fund name but not its CIK or ticker.

- **Key Inputs**: `name` (fund name or partial name, e.g. `"Berkshire"`, `"Bridgewater"`)
- **Returns**: Matching fund names with identifiers that can be used with other disclosure tools
- **Use when**: Looking up a specific fund manager's filings by name

```
fmp_fund_disclosure_search(name="Berkshire")
```

#### `fmp_fund_disclosure_dates`

Lists all available disclosure filing dates for a given fund, so you know which quarters have data.

- **Key Inputs**: `symbol` (fund CIK or ticker)
- **Returns**: List of available filing dates/quarters
- **Use when**: Determining which periods are available before pulling a specific disclosure

```
fmp_fund_disclosure_dates(symbol="VWO")
```

---

## Usage Patterns

### 1. ETF Due Diligence

A full top-down analysis of an ETF before recommending or investing. Start with the fund profile, then drill into what it holds and how it is allocated.

```
Step 1: fmp_etf_info(symbol="VWO")              → Expense ratio, AUM, inception, strategy
Step 2: fmp_etf_holdings(symbol="VWO")           → Top holdings and their weights
Step 3: fmp_etf_sector_weightings(symbol="VWO")  → Sector allocation breakdown
Step 4: fmp_etf_country_weightings(symbol="VWO") → Geographic allocation breakdown
```

**Analyst output**: Summarize fund characteristics, highlight concentration risks (top-10 holdings weight, single-country or single-sector dominance), compare expense ratio to peer group, and flag any allocation surprises.

### 2. Passive Ownership Analysis

Determine which ETFs hold a specific stock and quantify passive fund exposure. Critical for understanding how index rebalancing or ETF flows might impact a stock.

```
Step 1: fmp_etf_asset_exposure(symbol="AAPL")    → Which ETFs hold Apple and at what weight
Step 2: For top ETFs returned, call fmp_etf_info  → Get AUM to estimate dollar exposure
```

**Analyst output**: Rank ETFs by estimated dollar exposure (weight * AUM), calculate total passive ownership as a percentage of the stock's market cap, and identify potential flow-driven price sensitivity.

### 3. Fund Flow Tracking

Compare a fund's holdings across consecutive quarters to track position changes, new positions, and exits.

```
Step 1: fmp_fund_disclosure_dates(symbol="VWO")              → Confirm available quarters
Step 2: fmp_fund_disclosure(symbol="VWO", year=2025, quarter="Q3") → Q3 snapshot
Step 3: fmp_fund_disclosure(symbol="VWO", year=2025, quarter="Q4") → Q4 snapshot
Step 4: Diff the two disclosures                              → Identify adds, trims, exits
```

**Analyst output**: Highlight the largest position increases and decreases by shares and market value, flag new positions and full exits, and interpret the directional thesis implied by the changes.

### 4. Institutional Ownership Mapping

Identify who the largest fund holders of a stock are, then drill into their full portfolios to understand conviction level.

```
Step 1: fmp_fund_disclosure_holders(symbol="NVDA")            → Largest fund holders of NVDA
Step 2: fmp_fund_disclosure_search(name="Vanguard")           → Find Vanguard's fund identifier
Step 3: fmp_fund_disclosure(symbol=<id>, year=2025, quarter="Q4") → Full Vanguard portfolio
Step 4: Calculate NVDA weight in Vanguard's total portfolio    → Gauge conviction
```

**Analyst output**: Rank institutional holders by position size, distinguish between passive index holders and active managers with conviction positions, and track quarter-over-quarter changes in institutional ownership.

---

## Best Practices

- **Combine ETF tools with market data tools**: After pulling holdings via `fmp_etf_holdings`, use `fmp_quote` or `fmp_company_profile` from the market data skill to enrich each holding with live prices, market cap, and fundamentals.
- **Cross-reference sector weightings**: Compare `fmp_etf_sector_weightings` output against benchmark sector weights to quantify over/underweight positions.
- **Use disclosure dates before disclosure pulls**: Always call `fmp_fund_disclosure_dates` first to confirm data availability before requesting a specific quarter, avoiding empty responses.
- **Batch ETF lookups**: When comparing multiple ETFs (e.g. SPY vs QQQ vs IWM), run the same tool across all symbols and present results side by side for quick comparison.
- **Track disclosure cadence**: Fund disclosures are filed quarterly with a 45-day delay. The most recent quarter may not yet be available. Check dates first.
