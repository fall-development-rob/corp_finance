# Earnings Analysis

Post-earnings update report using the Earnings Analysis workflow from `workflow-equity-research`.

## What It Does
Analyzes quarterly earnings: beat/miss summary, guidance revisions, updated estimates, thesis impact assessment. Produces an 8-12 page update note.

## Agent
Routes to `cfa-equity-analyst`.

## Key Tools
`fmp_earnings`, `fmp_analyst_estimates`, `fmp_income_statement`, `sensitivity_matrix`, `target_price`

## Usage
Provide ticker and quarter (e.g., "AAPL Q4 2025"). The agent retrieves actual vs estimate data and produces the update.
