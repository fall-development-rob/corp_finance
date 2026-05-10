---
name: workflow-fa-excel-authoring
description: |
  WHAT: Excel financial model authoring conventions — runtime detection (Office JS vs openpyxl), computation via cfa-core MCP tools, merged-cell pitfall fix pattern, formulas-over-hardcodes constraint, colour conventions, and step-by-step confirmation gates.
  WHEN: Invoke when writing or structuring an Excel financial model; when building DCF, LBO, or three-statement workbooks using Office JS or openpyxl; when enforcing formula discipline, handling merged cells, or setting up user confirmation gates during model construction.
---

# Excel Authoring Conventions

You are building institutional-grade Excel financial models. All financial math runs in the Rust core via cfa-core MCP tools; Excel is the output medium only.

## Runtime Selection

Identify the authoring environment before writing any cell.

| Environment | Detection | Write method |
|---|---|---|
| **Office JS** (live Excel session) | Add-in context present, `Office.context` available | `range.formulas = [["=formula_string"]]` |
| **openpyxl** (standalone .xlsx) | No live session, generating a file for download | `cell.value = "=formula_string"` then run `recalc.py` |

Confirm the environment at the start of every Excel authoring task. Never silently assume one mode.

## Computation Source: cfa-core MCP Tools

All financial math lives in the Rust core, exposed as MCP tools. The correct flow is:

1. **Compute** via cfa-core MCP tools (`dcf_model`, `lbo_model`, `three_statement_model`, etc.).
2. **Receive** structured JSON output containing schedule rows, rates, and derived values.
3. **Write to Excel** using Office JS or openpyxl as the output medium — translating MCP results into formula strings and cell ranges.

Never re-implement DCF discounting, LBO debt scheduling, or three-statement linkage in Python. That math already runs in 128-bit Decimal precision inside Rust.

## Merged-Cell Pitfall and Fix Pattern

Office JS raises `InvalidArgument` when writing an array onto a range containing a merged cell.

Fix pattern (always apply when writing near merged cells):
1. Unmerge the target range before writing: `range.unmerge()`.
2. Write the value or formula to the top-left single cell only.
3. Re-merge to the desired span: `range.merge(Excel.MergeBy.across)`.
4. Never pass a multi-element array to a merged range.

## Formulas-Over-Hardcodes Constraint

Every projection cell, roll-forward, linkage, and sensitivity output must be a live Excel formula — not a pre-computed value pasted in as a literal.

Hardcodes are permitted only for:
- Raw historical actuals sourced directly from filings.
- Discrete assumption drivers in a named assumptions block.
- Universal constants (12, 365, 100).

Anti-pattern detection heuristics — flag any cell that:
- Is numeric but its neighbours in the same row/column are formulas (broken formula row).
- Contains a value equal to a formula result elsewhere but has no `=` prefix (shadow hardcode).
- Embeds a literal rate, multiple, or growth figure inside a formula string (e.g., `=B5*0.25` where `0.25` is an assumption).
- Is in a projection column (FYxxE / FYxxF) but has no formula bar content.

## Colour Convention

- Blue font: hardcoded inputs (assumption cells).
- Black font: formula outputs (calculated cells).
- Green font: cross-sheet references.

## Step-by-Step Confirmation Gates

Work section-by-section and pause for user confirmation before advancing.

| Gate | What to show before proceeding |
|---|---|
| 1. Raw inputs | Assumption block with sources and cell addresses |
| 2. Revenue / operating projections | Revenue build, growth rates, margin schedule |
| 3. FCF or EBITDA schedule | Full bridge from revenue to free cash flow |
| 4. WACC / returns calculation | Cost of equity, cost of debt, weights, blended rate |
| 5. Valuation bridge | EV → equity value → per-share or MOIC/IRR |
| 6. Sensitivity / scenario tables | All populated cells, confirming formulas not literals |

Do not proceed past any gate unless the user explicitly confirms. Record confirmation in the chat before continuing.

## Quality Gates

- [ ] Authoring environment confirmed before first cell write
- [ ] All financial math computed via cfa-core MCP tools — no re-implementation in Python
- [ ] Merged cells unmerged before array writes, then re-merged
- [ ] No hardcoded values in projection cells (only in assumption block and universal constants)
- [ ] Colour convention applied: blue inputs, black formulas, green cross-sheet refs
- [ ] Each confirmation gate shown to user and explicitly approved before proceeding

## Related Skills

- `workflow-fa-model-checking` — auditing completed models for formula and balance sheet errors
- `workflow-xlsx-author` — tabular markdown and CSV authoring for headless Excel-equivalent deliverables
- `corp-finance-tools-core` — `dcf_model`, `lbo_model`, `three_statement_model` tool references
