# Portfolio Rebalance

Perform a portfolio rebalancing analysis using the Rebalance workflow from `workflow-wealth-management`.

## What It Does
Produces a rebalance plan: drift assessment of current vs strategic target allocations, optimised target weights, lot-level trade list, tax-aware ordering (prioritise losses and long-term gains), wash-sale safety, and transaction-cost estimate. Also outputs before/after risk metrics so the client sees the net impact.

## Agent
Routes to `cfa-quant-risk-analyst` with `workflow-wealth-management` skill.

## Key Tools
`mean_variance_optimization`, `black_litterman_portfolio`, `risk_parity`, `risk_metrics`, `optimal_execution`, `index_rebalancing`

## Quality Standards
- Flag any asset class with >3% absolute drift; total drift threshold 5%
- Lot-level detail for tax efficiency (specific identification)
- 30-day wash-sale check across ALL household accounts
- Compare Sharpe, VaR, and max drawdown pre vs post rebalance

## Usage
Provide current holdings (with lots and cost basis), strategic target weights, and any constraints (no-sell list, tax-rate assumptions, cash flows).

$ARGUMENTS
