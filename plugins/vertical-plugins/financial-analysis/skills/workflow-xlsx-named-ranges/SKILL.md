---
name: workflow-xlsx-named-ranges
description: |
  WHAT: Named-range substitutes in headless markdown-tabular deliverables — heading anchor declarations, `->Sheet#anchor` reference syntax, and how named-range anchors map to `defined_names` entries in the `office_xlsx_write` MCP tool.
  WHEN: Invoke when a multi-sheet tabular deliverable has cross-sheet scalar references (WACC, discount rate, growth rate) that would normally be Excel named ranges; when a recipient needs to link a downstream model to assumption cells without hardcoding cell coordinates.
---

# Excel Named-Range Substitutes

## What this skill covers

Excel named ranges (`=WACC`, `=TerminalGrowth`) have no direct markdown equivalent. This skill defines the heading-anchor substitute pattern so recipients can reconstruct named-range references after pasting the markdown sections into Excel.

**Prerequisite**: the layout conventions in `workflow-xlsx-tabular-conventions` must be applied; the cross-reference syntax in `workflow-xlsx-formula-cells` should be understood.

## The heading anchor pattern

### Declaration

A named-range substitute is declared as a markdown heading anchor at the assumption block that defines the value:

```markdown
## Assumptions

### WACC {#assumptions-wacc}

| Component         | Value | Formula              | Source                |
|-------------------|-------|----------------------|----------------------|
| Risk-free rate    | 4.5%  | =Treasury10Y         | tool: wacc_calculator |
| ERP               | 5.5%  | =Damodaran           | tool: wacc_calculator |
| Beta              | 1.10  | =OLS regression      | derived               |
| Cost of equity    | 10.6% | =Rf + Beta * ERP     | tool: wacc_calculator |
| WACC              | 9.5%  | =E*Ke + D*Kd*(1-T)   | tool: wacc_calculator |
```

The anchor `{#assumptions-wacc}` makes this block addressable in cross-references.

### Reference syntax

Downstream cells reference the anchor with the `->Sheet#anchor` form:

```
| Discount Rate | 9.5% | =->Assumptions#assumptions-wacc | ->Assumptions#assumptions-wacc |
```

The `#` separator signals a heading anchor (vs `!` for a row label). In the final Excel workbook, this resolves to whichever cell holds the WACC value on the Assumptions sheet.

### Naming convention for anchors

Format: `{#<sheet-slug>-<variable-slug>}` (lowercase, hyphenated):

| Anchor | Meaning |
|--------|---------|
| `{#assumptions-wacc}` | WACC on the Assumptions sheet |
| `{#assumptions-tgr}` | Terminal growth rate |
| `{#assumptions-entry-ebitda}` | Entry EBITDA for LBO model |
| `{#debt-schedule-exit-debt}` | Total debt at exit year on Debt Schedule |
| `{#op-model-terminal-ebitda}` | Terminal EBITDA on Operating Model |

## Named-range index

For deliverables with 3+ named-range substitutes, include a named-range index immediately after the document header:

```markdown
## Named-Range Index

| Anchor | Value | Sheet | Description |
|--------|-------|-------|-------------|
| `#assumptions-wacc` | 9.5% | Assumptions | Weighted average cost of capital |
| `#assumptions-tgr` | 2.0% | Assumptions | Terminal growth rate (Gordon Growth) |
| `#assumptions-margin` | 25.0% | Assumptions | Target EBITDA margin at steady state |
| `#debt-schedule-exit-debt` | 250.0 | Debt Schedule | Net debt at exit year ($M) |
```

The index lets a recipient scan all named-range substitutes before diving into individual sheets.

## Mapping to `office_xlsx_write` defined_names

When the same deliverable is also produced via the `office_xlsx_write` MCP tool (see `workflow-xlsx-workbook-writer`), each heading-anchor substitute maps to a `defined_names` entry in the workbook spec:

```json
"defined_names": [
  {"name": "WACC", "range": "Assumptions!$B$5"},
  {"name": "TerminalGrowthRate", "range": "Assumptions!$B$6"},
  {"name": "EntryEBITDA", "range": "Assumptions!$B$9"}
]
```

The mapping process:
1. Identify each heading anchor in the markdown delivery
2. Locate the corresponding row in the Assumptions sheet (by row label)
3. Compute the cell coordinate after the header rows are accounted for
4. Create a `defined_names` entry with a PascalCase name matching the anchor variable

## Common assumption anchors

These anchors appear in most DCF and LBO deliverables. Use the standard names for consistency:

| Anchor | Standard Named Range | Typical Location |
|--------|---------------------|-----------------|
| `#assumptions-wacc` | `WACC` | Assumptions!$B$[n] |
| `#assumptions-tgr` | `TerminalGrowthRate` | Assumptions!$B$[n] |
| `#assumptions-tax-rate` | `TaxRate` | Assumptions!$B$[n] |
| `#assumptions-entry-multiple` | `EntryMultiple` | Assumptions!$B$[n] |
| `#assumptions-exit-multiple` | `ExitMultiple` | Assumptions!$B$[n] |
| `#assumptions-hold-period` | `HoldPeriod` | Assumptions!$B$[n] |

## Quality gates

- [ ] Named-range index present for deliverables with 3+ anchors
- [ ] Anchor naming convention followed: `#<sheet-slug>-<variable-slug>`
- [ ] Every anchor declaration appears exactly once (in the defining section)
- [ ] Every `->Sheet#anchor` reference in formula and source columns resolves to a declared anchor
- [ ] When `office_xlsx_write` is used, `defined_names` array entries match every heading anchor

## Related skills

- `workflow-xlsx-tabular-conventions` — prerequisite: layout standard
- `workflow-xlsx-formula-cells` — prerequisite: formula column and `->Sheet!Label` syntax
- `workflow-xlsx-workbook-writer` — `office_xlsx_write` MCP tool where `defined_names` are materialised as actual Excel named ranges
