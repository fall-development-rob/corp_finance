---
name: "workflow-fi-credit-spreads"
description: |
  WHAT: Decompose a bond yield into government, swap, and credit components; compute Z-spread, OAS, I-spread, and G-spread; perform credit-spread relative value.
  WHEN: Credit relative-value trades, sector rotation decisions, single-name credit screening.
---

# Fixed Income: Credit Spread Decomposition and Relative Value

## What this skill covers

A four-phase pipeline for decomposing a corporate (or structured) bond yield into its constituent spread components, benchmarking against sector peers, and generating a relative-value signal. Spread metrics are computed via `credit_spreads`; the government reference curve is built via `bootstrap_spot_curve`; peer comparison uses `peer_benchmarking`.

## Core Rules

- Spread decomposition sums must reconcile: G-spread ≈ I-spread ± swap basis; OAS ≤ Z-spread (option cost is non-negative for callable bonds).
- Government spot curve must be built on the same as-of date as the bond price.
- Peer set requires minimum 3 bonds in the same rating category, sector, and comparable tenor.
- All spread arithmetic is in basis points; yields in percent to two decimal places.

## Workflow

### Phase 1 — Market Data Collection

1. Obtain the target bond's clean price, coupon, maturity, and credit rating.
   - Preferred: `lseg_bond_pricing` or `factset_bond_pricing` for evaluated pricing.
   - Fallback: derive from `bond_pricer` + `bond_yield` using indicative mid-market yield.

2. Pull the government par curve (same currency, same date):
   - USD: `fred_yield_curve` or `fmp_treasury_rates`
   - EUR/GBP: `lseg_yield_curve` with the relevant sovereign benchmark
   - Bootstrap the spot curve with `bootstrap_spot_curve` (see `workflow-fi-yield-curve-construction`).

3. Pull the swap curve (same currency):
   - USD SOFR OIS curve: `fred_series` with series SOFR or `lseg_yield_curve` (OIS)
   - EUR OIS (ESTR): `lseg_yield_curve`
   - Note the swap basis vs government at the bond's maturity tenor; document any sign/direction.

### Phase 2 — Spread Computation

4. Call `credit_spreads` with: bond price or yield, government spot curve, swap curve, embedded option flag, and settlement date.
   - Outputs:
     - **G-spread** (yield spread to linearly interpolated government par yield at matched maturity)
     - **I-spread** (yield spread to swap rate at matched maturity)
     - **Z-spread** (constant spread over the spot curve that discounts cash flows to the dirty price)
     - **OAS** (Z-spread minus embedded option cost; equals Z-spread for bullet bonds)

5. Verify spread relationship:
   - G-spread ≈ I-spread + (swap − govt at matched tenor) ← swap-to-govt spread adjustment
   - OAS = Z-spread − option-adjusted cost; option cost ≥ 0 for callable bonds
   - If OAS > Z-spread for a callable bond, flag as data error.

6. Optionally call `spread_analysis` for further decomposition into liquidity premium vs pure credit premium:
   - Liquidity premium estimated from bid-ask or using on-the-run / off-the-run differential.
   - Credit premium = OAS − liquidity premium.

### Phase 3 — Peer Benchmarking

7. Define peer set: same currency, same broad sector (e.g., US IG Financials), same rating bucket (e.g., A−/A/A+), maturity within ±2 years.
   - Pull peer bond spreads from `lseg_credit_spreads` or `fred_spread` (sector indices).
   - For each peer: call `credit_spreads` to ensure consistent spread methodology.

8. Call `peer_benchmarking` with the target bond OAS and the peer OAS distribution.
   - Outputs: percentile rank, mean, median, standard deviation of peer OAS, Z-score vs peers.
   - Sector-median OAS: use as the fair-value anchor.

9. RV signal:
   - Target OAS > sector median OAS by > 1 standard deviation → **cheap** (buy signal)
   - Target OAS < sector median OAS by > 1 standard deviation → **rich** (sell / underweight signal)
   - Within ±1 SD → **fair value**

### Phase 4 — Spread Summary and Trade Setup

10. Present spread summary table and RV conclusion.
11. If a trade is being set up:
    - State the spread entry level, target (convergence to sector median), and stop (further widening trigger).
    - Compute spread DV01 = bond DV01 × (spread entry − target) to size the position.
    - Document the hedge: duration-neutral hedge via government bond of matched DV01 if desired.

## Output Format

**Spread Decomposition**

| Metric | Value (bps) | Notes |
|---|---|---|
| YTM | XXX.XX bps over | vs risk-free |
| G-spread | +XXX bps | vs interp govt par |
| I-spread | +XXX bps | vs matched swap rate |
| Z-spread | +XXX bps | constant spread vs spot curve |
| OAS | +XXX bps | option-adjusted (= Z-spread if bullet) |
| Option cost | +XXX bps | Z-spread − OAS; 0 for bullet |
| Liquidity premium | ~XX bps | estimated |
| Pure credit spread | ~XXX bps | OAS − liquidity premium |

**Peer Comparison**

| Bond | Rating | Maturity | OAS (bps) | vs Median |
|---|---|---|---|---|
| Target | AA− | 5Y | XXX | +XX bps (Xth pctile) |
| Peer A | A+ | 5Y | XXX | − |
| ... | | | | |
| Sector median | — | — | XXX | — |

**RV Signal:** [CHEAP / RICH / FAIR VALUE] — Target OAS at XXth percentile vs peer set.

**Tool-Call Traceability Table**

| # | Tool | Key Inputs | Output |
|---|---|---|---|
| 1 | `fred_yield_curve` | as-of date | govt par curve |
| 2 | `bootstrap_spot_curve` | par rates | spot curve, discount factors |
| 3 | `credit_spreads` | bond yield, spot curve, swap curve | G/I/Z/OAS spreads |
| 4 | `spread_analysis` | OAS, bid-ask | liquidity premium |
| 5 | `peer_benchmarking` | target OAS, peer OAS set | percentile, Z-score |

## Quality Gates

- [ ] Spread decomposition consistency: G-spread ≈ I-spread + swap-to-govt basis (±5 bps)
- [ ] OAS <= Z-spread for callable bonds; OAS = Z-spread for bullet bonds
- [ ] Pure credit spread = OAS − liquidity premium and sums to OAS (±1 bps)
- [ ] Peer set has minimum 3 bonds; all priced on same as-of date
- [ ] Sector-median OAS reported with peer set n stated
- [ ] RV signal direction stated with percentile rank
- [ ] All spread figures from tool outputs — no hand-calculated spreads

## Related Skills

- `workflow-fi-bond-valuation` — provides bond pricing inputs and DV01 for spread-DV01 calculation
- `workflow-fi-yield-curve-construction` — provides the government spot curve reference
- `workflow-fi-sovereign-em` — sovereign spread decomposition for EM credit
- `corp-finance-analyst-fixed-income` — deeper credit analysis (CDS, Merton PD, rating migration)
