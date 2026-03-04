---
description: "Comprehensive institutional property valuation combining income, sales comparison, cost, and HBU approaches"
---

Perform a comprehensive institutional-grade property valuation for: $ARGUMENTS

Use the following analytical framework:

1. **Rent Roll Analysis** — Use `institutional_rent_roll` to model tenant-by-tenant cash flows, compute WALT, and identify rollover risk
2. **Income Approach** — Use `property_valuation` (existing) for direct cap and DCF, feeding in the rent roll NOI
3. **Sales Comparison** — Use `institutional_comparable_sales` to build an adjustment grid from recent comparable transactions
4. **Cost Approach** — Use `institutional_replacement_cost` for Marshall & Swift replacement cost less depreciation
5. **Highest & Best Use** — Use `institutional_hbu_analysis` to confirm current use is the HBU
6. **Benchmark Context** — Use `institutional_benchmark` to compare cap rates and returns vs NCREIF/ODCE

**Output a valuation summary** reconciling all approaches with a recommended value range, key risks, and investment thesis.

Use tools from: `corp-finance-tools-core`, `fmp-market-data` (for market context)
Route to: `cfa-equity-analyst`
