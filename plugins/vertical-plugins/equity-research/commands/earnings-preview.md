# Earnings Preview

Produce a pre-earnings preview note using the `workflow-equity-research` skill ahead of an upcoming print.

## What It Does
Builds a structured preview ahead of the earnings release: consensus estimates (revenue/EPS/key segment KPIs), whisper numbers vs published consensus, scenario tree (beat / in-line / miss with implied price impact), key questions for the call, and a historical earnings-reaction table (1-day and 5-day moves over prior 8 quarters).

## Agent
Routes to `cfa-equity-analyst` with `workflow-equity-research` skill.

## Key Tools
`fmp_earnings_calendar`, `fmp_analyst_estimates`, `fmp_earnings_surprises`, `fmp_historical_price`, `build_sensitivity_grid`, `calculate_target_price`, `comps_analysis`

## Usage
Provide a ticker and the upcoming reporting date. The agent will pull consensus, build the scenario tree, and draft the preview note.
