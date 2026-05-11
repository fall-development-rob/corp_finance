---
name: "workflow-fi-bond-valuation"
description: |
  WHAT: Price corporate and government bonds; compute YTM, duration (Macaulay / modified / effective), convexity, and DV01.
  WHEN: Bond MTM runs, single-name credit screening, yield-curve relative-value setup, portfolio risk attribution prep.
---

# Fixed Income: Bond Valuation

## What this skill covers

Full bond analytics pipeline for pricing corporate and government bonds and deriving risk metrics. Outputs a priced bond sheet with clean and dirty price, yield measures, duration metrics, DV01, and rate-sensitivity table. Every figure must trace to a `cfa-core` tool call — no LLM-generated arithmetic.

## Core Rules

- All computation via `bond_pricer`, `bond_yield`, `bond_duration` — never hand-calculated.
- Explicit day count convention required for every bond (Act/Act, 30/360, Act/360, Act/365).
- Settlement date, coupon frequency, face value, and benchmark curve stated before any tool call.
- DV01 and duration figures must agree to within 0.1% (cross-check: DV01 ≈ modified duration × price × 0.0001 × face / 100).

## Workflow

### Phase 1 — Bond Specification

Collect from the user or data source:

| Field | Notes |
|---|---|
| Issuer, CUSIP / ISIN | For cross-referencing market data |
| Face value | Typically 1,000 or 100 for normalised |
| Coupon rate and frequency | Semi-annual for US; annual for most EUR sovereigns |
| Maturity date | Required for YTM and duration |
| Day count convention | US Treasuries: Act/Act; US corp: 30/360 |
| Settlement date | Typically T+1 for govts; T+2 for corps |
| Discount rate / required yield | If pricing to a required yield; else solve for YTM |

Use `fmp_treasury_rates` or `fred_yield_curve` to pull the benchmark par curve if market yield is not supplied.

### Phase 2 — Pricing

1. Call `bond_pricer` with coupon, maturity, face value, yield (or price), settlement date, and day count.
   - Outputs: **clean price**, **dirty price** (clean + accrued), **accrued interest**, **accrued days**.
2. If only clean price is supplied, derive yield by calling `bond_yield`.
   - Outputs: **YTM** (bond-equivalent yield), **BEY**, **effective annual yield**, **current yield**.
3. Cross-check: BEY = 2 × semi-annual yield; effective annual yield = (1 + BEY/2)² − 1.

### Phase 3 — Duration and Convexity

4. Call `bond_duration` with pricing inputs from Phase 2.
   - Outputs: **Macaulay duration** (in years), **modified duration**, **effective duration**, **convexity**, **DV01**, **key-rate durations** (2Y, 5Y, 10Y, 30Y nodes).
5. Sanity checks:
   - Macaulay duration <= maturity; for zero-coupon bond, Macaulay = maturity exactly.
   - Modified duration = Macaulay duration / (1 + YTM / coupon frequency).
   - Convexity must be positive for plain vanilla bonds.
   - DV01 = modified duration × dirty price × 0.0001 (verify ±1%).

### Phase 4 — Sensitivity Table

6. Compute price change for ±25 bps, ±50 bps, ±100 bps, ±200 bps shifts using the duration–convexity approximation:

   ΔP/P ≈ −ModDur × Δy + ½ × Convexity × (Δy)²

7. Present results as a structured table (see Output format).
8. If the bond is callable or putable, flag that effective duration from `bond_duration` (OAS-based) should replace modified duration for the sensitivity table.

### Phase 5 — Benchmark Spread

9. Pull the benchmark yield for the matched tenor from `fred_yield_curve` or `fmp_treasury_rates`.
10. Compute spread-to-benchmark = bond YTM − benchmark yield at matched maturity.
11. Note whether this is an interpolated benchmark (state interpolation method: linear or log-linear).

## Output Format

**Bond Pricing Summary**

| Metric | Value |
|---|---|
| Clean price | XXX.XXX |
| Dirty price | XXX.XXX |
| Accrued interest | XX.XX |
| YTM (BEY) | XX.XX% |
| Effective annual yield | XX.XX% |
| Current yield | XX.XX% |
| Macaulay duration | X.XX yrs |
| Modified duration | X.XX yrs |
| Effective duration | X.XX yrs |
| Convexity | XX.XX |
| DV01 | $XX.XX per $1M face |
| Spread to benchmark | +XXX bps |

**Price Sensitivity Table**

| Rate Shift | Duration approx | Duration + Convexity approx |
|---|---|---|
| −200 bps | +XX.XX% | +XX.XX% |
| −100 bps | +XX.XX% | +XX.XX% |
| −50 bps | +XX.XX% | +XX.XX% |
| −25 bps | +XX.XX% | +XX.XX% |
| +25 bps | −XX.XX% | −XX.XX% |
| +50 bps | −XX.XX% | −XX.XX% |
| +100 bps | −XX.XX% | −XX.XX% |
| +200 bps | −XX.XX% | −XX.XX% |

**Tool-Call Traceability Table**

| # | Tool | Key Inputs | Output |
|---|---|---|---|
| 1 | `bond_pricer` | coupon, maturity, yield, day count | clean/dirty price, accrued |
| 2 | `bond_yield` | price, coupon, maturity | YTM, BEY, effective yield |
| 3 | `bond_duration` | all pricing inputs | ModDur, EffDur, Convexity, DV01 |
| 4 | `fred_yield_curve` | tenor | benchmark par yield |

## Quality Gates

- [ ] Day count convention explicitly stated
- [ ] Dirty price = clean price + accrued interest (±0.001)
- [ ] Modified duration = Macaulay / (1 + YTM/freq) verified (±0.01)
- [ ] DV01 = ModDur × dirty price × 0.0001 verified (±1%)
- [ ] Convexity is positive for plain vanilla bond
- [ ] Sensitivity table covers ±200 bps at minimum
- [ ] Spread-to-benchmark stated with tenor interpolation method noted
- [ ] Every number in the summary has a row in the traceability table

## Related Skills

- `workflow-fi-yield-curve-construction` — build the benchmark curve used in Phase 5
- `workflow-fi-credit-spreads` — decompose the spread-to-benchmark into credit components
- `workflow-fi-alm-strategy` — consume DV01 and key-rate durations for liability matching
- `corp-finance-analyst-fixed-income` — deep specialist for complex structures (callable, putable, convertible)
