---
name: "workflow-ma-link-tracing"
description: |
  WHAT: Dependency-graph construction and circular-reference assessment for financial model worksheets — directed cell/formula link tracing, cycle detection, and classification of legitimate vs pathological circular references.
  WHEN: Invoke when a financial model is presented for audit and the reviewer needs to map which cells feed which, detect unintended formula cycles, or confirm that iterative-calculation cycles (interest-expense circularity, revolver draw circularity) converge correctly.
---

# Model Audit — Link Tracing and Circular Reference Assessment

## What this skill covers

Construction of the directed dependency graph for a financial model, identification of all formula cycles (circular references), and classification of each cycle as either a legitimate (convergent) model mechanic or a pathological (non-convergent or unintended) error.

## Workflow

### Step 1 — Build the dependency map

For every formula cell in the model:
- Record all direct precedent cells (inputs to the formula).
- Record the formula string (for display in the output report).
- Label each cell with its sheet name and cell address (`Sheet!A1` notation).

Construct a directed acyclic graph (DAG) of all non-circular dependencies. The output is an adjacency list suitable for cycle detection.

### Step 2 — Detect circular references

Apply a depth-first search (DFS) with cycle tracking on the full dependency graph. Every back-edge marks a circular reference. Record:
- The full cycle path (`Sheet1!B10 → Sheet2!C5 → Sheet1!B10`).
- The cycle length (number of edges).
- All cells participating in the cycle.

### Step 3 — Classify each cycle

| Cycle type | Description | Disposition |
|------------|-------------|-------------|
| Legitimate — interest expense | Interest expense → net income → cash → debt balance → interest expense | Acceptable if iterative calculation is ON and converges in ≤10 iterations |
| Legitimate — revolver draw | Revolver draw → ending cash → revolver draw | Acceptable if iterative calculation is ON and converges in ≤10 iterations |
| Pathological — non-convergent | Any cycle where iteration does not reach a stable solution within 10 iterations | Critical finding |
| Pathological — unintended | Revenue → cost → revenue; any business-logic cycle with no modelling rationale | Critical finding |

### Step 4 — Verify convergence of legitimate cycles

For each legitimate cycle, check that:
- The model's workbook has iterative calculation enabled.
- The maximum iteration count is ≥5 (recommended: 100).
- The maximum change threshold is ≤0.001.
- Manually stepping through 5 iterations shows a converging sequence.

Non-convergence within 10 iterations is a critical finding regardless of cycle classification.

### Step 5 — Produce the dependency map summary

Output:
- Total cells in the model, formula cells vs hard-coded cells.
- Count of distinct circular references.
- Per-cycle table: path, length, classification, convergence status, severity.
- Flattened precedent list for each output cell (show all upstream inputs).

## Output format

```
LINK-TRACE REPORT
-----------------
Model: [name / file reference]
Total formula cells: [n]
Total circular references detected: [n]

CYCLE TABLE
| Cycle ID | Path | Length | Classification | Converges? | Severity |
|----------|------|--------|----------------|------------|----------|
| C1 | Sheet1!B10 → Sheet2!C5 → Sheet1!B10 | 2 | Legitimate (interest) | Yes | — |
| C2 | Rev!D8 → IS!C12 → Rev!D8 | 2 | Legitimate (revolver) | Yes | — |
| C3 | Rev!E8 → Rev!E9 → Rev!E8 | 2 | Pathological (unintended) | N/A | CRITICAL |

PRECEDENT MAP (selected output cells)
| Output cell | All upstream inputs |
|-------------|---------------------|
| IS!C20 (Net Income) | IS!C5, IS!C8, IS!C14, Tax!B3, ... |
```

## Quality gates

- Every formula cell has a recorded precedent list — no cells skipped.
- Every cycle is classified; no cycle is left as "unknown."
- Legitimate cycles have convergence confirmed; non-confirmation is a major finding.
- Pathological cycles are always critical.

## Related skills

- `workflow-ma-formula-consistency` — once the link map is built, consistency checks run across formula rows
- `workflow-ma-hardcode-detection` — hardcodes are identified from the same cell inventory used here
- `workflow-ma-three-statement-tieout` — the articulation check uses the dependency map to confirm IS/BS/CF linkages
- `workflow-ma-rederivation` — re-derivation uses corp-finance-mcp tools as ground truth after link-tracing confirms model topology

## Routing

**Primary agent:** `cfa-chief-analyst`
