---
name: workflow-derivatives-swaps
description: |
  WHAT: Value interest rate swaps and cross-currency swaps; bootstrap a par-coupon discount curve; compute DV01, par rate, fixed/floating leg PVs, and swap NPV.
  WHEN: Invoke for swap mark-to-market, ALM hedge book valuation, IRS package pricing, new trade fair-value verification, or any task requiring a defensible swap NPV with duration sensitivity.
---

# Interest Rate and Currency Swap Valuation

## What this skill covers

A structured pipeline for valuing vanilla interest rate swaps (pay-fixed / receive-fixed) and cross-currency swaps. Builds or validates the discount curve from market data, decomposes the swap into fixed and floating legs, computes NPV and DV01, and quantifies scenario exposure across rate shifts. Applicable to single-currency IRS, overnight index swaps (OIS), and multi-currency cross-currency basis swaps.

## Workflow

### Phase 1 — Curve Construction

1. **Risk-free / SOFR curve**: call `fred_yield_curve` for the SOFR or US Treasury curve. Alternatively call `fmp_treasury_rates` for the Treasury par-coupon curve. For non-USD swaps, call `lseg_yield_curve` if LSEG access is available, or `fred_series` for the central bank policy rate and short-term benchmarks.
2. **Bootstrapped spot rates**: call `bootstrap_spot_curve` with the par-coupon rates and payment frequencies to produce a zero-coupon (spot) rate curve. This is the discount curve used for all leg valuations.
3. **Floating rate index**: identify the reference rate (SOFR, EURIBOR, SONIA, CDOR) and its term (1M, 3M, 6M). Source the current fixing from `fred_series` or `lseg_economic_indicators`.

### Phase 2 — Interest Rate Swap Valuation

4. Call `interest_rate_swap` with:
   - `notional`: the swap notional principal.
   - `fixed_rate`: the contractual fixed coupon rate.
   - `tenor`: swap maturity in years.
   - `payment_frequency`: typically semi-annual (2) for USD swaps.
   - `discount_curve`: the spot rates from Phase 2 (pass as array of {tenor, rate} pairs).
   - `floating_rate`: current floating index rate.
   - `position`: `"pay_fixed"` or `"receive_fixed"`.
5. Extract from the tool response:
   - **Fixed leg PV**: present value of all contractual fixed payments.
   - **Floating leg PV**: present value of projected floating payments.
   - **Swap NPV**: floating leg PV minus fixed leg PV (positive = asset for receiver of fixed).
   - **Par rate**: the fixed rate that makes the swap NPV = 0 at current market levels.
6. Compute **annualized DV01** (dollar value of a 1 basis point shift): call `bond_duration` with the fixed leg cash flows and the discount curve, then convert modified duration to DV01:
   - `DV01 = (Modified Duration × Fixed Leg PV) / 10,000`.

### Phase 3 — Cross-Currency Swap Valuation (if applicable)

7. When the task involves two currencies, call `currency_swap` with:
   - Domestic and foreign notionals (typically principal exchange at inception and maturity).
   - Domestic and foreign fixed or floating rates.
   - Spot FX rate and term structure from `fx_forward` or `fred_series`.
   - Cross-currency basis spread (source from `lseg_yield_curve` or analyst assumption; document source).
8. Extract: NPV in domestic currency, per-leg PV in each currency, implied FX swap rate, cross-currency basis.

### Phase 4 — Sensitivity Analysis

9. Call `sensitivity_matrix` with parallel rate shifts of −100, −50, 0, +50, +100 bps on both the discount curve and the floating rate index to produce a 5 × 5 NPV grid (or a 5-row DV01 profile if single-variable sensitivity is sufficient).
10. For convexity assessment, compute the change in DV01 between the +50 bp and −50 bp scenarios. A meaningful convexity difference (> 5% of DV01) should be flagged.

### Phase 5 — Par Rate and Breakeven Analysis

11. Document the breakeven analysis:
    - **Breakeven rate**: the fixed rate at which the swap has zero NPV (= par rate from Phase 2).
    - **Current fixed rate vs par rate**: positive difference means the fixed leg is above-market (positive value for fixed receiver).
    - **Rate required to wipe out current NPV**: the parallel shift that drives NPV to zero.

## Output Format

### Swap Valuation Summary

| Parameter | Value | Source |
|---|---|---|
| Swap type | Pay-fixed / Receive-fixed | Input |
| Notional | — | Input |
| Fixed rate | — | Input |
| Tenor (years) | — | Input |
| Floating index | — | Input |
| Current floating fixing | — | `fred_series` |
| Discount curve source | — | `fred_yield_curve` / `fmp_treasury_rates` |
| **Fixed leg PV** | — | `interest_rate_swap` |
| **Floating leg PV** | — | `interest_rate_swap` |
| **Swap NPV** | — | `interest_rate_swap` |
| Par rate | — | `interest_rate_swap` |
| DV01 (per $1M notional) | — | `bond_duration` |

### Cross-Currency Swap (if applicable)

| Parameter | Value | Source |
|---|---|---|
| Domestic currency NPV | — | `currency_swap` |
| Foreign currency NPV | — | `currency_swap` |
| Cross-currency basis spread | — | `lseg_yield_curve` / assumption |
| Implied FX swap rate | — | `currency_swap` |

### Rate Sensitivity (DV01 Profile)

| Rate Shift (bps) | Fixed Leg PV | Floating Leg PV | Swap NPV | DV01 |
|---|---|---|---|---|
| −100 | | | | |
| −50 | | | | |
| 0 (base) | | | | |
| +50 | | | | |
| +100 | | | | |

### Bootstrapped Spot Curve

| Tenor | Par Rate | Spot Rate |
|---|---|---|

### Tool-Call Traceability

| # | Tool | Key Inputs | Output |
|---|---|---|---|

## Quality Gates

- [ ] Discount curve bootstrapped from market par rates via `bootstrap_spot_curve`; not assumed flat.
- [ ] Fixed leg PV and floating leg PV both extracted from `interest_rate_swap` tool response.
- [ ] Par rate reported; current contractual rate vs par rate spread documented.
- [ ] DV01 computed via `bond_duration` on fixed leg cash flows; not hand-estimated.
- [ ] Rate sensitivity grid covers at least ±100 bps range.
- [ ] Cross-currency basis spread sourced and documented when cross-currency swap is in scope.
- [ ] Convexity assessed between +50 and −50 bp DV01 values; flagged if > 5% difference.
- [ ] Every number in output maps to a row in the traceability table.

## Related Skills

- `workflow-derivatives-futures-forwards` — forward pricing shares the same discounting framework.
- `workflow-derivatives-option-pricing` — swaptions require the swap valuation as the underlying.
- `workflow-derivatives-structured-products` — interest rate swap components are embedded in many structured notes.
- `corp-finance-analyst-derivatives` — agent body with dual-curve discounting conventions and tool calling patterns.
