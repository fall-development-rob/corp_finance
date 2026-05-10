---
workflow:
  slug: fa-trading-comps
  auto_route: true
  advisory: false
---

# Trading Comps

Construct a trading comparables analysis using the `workflow-financial-analysis` skill with `corp-finance-analyst-core` and FMP screener data.

## What It Does
Builds a peer-based relative valuation: (1) peer screen by sector, size, geography, and growth/margin profile, (2) multiples table (EV/Revenue, EV/EBITDA, P/E, P/B, EV/FCF) on LTM and NTM basis, (3) descriptive statistics (mean, median, quartiles), (4) regression-based justification of premium/discount (e.g. EV/EBITDA vs growth, EV/EBITDA vs margin), (5) implied valuation range for the subject company.

## Agent
Routes to `cfa-chief-analyst` with `workflow-financial-analysis` and `corp-finance-analyst-core` skills.

## Key Tools
`comps_analysis`, `fmp_stock_screener`, `fmp_profile`, `fmp_key_metrics`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_ratios`

## Usage
Provide the subject ticker plus screening parameters (sector, market-cap range, geographic scope, optional growth/margin filters). The agent will pull a 6-12 name peer set, build the multiples table, run regression diagnostics, and produce the implied valuation range.

## Output
Markdown report with: peer screen summary (count and rationale), multiples table (rows = peers, columns = LTM/NTM EV/Rev, EV/EBITDA, P/E with mean/median/quartile rows), regression chart commentary (R-squared and coefficient of the relevant driver), subject company position vs peer median, and implied EV range applying peer median multiples to subject financials.
