---
name: workflow-derivatives-structured-products
description: |
  WHAT: Price structured notes (autocalls, reverse convertibles, range accruals, principal-protected notes); decompose each product into its vanilla component instruments; assess embedded-option fair value and investor net economics.
  WHEN: Invoke for structured product mark-to-market, embedded-option valuation, retail-product fairness check, or when decomposing a structured note into its bond floor and derivative overlay for accounting or risk management purposes.
---

# Structured Products Pricing and Decomposition

## What this skill covers

A pipeline for pricing and decomposing structured investment products into their constituent components. Every structured note is treated as a combination of a zero-coupon bond (or coupon-bearing bond) and one or more embedded derivative positions. Pricing uses the appropriate tool for each component type: `structured_note_pricing` for standard catalog products, `exotic_product_pricing` for non-standard payoffs, `monte_carlo_simulation` for path-dependent features, and `convertible_bond_pricing` for equity-linked convertible structures. The output establishes a fair-value breakdown that supports both client disclosure and internal risk reporting.

## Workflow

### Phase 1 — Product Classification

1. Classify the structured note by payoff type:
   - **Principal-protected note (PPN)**: zero-coupon bond + long call.
   - **Reverse convertible**: coupon bond + short put (investor is short the downside).
   - **Autocall / Autocallable**: callable structure with conditional coupons and barrier observation.
   - **Range accrual**: coupon accrues only on days the underlying closes within a defined range.
   - **Convertible note**: bond with equity conversion option (long call on issuer equity).
   - **Custom / exotic**: any payoff not covered above; treated via Monte Carlo.
2. Document: underlying(s), notional, tenor, coupon / participation rate, barrier levels, call schedule, and protection level.

### Phase 2 — Funding Leg (Bond Floor)

3. Source the issuer's credit spread:
   - Call `credit_spreads` or `fmp_quote` for the issuer's CDS or bond yield spread over the risk-free curve.
4. Compute the bond floor (present value of the capital repayment and any fixed coupons):
   - Use `bond_pricer` with the issuer's all-in discount rate (risk-free rate + credit spread) and the scheduled cash flows.
   - For principal-protected notes: bond floor = PV of par repayment at maturity. The residual budget = issue price minus bond floor = maximum option budget.

### Phase 3 — Embedded Derivative Valuation

5. **Vanilla components** (calls, puts, spreads): call `option_pricer` per leg using Black-Scholes or binomial as appropriate. Source volatility from `implied_volatility` or the vol surface from `workflow-derivatives-vol-surface`.
6. **Autocall and barrier features**: call `structured_note_pricing` with the product type set to `"autocall"` or the relevant catalog identifier. Required inputs include barrier level, observation frequency, conditional coupon, and early redemption premium.
7. **Exotic / path-dependent payoffs**: call `exotic_product_pricing` for Asian options, barrier options, or lookback features embedded in the note.
8. **Monte Carlo path-dependent pricing**: call `monte_carlo_simulation` with the payoff function specified as the simulation rule. Minimum 10,000 paths; document path count, seed, and distributional assumptions (log-normal for equity; Hull-White for rate underlyings).
9. **Convertible equity-linked structures**: call `convertible_bond_pricing` using the CRR binomial tree with the specified conversion ratio, call/put provisions, and credit spread. Extract: bond floor, conversion premium, CB delta, and investment vs parity value.

### Phase 4 — Decomposition and Fairness Check

10. Assemble the component price table:
    - Issue price = Bond floor + Embedded option value + Distributor margin.
    - Compute implied distributor margin: Issue price minus (bond floor plus fair option value).
    - Benchmark: distributor margin > 3% of notional for a 1-3 year structured note is a red flag for retail fairness.
11. Call `sensitivity_matrix` with the underlying spot ±10% / ±20% and implied vol ±5 vol points to produce the structured note's price sensitivity grid.

### Phase 5 — Investor Net Economics

12. Compute the investor's breakeven:
    - For capital-at-risk products: the underlying must close above the barrier at maturity for the investor to recover principal; compute the barrier return required.
    - Effective yield: if the investor simply reinvested in the risk-free rate over the same tenor, what terminal value would they receive? Compare to the expected payout from `monte_carlo_simulation` (mean path outcome).
13. Document the investor's economic position versus a simple bond + direct equity investment alternative.

## Output Format

### Product Classification and Terms

| Field | Value |
|---|---|
| Product type | — |
| Underlying | — |
| Notional | — |
| Tenor | — |
| Issue price | — |
| Coupon / participation rate | — |
| Barrier level | — |
| Capital protection level | — |

### Component Decomposition

| Component | Type | Tool Used | Fair Value | % of Issue Price |
|---|---|---|---|---|
| Bond floor | Zero-coupon bond | `bond_pricer` | — | — |
| Embedded option(s) | Call / put / barrier | `option_pricer` / `exotic_product_pricing` | — | — |
| Monte Carlo correction | Path-dependent feature | `monte_carlo_simulation` | — | — |
| Distributor margin | Residual | Implied | — | — |
| **Issue price (total)** | | | — | 100% |

### Monte Carlo Output (if applicable)

| Metric | Value |
|---|---|
| Path count | — |
| Seed | — |
| Mean payout | — |
| 5th percentile payout | — |
| 95th percentile payout | — |
| Probability of capital loss | — |

### Convertible Bond Metrics (if applicable)

| Metric | Value | Source |
|---|---|---|
| Bond floor | — | `convertible_bond_pricing` |
| Conversion premium | — | `convertible_bond_pricing` |
| CB delta | — | `convertible_bond_pricing` |
| CB category | Investment / Balanced / Busted | `convertible_bond_pricing` |

### Sensitivity Matrix: Note Price vs Spot and Vol

(5 × 5 grid — output from `sensitivity_matrix`)

### Investor Economics Comparison

| Scenario | Structured Note Payout | Simple Bond + Equity Payout |
|---|---|---|
| Bear (underlying −30%) | — | — |
| Base (underlying flat) | — | — |
| Bull (underlying +30%) | — | — |

### Tool-Call Traceability

| # | Tool | Key Inputs | Output |
|---|---|---|---|

## Quality Gates

- [ ] Product correctly classified before any pricing tool is called.
- [ ] Bond floor computed with issuer credit spread included; not priced off the risk-free curve alone.
- [ ] Embedded option fair value computed with a tool call; not estimated from the residual.
- [ ] Distributor margin computed; flagged if > 3% of notional for retail products.
- [ ] Monte Carlo path count ≥ 10,000; seed and distributional assumptions documented.
- [ ] Convertible bond: bond floor, conversion premium, and delta all extracted from `convertible_bond_pricing`.
- [ ] Sensitivity matrix produced: underlying spot ± 20% × implied vol ± 5 pts.
- [ ] Investor breakeven and capital-loss probability documented for capital-at-risk products.
- [ ] Every number in output maps to a row in the traceability table.

## Related Skills

- `workflow-derivatives-option-pricing` — vanilla component pricing for each embedded option leg.
- `workflow-derivatives-vol-surface` — vol surface inputs for exotic and barrier option components.
- `workflow-derivatives-swaps` — interest rate swap components embedded in structured rate notes.
- `corp-finance-analyst-derivatives` — agent body with CB benchmarks (balanced: 20-40% premium, delta 0.4-0.6; busted: >60% premium, delta <0.3) and Monte Carlo conventions.
