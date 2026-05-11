---
name: workflow-derivatives-option-pricing
description: |
  WHAT: Price European and American options via Black-Scholes and binomial tree; extract full Greeks; cross-check implied volatility against historical realized volatility.
  WHEN: Invoke for single-leg option pricing, hedging analysis, position mark-to-market, or any task requiring a defensible fair-value estimate with Greeks.
---

# Option Pricing Workflow

## What this skill covers

A structured pipeline for pricing vanilla options (European and American), extracting a complete Greeks profile, and validating the implied-volatility input against historical realized volatility. Every number in the output must originate from a tool call; no hand-calculation is permitted. Covers equity, index, FX, and commodity underlyings.

## Workflow

### Phase 1 — Market Data Ingestion

1. **Spot price**: call `fmp_quote` with the underlying symbol to obtain the current mid price.
2. **Risk-free rate**: call `fmp_treasury_rates` for the US Treasury yield matching the option tenor, or `fred_yield_curve` for the SOFR/LIBOR-equivalent curve.
3. **Dividend yield / carry cost**: retrieve from `fmp_key_metrics` (trailing dividend yield) or `fmp_financial_ratios` for equity underlyings. For FX underlyings use the foreign interest rate sourced from `fred_series`.
4. **Observed market price** (if MTM mode): pull `yf_options_chain` for the specific expiry and strike to obtain the market mid price and existing market-derived Greeks.

### Phase 2 — Implied Volatility Extraction

5. When a market option price is available, call `implied_volatility` with the market price, spot, strike, rate, tenor, and dividend yield.
   - Flag any result where IV < 2% or IV > 150% as suspect; request an alternative market price before continuing.
   - For American options, specify the binomial-tree solver to account for early exercise.

### Phase 3 — Historical Volatility Cross-Check

6. Call `fmp_historical_price` for the underlying with a look-back window matching the option tenor (e.g., 30-day option → 30 days of daily closes).
7. Compute realized volatility annualized as `sqrt(252) * std(log returns)`. Pass the raw close series to `monte_carlo_simulation` with distributional mode set to normal to obtain the annualized vol estimate independently.
8. Compare IV from Phase 2 against historical realized vol:
   - IV / HV > 1.3: premium selling bias; flag.
   - IV / HV < 0.7: cheap vol; flag.
   - Document the IV-HV spread in the output table.

### Phase 4 — Option Pricing

9. Call `option_pricer` with the following parameters:
   - `model`: `"black_scholes"` for European; `"binomial"` for American, barrier, or dividend-paying underlyings with discrete dividends.
   - `spot`, `strike`, `rate`, `tenor`, `volatility` (use IV from Phase 2 if available; else use HV or analyst assumption).
   - `dividend_yield` or `cost_of_carry` as applicable.
   - `option_type`: `"call"` or `"put"`.
   - `steps` (binomial only): minimum 200 for American options.
10. Extract from the tool response: `price`, `delta`, `gamma`, `theta`, `vega`, `rho`.
11. Run put-call parity cross-check: `C - P = S * exp(-q*T) - K * exp(-r*T)`. Any discrepancy > $0.01 for liquid underlyings must be investigated.

### Phase 5 — Intrinsic vs Time Value Decomposition

12. Compute:
   - **Intrinsic value**: `max(S - K, 0)` for calls; `max(K - S, 0)` for puts.
   - **Time value**: `option price - intrinsic value`.
   - State whether the option is ITM, ATM, or OTM and quantify the moneyness ratio `S/K`.

### Phase 6 — Scenario Sensitivity

13. Call `sensitivity_matrix` with spot ± 10% / ± 20% on one axis and implied vol ± 5 vol points on the other axis to produce a 5 × 5 price grid.

## Output Format

### Option Pricing Summary

| Parameter | Value | Source |
|---|---|---|
| Underlying | — | `fmp_quote` |
| Spot price | — | `fmp_quote` |
| Strike | — | Input |
| Expiry (days) | — | Input |
| Risk-free rate | — | `fmp_treasury_rates` |
| Dividend yield | — | `fmp_key_metrics` |
| Implied volatility | — | `implied_volatility` |
| Historical vol (HV) | — | `fmp_historical_price` |
| IV / HV ratio | — | Computed |
| **Option fair value** | — | `option_pricer` |
| Intrinsic value | — | Computed |
| Time value | — | Computed |

### Greeks Table

| Greek | Value | Interpretation |
|---|---|---|
| Delta | — | $ change per $1 spot move |
| Gamma | — | Delta change per $1 spot move |
| Theta | — | $ decay per calendar day |
| Vega | — | $ change per 1 vol point |
| Rho | — | $ change per 1% rate move |

### Sensitivity Matrix: Price vs Spot and Vol

(5 × 5 grid — output from `sensitivity_matrix`)

### Tool-Call Traceability

| # | Tool | Key Inputs | Output |
|---|---|---|---|

## Quality Gates

- [ ] Spot price sourced from `fmp_quote` or equivalent live data; not hard-coded.
- [ ] Risk-free rate tenor matches option expiry within 30 days.
- [ ] IV extracted by `implied_volatility` tool call; never hand-estimated.
- [ ] HV cross-check performed; IV/HV ratio documented.
- [ ] Put-call parity verified: discrepancy ≤ $0.01 for liquid underlyings.
- [ ] American-exercise options priced with binomial (`steps` ≥ 200), not Black-Scholes.
- [ ] Greeks sign conventions confirmed: long call — delta > 0, gamma > 0, theta < 0, vega > 0.
- [ ] Sensitivity matrix (`sensitivity_matrix`) produced for every deliverable.
- [ ] Every number in output maps to a row in the traceability table.

## Related Skills

- `workflow-derivatives-vol-surface` — construct a full IV surface; inputs feed this skill's Phase 2.
- `workflow-derivatives-option-strategies` — aggregate Greeks across multi-leg structures.
- `workflow-derivatives-structured-products` — vanilla option pricing as component of structured notes.
- `corp-finance-analyst-derivatives` — agent body with full tool inventory and calling conventions.
