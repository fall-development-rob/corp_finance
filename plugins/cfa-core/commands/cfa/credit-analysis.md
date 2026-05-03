---
requires_tools:
  - altman_zscore
  - build_sensitivity_grid
  - calculate_cecl_provision
  - calculate_intensity_model
  - calculate_merton
  - calculate_migration
  - calculate_portfolio_credit_risk
  - calculate_scorecard
  - covenant_compliance
  - credit_metrics
  - debt_capacity
requires_external_tools:
  - fmp_balance_sheet
  - fmp_cash_flow
  - fmp_income_statement
  - fmp_key_metrics
---
# Credit Analysis

Perform a comprehensive credit analysis using the `corp-finance-tools-core` and `corp-finance-tools-regulatory` skills. Covers credit metrics, default risk, debt capacity, and covenant compliance.

## What It Does
Executes a multi-step credit assessment: (1) Credit metrics computation (leverage, coverage, liquidity), (2) Altman Z-score distress screening, (3) Debt capacity analysis under stress scenarios, (4) Covenant compliance testing, (5) Default probability estimation (Merton structural + intensity models), (6) Credit scoring and rating assignment.

## Agent
Routes to `cfa-credit-analyst` with `corp-finance-tools-core` and `corp-finance-tools-regulatory` skills.

## Key Tools
`credit_metrics`, `altman_zscore`, `debt_capacity`, `covenant_compliance`, `calculate_scorecard`, `calculate_merton`, `calculate_intensity_model`, `calculate_cecl_provision`, `calculate_portfolio_credit_risk`, `calculate_migration`, `build_sensitivity_grid`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `fmp_key_metrics`

## Quality Standards
- Cite every ratio and score to the specific tool call that produced it
- State all assumptions (recovery rates, correlation, risk-free rate, asset volatility)
- Flag any input data gaps and their impact on conclusions
- Cross-check Altman Z-score zone against Merton-implied default probability
- Present debt capacity under base, downside, and stress scenarios
- Compare derived metrics to sector medians where available

## Usage
Provide the company name, ticker, or raw financial metrics (revenue, EBITDA, total debt, interest expense, cash). The agent will compute all credit dimensions and synthesize a credit opinion.

$ARGUMENTS
