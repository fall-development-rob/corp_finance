---
requires_tools:
  - analyze_merger
  - build_lbo
  - build_sensitivity_grid
  - comps_analysis
  - dcf_model
---
# Pitch Deck

Structure a pitch book using the Pitch Deck workflow from `workflow-investment-banking`.

## What It Does
Creates pitch book structure: Situation Overview, Market Context, Valuation Analysis (football field), Transaction Structure, Execution Timeline. Uses multiple valuation methods for comparison.

## Agent
Routes to `cfa-private-markets-analyst`.

## Key Tools
`dcf_model`, `comps_analysis`, `build_lbo`, `analyze_merger`, `build_sensitivity_grid`

## Usage
Provide the deal context and pitch objective (sell-side, buy-side, financing, restructuring).
