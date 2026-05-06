# Model Update

Refresh an existing equity model post-earnings using the `workflow-equity-research` skill.

## What It Does
Executes a quarterly model refresh after the print: updates revenue/margin/growth assumptions to the as-reported quarter, refreshes the comps set with current trading multiples, recomputes the DCF and target price, and flags any thesis violations (margin compression, guidance cut, KPI miss vs prior bull/base/bear).

## Agent
Routes to `cfa-equity-analyst` with `workflow-equity-research` skill.

## Key Tools
`fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `fmp_key_metrics`, `dcf_model`, `comps_analysis`, `calculate_target_price`, `three_statement_model`, `wacc_calculator`, `build_sensitivity_grid`

## Usage
Provide the ticker and the most recent reporting period. The agent will pull the new financials, refresh the model, and produce a delta vs prior estimates with any thesis-violation flags.
