# Swap Curve Strategy

Construct swap-curve relative-value trades using the `vendor-lseg` and `corp-finance-analyst-markets` skills.

## What It Does
Executes a curve strategy workflow: (1) spot vs forward curve analysis (1m/3m/6m/1y forwards), (2) curve shape decomposition (level/slope/curvature) and Nelson-Siegel/Svensson fit, (3) butterfly and spread trade identification (2s5s10s, 5s10s30s), (4) duration-neutral construction with DV01 weights, (5) carry-and-roll analysis over the holding horizon, (6) scenario analysis under parallel and non-parallel shifts.

## Agent
Routes to `cfa-fixed-income-analyst` with `vendor-lseg` and `corp-finance-analyst-markets` skills.

## Key Tools
`lseg_yield_curve`, `nelson_siegel_fit`, `term_structure_fit`, `interest_rate_swap`, `scenario_analysis`, `bond_duration`

## Quality Standards
- Cite curve source, build date, and interpolation method
- Show DV01 for each leg and net DV01 of the package
- Carry-and-roll separated from outright directional P&L
- At least three scenarios (steepener, flattener, parallel +/-50bps)

## Usage
Provide curve currency (USD/EUR/GBP), tenor pair or butterfly, holding horizon, and any view to bias the trade.

$ARGUMENTS
