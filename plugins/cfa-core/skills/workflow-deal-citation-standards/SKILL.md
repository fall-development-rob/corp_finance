---
name: "workflow-deal-citation-standards"
description: |
  WHAT: Formal citation rules for all data sources used in institutional financial deliverables — SEC filings, earnings materials, market data (FMP), corp-finance-mcp tool outputs, third-party research, and vendor data — with exact format strings for each source type.
  WHEN: Invoke when adding a source footnote below a table, attributing a financial figure in narrative text, or citing a tool invocation in any financial deliverable. Apply before workflow-deal-quality-checklist runs its attribution check.
---

# Deal Document Citation Standards

## What this skill covers

Specifies the exact format string for every source type used in institutional financial deliverables. Consistent citations allow readers to verify figures independently and satisfy regulatory requirements (MiFID II, FINRA Rule 2241) for research attributability. The citation schema covers six source categories: SEC filings, earnings materials, market data, computed metrics, third-party research, and vendor/proprietary data.

## Citation Format Strings

### SEC Filings

| Filing type | Format |
|-------------|--------|
| Annual report | `Source: [Company] 10-K (FY[year]), pg. [X]` |
| Quarterly report | `Source: [Company] 10-Q ([Q][n] [year]), pg. [X]` |
| Proxy statement | `Source: [Company] DEF 14A ([year]), pg. [X]` |
| 8-K (material event) | `Source: [Company] 8-K filed [DD-Mon-YYYY]` |
| Registration statement | `Source: [Company] S-1 filed [DD-Mon-YYYY], pg. [X]` |

Page numbers are mandatory for 10-K and 10-Q citations. Use EDGAR document viewer page numbers.

### Earnings Materials

| Material | Format |
|----------|--------|
| Earnings release | `Source: [Company] [Q][n] [year] Earnings Release` |
| Earnings call transcript | `Source: [Company] [Q][n] [year] Earnings Call Transcript` |
| Annual guidance | `Source: [Company] FY[year] Guidance ([Q][n] [year] Earnings Release)` |
| Investor day / analyst day | `Source: [Company] Investor Day Presentation, [DD-Mon-YYYY]` |

### Market Data (FMP)

| Data type | Format |
|-----------|--------|
| Price / quote data | `Source: FMP Market Data, closing prices as of [DD-Mon-YYYY]` |
| Financial statements | `Source: FMP [fmp_income_statement / fmp_balance_sheet / fmp_cash_flow], [Company], as of [DD-Mon-YYYY]` |
| Ratios and metrics | `Source: FMP [fmp_ratios / fmp_key_metrics], [Company], as of [DD-Mon-YYYY]` |
| Sector performance | `Source: FMP Sector Performance Data as of [DD-Mon-YYYY]` |

Always include the specific FMP tool name and the data-as-of date.

### Computed Metrics (corp-finance-mcp tools)

Every figure produced by a corp-finance-mcp computation tool must be cited with:
1. The tool name
2. Key assumptions (minimum: the two variables with the greatest impact on the output)

| Metric | Citation format |
|--------|-----------------|
| DCF valuation | `Source: corp-finance-mcp dcf_model (WACC [x]%, TGR [y]%)` |
| WACC | `Source: corp-finance-mcp wacc_calculator (Beta [x], Rf [y]%, ERP [z]%)` |
| Comps | `Source: corp-finance-mcp comps_analysis ([N] peers, EV/EBITDA median [x]x)` |
| LBO | `Source: corp-finance-mcp lbo_model (entry [x]x, exit [y]x, hold [z] yrs)` |
| Bond price | `Source: corp-finance-mcp bond_pricer (YTM [x]%, duration [y] yrs)` |
| Scenario | `Source: corp-finance-mcp scenario_analysis ([bull/base/bear] case)` |

General pattern: `Source: corp-finance-mcp [tool_name] ([key_assumption_1], [key_assumption_2])`

### Third-Party Research

Format: `Source: [Provider] [Report Title] ([Mon-YYYY])`

Examples:
- `Source: McKinsey Global Banking Report (Jan-2026)`
- `Source: BCG Global Asset Management Report (Mar-2026)`
- `Source: Gartner Magic Quadrant for [Category] (2025)`

Do not cite third-party research without a report title and date. "Source: McKinsey" is not a valid citation.

### Vendor and Proprietary Data

| Vendor | Format |
|--------|--------|
| Bloomberg | `Source: Bloomberg Terminal, [field/function], as of [DD-Mon-YYYY]` |
| FactSet | `Source: FactSet [dataset], as of [DD-Mon-YYYY]` |
| LSEG / Refinitiv | `Source: LSEG [dataset], as of [DD-Mon-YYYY]` |
| S&P Capital IQ | `Source: S&P Capital IQ, as of [DD-Mon-YYYY]` |
| Moody's | `Source: Moody's [report/dataset], as of [DD-Mon-YYYY]` |
| PitchBook | `Source: PitchBook [dataset], as of [DD-Mon-YYYY]` |

### Company-Provided Data

When a company or management team provides data directly (e.g., in a data room or management presentation):

`Source: [Company] Management [presentation/data room], [DD-Mon-YYYY]`

Note: company-provided data must be cross-checked against public sources where possible; add a footnote if it cannot be independently verified.

## Placement Rules

- **Below tables:** every table must have at least one "Source:" footnote immediately below the table border
- **Inline citations:** for figures cited in narrative text, append in parentheses: "Revenue grew 12.3% YoY (Source: FMP fmp_income_statement, AAPL, as of 28-Feb-2026)"
- **Multiple sources in one table:** list all sources as a numbered footnote block below the table

## Output format

When applying these standards to a document:

1. **Citation log** — table listing every cited figure: document location, source type, citation string
2. **Missing citation list** — any figure without a citation flagged for completion before delivery

## Quality gates

- [ ] Every financial figure has a citation (tool output, filing page, or market data as-of date)
- [ ] corp-finance-mcp tool citations include at least two key assumptions
- [ ] Market data citations include the data-as-of date
- [ ] No citation reads only a provider name without a report title and date
- [ ] Source footnote present below every table

## Related skills

- `workflow-deal-formatting-conventions` — table layout standards that specify where source footnotes are placed
- `workflow-deal-quality-checklist` — attribution review step verifies citations are complete
- `workflow-confidentiality-disclaimers` — legal disclaimers that accompany cited research deliverables
