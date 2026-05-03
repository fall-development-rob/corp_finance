---
description: Comprehensive institutional property valuation combining income, sales comparison, cost, and HBU approaches
requires_tools:
  - comp_adjustment_grid
  - cost_approach
  - hbu_analysis
  - ncreif_attribution
  - tenant_schedule
  - value_property
---
Perform a comprehensive institutional-grade property valuation for: $ARGUMENTS

Use the following analytical framework:

1. **Rent Roll Analysis** — Use `tenant_schedule` to model tenant-by-tenant cash flows, compute WALT, and identify rollover risk
2. **Income Approach** — Use `value_property` (existing) for direct cap and DCF, feeding in the rent roll NOI
3. **Sales Comparison** — Use `comp_adjustment_grid` to build an adjustment grid from recent comparable transactions
4. **Cost Approach** — Use `cost_approach` for Marshall & Swift replacement cost less depreciation
5. **Highest & Best Use** — Use `hbu_analysis` to confirm current use is the HBU
6. **Benchmark Context** — Use `ncreif_attribution` to compare cap rates and returns vs NCREIF/ODCE

**Output a valuation summary** reconciling all approaches with a recommended value range, key risks, and investment thesis.

Use tools from: `corp-finance-tools-core`, `fmp-market-data` (for market context)
Route to: `cfa-equity-analyst`
