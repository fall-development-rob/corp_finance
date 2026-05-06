# Returns Analysis

Run a full returns analysis on a PE deal using the Returns Analysis section of the `workflow-private-equity` skill.

## What It Does
Builds the gross IRR and MOIC, walks gross-to-net (management fee, carry, fund expenses) to LP net returns, attributes returns by year and by lever (multiple expansion, EBITDA growth, debt paydown, FCF), and runs sensitivities on entry multiple, exit multiple, leverage, and hold period. Output is a returns waterfall, attribution table, and bull/base/bear sensitivity grid.

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-private-equity` skill.

## Key Tools
`calculate_returns`, `lbo_model`, `gp_economics`, `investor_net_returns`, `build_sensitivity_grid`, `sources_uses`, `debt_schedule`, `waterfall_calculator`

## Usage
Provide entry assumptions (purchase price, leverage, fees), operating projections, and exit assumptions. The agent returns the IRR/MOIC build, gross-to-net bridge, attribution, and sensitivity output.
