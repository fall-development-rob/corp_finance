# Catalyst Calendar

Build a forward 12-month catalyst calendar using the Catalyst Calendar section of the `workflow-equity-research` skill.

## What It Does
Assembles a dated catalyst list for the covered name(s): scheduled earnings dates, capital markets days / investor days, FDA or regulatory milestones, product launches, contract renewals, lock-up expiries, and competitor events. Each catalyst is tagged with type, date (or window), confidence (confirmed/expected/speculative), and ranked by potential price impact (high/medium/low).

## Agent
Routes to `cfa-equity-analyst` with `workflow-equity-research` skill.

## Key Tools
`fmp_earnings_calendar`, `fmp_economic_calendar`, `fmp_ipo_calendar`, `fmp_press_releases`, `fmp_sec_filings`, `build_sensitivity_grid`

## Usage
Provide a ticker (or coverage list). The agent will return a dated catalyst table with impact ranking and a short note on the bull/bear read for each item.
