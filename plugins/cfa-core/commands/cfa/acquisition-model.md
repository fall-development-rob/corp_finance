---
description: "Full acquisition underwriting with sources & uses, pro forma, debt sizing, and hold/sell analysis"
---

Build a full acquisition underwriting model for: $ARGUMENTS

Use the following analytical framework:

1. **Acquisition Underwriting** — Use `institutional_acquisition` to build sources & uses, year-by-year pro forma, and compute levered/unlevered IRR
2. **Rent Roll Foundation** — Use `institutional_rent_roll` for tenant-level NOI projections feeding the pro forma
3. **Comparable Sales** — Use `institutional_comparable_sales` to validate the purchase price against recent transactions
4. **Hold/Sell Analysis** — Compute break-even exit cap rate and optimal hold period
5. **Value-Add Scenario** — If applicable, model renovation CapEx, lease-up, and value-add IRR
6. **Development Alternative** — If applicable, run development feasibility analysis
7. **Refinancing** — Model potential refinancing scenarios and their impact on levered returns

**Output a Go/No-Go investment memo** with key metrics (IRR, equity multiple, DSCR, cash-on-cash), risk factors, and sensitivity analysis.

Use tools from: `corp-finance-tools-core`, `fmp-market-data` (for rate/market context)
Route to: `cfa-private-markets-analyst`
