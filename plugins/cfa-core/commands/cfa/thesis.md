---
requires_tools:
  - build_sensitivity_grid
  - comps_analysis
  - dcf_model
requires_external_tools:
  - fmp_analyst_estimates
  - fmp_quote
---
# Thesis Tracker

Build and maintain an investment thesis using the Thesis Tracker workflow from `workflow-equity-research`.

## What It Does
Creates a structured bull/base/bear thesis with catalyst milestones, probability weightings, and quarterly update cadence.

## Agent
Routes to `cfa-equity-analyst`.

## Key Tools
`dcf_model`, `comps_analysis`, `build_sensitivity_grid`, `fmp_quote`, `fmp_analyst_estimates`

## Usage
Provide a ticker to create a new thesis or update an existing one.
