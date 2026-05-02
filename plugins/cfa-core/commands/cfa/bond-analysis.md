# Bond Analysis

Perform fixed income analysis using the `corp-finance-tools-markets` skill. Covers bond pricing, yield analytics, duration/convexity, spread decomposition, and rate sensitivity.

## What It Does
Executes a multi-step fixed income assessment: (1) Bond pricing and yield-to-maturity calculation, (2) Yield curve construction and interpolation, (3) Duration (Macaulay, modified, effective) and convexity computation, (4) Spread analysis (G-spread, Z-spread, OAS, ASW), (5) Rate sensitivity and scenario analysis under parallel and non-parallel curve shifts.

## Agent
Routes to `cfa-fixed-income-analyst` with `corp-finance-tools-markets` skill.

## Key Tools
`bond_pricing`, `yield_curve`, `duration_convexity`, `spread_analysis`, `tips_pricing`, `inflation_derivatives`, `repo_rates`, `short_rate_model`, `term_structure_model`, `mbs_analytics`, `prepayment_model`, `sensitivity_matrix`, `fred_treasury_rates`, `fred_yield_curve`

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
