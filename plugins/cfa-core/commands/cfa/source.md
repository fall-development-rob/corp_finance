# Deal Sourcing

Build and prioritise a deal-sourcing pipeline using the Deal Sourcing section of the `workflow-private-equity` skill.

## What It Does
Constructs a target universe based on the fund's mandate (sector, size, geography, control type), applies screening criteria (revenue band, EBITDA margin, growth, ownership status), prioritises outreach by fit and accessibility, and runs theme-driven sourcing against a thesis (e.g., consolidation play, tech enablement, succession). Output is a ranked target list with rationale and recommended approach (banker, direct, intermediated).

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-private-equity` skill.

## Key Tools
`fmp_screener`, `fmp_company_profile`, `fmp_key_metrics`, `peer_benchmarking`, `comps_analysis`, `pitchbook_company_search`, `pitchbook_deal_search`

## Usage
Provide the fund mandate (sector, EV range, geography) and any investment theme. The agent returns the ranked target list with screening scorecard and outreach plan.
