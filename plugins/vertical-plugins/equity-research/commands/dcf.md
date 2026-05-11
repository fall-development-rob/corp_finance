---
workflow:
  slug: fa-dcf-model
  auto_route: true
  advisory: false
---

# DCF Model

Build an institutional-grade discounted cash flow model using the `workflow-financial-analysis` skill in concert with `corp-finance-analyst-core` computation tools.

## What It Does
Constructs a DCF valuation: (1) FCFF / FCFE / APV variant selection, (2) WACC build (cost of equity via CAPM, after-tax cost of debt, target weights), (3) explicit forecast period free cash flow projection, (4) terminal value via Gordon Growth and Exit Multiple methods (cross-checked within 20%), (5) present value bridge to enterprise and equity value, (6) WACC vs terminal-growth and WACC vs exit-multiple sensitivity grids, (7) optional Monte Carlo DCF for distributional view.

## Agent
Routes to `cfa-chief-analyst` with `workflow-financial-analysis` and `corp-finance-analyst-core` skills.

## Key Tools
`dcf_model`, `wacc_calculator`, `sensitivity_matrix`, `monte_carlo_dcf`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `fmp_key_metrics`, `credit_metrics`

## Usage
Provide a ticker or company name plus the DCF variant (FCFF, FCFE, or APV). Optionally specify forecast horizon (default 5-10 years), terminal-value method (Gordon, Exit Multiple, or both), and whether Monte Carlo overlay is required. The agent will pull historical financials, build the WACC, project free cash flow, and produce a markdown report with the valuation bridge plus sensitivity tables.

## Output
Markdown report containing: WACC build table, forecast FCF schedule (5-10 years), terminal value calculation (both methods), enterprise-to-equity bridge, per-share fair value, two-axis sensitivity grids (WACC vs g, WACC vs exit multiple), and optional Monte Carlo distribution summary (mean, P10, P50, P90).
