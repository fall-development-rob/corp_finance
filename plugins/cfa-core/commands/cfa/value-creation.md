---
requires_tools:
  - build_lbo
  - build_sensitivity_grid
  - calculate_returns
---
# Value Creation Plan

Build a post-acquisition value creation plan using the VCP workflow from `workflow-private-equity`.

## What It Does
Produces a value creation plan with revenue levers, cost levers, EBITDA bridge (Year 0-5), 100-day priorities, and KPI dashboard with monthly milestones.

## Agent
Routes to `cfa-private-markets-analyst`.

## Key Tools
`build_lbo`, `build_sensitivity_grid`, `calculate_returns`

## Usage
Provide the portfolio company details, entry financials, and strategic priorities.
