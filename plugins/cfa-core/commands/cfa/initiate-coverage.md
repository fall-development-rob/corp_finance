---
requires_tools:
  - build_sensitivity_grid
  - build_three_statement
  - calculate_sotp
  - calculate_target_price
  - comps_analysis
  - dcf_model
  - run_mc_dcf
  - wacc_calculator
requires_external_tools:
  - fmp_balance_sheet
  - fmp_cash_flow
  - fmp_income_statement
  - fmp_key_metrics
  - fmp_profile
---
# Initiate Coverage

Invoke the Initiating Coverage workflow from the `workflow-equity-research` skill to create an institutional-quality equity research initiation report.

## What It Does
Executes a 5-task pipeline: (1) Company research, (2) Financial modelling, (3) Valuation analysis, (4) Exhibit generation, (5) Report assembly. Each task runs individually with verified prerequisites.

## Agent
Routes to `cfa-equity-analyst` with `workflow-equity-research` skill.

## Key Tools
`fmp_profile`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `fmp_key_metrics`, `wacc_calculator`, `dcf_model`, `comps_analysis`, `build_three_statement`, `calculate_sotp`, `calculate_target_price`, `build_sensitivity_grid`, `run_mc_dcf`

## Usage
Provide a company name or ticker. The agent will ask which task to start with.
