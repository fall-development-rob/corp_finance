# Unit Economics

Tear down target-company unit economics using the Unit Economics section of the `workflow-private-equity` skill.

## What It Does
Decomposes the business into per-unit drivers: customer acquisition cost (CAC), lifetime value (LTV), LTV/CAC ratio, payback period, cohort retention curves, contribution margin per unit, and channel mix (paid vs organic, direct vs partner). Benchmarks each metric against peer set and flags deterioration vs prior cohorts.

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-private-equity` skill.

## Key Tools
`calculate_dupont`, `peer_benchmarking`, `variance_analysis`, `fmp_key_metrics`, `fmp_ratios`, `comps_analysis`

## Usage
Provide cohort-level revenue, customer counts, marketing spend, and gross margin data (or a data-room extract). The agent returns the unit-economics scorecard, cohort table, and peer-benchmarked verdict.
