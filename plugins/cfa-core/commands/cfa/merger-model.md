---
workflow:
  slug: ib-merger-model
  auto_route: true
  advisory: false
---

# Merger Model

Build an M&A merger model with accretion / dilution analysis using the Merger Model workflow from `workflow-investment-banking`.

## What It Does
Produces a full merger consequences analysis: (1) purchase price analysis (implied EV/EBITDA, EV/Revenue, P/E at offer; premium to 30/60/90-day VWAP), (2) sources & uses (Sources = Uses tie-out across equity, term loans, bonds, revolver, rollover, seller note), (3) synergies build with phase-in (Year 1: 25%, Year 2: 75%, Year 3: 100%) and integration costs, (4) pro-forma EPS Year 1 through Year 3 (accretive vs dilutive), (5) sensitivity (synergy level vs offer premium; cash/stock mix vs EPS) and breakeven synergies, (6) credit impact on combined entity (post-deal leverage, coverage, synthetic rating).

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-investment-banking` skill.

## Key Tools
`merger_model`, `merger_model`, `sources_uses`, `sensitivity_matrix`, `credit_metrics`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `fmp_key_metrics`

## Usage
Provide acquirer and target tickers or financials, consideration mix (AllCash, AllStock, Mixed), offer price or premium, expected synergies (cost and revenue, with phase-in), integration costs, and financing structure.

## Output
One-page merger consequences summary plus detailed supporting model: purchase price table, S&U with tie-out, synergy build, three-year pro-forma EPS comparison (standalone vs combined), accretion/dilution percent and basis, two-axis sensitivity grids, breakeven synergy required for EPS-neutral, post-deal credit metrics, and rating-agency threshold flags.
