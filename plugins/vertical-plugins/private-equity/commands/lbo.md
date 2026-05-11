---
workflow:
  slug: pe-lbo-model
  auto_route: true
  advisory: false
---

# LBO Model

Build a leveraged buyout model using the `workflow-financial-analysis` skill with `corp-finance-analyst-core` computation tools.

## What It Does
Executes an LBO build: (1) sources & uses of funds (sponsor equity, term loans, high yield, mezzanine, rollover, seller note), (2) opening capitalisation and goodwill calculation, (3) integrated operating model with debt schedule (mandatory amortisation, cash sweep, revolver), (4) exit assumption (exit-year multiple or DCF), (5) returns waterfall (sponsor IRR / MOIC, management promote, hurdle rate, GP carry), (6) IRR and MOIC sensitivity to entry multiple, exit multiple, leverage, and hold period.

## Agent
Routes to `cfa-chief-analyst` with `workflow-financial-analysis` and `corp-finance-analyst-core` skills.

## Key Tools
`lbo_model`, `debt_schedule`, `sources_uses`, `returns_calculator`, `sensitivity_matrix`, `waterfall_calculator`, `credit_metrics`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`

## Usage
Provide target company (ticker or name), entry multiple, total purchase price or per-share offer, financing structure (debt tranches with rate and tenor), assumed hold period, and exit assumption. The agent will assemble S&U, run the operating projections, layer the debt schedule, compute returns, and produce a sensitivity-driven recommendation.

## Output
Markdown report with: sources & uses table (with Sources = Uses tie-out), pro-forma opening balance sheet, year-by-year debt schedule with covenant headroom, exit-year EBITDA bridge, returns summary (sponsor IRR, MOIC, cash-on-cash), 2D sensitivity grids (entry vs exit multiple, leverage vs hold period), and base/upside/downside case IRR comparison.
