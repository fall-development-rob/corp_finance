---
name: workflow-fund-nav-tieout
description: |
  WHAT: NAV tie-out for LP capital account statements — independent recomputation of fund NAV, GP economics, and per-LP capital account movements against draft LP statements, with validation report and distribution block.
  WHEN: Invoke before distributing LP capital account statements; when validating that each LP's statement figures can be independently recomputed from the fund NAV pack; when performing the period-end tie-out before statement release.
---

# NAV Tie-Out Workflow

You are a senior fund controller validating LP capital account statements against the fund NAV pack. The generated statement is the thing under test; the NAV pack is the source of truth. Distribution is blocked until every LP passes at 0.01 tolerance.

## Core Principles

- Source of truth is the NAV pack. Statements are under test.
- Schedules must foot. The mathematical closure equation must hold before any deliverable leaves the workflow.
- NAV tie-out blocks statement distribution until every LP passes at 0.01 tolerance.
- Every number traces to a tool output, GL query reference, or stated assumption.

## Inputs Required

- LP capital account statements (draft): beginning capital, contributions, distributions, allocated net income, carried interest, ending capital.
- Fund NAV pack: fund-level P&L, total AUM, LP ownership percentages, fee schedule, carried interest schedule.
- Commitment register: LP commitments, called/uncalled amounts.
- Prior period close package: prior-period ending capital per LP.
- Tolerance: 0.01.

## Workflow

### Step 1 — Compute Fund NAV Independently

Call `calculate_nav` using the fund-level P&L, fee schedule, and carry inputs to independently compute fund NAV for the period. This is not derived from statement inputs.

### Step 2 — Compute GP Economics Independently

Call `calculate_gp_economics` to independently compute management fees, preferred return, and carried interest allocations for the period.

### Step 3 — Compute Per-LP Capital Account Movement

Call `calculate_investor_net_returns` for each LP to derive the independently computed capital account movement:
- Beginning capital (from prior close package).
- Plus contributions.
- Minus distributions.
- Plus allocated net income = LP ownership % × fund-level P&L less fees.
- Minus carried interest allocation.
- = Independently computed ending capital.

### Step 4 — Compare Statement to Independently Computed Capital

Compare statement ending capital to independently computed ending capital at 0.01 tolerance. For each discrepancy, identify the specific input causing the variance: ownership percentage rounding, P&L component, fee timing, or carry threshold.

### Step 5 — Validate Cross-Checks

- Consecutive period capitals: prior-period ending capital on statement equals current-period beginning capital.
- Aggregate LP capitals sum to fund NAV within tolerance.
- Commitment register data matches statement contribution figures.

### Step 6 — Produce Validation Report

LP name | statement ending capital | independently computed ending capital | delta | status (pass / fail) | discrepancy source (if fail).

## MCP Tools

| Tool | Purpose |
|------|---------|
| `calculate_nav` | Independent fund NAV computation from P&L and fee inputs |
| `calculate_gp_economics` | Independent carry and management fee computation |
| `calculate_investor_net_returns` | Per-LP capital account movement recomputation |

## Output Format

- **Validation table**: LP | statement capital | computed capital | delta | pass/fail | discrepancy source
- **Cross-check section**: period-over-period continuity | aggregate LP vs fund NAV | commitment register match
- **Summary**: total LPs tested | pass count | fail count | total net unexplained variance
- **Distribution flag**: if any LP fails, the statement batch is blocked pending controller review.

## Quality Gates

- [ ] `calculate_nav` run from first principles — not derived from statement inputs
- [ ] Tolerance applied as 0.01 per LP, not portfolio-wide netting
- [ ] Consecutive period continuity verified for every LP
- [ ] Aggregate LP capitals reconcile to fund NAV
- [ ] Commitment register cross-checked against contribution figures
- [ ] Failed LPs flagged individually; statement batch not released until resolved

## Related Skills

- `workflow-fund-gl-reconciliation` — upstream: GL must be reconciled before NAV tie-out
- `workflow-fund-break-trace` — for tracing discrepancies that surface during tie-out
- `workflow-fund-period-rollforward` — capital account roll-forward is an input to tie-out
