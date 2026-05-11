# Credit Analysis

Perform a comprehensive credit analysis using the `corp-finance-tools-core` and `corp-finance-tools-regulatory` skills. Covers credit metrics, default risk, debt capacity, and covenant compliance.

## What It Does
Executes a multi-step credit assessment: (1) Credit metrics computation (leverage, coverage, liquidity), (2) Altman Z-score distress screening, (3) Debt capacity analysis under stress scenarios, (4) Covenant compliance testing, (5) Default probability estimation (Merton structural + intensity models), (6) Credit scoring and rating assignment.

## Agent
Routes to `cfa-credit-analyst` with `corp-finance-tools-core` and `corp-finance-tools-regulatory` skills.

## Key Tools
`credit_metrics`, `altman_zscore`, `debt_capacity`, `covenant_compliance`, `credit_scorecard`, `merton_model`, `intensity_model`, `cecl_provisioning`, `credit_portfolio_var`, `rating_migration`, `sensitivity_matrix`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `fmp_key_metrics`

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
