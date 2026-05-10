---
workflow:
  slug: fa-three-statement-model
  auto_route: true
  advisory: false
---

# 3-Statement Model

Build an integrated three-statement model using the `workflow-financial-analysis` skill with `corp-finance-analyst-core` computation tools.

## What It Does
Constructs a fully linked income statement, balance sheet, and cash flow statement: (1) revenue build (volume x price or driver-based), (2) operating expense and margin schedule, (3) working capital schedule (DSO, DIO, DPO), (4) capex and D&A roll-forward, (5) debt schedule with interest expense feedback, (6) equity roll-forward (retained earnings, dividends, buybacks), (7) integrated cash flow statement with cash tie-out to balance sheet, (8) balance-sheet integrity check (Assets = Liabilities + Equity at every period).

## Agent
Routes to `cfa-chief-analyst` with `workflow-financial-analysis` and `corp-finance-analyst-core` skills.

## Key Tools
`three_statement_model`, `credit_metrics`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `fmp_key_metrics`

## Usage
Provide a ticker or company name, forecast horizon (default 5 years), and key driver assumptions (revenue growth path, target EBITDA margin, capex intensity, working capital days, dividend/buyback policy). The agent will pull historical financials, project the three statements, and run integrity checks.

## Output
Markdown report containing: historical and projected income statement, balance sheet, and cash flow statement (5-10 year horizon), working capital schedule, debt schedule, equity roll-forward, balance-sheet check (must reconcile to zero), cash tie-out, and projected credit metrics (leverage, coverage, liquidity) with covenant flags.
