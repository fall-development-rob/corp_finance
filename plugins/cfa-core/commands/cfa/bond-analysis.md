---
requires_tools:
  - analyze_inflation_derivatives
  - analyze_prepayment
  - analyze_repo
  - analyze_short_rate
  - analyze_spreads
  - analyze_tips
  - bootstrap_spot_curve
  - build_sensitivity_grid
  - calculate_duration
  - fit_term_structure
  - price_bond
requires_external_tools:
  - fred_treasury_rates
  - fred_yield_curve
---
# Bond Analysis

Perform fixed income analysis using the `corp-finance-tools-markets` skill. Covers bond pricing, yield analytics, duration/convexity, spread decomposition, and rate sensitivity.

## What It Does
Executes a multi-step fixed income assessment: (1) Bond pricing and yield-to-maturity calculation, (2) Yield curve construction and interpolation, (3) Duration (Macaulay, modified, effective) and convexity computation, (4) Spread analysis (G-spread, Z-spread, OAS, ASW), (5) Rate sensitivity and scenario analysis under parallel and non-parallel curve shifts.

## Agent
Routes to `cfa-fixed-income-analyst` with `corp-finance-tools-markets` skill.

## Key Tools
`price_bond`, `bootstrap_spot_curve`, `calculate_duration`, `analyze_spreads`, `analyze_tips`, `analyze_inflation_derivatives`, `analyze_repo`, `analyze_short_rate`, `fit_term_structure`, `analyze_prepayment`, `build_sensitivity_grid`, `fred_treasury_rates`, `fred_yield_curve`

## Quality Standards
- Cite every price, yield, and spread to the specific tool call that produced it
- State all assumptions (day count convention, compounding frequency, settlement date, recovery assumption for spreads)
- Show duration and convexity approximation alongside closed-form where applicable
- Present rate sensitivity under at least three scenarios (+/-50bps, +/-100bps, +/-200bps)
- Identify key risk drivers (spread duration, curve risk, roll-down)
- Flag any liquidity or data quality concerns

## Usage
Provide bond details (coupon, maturity, price or yield, face value) or a portfolio of bonds. The agent will compute full analytics and produce a risk summary.

$ARGUMENTS
