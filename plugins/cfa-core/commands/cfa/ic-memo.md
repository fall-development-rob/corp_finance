---
requires_tools:
  - altman_zscore
  - build_debt_schedule
  - build_lbo
  - build_sensitivity_grid
  - calculate_returns
  - calculate_waterfall
  - credit_metrics
  - sources_and_uses
requires_external_tools:
  - fmp_balance_sheet
  - fmp_cash_flow
  - fmp_income_statement
---
# IC Memo

Draft an Investment Committee memo using the IC Memo workflow from `workflow-private-equity`.

## What It Does
Produces a 10-15 page IC memo with 9 sections: Executive Summary, Company Overview, Industry, Financial Analysis, Investment Thesis, Deal Terms, Returns Analysis, Risk Factors, Recommendation.

## Agent
Routes to `cfa-private-markets-analyst`.

## Key Tools
`build_lbo`, `calculate_returns`, `sources_and_uses`, `build_debt_schedule`, `calculate_waterfall`, `build_sensitivity_grid`, `credit_metrics`, `altman_zscore`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`

## Usage
Provide deal details including company info, financials, deal terms, and due diligence findings.
