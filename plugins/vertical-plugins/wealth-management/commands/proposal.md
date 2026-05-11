# Investment Proposal

Build an investment proposal for a new prospect or account using the Investment Proposal workflow from `workflow-wealth-management`.

## What It Does
Produces a recommendation document with risk-tolerance fit assessment, model portfolio recommendation, expected return and risk profile (base/bull/bear), portfolio fit and correlation analysis, fee schedule, and clear conviction-rated recommendation with suggested position size and funding source.

## Agent
Routes to `cfa-quant-risk-analyst` with `workflow-wealth-management` skill.

## Key Tools
`mean_variance_optimization`, `risk_parity`, `risk_metrics`, `sensitivity_matrix`, `black_litterman_portfolio`

## Quality Standards
- Risk tolerance assessed (capacity vs willingness) before recommendation
- Base/bull/bear scenarios with explicit probability weights
- Portfolio-level impact on Sharpe, VaR, and concentration shown
- Suitability rationale ties to client goals and constraints

## Usage
Provide prospect profile (age, income, assets, liabilities, goals, risk tolerance) and the candidate strategy or model portfolio.

$ARGUMENTS
