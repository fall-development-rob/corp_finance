---
name: "Deal Document Standards"
description: "Cross-cutting document production standards for institutional financial deliverables — confidentiality disclaimers, professional formatting conventions, output specifications, quality checklists, citation standards, and number formatting rules. Shared reference for all deal-related document workflows across IB, PE, ER, and WM domains."
---

# Deal Document Standards

These are the cross-cutting formatting, citation, and quality standards applied to all institutional financial deliverables. Reference this skill whenever producing client-facing documents across IB, PE, equity research, and wealth management domains.

## Document Format Standards

### Number Formatting
- **Convention**: US format with comma thousands separator (1,234.5)
- **Negatives**: parentheses, never minus sign — $(1,234.5) not -$1,234.5
- **Not meaningful**: "nm" when a ratio or metric is mathematically undefined or misleading
- **Rounding**: match precision to materiality — do not imply false precision

### Currency
- **Symbols**: $, GBP, EUR, JPY — use standard symbols, not spelled out
- **Large amounts** (>$1M): one decimal place ($12.3M, $1.5B)
- **Small amounts** (<$1M): integers ($750,000, $42,500)
- **Mixed currencies**: state base currency and conversion date explicitly

### Multiples
- **Format**: one decimal place with lowercase "x" (8.5x, 12.3x)
- **Never**: spell out "times" — always use "x" notation
- **Ranges**: 8.0x-10.0x (both endpoints to same precision)

### Percentages
- **Standard**: one decimal place (12.3%, 8.0%)
- **Basis points**: use for small changes or spread analysis (+25bps, -50bps)
- **Conversion**: 1% = 100bps — use bps when change is <100bps

### Dates
- **In reports**: DD-Mon-YYYY (15-Jan-2026)
- **Fiscal years**: FY2025, FY2026 (or CY2025 for calendar year if distinction matters)
- **Quarters**: Q1 2026, Q4 2025 (space between Q and year)
- **Periods**: LTM (last twelve months), NTM (next twelve months), TTM (trailing twelve months)

## Table Standards

- **Gridlines**: no vertical gridlines; thin horizontal rules only
- **Header row**: bold text, light shaded background, bottom border
- **Data rows**: alternating white/light grey for readability in long tables
- **Totals row**: bold text, top border (single line), bottom border (double line)
- **Negatives**: parentheses — never minus sign, never red font in printed materials
- **Alignment**: numbers right-aligned, text left-aligned, headers match their column
- **Units**: stated in column header, not repeated in every cell ($M, %, x)
- **Source footnotes**: below every table — "Source: [specific source]"

## Financial Statement Conventions

- **Blue font**: hardcoded inputs and assumptions (user-adjustable)
- **Black font**: formulas and calculations (derived values)
- **Green font**: cross-sheet links (references to other worksheets)
- **Purple font**: same-sheet section links (references within the sheet)
- **Bold**: totals and subtotals only — not for emphasis
- **Italics**: footnotes, assumptions notes, non-GAAP adjustments

## Citation Standards

### SEC Filings
- Format: "Source: Company 10-K (FY2025), pg. X"
- Quarterly: "Source: Company 10-Q (Q3 2025), pg. X"
- Proxy: "Source: Company DEF 14A (2025), pg. X"

### Earnings Materials
- Format: "Source: Company Q4 2025 Earnings Release"
- Transcripts: "Source: Company Q4 2025 Earnings Call Transcript"
- Guidance: "Source: Company FY2026 Guidance (Q4 2025 Earnings Release)"

### Market Data
- Format: "Source: FMP Market Data as of DD-Mon-YYYY"
- Specify close: "Source: FMP Market Data, closing prices as of 28-Feb-2026"

### Computed Metrics
- Format: "Source: corp-finance-mcp [tool_name]"
- Example: "Source: corp-finance-mcp dcf_model" or "Source: corp-finance-mcp wacc_calculator"
- Include key assumptions inline: "Source: corp-finance-mcp dcf_model (WACC 9.5%, TGR 2.5%)"

### Third-Party Research
- Format: "Source: [Provider] [Report Title] (Date)"
- Example: "Source: McKinsey Global Banking Report (Jan-2026)"

## Confidentiality

### Standard Confidentiality Notice (every page)
> CONFIDENTIAL — FOR DISCUSSION PURPOSES ONLY

### CIM / Teaser Cover Page
> This document has been prepared by [Advisor] on behalf of [Client] and is strictly confidential. Distribution or reproduction of this document, in whole or in part, without the prior written consent of [Advisor] is prohibited.

### Legal Disclaimer (final page or footer)
> This presentation does not constitute an offer to sell or a solicitation of an offer to buy any securities. The information contained herein is preliminary and subject to change. [Advisor] makes no representation or warranty, express or implied, as to the accuracy or completeness of the information contained herein.

### Distribution Control
- Mark every page with confidentiality notice
- Number copies if physical distribution
- Watermark with recipient name for electronic distribution if required
- Track and document all recipients

## Pre-Delivery Quality Checklist

### Numerical Integrity
- [ ] Every number traces to a tool output or stated assumption
- [ ] Base / bull / bear scenarios provided for all valuations
- [ ] Sensitivity analysis on 2+ key variables
- [ ] Financial tables internally consistent (revenue matches across all sections)
- [ ] Sources & Uses balance (total sources = total uses, if applicable)
- [ ] Balance sheet balances in every forecast period (if applicable)

### Formatting Compliance
- [ ] Consistent number formatting throughout (commas, decimals, currency)
- [ ] Negatives in parentheses, no minus signs
- [ ] Multiples use "x" notation with one decimal
- [ ] Dates in DD-Mon-YYYY format
- [ ] Tables follow gridline and alignment standards

### Documentation
- [ ] Confidentiality notice on every page
- [ ] Date stamps on all market data citations
- [ ] Source footnotes below every table and chart
- [ ] All corp-finance-mcp tool outputs cited with tool name

### Final Review
- [ ] Spell check and terminology review complete
- [ ] Consistent use of IB/PE standard terms throughout
- [ ] No first-person language in client-facing sections
- [ ] Executive summary accurately reflects detailed findings
- [ ] All cross-references within the document are correct
- [ ] Page numbers and table of contents are accurate
