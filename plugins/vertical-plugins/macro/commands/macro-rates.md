# Macro Rates Monitor

Produce a macro rates dashboard using the `vendor-lseg` and `corp-finance-analyst-markets` skills.

## What It Does
Executes a macro-rates monitor: (1) yield curve shape across G3+UK (steepener/flattener signals, 2s10s and 3m10y), (2) real-rate vs breakeven inflation decomposition, (3) central bank policy stance and Taylor-rule fair value, (4) terminal rate pricing from OIS and SOFR/ESTR/SONIA futures, (5) cross-market relative value and curve regime classification.

## Agent
Routes to `cfa-fixed-income-analyst` with `vendor-lseg` and `corp-finance-analyst-markets` skills.

## Key Tools
`lseg_economic_indicators`, `lseg_yield_curve`, `nelson_siegel_fit`, `taylor_rule`, `term_structure_fit`

## Quality Standards
- Cite policy rate, last meeting date, and forward guidance
- Real vs breakeven decomposition uses TIPS/linker market data, not survey
- Taylor-rule fair value shown with output gap and inflation gap inputs
- Flag divergence between OIS-implied terminal rate and central bank dots/MPC

## Usage
Provide jurisdiction(s) (US/EUR/UK/JP/etc.), horizon (next meeting, next 12m), and any specific theme (cut cycle, hike cycle, hold).

$ARGUMENTS
