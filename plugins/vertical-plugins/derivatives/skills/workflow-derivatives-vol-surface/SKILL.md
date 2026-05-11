---
name: workflow-derivatives-vol-surface
description: |
  WHAT: Construct and analyze the implied volatility surface (smile, term structure, skew); calibrate the SABR stochastic volatility model to the observed grid.
  WHEN: Invoke when building a vol surface for exotic pricing inputs, assessing vol-of-vol risk, performing pre-hedging vol risk analysis, or generating market-implied vol parameters for a structured product model.
---

# Volatility Surface Construction and SABR Calibration

## What this skill covers

A two-phase pipeline that builds a full implied volatility surface from listed option market prices and then fits the SABR stochastic volatility model to the resulting grid. Outputs provide the parametric representation needed to price exotic derivatives, interpolate missing strikes, and monitor surface arbitrage conditions. Every vol datapoint must originate from a market data tool call.

## Workflow

### Phase 1 — Option Chain Collection

1. **Expiry enumeration**: call `yf_options_expirations` for the underlying symbol to obtain the full list of available expiry dates. Select expiries spanning the desired term structure (minimum: 1-month, 3-month, 6-month, 12-month tenors).
2. **Per-expiry chain extraction**: for each selected expiry, call `yf_options_all` or `yf_options_chain` to retrieve the full strike ladder with bid/ask mid prices and open interest.
   - Use `lseg_options_chain` when LSEG access is available for more complete institutional option data.
3. **Spot and forward reference**: call `fmp_quote` for current spot. Compute the at-the-money (ATM) forward for each expiry using the cost-of-carry formula (spot × exp((r − q) × T)) where `r` comes from `fmp_treasury_rates` and `q` from `fmp_key_metrics`.

### Phase 2 — Implied Volatility Grid Construction

4. For each (expiry, strike) combination with sufficient liquidity (open interest > 100 contracts, bid > 0):
   - Call `implied_volatility` with the mid option price, spot, strike, risk-free rate, tenor, and dividend yield.
   - Discard results with IV < 2% or IV > 200% as data errors; log excluded strikes.
5. Call `implied_vol_surface` with the collected (tenor, strike, IV) triples to:
   - Fit the surface using the specified interpolation method (`linear`, `cubic_spline`, or `svi`).
   - Run calendar-spread arbitrage checks (vol must not decrease monotonically with tenor for same strike).
   - Run butterfly arbitrage checks (convexity of vol with respect to strike must be non-negative).
   - Report any arbitrage violations by (tenor, strike) location.

### Phase 3 — Surface Diagnostics

6. Extract and report from the fitted surface:
   - **ATM vol curve**: IV at the forward strike for each tenor.
   - **Vol term structure slope**: ATM vol at 12M minus ATM vol at 1M (positive = normal contango; negative = backwardation).
   - **Skew (risk reversal)**: 25-delta put IV minus 25-delta call IV per expiry. Benchmark: equity skew −0.5 to −2.0 vol points per 10 delta.
   - **Smile (butterfly)**: 0.5 × (25-delta put IV + 25-delta call IV) minus ATM IV per expiry.

### Phase 4 — SABR Calibration

7. For each expiry tenor, call `sabr_calibration` with the strike-IV pairs from Phase 2 and the following initial parameter ranges:
   - `alpha` (ATM vol level): start from observed ATM IV.
   - `beta`: fix at 0.5 for equity/FX; fix at 1.0 for rates; set to 0.0 for normal SABR.
   - `rho` (correlation): equity benchmark −0.3 to −0.7 (negative skew).
   - `nu` (vol of vol): equity benchmark 0.3 to 0.8.
8. Validate calibration quality: RMSE < 0.5 vol points across the fitted strike range. If RMSE ≥ 0.5, expand the strike exclusion window near deep OTM strikes and recalibrate.
9. Report the per-expiry SABR parameter set `(alpha, beta, rho, nu)` and the calibration RMSE.

### Phase 5 — Surface Export

10. Produce the vol surface grid as a markdown table (tenors as columns, moneyness levels as rows) suitable for direct paste into an Excel model.

## Output Format

### ATM Volatility Term Structure

| Tenor | ATM Forward | ATM IV | Source Expiry |
|---|---|---|---|

### Skew and Smile by Expiry

| Expiry | 25D Put IV | ATM IV | 25D Call IV | Risk Reversal | Butterfly |
|---|---|---|---|---|---|

### SABR Parameters by Tenor

| Tenor | Alpha (α) | Beta (β) | Rho (ρ) | Nu (ν) | RMSE (vol pts) |
|---|---|---|---|---|---|

### Implied Vol Surface Grid (Moneyness × Tenor)

| Moneyness (K/F) | 1M | 3M | 6M | 12M | 24M |
|---|---|---|---|---|---|
| 80% | | | | | |
| 90% | | | | | |
| 95% | | | | | |
| 100% (ATM) | | | | | |
| 105% | | | | | |
| 110% | | | | | |
| 120% | | | | | |

### Arbitrage Violations (if any)

| Type | Expiry | Strike | Detail |
|---|---|---|---|

### Tool-Call Traceability

| # | Tool | Key Inputs | Output |
|---|---|---|---|

## Quality Gates

- [ ] Minimum 4 distinct expiry tenors included in the surface.
- [ ] Minimum 5 strikes per expiry retained after liquidity filtering.
- [ ] All IV datapoints sourced via `implied_volatility` tool call; none hand-estimated.
- [ ] Calendar-spread arbitrage check run and results documented.
- [ ] Butterfly arbitrage check run and results documented.
- [ ] SABR RMSE < 0.5 vol points per expiry; any exceedance documented with mitigation.
- [ ] Skew sign confirmed: equity underlyings should exhibit negative risk reversal (put IV > call IV).
- [ ] ATM vol term structure slope sign documented (contango vs backwardation).
- [ ] Every number in output maps to a row in the traceability table.

## Related Skills

- `workflow-derivatives-option-pricing` — uses ATM vol from this surface as the pricing volatility input.
- `workflow-derivatives-option-strategies` — multi-leg strategies require consistent vol inputs from a single surface.
- `workflow-derivatives-structured-products` — exotic pricing requires the full fitted surface or SABR parameters.
- `corp-finance-analyst-derivatives` — agent body with SABR benchmarks and calling conventions.
