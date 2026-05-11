# Portfolio Monitoring

Monitor a portfolio company using the Portfolio Monitoring section of the `workflow-private-equity` skill.

## What It Does
Builds a board-pack-ready monitoring summary: KPI dashboard (revenue, EBITDA, cash, leverage, working-capital days, key operating metrics), variance vs plan and vs prior quarter, value-creation-plan progress against initiatives (commercial, operational, M&A, exit-readiness), covenant headroom check, and a watchlist flag if any KPI is off-track.

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-private-equity` skill.

## Key Tools
`variance_analysis`, `peer_benchmarking`, `credit_metrics`, `altman_zscore`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `calculate_dupont`

## Usage
Provide the portfolio company's most recent monthly/quarterly results, the original VCP, and the latest budget. The agent returns the KPI dashboard, variance bridge, VCP progress table, and watchlist verdict.
