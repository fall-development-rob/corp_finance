---
name: workflow-derivatives-futures-forwards
description: |
  WHAT: Price futures and forward contracts using cost-of-carry; analyze basis, contango/backwardation, and roll yield; derive minimum-variance hedge ratios.
  WHEN: Invoke for FX or commodity hedge sizing, futures basis trade analysis, cash-and-carry arbitrage checks, or any task requiring a defensible forward price and associated hedge ratio.
---

# Futures and Forwards Pricing Workflow

## What this skill covers

A structured pipeline for pricing forwards and futures across asset classes (equity index, FX, commodity, and fixed income) using the cost-of-carry framework. Covers basis analysis including contango/backwardation classification and roll yield quantification, plus minimum-variance hedge ratio derivation. Arbitrage bounds are verified for every price produced. Every number must originate from a tool call.

## Workflow

### Phase 1 — Market Data Collection

1. **Spot price**: call `fmp_quote` for equity or index underlyings; use `fmp_batch_commodity_quotes` for commodity underlyings; use `fmp_batch_forex_quotes` for FX pairs.
2. **Risk-free rate**: call `fmp_treasury_rates` for the US risk-free rate at the forward tenor. For FX, also call `fred_series` for the foreign country policy rate or short-term benchmark rate.
3. **Carry cost components** by asset class:
   - **Equity / index**: dividend yield from `fmp_key_metrics` or `fmp_financial_ratios`.
   - **FX**: foreign interest rate from `fred_series`; confirm interest-rate parity assumptions.
   - **Commodity**: storage cost and convenience yield must be specified by analyst as input assumptions; document source (exchange settlement, industry convention, or analyst estimate).
   - **Fixed income**: coupon accruals and repo rate from `fred_series` (SOFR or GC repo).

### Phase 2 — Forward / Futures Pricing

4. **Generic forward**: call `forward_pricer` with spot, risk-free rate, dividend yield / convenience yield / storage cost, and tenor. Returns the fair forward price and implied carry rate.
5. **FX forward**: call `fx_forward` with spot FX rate, domestic rate, foreign rate, and tenor. Returns forward points and the all-in forward rate.
6. **Commodity forward**: call `commodity_forward` with spot, storage cost, convenience yield, risk-free rate, and tenor. Returns the futures fair value and carry breakdown.
7. **Existing position MTM**: if pricing an existing forward (not a new trade), call `forward_position_value` with the contracted forward price and current market inputs to obtain the MTM gain/loss.

### Phase 3 — Basis Analysis

8. Call `futures_basis_analysis` with the current spot price, observed futures settlement price, risk-free rate, storage/carry cost, and tenor.
   - Tool returns: theoretical basis (fair value minus spot), actual basis (observed futures minus spot), basis error (actual minus theoretical), contango/backwardation classification, implied repo rate, and roll yield.
9. Classify market structure:
   - **Contango**: futures > spot (normal for financial assets with positive carry).
   - **Backwardation**: futures < spot (indicates convenience yield exceeds carry; common in energy).
   - **Basis risk**: document if actual basis deviates from theoretical by more than 0.5%.

### Phase 4 — Arbitrage Bounds Check

10. Verify no-arbitrage boundaries:
    - **Upper bound**: futures price ≤ spot × exp((r + storage) × T). If breached: cash-and-carry arbitrage exists.
    - **Lower bound**: futures price ≥ spot × exp((r − convenience yield) × T). If breached: reverse cash-and-carry exists.
11. Document whether arbitrage bounds are satisfied or violated and the magnitude of any violation.

### Phase 5 — Hedge Ratio Derivation

12. Compute the minimum-variance hedge ratio:
    - `h* = ρ × (σ_S / σ_F)` where ρ is spot-futures correlation, σ_S is spot price volatility, σ_F is futures price volatility.
    - Source σ_S and σ_F from `fmp_historical_price` for both series; compute realized vols over the hedge horizon.
13. Compute the number of contracts required:
    - `N* = h* × (Portfolio Value / Contract Notional)`.
14. Assess hedge effectiveness: an R² ≥ 0.85 between spot and futures returns is considered effective for accounting hedge designation purposes.

### Phase 6 — Roll Yield and Curve Analysis

15. For commodity underlyings, call `commodity_curve` to obtain the full forward curve across all listed tenors. Classify the curve as contango, backwardation, or humped and compute the annualized roll yield from front to second month.

## Output Format

### Forward Price Summary

| Parameter | Value | Source |
|---|---|---|
| Underlying | — | Input |
| Spot price | — | `fmp_quote` / `fmp_batch_commodity_quotes` |
| Tenor (days) | — | Input |
| Risk-free rate | — | `fmp_treasury_rates` |
| Dividend / convenience yield | — | `fmp_key_metrics` / analyst assumption |
| Storage cost | — | Analyst assumption |
| **Fair forward price** | — | `forward_pricer` / `commodity_forward` |
| Implied carry rate | — | `forward_pricer` |
| Observed futures price | — | Market data |
| Basis (theoretical) | — | `futures_basis_analysis` |
| Basis (actual) | — | `futures_basis_analysis` |
| Market structure | Contango / Backwardation | `futures_basis_analysis` |

### Arbitrage Bounds

| Bound | Value | Status |
|---|---|---|
| Upper bound (cash-and-carry) | — | Pass / Violated |
| Lower bound (reverse carry) | — | Pass / Violated |

### Hedge Ratio

| Field | Value |
|---|---|
| Spot-futures correlation (ρ) | — |
| Spot vol (σ_S, annualized) | — |
| Futures vol (σ_F, annualized) | — |
| Minimum-variance hedge ratio (h*) | — |
| Portfolio value | — |
| Contract notional | — |
| Number of contracts (N*) | — |
| Hedge R² | — |

### Commodity Forward Curve (if applicable)

| Tenor | Forward Price | Roll Yield vs Prior |
|---|---|---|

### Tool-Call Traceability

| # | Tool | Key Inputs | Output |
|---|---|---|---|

## Quality Gates

- [ ] Spot price sourced from live market data tool; not hand-entered.
- [ ] Risk-free rate tenor matches forward tenor within 30 days.
- [ ] Both `forward_pricer` (or `fx_forward` / `commodity_forward`) and `futures_basis_analysis` called to cross-check fair value.
- [ ] Contango/backwardation classification stated and basis error quantified.
- [ ] Arbitrage bounds verified; any violation flagged with magnitude.
- [ ] Hedge ratio derived from `fmp_historical_price` volatilities; correlation and R² documented.
- [ ] For commodity underlyings: `commodity_curve` called to provide full tenor curve view.
- [ ] Every number in output maps to a row in the traceability table.

## Related Skills

- `workflow-derivatives-swaps` — interest rate swaps share the discounting framework used for forward pricing.
- `workflow-derivatives-option-pricing` — forwards are the reference for forward-starting option pricing.
- `workflow-derivatives-structured-products` — commodity-linked notes embed forward pricing as a component.
- `corp-finance-analyst-derivatives` — agent body with full tool inventory and asset-class carry conventions.
