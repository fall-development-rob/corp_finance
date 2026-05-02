# Initiate Coverage

Invoke the Initiating Coverage workflow from the `workflow-equity-research` skill to create an institutional-quality equity research initiation report.

## What It Does
Executes a 5-task pipeline: (1) Company research, (2) Financial modelling, (3) Valuation analysis, (4) Exhibit generation, (5) Report assembly. Each task runs individually with verified prerequisites.

## Agent
Routes to `cfa-equity-analyst` with `workflow-equity-research` skill.

## Key Tools
`fmp_profile`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `fmp_key_metrics`, `wacc_calculator`, `dcf_model`, `comps_analysis`, `three_statement_model`, `sotp_valuation`, `target_price`, `sensitivity_matrix`, `monte_carlo_dcf`

## Usage
Provide a company name or ticker. The agent will ask which task to start with.
