# Bond Relative Value

Screen and construct bond relative-value trades using the `corp-finance-analyst-markets` skill.

## What It Does
Executes a bond RV workflow: (1) cheap/rich screening across the candidate universe vs peer curve, (2) asset-swap spread (ASW), Z-spread, and OAS comparison, (3) credit-curve fit per issuer/sector to identify outliers, (4) pair-trade construction (long cheap / short rich) with DV01 and spread-DV01 weighting, (5) carry, roll, and breakeven calculation, (6) liquidity and crossover risk flags.

## Agent
Routes to `cfa-fixed-income-analyst` with `corp-finance-analyst-markets` skill.

## Key Tools
`bond_pricer`, `credit_spreads`, `bond_yield`, `bond_duration`, `lseg_bond_pricing`, `lseg_credit_spreads`

## Quality Standards
- Cite reference curve and benchmark for each spread
- Show ASW, Z-spread, and OAS side by side
- Pair trades duration- AND spread-duration-neutral
- Breakeven spread move and expected holding-period return

## Usage
Provide bond universe (issuer, sector, or rating bucket), reference curve (UST/Bund/Gilt/SOFR), holding horizon, and any constraints (ratings, liquidity, ESG).

$ARGUMENTS
