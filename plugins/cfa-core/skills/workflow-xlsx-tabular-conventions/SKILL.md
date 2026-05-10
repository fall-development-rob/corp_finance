---
name: workflow-xlsx-tabular-conventions
description: |
  WHAT: Core markdown-table layout conventions for headless Excel-equivalent deliverables — header row, units row, source-of-truth column, negatives in parentheses, totals rows with separator, and one-sheet-per-H2-section structure for multi-sheet workbooks.
  WHEN: Invoke whenever producing a tabular financial deliverable (DCF schedule, comp set, sensitivity grid, sources & uses, debt schedule) that a recipient will paste into Excel; when formatting consistency across tables is required.
---

# Excel-Pasteable Tabular Conventions

## What this skill covers

The canonical markdown-table layout standard for headless Excel-equivalent deliverables. These conventions let a recipient paste the output directly into Excel and recover structure, units, sources, and cross-references without rework.

**Scope**: header row, units row, source column, negatives, totals rows, multi-sheet structure. For formula text and cross-references, see `workflow-xlsx-formula-cells`. For named-range substitutes, see `workflow-xlsx-named-ranges`.

## Core conventions

### Header row

The first table row is the column header. Use clear Excel-style column names. No abbreviations without a legend.

```
| Year | Revenue | EBITDA | EBITDA Margin |
|------|---------|--------|---------------|
```

### Units row

The second row (immediately below the column header) declares units for every column. Every numeric column must have a unit declared.

```
| Year | Revenue | EBITDA | EBITDA Margin |
|------|---------|--------|---------------|
| (FY) | ($M)    | ($M)   | (%)           |
```

Standard unit tokens:
- `($M)` — US dollars, millions
- `($B)` — US dollars, billions
- `(%)` — percentage
- `(x)` — multiple (e.g., EV/EBITDA)
- `(bps)` — basis points
- `(days)` — day count
- `(count)` — whole units
- `(date)` — date string

### Source-of-truth column

Every derived table includes a `Source` column as the rightmost column. Acceptable values:

| Value | Meaning |
|-------|---------|
| `actuals` | Historical data from a filing or extract |
| `assumption` | Analyst assumption — defined in the Assumptions sheet |
| `derived` | Computed from other cells in this table |
| `tool: <tool_name>` | Direct corp-finance-mcp tool output |
| `->Sheet!Label` | Cross-reference to another sheet (see `workflow-xlsx-formula-cells`) |

### Negatives in parentheses

Negative values always use parentheses, never a minus sign (consistent with financial statement convention):

```
| Net debt | (125.0) |
```

### Multiples and percentages

- Multiples with `x` suffix: `12.5x`
- Percentages with `%` suffix: `24.3%`
- Basis points: `+45 bps`

### Totals row

Every schedule with a total uses a bold `**Total**` row preceded by a separator row:

```
| Item        | Amount ($M) |
|-------------|------------|
| Senior debt | 400.0      |
| Mezzanine   | 100.0      |
| Equity      | 250.0      |
|-------------|------------|
| **Total**   | **750.0**  |
```

### Multi-sheet workbook structure

One H2 section per sheet. Sheet order follows execution order (Assumptions before derived sheets):

```
## Assumptions

## Operating Model

## Debt Schedule

## Valuation

## Returns

## Sensitivity
```

The recipient creates one Excel tab per H2. Sheet names should match exactly.

### Document header

Every tabular deliverable begins with:

```
# [Deliverable Title]

**Date**: YYYY-MM-DD
**Author**: cfa-[agent-name]
**Source**: corp-finance-mcp tools [tool list]
**Cut date**: [data cut date]
```

### Section footer

Each section whose primary content came from a single tool ends with:

```
_Source: corp-finance-mcp `tool_name`_
```

## Numeric formatting

| Rule | Example |
|------|---------|
| Decimal precision: 1 decimal place for $M amounts | 125.4 |
| Decimal precision: 3+ places for rates and margins | 24.3% |
| Thousands separator omitted (recipient's locale handles) | 1250.0 not 1,250.0 |
| Negative in parentheses | (45.2) not -45.2 |
| Empty cell: blank, never zero unless zero is meaningful | (blank) |

## Quality gates

- [ ] Header row present on every table
- [ ] Units row present immediately below header row on every table
- [ ] Source column rightmost on every derived table
- [ ] Negatives in parentheses throughout
- [ ] Totals rows preceded by separator and rendered bold
- [ ] Multi-sheet output uses one H2 section per sheet, matching proposed tab names
- [ ] Document header includes date, author, source tools, and cut date

## Related skills

- `workflow-xlsx-formula-cells` — formula text column and cross-reference `->Sheet!Label` convention
- `workflow-xlsx-named-ranges` — named-range substitutes via heading anchors
- `workflow-xlsx-workbook-writer` — `office_xlsx_write` MCP tool for binary `.xlsx` output
