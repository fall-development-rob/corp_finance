# Client Report

Produce a quarterly client performance report using the Client Report workflow from `workflow-wealth-management`.

## What It Does
Generates a 5-8 page quarterly report covering portfolio returns vs benchmark (period, QTD, YTD, 1Y, 3Y, 5Y, ITD), top contributors and detractors, current asset allocation, holdings activity (buys/sells/distributions), fee summary (gross vs net), and market commentary contextualising performance.

## Agent
Routes to `cfa-quant-risk-analyst` with `workflow-wealth-management` skill.

## Key Tools
`risk_adjusted_returns`, `risk_metrics`, `calculate_brinson_attribution`, `factor_attribution`

## Quality Standards
- Net-of-fee returns shown alongside gross returns
- Allocation, selection, and interaction effects from Brinson attribution
- Factor-based decomposition (market, size, value, momentum)
- Every return number traces to a tool call

## Usage
Provide client name, account(s), reporting period, benchmark, and holdings/transactions for the period.

$ARGUMENTS
