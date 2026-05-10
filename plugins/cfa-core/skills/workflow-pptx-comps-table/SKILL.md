---
name: "workflow-pptx-comps-table"
description: |
  WHAT: Comps table and sensitivity slide templates for financial decks — trading comparables table layout (ticker, market cap, EV, EV/Revenue, EV/EBITDA, P/E, revenue growth, EBITDA margin, median and target rows), 2D sensitivity grid with base-case marking, and positioning takeaway conventions; numbers sourced from `comps_analysis` and `sensitivity_matrix`.
  WHEN: Invoke when authoring the trading comparables slide or sensitivity analysis slide in any IB pitch, PE IC presentation, or equity research deck.
---

# Slide Deck — Comps Table and Sensitivity Slide Layouts

## What this skill covers

Standardised markdown templates and conventions for the trading comparables slide and sensitivity analysis slide in financial decks. Both slides rely on corp-finance-mcp tools as the sole data source; LLM-generated figures are not permitted.

## Tool references

| Tool | Slide |
|------|-------|
| `comps_analysis` | Trading comparables table — all multiples and operating metrics |
| `sensitivity_matrix` | Sensitivity grid — all scenario values |
| `fmp_key_metrics` | Supplementary market cap, EV, and per-share data |

## Trading comparables slide

### Layout

Slide title: `## Trading Comparables`

Column order (per `workflow-xlsx-author` conventions):

| Column | Unit | Format |
|--------|------|--------|
| Company | — | Text |
| Ticker | — | Text |
| Market Cap | $M | Comma-separated integer |
| Enterprise Value (EV) | $M | Comma-separated integer |
| EV / Revenue (LTM) | x | One decimal |
| EV / EBITDA (LTM) | x | One decimal |
| P / E (LTM) | x | One decimal |
| Revenue growth (YoY) | % | One decimal |
| EBITDA margin (LTM) | % | One decimal |

### Required rows

1. Each comparable company (one row per comp).
2. A separator row (`|---|---|...|`).
3. **Median** row — bolded — showing median for every metric column.
4. **Target** row — the company being valued — bolded and visually distinguished (use `**bold**` in every cell).

### Positioning takeaway

Below the table, a single sentence summarising the positioning: e.g., "Target trades at a [discount / premium] to the peer median on EV/EBITDA ([target]x vs [median]x) reflecting [one-line rationale]."

### Template

```
## Trading Comparables

| Company | Ticker | Mkt Cap ($M) | EV ($M) | EV/Rev | EV/EBITDA | P/E | Rev Growth | EBITDA Margin |
|---------|--------|--------------|---------|--------|-----------|-----|------------|---------------|
| ACME Corp | ACME | 2,450 | 2,850 | 2.4x | 9.2x | 18.5x | 12.3% | 26.1% |
| Beta Inc | BETA | 1,120 | 1,380 | 1.9x | 7.8x | 14.2x | 8.7% | 24.3% |
| Gamma Ltd | GMMA | 3,800 | 4,100 | 3.1x | 10.5x | 22.1x | 15.2% | 29.7% |
| Delta Corp | DELT | 890 | 1,050 | 1.6x | 7.1x | 13.8x | 6.1% | 22.5% |
|---|---|---|---|---|---|---|---|---|
| **Median** | | **1,785** | **2,115** | **2.2x** | **8.5x** | **16.4x** | **10.5%** | **25.2%** |
| **Target Co** | **TARG** | **1,650** | **1,950** | **2.0x** | **8.0x** | **15.5x** | **11.2%** | **24.8%** |

Target trades at a slight discount to the peer median on EV/EBITDA (8.0x vs 8.5x) reflecting integration uncertainty post-acquisition; management's operational improvement plan supports normalisation within 18 months.

*Source: corp-finance-mcp comps_analysis, as of [DD-Mon-YYYY] | CONFIDENTIAL*

---
```

### Handling N/A values

Use `n.m.` (not meaningful) for negative EBITDA or negative P/E. Do not show a number for a metric that would be misleading. Exclude `n.m.` values from the median calculation and note the exclusion below the table.

## Sensitivity analysis slide

### Layout

Slide title: `## Sensitivity: [Variable 1] vs [Variable 2]`

Format: 2D markdown grid where rows are one variable (e.g., exit multiple) and columns are the other variable (e.g., WACC or EBITDA growth).

Rules:
- Base case row header: bolded. Base case column header: bolded.
- Values in the table body: Equity Value in $M or EV/EBITDA in x, as appropriate.
- Below the grid: one-line takeaway citing the sensitivity range (e.g., "Equity value ranges from $3.2B to $6.8B across the stated WACC and terminal growth scenarios").

### Template — DCF WACC vs Terminal Growth sensitivity

```
## Sensitivity: WACC vs Terminal Growth Rate

|                | **WACC 9.0%** | WACC 9.5% | **WACC 10.0%** | WACC 10.5% | WACC 11.0% |
|----------------|---------------|-----------|----------------|------------|------------|
| **TGR 3.0%**   | **$5,450M**   | $5,120M   | **$4,820M**    | $4,550M    | $4,300M    |
| TGR 2.5%       | $5,200M       | $4,890M   | $4,610M        | $4,350M    | $4,110M    |
| **TGR 2.0%**   | **$4,980M**   | $4,680M   | **$4,420M**    | $4,170M    | $3,940M    |
| TGR 1.5%       | $4,770M       | $4,490M   | $4,240M        | $4,000M    | $3,780M    |
| TGR 1.0%       | $4,570M       | $4,310M   | $4,070M        | $3,840M    | $3,630M    |

**Base case: WACC 10.0%, TGR 2.0% → $4,420M equity value**
Equity value ranges from $3,630M to $5,450M across the stated WACC/TGR scenarios.

*Source: corp-finance-mcp sensitivity_matrix | CONFIDENTIAL*

---
```

### Template — LBO exit multiple vs EBITDA growth sensitivity

```
## Sensitivity: Exit Multiple vs EBITDA Growth

|              | EBITDA Growth 5% | **EBITDA Growth 8%** | EBITDA Growth 11% | EBITDA Growth 14% |
|--------------|-----------------|---------------------|-------------------|-------------------|
| Exit 7.0x    | 1.6x MOIC       | 1.9x MOIC           | 2.2x MOIC         | 2.6x MOIC         |
| Exit 8.0x    | 1.9x MOIC       | 2.2x MOIC           | 2.6x MOIC         | 3.0x MOIC         |
| **Exit 9.0x**| **2.2x MOIC**   | **2.6x MOIC**       | **3.1x MOIC**     | **3.6x MOIC**     |
| Exit 10.0x   | 2.5x MOIC       | 3.0x MOIC           | 3.5x MOIC         | 4.1x MOIC         |

**Base case: 9.0x exit, 8% EBITDA growth → 2.6x MOIC**
MOIC ranges from 1.6x to 4.1x across the stated exit multiple and growth scenarios.

*Source: corp-finance-mcp sensitivity_matrix, lbo_model | CONFIDENTIAL*

---
```

## Quality gates

- Every multiple in the comps table cites `comps_analysis` in the footer — no manually estimated multiples.
- Median and Target rows always present and bolded; separator row always present.
- `n.m.` used for negative/misleading metrics; exclusion from median noted.
- Sensitivity grid includes the base case cell explicitly identified (bold header).
- One-line takeaway present on both comps and sensitivity slides.
- Sensitivity values independently generated by `sensitivity_matrix` — not extrapolated by the analyst.

## Related skills

- `workflow-pptx-deck-structure` — master deck structure and mandatory slide order
- `workflow-pptx-valuation-layout` — bull/base/bear valuation slides that cross-reference the comps table
- `workflow-pptx-mcp-writer-binding` — converting comps and sensitivity slides to SlideDeckSpec table kind for `office_pptx_write`
- `workflow-xlsx-author` — number formatting conventions shared between deck and spreadsheet deliverables

## Routing

**Primary agent:** `cfa-private-markets-analyst` (pitch and IC decks)
**Secondary agent:** `cfa-equity-analyst` (research decks)
