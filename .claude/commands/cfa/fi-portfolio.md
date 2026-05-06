# Fixed-Income Portfolio Review

Review a fixed-income portfolio using the `corp-finance-analyst-markets` skill.

## What It Does
Executes a fixed-income portfolio review: (1) duration and convexity exposure (modified, effective, spread duration), (2) key-rate-duration breakdown across the curve buckets, (3) credit-quality distribution and sector concentration, (4) yield-to-worst, current yield, and book yield, (5) scenario VaR under parallel and non-parallel shifts plus credit-spread widening, (6) cash-flow ladder and reinvestment risk.

## Agent
Routes to `cfa-fixed-income-analyst` with `corp-finance-analyst-markets` skill.

## Key Tools
`bond_pricer`, `bond_duration`, `credit_spreads`, `scenario_analysis`, `bond_duration`, `lseg_bond_pricing`

## Quality Standards
- Portfolio aggregates weight-summed, not equally weighted
- Show effective duration where bonds have embedded options (callables, MBS)
- Credit quality reported by issue rating AND issuer rating
- At least three scenarios (+/-50bps parallel, steepener, +100bps spread widening)

## Usage
Provide the portfolio holdings (issuer, coupon, maturity, par, price/yield, rating) and benchmark (e.g., Agg, Treasury, custom).

$ARGUMENTS
