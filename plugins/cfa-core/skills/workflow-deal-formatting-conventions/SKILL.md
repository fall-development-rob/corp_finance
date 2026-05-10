---
name: "workflow-deal-formatting-conventions"
description: |
  WHAT: Number formatting, currency display, multiple notation, percentage conventions, date formats, table layout standards, and financial statement colour-coding rules for institutional financial deliverables across IB, PE, equity research, and wealth management domains.
  WHEN: Invoke when authoring any financial table, chart data set, or narrative section of a client-facing or committee-facing document, to ensure output matches institutional formatting conventions before review or distribution.
---

# Deal Document Formatting Conventions

## What this skill covers

Defines the precise formatting rules for numbers, currencies, multiples, percentages, dates, tables, and financial statement cell conventions used in all institutional financial deliverables. Apply these rules consistently across every document produced; deviation is a quality failure.

## Number Formatting

- **Thousands separator:** US format with comma (1,234.5)
- **Negative numbers:** parentheses — $(1,234.5) not -$1,234.5; never use minus sign in client-facing documents
- **Not meaningful:** "nm" when a ratio or metric is mathematically undefined or misleading
- **Precision:** match precision to materiality — do not imply false precision with excess decimal places

## Currency Display

| Amount range | Format | Example |
|-------------|--------|---------|
| >= $1B | one decimal, B suffix | $1.5B |
| >= $1M | one decimal, M suffix | $12.3M |
| < $1M | integer, no suffix | $750,000 |

- Use standard symbols: $, £, €, ¥ — never spell out "dollars", "pounds", etc.
- Mixed currencies: state base currency and FX conversion date explicitly in the table header or footnote

## Multiple Notation

- **Format:** one decimal place with lowercase "x" (8.5x, 12.3x)
- **Never:** spell out "times" — always "x"
- **Ranges:** both endpoints to same precision (8.0x-10.0x)
- **In narrative:** "trades at 8.5x EV/EBITDA" — lowercase x, no space between number and x

## Percentage Formatting

- **Standard:** one decimal place (12.3%, 8.0%)
- **Basis points:** use for spreads and changes <1% (+25bps, -50bps)
- **Conversion rule:** 1% = 100bps; use bps when the change is <100bps
- **In tables:** percent sign in the column header, not repeated in every cell

## Date Formatting

| Context | Format | Example |
|---------|--------|---------|
| In reports (narrative and tables) | DD-Mon-YYYY | 15-Jan-2026 |
| Fiscal years | FY[year] | FY2025, FY2026 |
| Calendar years (when distinction matters) | CY[year] | CY2025 |
| Quarters | Q[n] [year] (space between Q and year) | Q1 2026, Q4 2025 |
| Period abbreviations | LTM, NTM, TTM | LTM 12/31/2025 |

## Table Layout Standards

- **Gridlines:** no vertical gridlines; thin horizontal rules only
- **Header row:** bold text, light shaded background, bottom border
- **Data rows:** alternating white/light grey for readability in tables exceeding 10 rows
- **Totals row:** bold text, top border (single line), bottom border (double line)
- **Alignment:** numbers right-aligned; text left-aligned; column headers match their column type
- **Units:** stated in column header, not repeated in every cell ($M, %, x)
- **Negatives in tables:** parentheses — never minus sign, never red font in printed or PDF materials
- **Source footnotes:** below every table — "Source: [specific source]"

## Financial Statement Colour-Coding

Apply when producing Excel-style financial models (or markdown equivalent with annotation):

| Colour | Meaning |
|--------|---------|
| Blue | Hardcoded inputs and assumptions (user-adjustable) |
| Black | Formulas and calculations (derived values) |
| Green | Cross-sheet links (references to other worksheets) |
| Purple | Same-sheet section links (references within the worksheet) |
| Bold | Totals and subtotals only — not for emphasis |
| Italic | Footnotes, assumption notes, non-GAAP adjustments |

## Output format

When these conventions are applied during document authoring, produce:

1. **Formatted table(s)** — following all layout standards above
2. **Formatting compliance note** — confirm which conventions were applied; flag any deviation with rationale
3. **Non-conformances list** — any place where a convention could not be applied (e.g., a source system provides mixed formats) with the remediation taken

## Quality gates

- [ ] All negative numbers in parentheses — no minus signs
- [ ] All multiples use lowercase "x" with one decimal
- [ ] All percentages use one decimal; changes <100bps use "bps" notation
- [ ] All dates follow DD-Mon-YYYY convention
- [ ] All tables: no vertical gridlines; units in column header only; source footnote present
- [ ] Large currency amounts ($1M+) use one-decimal M/B suffix notation

## Related skills

- `workflow-confidentiality-disclaimers` — confidentiality banners that accompany formatted documents
- `workflow-deal-citation-standards` — source footnote rules that pair with every table's source line
- `workflow-deal-quality-checklist` — final QC step that verifies formatting compliance before delivery
