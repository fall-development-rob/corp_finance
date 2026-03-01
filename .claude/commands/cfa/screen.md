# Equity Screening

Investment idea generation and screening using the Idea Generation workflow from `workflow-equity-research`.

## What It Does
Runs quantitative screens (Piotroski F-Score, Beneish M-Score, ROIC vs WACC), applies thematic filters, and produces a ranked list of investment candidates with one-paragraph thesis per idea.

## Agent
Routes to `cfa-equity-analyst`.

## Key Tools
`piotroski_fscore`, `beneish_mscore`, `fmp_ratios`, `fmp_key_metrics`, `fmp_stock_screener`

## Usage
Specify screening criteria (e.g., sector, market cap range, minimum quality score) or use defaults.
