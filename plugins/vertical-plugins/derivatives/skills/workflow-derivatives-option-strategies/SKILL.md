---
name: workflow-derivatives-option-strategies
description: |
  WHAT: Construct and analyze multi-leg option strategies (straddle, strangle, iron condor, butterfly, collar, spread, and custom structures); aggregate P&L and Greeks across all legs; map breakeven points and payoff profile.
  WHEN: Invoke for hedging recommendations, structured payoff design, yield-enhancement strategies, or any analysis requiring a net position view across two or more option legs.
---

# Multi-Leg Option Strategy Analysis

## What this skill covers

A structured pipeline for building multi-leg option strategies, aggregating Greeks across legs, running scenario analysis, and producing a full payoff diagram dataset. Supports all standard strategy types recognized by `option_strategy` (straddle, strangle, iron condor, butterfly, bull/bear spread, collar, covered call, protective put, calendar spread, diagonal spread, risk reversal, and custom). Strategies are fully decomposed: each leg is priced individually and then consolidated into a net position view.

## Workflow

### Phase 1 — Strategy Specification

1. Define the strategy structure: strategy type, underlying, legs (option type, strike, expiry, quantity, long/short direction), and net position size.
2. Call `fmp_quote` to obtain current spot price.
3. Call `fmp_treasury_rates` for the risk-free rate at the weighted-average leg tenor.
4. Call `fmp_key_metrics` for current dividend yield.

### Phase 2 — Per-Leg Pricing

5. For each leg, call `option_pricer` with:
   - `model`: `"black_scholes"` for European legs; `"binomial"` for American legs.
   - Leg-specific strike, expiry, option type, and volatility.
   - Use a consistent volatility source per leg — preferably from `workflow-derivatives-vol-surface` or `implied_volatility` solved from market prices via `yf_options_chain`.
6. Record each leg's fair value and Greeks: delta, gamma, theta, vega, rho.

### Phase 3 — Strategy Aggregation

7. Call `option_strategy` with the full multi-leg specification including all strikes, expiries, quantities, and directions.
   - The tool returns: net premium paid/received, max profit, max loss, breakeven points, and aggregated Greeks.
8. Cross-check the net premium from `option_strategy` against the sum of per-leg prices from Phase 2. Discrepancy > $0.01 per share must be investigated.
9. Classify the strategy net position:
   - Net premium paid (debit spread): maximum loss equals the debit.
   - Net premium received (credit spread): maximum loss equals the spread width minus the credit.

### Phase 4 — Greeks Aggregation

10. Aggregate net Greeks across all legs (weighted by quantity and direction):
    - **Net delta**: directional exposure equivalent to this many shares.
    - **Net gamma**: convexity; indicates delta stability.
    - **Net theta**: daily carry; positive for net short vol strategies.
    - **Net vega**: volatility sensitivity; positive for long vol strategies.
    - **Net rho**: interest rate sensitivity; typically minor for short-dated structures.
11. Flag any strategy where net vega changes sign across the current spot (e.g., vega-flipping butterfly wings) — document the inflection point.

### Phase 5 — Scenario Analysis

12. Call `sensitivity_matrix` with spot ranging ±10% and ±20% from current on one axis and implied vol ±5 vol points on the other axis to generate a 5 × 5 P&L grid.
13. Call `scenario_analysis` with base (current spot, current vol), bull (spot +15%, vol −3 pts), and bear (spot −15%, vol +5 pts) cases for the overall strategy P&L.

### Phase 6 — Payoff Diagram Data

14. Generate payoff at expiry across a spot range of ±30% in 2% increments:
    - For each spot level, compute intrinsic value per leg and aggregate.
    - Output as a two-column table (spot price, strategy P&L) for direct chart import.

## Output Format

### Strategy Summary

| Field | Value |
|---|---|
| Strategy type | — |
| Underlying | — |
| Current spot | — |
| Net premium | — (debit/credit) |
| Max profit | — |
| Max loss | — |
| Breakeven point(s) | — |
| Probability of profit (approx.) | — |

### Leg Detail

| Leg | Type | Strike | Expiry | Direction | Qty | Price | Delta | Gamma | Theta | Vega |
|---|---|---|---|---|---|---|---|---|---|---|

### Net Greeks

| Greek | Net Value | Interpretation |
|---|---|---|
| Net delta | — | Directional shares equivalent |
| Net gamma | — | Convexity (positive = accelerating delta) |
| Net theta | — | Daily time decay ($) |
| Net vega | — | P&L per 1 vol point move |
| Net rho | — | P&L per 1% rate move |

### Scenario Analysis

| Scenario | Spot | Vol | Strategy P&L |
|---|---|---|---|
| Bear | −15% | +5 pts | — |
| Base | flat | flat | — |
| Bull | +15% | −3 pts | — |

### Sensitivity Matrix: P&L vs Spot and Vol

(5 × 5 grid — output from `sensitivity_matrix`)

### Payoff at Expiry

| Spot | Strategy P&L |
|---|---|

### Tool-Call Traceability

| # | Tool | Key Inputs | Output |
|---|---|---|---|

## Quality Gates

- [ ] Every leg priced with `option_pricer` using a consistent volatility source.
- [ ] `option_strategy` net premium cross-checked against sum of per-leg prices; discrepancy ≤ $0.01.
- [ ] Net Greeks computed for all five Greeks across all legs.
- [ ] Maximum profit and maximum loss verified to be analytically consistent with payoff diagram.
- [ ] Breakeven points verified by interpolating the payoff table (P&L crosses zero at stated breakeven).
- [ ] Scenario analysis (`scenario_analysis`) present with base, bull, and bear.
- [ ] Sensitivity matrix (`sensitivity_matrix`) produced: spot ± 20% × vol ± 5 pts.
- [ ] Volatility source consistent across all legs (do not mix ATM vol for one leg with skew vol for another without documentation).
- [ ] Every number in output maps to a row in the traceability table.

## Related Skills

- `workflow-derivatives-option-pricing` — single-leg pricing as the building block for each leg.
- `workflow-derivatives-vol-surface` — consistent vol inputs by strike and expiry.
- `workflow-derivatives-structured-products` — structured notes embed multi-leg option strategies as components.
- `corp-finance-analyst-derivatives` — agent body with full strategy type list and calling conventions.
