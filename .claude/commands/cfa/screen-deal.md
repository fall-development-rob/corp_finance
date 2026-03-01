# Deal Screening

Screen inbound deal flow using the Deal Screening workflow from `workflow-private-equity`.

## What It Does
Extracts key deal facts, runs pass/fail against fund criteria, performs quick valuation and credit check, produces a one-page screening memo with verdict (Proceed/Further DD/Pass) and bull/bear cases.

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-private-equity` skill.

## Key Tools
`credit_metrics`, `altman_zscore`, `comps_analysis`, `fmp_key_metrics`

## Usage
Provide the deal details (from CIM, teaser, or broker package).
