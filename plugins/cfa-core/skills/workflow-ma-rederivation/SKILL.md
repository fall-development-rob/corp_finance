---
name: "workflow-ma-rederivation"
description: |
  WHAT: Independent re-derivation of financial model outputs using corp-finance-mcp tools as the 128-bit Decimal ground truth — covers DCF/WACC, LBO/debt schedule/returns/waterfall, three-statement model, merger accretion/dilution, and sensitivity/scenario stress tables.
  WHEN: Invoke when auditing any vendor-supplied or junior-prepared financial model (DCF, LBO, three-statement, merger) and all material numbers must be recomputed independently before the model's conclusions can be trusted in a client-facing or IC deliverable.
---

# Model Audit — Independent Re-Derivation

## What this skill covers

Re-derivation of every material number in a financial model using corp-finance-mcp tools as the independent, authoritative benchmark. The model is the thing under test; tool output is the ground truth. Discrepancies mean the model is suspect, not the tool.

## Core principle

> Re-derive, do not trust. Every material number is recomputed via corp-finance-mcp tools and compared to the model's stated value. The tool runs on 128-bit Decimal precision; LLM-estimated benchmarks are never used.

## Tool references

| Tool | Model type | What is validated |
|------|------------|-------------------|
| `dcf_model` | DCF | FCFF schedule, discounted PV column, enterprise value |
| `wacc_calculator` | DCF | WACC from stated CAPM inputs — match to four decimal places |
| `lbo_model` | LBO | Debt balances, FCF sweep, exit equity, MOIC, IRR |
| `debt_schedule` | LBO / credit | Per-tranche interest, amortisation, ending balance |
| `sources_uses` | LBO / M&A | Sources = Uses to the cent |
| `returns_calculator` | LBO / PE | IRR, XIRR, MOIC, cash-on-cash from dated cash flows |
| `waterfall_calculator` | LBO / PE | GP/LP cash flows by tier and period |
| `three_statement_model` | 3-stmt | IS/BS/CF articulation (see also `workflow-ma-three-statement-tieout`) |
| `merger_model` | M&A | Pro forma EPS, accretion/dilution %, pro forma leverage |
| `credit_metrics` | M&A / credit | Pro forma leverage and coverage for combined entity |
| `sensitivity_matrix` | All | Independently regenerated sensitivity tables |
| `scenario_analysis` | All | Bull/base/bear weighted scenario validation |

## Workflow

### DCF re-derivation

1. Call `dcf_model` with the model's stated assumptions: revenue growth, EBITDA margin, WACC, terminal growth rate or exit multiple, forecast period.
2. Call `wacc_calculator` with: risk-free rate, beta, ERP, cost of debt, tax rate, capital weights. Confirm model WACC matches to four decimal places.
3. Compare every line of the FCFF schedule and discounted PV column.
4. Terminal value sanity: TV should be 50-75% of EV. Flag if >80% (extend forecast) or <40% (near-term assumptions too aggressive).
5. Cross-check terminal methods: Gordon Growth vs Exit Multiple terminal values should reconcile within 20%. Wider gap means one method is mis-specified.
6. Call `sensitivity_matrix` on WACC vs terminal growth (±100bps and ±50bps) and exit multiple vs EBITDA growth. Confirm model sensitivity table replicates.

### LBO re-derivation

1. Call `lbo_model` with: entry EV, entry EBITDA, debt tranches, equity, growth/margin assumptions, exit year, exit multiple, cash sweep.
2. Compare year-by-year: debt balances, FCF available for sweep, ending cash, exit equity value, MOIC, IRR.
3. Call `debt_schedule` for each tranche with: interest rate, amortisation type (level/sculpted/bullet), maturity, PIK toggle, seniority.
4. Call `sources_uses` — verify total sources = total uses to the cent. Imbalance is **Critical**.
5. Call `returns_calculator` with entry equity, exit equity, and dated cash flows. Confirm IRR, XIRR, MOIC, cash-on-cash.
6. Call `waterfall_calculator` with tier definitions (return of capital, preferred return, GP catch-up, carry split). Confirm GP and LP cash flows by period.
7. Call `sensitivity_matrix` on entry/exit multiple, leverage, and EBITDA growth.

### Merger model re-derivation

1. Call `merger_model` with: acquirer/target financials, offer price, consideration mix (cash/stock/mixed), synergies, financing rates.
2. Compare: pro forma EPS, accretion/dilution %, pro forma leverage.
3. Synergy realism check: flag synergies >5% of combined revenue or >25% of target opex without explicit phasing. Cost synergies: 50% Y1 / 75% Y2 / 100% Y3 is typical.
4. Call `credit_metrics` on combined entity. Flag if pro forma leverage exceeds covenant headroom or pushes synthetic rating below stated target.
5. Call `sensitivity_matrix` on synergies vs offer premium, financing rate vs target growth.

### Scenario and sensitivity validation

1. For every sensitivity table in the model, call `sensitivity_matrix` with the same variable ranges and compare the model's table to the tool output cell by cell.
2. For every bull/base/bear scenario block, call `scenario_analysis` with stated assumptions and confirm the model's probability-weighted outcome matches.

## Output format

```
RE-DERIVATION REPORT
---------------------
Model: [name / file reference] | Type: DCF / LBO / Merger / Three-Statement

SUMMARY
| Section | Lines checked | Exact matches | Discrepancies | Max delta |
|---------|--------------|---------------|---------------|-----------|
| FCFF schedule | 42 | 39 | 3 | 2.1% |
| WACC | 1 | 1 | 0 | — |
| Terminal value | 2 | 1 | 1 | 8.3% |

FINDINGS TABLE
| # | Location | Severity | Model value | Recomputed value | Delta (abs) | Delta (%) | Root cause hypothesis | Recommended fix |
|---|----------|----------|-------------|------------------|------------|-----------|----------------------|----------------|
| 1 | DCF!C25 (TV) | Critical | 3,450 | 3,738 | (288) | (7.7%) | Terminal growth rate hardcoded at 2.0% vs assumption cell 2.5% | Link C25 to assumption cell |
| 2 | DCF!D10 (FCFF Y3) | Major | 412 | 398 | 14 | 3.5% | D&A add-back omitted in Y3 | Confirm D&A linkage |
```

## Quality gates

- Every material number recomputed via corp-finance-mcp — no LLM-estimated benchmarks.
- Sources & uses must equal to the cent — non-zero is **Critical**.
- WACC validated to four decimal places — discrepancy is **Major**.
- Sensitivity tables independently regenerated and compared cell by cell.
- Every finding cites: location, severity, model value, recomputed value, absolute delta, percentage delta, root cause, and recommended fix.
- Severity: Critical = blocks use; Major = material impact; Minor = cosmetic or precision.

## Related skills

- `workflow-ma-three-statement-tieout` — structural articulation audit runs in parallel; rederivation validates the numbers, tie-out validates the structure
- `workflow-ma-hardcode-detection` — Type 3 shadow hardcodes often surface as discrepancies during rederivation
- `workflow-ma-formula-consistency` — formula breaks are a common root cause of rederivation discrepancies

## Routing

**Primary agent:** `cfa-chief-analyst`
