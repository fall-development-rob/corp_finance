---
name: "workflow-fi-yield-curve-construction"
description: |
  WHAT: Bootstrap a spot (zero-coupon) curve from par yields; fit Nelson-Siegel-Svensson term structure; extract forward rates.
  WHEN: Pricing illiquid bonds, curve trading setup, hedge construction requiring curve-risk decomposition.
---

# Fixed Income: Yield Curve Construction

## What this skill covers

A three-phase pipeline for building an institutional-grade yield curve: raw par-rate ingestion, spot-curve bootstrapping, model fitting (Nelson-Siegel or Nelson-Siegel-Svensson), and forward-rate extraction. The output curve is the reference input for bond pricing, spread analysis, ALM, and relative-value trading.

## Core Rules

- Every par yield must come from a market data tool (`fred_yield_curve`, `fmp_treasury_rates`, `lseg_yield_curve`) — no manual inputs.
- Bootstrapping uses `bootstrap_spot_curve`; model fitting uses `nelson_siegel_fit` or `term_structure_fit`.
- State the reference currency, benchmark instrument (OIS, LIBOR/SOFR swap, government par), and as-of date before any computation.
- Nelson-Siegel R-squared must exceed 0.99; flag and investigate if it does not.
- Nelson-Siegel-Svensson RMSE target < 3 bps.

## Workflow

### Phase 1 — Par Rate Ingestion

1. Pull benchmark par yields with one of:
   - `fred_yield_curve` — US Treasury par yields (free, public)
   - `fmp_treasury_rates` — FMP Treasury par curve (freemium)
   - `lseg_yield_curve` — multi-currency swap and government curves (paid vendor)

2. Standard tenor nodes for USD: 1M, 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 20Y, 30Y.
   - For EUR/GBP/JPY: use equivalent government benchmark or OIS swap tenors.
   - Record the instrument type per tenor (deposit, FRA, futures, swap) for correct bootstrapping logic.

3. Verify: par yields should be monotonically increasing for normal curve; flag inversions explicitly (e.g., 2Y > 10Y inversion).

### Phase 2 — Bootstrapping

4. Call `bootstrap_spot_curve` with the par rates, tenors, day count, and compounding convention.
   - Outputs: **spot rates** at each tenor node, **discount factors**, **annualised continuously compounded zero rates**.
   - Bootstrapping solves sequentially: spot rate at each tenor is extracted from the par bond condition, stripping coupon cash flows priced at already-solved spot rates.

5. Sanity check: verify that the par rate implied by the bootstrapped spot curve equals the input par rate at each tenor (round-trip test). Tolerance: ±0.5 bps.

6. Extract **instantaneous forward rates** from adjacent discount factors:
   f(t₁, t₂) = −[ln(d(t₂)) − ln(d(t₁))] / (t₂ − t₁)

   Report the 1Y × 1Y, 2Y × 1Y, 5Y × 1Y, 10Y × 1Y forward rates.

### Phase 3 — Model Fitting

7. Call `nelson_siegel_fit` on the bootstrapped spot rates.
   - Parameters: β₀ (long-run level), β₁ (slope, short − long rate), β₂ (curvature / hump), λ (decay speed).
   - Outputs: 4-parameter vector, fitted rates at each node, R-squared, RMSE.
   - Quality gate: R² > 0.99; RMSE < 5 bps.

8. If the curve has a double hump or kink (e.g., QE-era curves, currency-specific anomalies), call `term_structure_fit` with the Nelson-Siegel-Svensson extension.
   - Adds β₃ and λ₂ for a second hump.
   - Quality gate: RMSE < 3 bps.

9. Report fit residuals (market − model) at each tenor. Residuals > 5 bps at any node indicate a data quality issue or structural break requiring investigation.

### Phase 4 — Forward Curve Derivation

10. Using the fitted NS/NSS model, derive the **full forward rate curve** at 6M intervals from 0.5Y to 30Y.
    - Identify the market-implied path of short rates.
    - Note: a humped forward curve peaks near the expected rate cycle peak; a flat forward curve implies no further hikes priced in.

11. Compute **term premium** estimates:
    - 10Y term premium = 10Y spot rate − expected average 3M rate over 10 years (from forward curve).
    - Elevated term premium (>50 bps) historically coincides with higher term-structure volatility.

## Output Format

**Spot Curve Table**

| Tenor | Par Yield | Spot Rate | Discount Factor | Fwd Rate (to next node) |
|---|---|---|---|---|
| 1M | X.XX% | X.XX% | X.XXXX | — |
| 3M | X.XX% | X.XX% | X.XXXX | X.XX% |
| ... | | | | |
| 30Y | X.XX% | X.XX% | X.XXXX | X.XX% |

**Nelson-Siegel Parameters**

| Parameter | Value | Interpretation |
|---|---|---|
| β₀ (level) | X.XX% | Long-run rate |
| β₁ (slope) | X.XX% | Short−long spread |
| β₂ (curvature) | X.XX% | Mid-maturity hump |
| λ (decay) | X.XX | Hump location |
| R² | X.XXXX | Must be > 0.99 |
| RMSE | X.X bps | Must be < 5 bps |

**Fit Residuals**

| Tenor | Market spot | Model spot | Residual (bps) | Status |
|---|---|---|---|---|
| 2Y | X.XX% | X.XX% | X.X | OK / WARN |

**Key Forward Rates**

| Forward | Rate |
|---|---|
| 1Y × 1Y | X.XX% |
| 2Y × 1Y | X.XX% |
| 5Y × 1Y | X.XX% |
| 10Y × 1Y | X.XX% |
| 10Y term premium | +XX bps |

**Tool-Call Traceability Table**

| # | Tool | Key Inputs | Output |
|---|---|---|---|
| 1 | `fred_yield_curve` / `fmp_treasury_rates` | as-of date, tenors | par yields |
| 2 | `bootstrap_spot_curve` | par rates, tenors, day count | spot rates, discount factors |
| 3 | `nelson_siegel_fit` | spot rates, tenors | β₀, β₁, β₂, λ, R², RMSE |
| 4 | `term_structure_fit` | spot rates (if NSS needed) | 6-param fit, RMSE |

## Quality Gates

- [ ] As-of date, currency, and benchmark instrument type stated
- [ ] Par yields sourced from market data tool (not hand-keyed)
- [ ] Round-trip test: bootstrapped spot curve reprices par bonds to ±0.5 bps
- [ ] Nelson-Siegel R² > 0.99
- [ ] RMSE < 5 bps (NS) or < 3 bps (NSS)
- [ ] Residuals table produced; any node > 5 bps flagged and investigated
- [ ] 1Y × 1Y, 2Y × 1Y, 5Y × 1Y, 10Y × 1Y forward rates reported
- [ ] Curve shape (normal / flat / inverted / humped) stated explicitly

## Related Skills

- `workflow-fi-bond-valuation` — consumes the spot curve as discount curve
- `workflow-fi-credit-spreads` — uses govt spot curve as the spread reference
- `workflow-fi-alm-strategy` — uses the full curve for key-rate duration matching
- `workflow-fi-inflation-linked` — real-yield curve construction mirrors this workflow
