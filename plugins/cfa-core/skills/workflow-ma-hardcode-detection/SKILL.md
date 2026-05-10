---
name: "workflow-ma-hardcode-detection"
description: |
  WHAT: Detection and classification of hardcoded values in financial models — numeric literals in formula rows (broken formulas), rate/multiple/growth literals embedded inside formula strings (shadow hardcodes), and shadow hardcodes that replicate tool output without an = prefix.
  WHEN: Invoke when auditing a financial model for hardcoded inputs that should be formula-driven, or when validating that all material assumptions flow from a documented assumption block rather than being embedded in calculation cells.
---

# Model Audit — Hardcode Detection

## What this skill covers

Systematic identification of all hardcoded values in a financial model, classification into permitted vs unpermitted categories, and production of a prioritised remediation list. Hardcodes that silently override formula logic are among the most common and most consequential model errors.

## Permitted hardcodes

The following are always acceptable and must never be flagged:

| Category | Examples |
|----------|---------|
| Raw historical actuals | Revenue from 10-K, net income from audited financials, balance sheet from filing date |
| Named assumption-block drivers | Cells in a dedicated assumptions sheet fed by a single input (e.g., `Assumptions!C4 = 0.08`) |
| Universal constants | 12 (months), 365 (days), 100 (percentage conversion), 0 (zero balance initialiser) |

## Unpermitted hardcodes (flag as findings)

### Type 1 — Numeric literal in a projection formula row

A numeric value occupies a cell in a row where the adjacent period cells are formulas, and the value is not a raw historical actual. This indicates a formula was manually overridden.

**Severity:** Critical if in a headline output row (Revenue, EBITDA, EBIT, Net Income, Total Assets, Total Debt, Ending Cash). Major otherwise.

### Type 2 — Embedded literal inside a formula string

A numeric constant (rate, multiple, growth %) is embedded directly in a formula, e.g., `=B5*0.25` or `=C10*(1+0.08)`, rather than referencing a named assumption cell.

**Severity:** Major. The literal is invisible in formula audits and breaks when the assumption changes.

### Type 3 — Shadow hardcode

A cell value matches a tool output (e.g., WACC from `wacc_calculator`, MOIC from `returns_calculator`) but has no `=` prefix — it was pasted as a value. This masks dependency on the tool and breaks model updates.

**Severity:** Major if in a material output cell. Minor if in a supporting schedule.

## Workflow

### Step 1 — Scan all cells

For each cell in the model:
- Classify: formula (`=` prefix) vs numeric literal vs text.
- For formula cells: extract any numeric literals embedded in the formula string.
- Flag any numeric literal that is not on the permitted list.

### Step 2 — Contextualise each finding

For each flagged cell, determine:
- Is the value plausible for a raw historical actual? (Check filing date; actuals columns are typically shaded differently.)
- Is the value ≡ to any named assumption? (If so, it may be a copy-paste of a permitted hardcode — still flag for traceability, severity Minor.)
- Does the value match any corp-finance-mcp tool output for the stated inputs? (Shadow hardcode.)

### Step 3 — Produce the hardcode register

Output a ranked register of all unpermitted hardcodes, sorted by severity then by proximity to headline outputs.

## Output format

```
HARDCODE DETECTION REPORT
--------------------------
Model: [name / file reference]
Total cells scanned: [n]
Unpermitted hardcodes found: [n] (Critical: [n] | Major: [n] | Minor: [n])

HARDCODE REGISTER
| # | Cell | Sheet | Row label | Type | Severity | Value | Note |
|---|------|-------|-----------|------|----------|-------|------|
| 1 | C10  | IS    | Revenue   | Type 1 (literal in formula row) | Critical | 1,200 | Manual override in FY2026 projection |
| 2 | D14  | IS    | EBITDA Margin | Type 2 (embedded literal) | Major | =D13*0.22 | Should reference Assumptions!$C$7 |
| 3 | F5   | Returns | MOIC | Type 3 (shadow hardcode) | Major | 2.8 | Matches returns_calculator output; not formula-linked |
```

## Quality gates

- Every cell with a numeric literal is reviewed — no sampling.
- Type 1 findings in headline rows are always Critical — no downgrade without explicit MLRO-equivalent sign-off.
- Type 2 findings always cite the assumption cell that should replace the literal.
- Type 3 findings state the specific tool whose output was replicated.
- Permitted hardcodes are documented (counted and listed) but not flagged in the findings register.

## Related skills

- `workflow-ma-link-tracing` — share the cell inventory built during link tracing to avoid duplicate scanning
- `workflow-ma-formula-consistency` — Type 1 hardcodes are a subset of broken-formula-row findings; coordinate to avoid duplicate reporting
- `workflow-ma-rederivation` — Type 3 shadow hardcodes are validated by running the relevant corp-finance-mcp tool and comparing

## Routing

**Primary agent:** `cfa-chief-analyst`
