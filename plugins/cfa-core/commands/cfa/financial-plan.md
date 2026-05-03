---
requires_tools:
  - build_sensitivity_grid
  - plan_estate
  - plan_retirement
  - risk_metrics
  - run_monte_carlo
---
# Financial Plan

Build a comprehensive financial plan using the Financial Plan workflow from `workflow-wealth-management`.

## What It Does
Produces a 15-25 page financial plan covering cash flow analysis, retirement projection, education funding, estate planning, Monte Carlo simulation, insurance review, tax optimization, and prioritized recommendations.

## Agent
Routes to `cfa-quant-risk-analyst` with `workflow-wealth-management` skill.

## Key Tools
`plan_retirement`, `plan_estate`, `run_monte_carlo`, `build_sensitivity_grid`, `risk_metrics`

## Usage
Provide client profile: age, income, assets, liabilities, risk tolerance, and financial goals.
