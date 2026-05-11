---
name: workflow-xlsx-formula-cells
description: |
  WHAT: Formula column and cross-reference conventions for headless Excel-equivalent tables — rendering formula intent as literal `=CELL` text strings, cross-referencing other sheets with `->Sheet!Label` arrows, and maintaining the canonical numeric value alongside the formula text.
  WHEN: Invoke when a recipient needs to reconstruct an Excel formula model from the tabular output; when a multi-sheet deliverable has cross-sheet dependencies; when the computation source must be traceable from formula text.
---

# Excel Formula Cell Conventions

## What this skill covers

How to render formula intent and cross-sheet dependencies in markdown-tabular output so a recipient can reconstruct a working Excel model by replacing the textual formula column with cell-relative Excel formulas.

**Prerequisite**: the layout conventions in `workflow-xlsx-tabular-conventions` (header row, units row, source column) must already be applied before adding formula columns.

## Formula column

### Purpose

The `Formula` column communicates the computational intent of each row. It is **descriptive text**, not an executable formula. The canonical numeric value computed by corp-finance-mcp tools appears in the data column alongside it. The recipient uses the formula text as a guide when rebuilding cell-relative Excel formulas.

### Format

The formula text uses human-readable Excel notation:
- Reference cells by their row label, not by coordinates: `=Revenue * Margin` not `=B5 * C5`
- Reference prior-period cells with a terse modifier: `=Prior * (1 + g)` for compounding
- Reference assumption cells by their label: `=WACC` (the recipient resolves this via the named-range substitute — see `workflow-xlsx-named-ranges`)
- Reference cross-sheet values with the `->` arrow: `=->Assumptions!WACC`

### Example

```
| Item             | FY2025 | FY2026 | FY2027 | Formula                | Source            |
|------------------|--------|--------|--------|------------------------|-------------------|
| Revenue          | 100.0  | 115.0  | 132.3  | =Prior * (1 + g)       | assumption: g=15% |
| EBITDA           | 25.0   | 28.8   | 33.1   | =Revenue * margin      | derived           |
| EBITDA Margin    | 25.0%  | 25.0%  | 25.0%  | =->Assumptions!margin  | ->Assumptions     |
| D&A              | (5.0)  | (5.8)  | (6.6)  | =Revenue * da_rate     | assumption        |
| EBIT             | 20.0   | 23.0   | 26.5   | =EBITDA - D&A          | derived           |
| Discount Factor  | 0.917  | 0.841  | 0.772  | =1 / (1 + WACC)^n      | ->Assumptions!WACC|
| PV of FCFF       | 18.3   | 19.3   | 20.5   | =FCFF * Discount Factor| derived           |
```

### When to include the formula column

Include the `Formula` column when:
- The recipient is expected to rebuild a formula model in Excel (not just read the output)
- The deliverable is a schedule with mathematical relationships (DCF FCFF, debt amortisation, three-statement, returns)
- The derivation from assumptions is non-trivial and the mapping is not obvious

Omit the `Formula` column for:
- Pure data tables (comp sets, trade lists, market data extracts)
- Single-computation outputs where the tool reference in the `Source` column is sufficient

## Cross-reference convention

### Syntax

Use the literal arrow `->` and bang `!` separator to denote cross-sheet references:

```
->SheetName!ColumnLabel
```

Example values in the `Source` column:
- `->Assumptions!WACC` — value comes from the WACC row in the Assumptions sheet
- `->DebtSchedule!TotalDebt_FY2026` — value comes from a row in the Debt Schedule sheet

### In the Formula column

When a formula's input comes from another sheet:

```
| WACC | 9.5% | =->Assumptions!WACC | ->Assumptions!WACC |
```

The formula text makes the cross-sheet dependency explicit so the recipient knows where to pull the linked cell from.

### Heading anchor form

When the target is a heading anchor (not a row label that appears as a cell):

```
->Assumptions#assumptions-wacc
```

The `#` separator denotes a markdown heading anchor (see `workflow-xlsx-named-ranges`). Use this form when the target is a named block rather than a single-row value.

## Workbook cross-sheet reference map

For large multi-sheet deliverables, include a reference map at the end of the document:

```
## Cross-Reference Map

| From Sheet | Label | To Sheet | Anchor |
|-----------|-------|---------|--------|
| FCFF Schedule | WACC | Assumptions | #assumptions-wacc |
| Debt Schedule | Terminal EBITDA | Operating Model | #op-model-terminal |
| Returns | Exit Equity | Debt Schedule | #debt-schedule-exit |
```

This map allows a post-paste automation script to resolve `->` references into cell coordinates.

## Quality gates

- [ ] Formula column present on all schedule sheets where the recipient is expected to rebuild the model
- [ ] Formula text uses label references, not coordinate references (e.g., `=Revenue * margin` not `=B5 * C5`)
- [ ] Cross-sheet references use `->SheetName!Label` syntax consistently
- [ ] Canonical numeric value from corp-finance-mcp appears in the data column alongside formula text
- [ ] Formula column omitted for pure data tables (comp sets, market data)
- [ ] Cross-reference map included for deliverables with more than 2 sheets

## Related skills

- `workflow-xlsx-tabular-conventions` — prerequisite: header row, units row, source column layout
- `workflow-xlsx-named-ranges` — heading anchor substitutes for Excel named ranges
- `workflow-xlsx-workbook-writer` — `office_xlsx_write` MCP tool; the `formulas` array parameter accepts actual Excel formula strings to inject after paste
