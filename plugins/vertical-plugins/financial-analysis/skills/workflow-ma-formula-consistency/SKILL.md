---
name: "workflow-ma-formula-consistency"
description: |
  WHAT: Formula consistency audit across financial model worksheet rows — detects broken formula rows (numeric value where neighbour cells are formulas), formula drift (formula changes mid-row without rationale), and row-level formula logic inconsistencies.
  WHEN: Invoke when a financial model is under audit and the reviewer needs to confirm that time-series formula rows are internally consistent across all periods, or when a suspicious single-period variance suggests a formula has been manually overridden.
---

# Model Audit — Formula Consistency

## What this skill covers

Formula-row consistency checking: verifying that each projected period in a financial model row uses the same formula structure as its neighbours, and flagging deviations that indicate a broken, overridden, or manually edited cell.

## Workflow

### Step 1 — Inventory all formula rows

For each row in the model that spans multiple time periods:
- Classify each cell as formula (`=`) or hardcode (numeric literal).
- Record the formula string for each formula cell.
- Record the cell address and period (e.g., FY2025, FY2026, ...).

### Step 2 — Identify broken formula rows

A broken formula row is any row where:
- At least two adjacent cells are formulas but one intervening cell is a numeric literal in a projection column.
- The numeric value in the gap cell is not on the permitted-hardcode list (raw historical actuals, named-assumption block drivers, universal constants 12/365/100).

Flag every broken-row cell as severity: **Critical** (if a material output line) or **Major** (if an intermediate calculation).

### Step 3 — Detect formula drift

Formula drift occurs when the formula string changes mid-row without a structural reason. Compare each cell's formula to the prior period's formula:
- Structural change = a different function, reference pattern, or operator (e.g., absolute vs relative reference switched). Flag for review.
- Parameter change = only the referenced cell address shifted by one column (normal relative-reference roll). Do not flag.
- Literal embedding = a numeric literal was introduced into the formula string (e.g., `=B5*0.25` where prior period was `=B5*Assumptions!$C$4`). Flag as **Major** — this is an embedded hardcode.

### Step 4 — Summarise by section

Group findings by model section (Income Statement, Balance Sheet, Cash Flow, Debt Schedule, Operating Model, etc.). Report:
- Total formula rows reviewed.
- Total broken cells found.
- Total formula-drift cells found.
- Worst-severity finding per section.

## Output format

```
FORMULA CONSISTENCY REPORT
--------------------------
Model: [name / file reference]
Total formula rows reviewed: [n]
Broken formula cells: [n] | Formula-drift cells: [n]

FINDINGS TABLE
| Row label | Sheet | Period | Cell | Finding type | Severity | Model value | Expected (formula) | Note |
|-----------|-------|--------|------|--------------|----------|-------------|-------------------|------|
| Revenue   | IS    | FY2026 | C10  | Broken row (hardcode in formula row) | Critical | 1,200 | =B10*(1+Growth!$C$3) | Manual override suspected |
| EBITDA Margin | IS | FY2028 | E14 | Embedded literal | Major | =E13*0.22 | =E13*Assumptions!$C$7 | Literal 0.22 embedded |
```

## Quality gates

- Every projection row spanning ≥2 periods is reviewed — no rows skipped.
- Broken cells in P&L headline rows (Revenue, EBITDA, Net Income) are always Critical.
- Embedded literals in a formula that differ from the assumption block are always Major.
- Findings table is sorted by severity (Critical first).

## Related skills

- `workflow-ma-link-tracing` — run first to build the cell inventory before formula consistency checking
- `workflow-ma-hardcode-detection` — hardcode detection is complementary and runs in parallel; share the cell inventory
- `workflow-ma-rederivation` — after consistency is confirmed, re-derivation validates the correct formula values against corp-finance-mcp tools

## Routing

**Primary agent:** `cfa-chief-analyst`
