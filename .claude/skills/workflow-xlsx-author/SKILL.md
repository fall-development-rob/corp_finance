---
name: "Tabular Output Authoring Workflows"
description: "Tabular markdown and CSV authoring conventions for headless Excel-equivalent deliverables — header row standards, units row, source-of-truth columns, formula columns rendered as =CELL text, cross-references rendered as ->Sheet:Cell, and named-range substitutes via heading anchors. The CFA agent stack does not have an Excel writer; this skill defines the markdown-tabular and CSV layout standard so an LP or recipient can paste output directly into Excel and recover formulas, sources, and sheet structure. Routes to cfa-chief-analyst."
---

# Tabular Output Authoring Workflows

You are producing structured tabular deliverables in a headless environment. The CFA agent stack does not have an Excel writer (no openpyxl, no Office JS). This skill defines the markdown-tabular and CSV authoring conventions so output is shaped exactly as Excel would render it — letting a recipient paste it into Excel and recover the structure, formulas, and source citations without rework.

## Core Principles

- **Markdown tables and CSV are the delivery medium.** Recipients paste into Excel; the structure must reconstruct cleanly.
- **Formulas are rendered as text.** A formula cell carries the literal string `=B5*B6`, not the evaluated number alone. The canonical computation is done by corp-finance-mcp; the formula text is metadata for the recipient.
- **Source-of-truth columns are explicit.** Every row that derives from another row, sheet, or tool output names the source.
- **One sheet = one markdown section.** A multi-sheet workbook is delivered as multiple markdown sections under H2 headings; recipients copy each section into a separate sheet.
- **Cross-references use the `->Sheet:Cell` arrow.** Visible to the human, parseable as a regex if the recipient runs a post-paste linker.
- **Named ranges become heading anchors.** A markdown anchor `{#assumptions-wacc}` substitutes for the named range `Assumptions!WACC`.

## When to Invoke

- An LP, IC, or client requests a model output, comp set, sensitivity table, or schedule in a form they can paste into Excel
- A multi-sheet output (assumptions, projections, valuation, sensitivity) must be delivered without binary file generation
- A handoff requires the recipient to be able to audit and modify the output downstream

## Workflow Selection

| Request | Workflow | Output |
|---------|----------|--------|
| "Output the DCF in Excel-pasteable form" | DCF Tabular | Multi-section markdown with formula text |
| "Comp set as a table" | Comp Set Tabular | Single-sheet markdown table |
| "Sensitivity grid" | Sensitivity Tabular | 2D grid markdown with axes labelled |
| "Sources & uses" | Schedule Tabular | Two-column markdown with totals row |
| "Multi-sheet model output" | Workbook Tabular | Multi-section markdown with cross-refs |

## Layout Standard

### Section Header

Each "sheet" is an H2 heading carrying the sheet name verbatim:

```
## Assumptions

## Operating Model

## Valuation
```

The recipient creates one Excel sheet per H2.

### Header Row Convention

The first table row is the column header. Bold-cased Excel headers become regular markdown header text — the markdown table syntax marks them as headers. Example:

```
| Year | Revenue | EBITDA | EBITDA Margin |
|------|---------|--------|---------------|
```

### Units Row

A second header-like row, immediately below the column header, declares units. This is necessary because Excel cells are unit-less and the recipient needs the unit context.

```
| Year | Revenue | EBITDA | EBITDA Margin |
|------|---------|--------|---------------|
| (FY) | ($M)    | ($M)   | (%)           |
```

Standard unit tokens: `($M)`, `($B)`, `(%)`, `(x)` for multiples, `(bps)`, `(days)`, `(count)`, `(date)`.

### Source-of-Truth Column

For every derived table, the rightmost column is `Source`. Acceptable values:

- `actuals` — historical from filing or extract
- `assumption` — analyst assumption from the assumptions sheet
- `derived` — computed from other cells in the table
- `tool: <tool_name>` — direct corp-finance-mcp tool output (e.g., `tool: dcf_model`)
- `->Assumptions!WACC` — cross-reference to another sheet

### Formula Column

For tables that the recipient will turn into an Excel formula model, include a `Formula` column whose value is the literal Excel formula string.

```
| Item             | FY2025 | FY2026 | FY2027 | Formula            | Source     |
|------------------|--------|--------|--------|--------------------|------------|
| Revenue          | 100.0  | 115.0  | 132.3  | =Prior * (1 + g)   | derived    |
| EBITDA           | 25.0   | 28.8   | 33.1   | =Revenue * margin  | derived    |
| EBITDA Margin    | 25.0   | 25.0   | 25.0   | =Assumption        | assumption |
```

The `Formula` column is **text describing the formula** in human-Excel form. It is not literally executable — the recipient may rebuild it into actual cell-relative formulas after pasting. The point is to communicate intent.

### Cross-Reference Convention

When a value comes from another sheet, render the source as `->SheetName!CellLabel` (the literal arrow `->` and the bang `!` separator):

```
| WACC | 9.5% | ->Assumptions!WACC | tool: wacc_calculator |
```

If the cell label is a heading anchor rather than a literal Excel coordinate, use the anchor form:

```
| Discount Rate | 9.5% | ->Assumptions#wacc | tool: wacc_calculator |
```

### Named Range Substitute

Excel named ranges have no markdown equivalent. The substitute is a markdown heading anchor declared at the assumption block:

```
## Assumptions

### WACC {#assumptions-wacc}

| Component         | Value | Formula              | Source                    |
|-------------------|-------|----------------------|---------------------------|
| Risk-free rate    | 4.5%  | =Treasury10Y         | tool: wacc_calculator     |
| ERP               | 5.5%  | =Damodaran           | tool: wacc_calculator     |
| Beta              | 1.10  | =Regression          | derived                   |
| Cost of equity    | 10.6% | =Rf + Beta * ERP     | tool: wacc_calculator     |
| WACC              | 9.5%  | =E*Ke + D*Kd*(1-T)   | tool: wacc_calculator     |
```

Downstream cells reference `->Assumptions#assumptions-wacc` as the named-range substitute.

### Totals Row

Every schedule with a total uses a bold `Total` row preceded by a separator:

```
| Item        | Amount |
|-------------|--------|
| Senior debt | 400.0  |
| Mezz        | 100.0  |
| Equity      | 250.0  |
|-------------|--------|
| **Total**   | **750.0** |
```

### Negatives

Negatives are rendered with parentheses, never minus signs (consistent with `workflow-deal-documents`):

```
| Net debt | (125.0) |
```

## DCF Tabular Workflow

1. **Compute via** `dcf_model`, `wacc_calculator`. Capture the structured JSON output.
2. **Section 1 — Assumptions** (H2 `## Assumptions`): one row per driver (revenue growth, EBITDA margin, capex %, WACC components, terminal method/value). Each row has `Source = tool: <name>` or `Source = assumption`.
3. **Section 2 — FCFF Schedule** (H2 `## FCFF Schedule`): year columns, line-item rows (revenue, EBITDA, D&A, EBIT, taxes, capex, change in WC, FCFF, discount factor, PV). The `Formula` column carries the human formula; the `Source` column references `->Assumptions#<anchor>` for inputs and `derived` for computed lines.
4. **Section 3 — Valuation Bridge** (H2 `## Valuation Bridge`): EV, less net debt, equity value, shares outstanding, value per share. Totals row in bold.
5. **Section 4 — Sensitivity** (H2 `## Sensitivity`): 2D grid with WACC axis on rows and terminal growth on columns. Tool: `sensitivity_matrix`.

## Comp Set Tabular Workflow

1. **Compute via** `comps_analysis`. Each comparable becomes a row.
2. **Columns:** ticker, market cap ($M), enterprise value ($M), revenue ($M), EBITDA ($M), EV/Revenue (x), EV/EBITDA (x), P/E (x), revenue growth (%), EBITDA margin (%), source.
3. **Rows:** one per comparable, plus median and mean rows at the bottom (separator-preceded).
4. **Formula column omitted** for comp sets — these are observed market data, not derived. The Source column carries `tool: comps_analysis` or the FMP source path.

## Sensitivity Tabular Workflow

Compute via `sensitivity_matrix`. Render as a 2D grid:

```
## Sensitivity: Equity Value per Share

| WACC \ TGR | 1.5%  | 2.0%  | 2.5%  | 3.0%  | 3.5%  |
|------------|-------|-------|-------|-------|-------|
| 8.5%       | 142.3 | 156.8 | 175.2 | 198.4 | 228.5 |
| 9.0%       | 132.1 | 144.5 | 159.7 | 178.6 | 202.4 |
| 9.5%       | 123.4 | 134.0 | 146.7 | 162.2 | 181.4 |
| 10.0%      | 115.8 | 124.9 | 135.7 | 148.6 | 164.2 |
| 10.5%      | 109.2 | 117.0 | 126.2 | 137.0 | 149.8 |
```

The base-case cell may be marked with a leading asterisk: `* 146.7`.

## Workbook (Multi-Sheet) Workflow

1. One H2 per sheet. Sheet order follows execution order: `## Assumptions`, `## Operating Model`, `## Debt Schedule`, `## Valuation`, `## Returns`, `## Sensitivity`.
2. All cross-sheet references use `->Sheet#anchor` form.
3. Each section ends with a one-line `Source: corp-finance-mcp <tool>` footer when its primary content came from a single tool.
4. The recipient can copy each section verbatim into a sheet of the same name.

## Tool References

| Tool | Use |
|------|-----|
| `dcf_model` | DCF schedule computation |
| `wacc_calculator` | Cost of capital schedule |
| `lbo_model` | LBO schedule computation |
| `three_statement_model` | Linked IS/BS/CF |
| `debt_schedule` | Per-tranche amortisation |
| `sources_uses` | Sources & uses schedule |
| `comps_analysis` | Trading comparables table |
| `sensitivity_matrix` | 2D sensitivity grid |
| `scenario_analysis` | Bull/base/bear scenarios |
| `merger_model` | Pro forma accretion/dilution |

## Output Standard

A tabular deliverable is structured as:

1. **Document header** — title, date, author (agent), source-of-truth notice
2. **Assumptions section** — every input with formula and source
3. **Computation sections** — one H2 per logical sheet
4. **Sensitivity / scenarios section**
5. **Footer** — `Source: corp-finance-mcp` tool list, dataset cut date, audit hash if available

Every numeric value carries a unit (declared in the units row), a source (declared in the Source column), and a formula text where applicable. Negatives in parentheses. Multiples with `x`. Percentages with `%`.

## Quality Standards

- Header row + units row present on every table
- Source column populated on every derived row
- Formula column present where the recipient is expected to rebuild a model
- Cross-references use `->Sheet#anchor` form consistently
- Totals rows preceded by a separator and rendered in bold
- Negatives in parentheses, multiples with `x`, percentages with `%`
- Multi-sheet output uses one H2 per sheet
- Every section attributable to a corp-finance-mcp tool cites the tool name in the source footer

## Adaptation Note

This skill is the headless replacement for an Excel writer. We do not have openpyxl, xlsxwriter, or Office JS in the CFA agent stack. The contract with the recipient is: paste the markdown sections into Excel sheets of the same name, replace the textual `Formula` column entries with cell-relative Excel formulas (using the source column as the dependency map), and the workbook will reconstitute. The numeric values in the markdown carry the canonical answer (computed by corp-finance-mcp in 128-bit Decimal), so the recipient's reconstructed Excel formulas should reproduce them.

## Routing

**Primary agent:** `cfa-chief-analyst`

Tabular authoring is the universal output medium for any model-derived deliverable. The chief analyst owns this skill so the convention is uniform across IB, PE, ER, and WM workflows that all produce tabular output.
