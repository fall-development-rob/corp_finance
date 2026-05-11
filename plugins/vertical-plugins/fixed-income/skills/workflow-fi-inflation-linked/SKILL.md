---
name: "workflow-fi-inflation-linked"
description: |
  WHAT: Price TIPS and global inflation-linked bonds; derive real-yield curves and inflation breakevens; price inflation swaps, caps, and floors.
  WHEN: TIPS / linker portfolio MTM, inflation hedge sizing, breakeven trade entry / exit signals.
---

# Fixed Income: Inflation-Linked Instruments

## What this skill covers

A three-phase pipeline covering inflation-linked bond analytics and inflation derivative pricing. Phase 1 prices TIPS and extracts real yields and breakevens. Phase 2 constructs the breakeven curve and interprets inflation expectations. Phase 3 prices zero-coupon and year-on-year inflation swaps, and caps/floors. Every number must come from `tips_analytics` or `inflation_derivatives` — no hand-calculated inflation arithmetic.

## Core Rules

- TIPS real yield = TIPS YTM net of CPI indexation; breakeven inflation = nominal Treasury yield − TIPS real yield at matched maturity.
- 10Y US breakeven of 2.0–2.5% = well-anchored; > 3.0% = elevated inflation fears; < 1.5% = deflation concern.
- Deflation floor: TIPS principal redeems at max(indexed principal, par) — price this floor explicitly using `tips_analytics`.
- Liquidity premium: TIPS yields embed a liquidity premium vs nominal Treasuries (typically 10–30 bps); adjust breakeven accordingly.
- Inflation swap fair value must be benchmarked against TIPS-implied breakeven (should be within 10–15 bps in liquid markets; larger gaps = dislocation).

## Workflow

### Phase 1 — TIPS Pricing and Real Yield

1. Collect TIPS security inputs:

   | Field | Notes |
   |---|---|
   | Coupon (real) | Typically 0.125%–2.0% for recent issuance |
   | Maturity | 5Y, 10Y, 30Y standard tenors |
   | Settlement date | T+1 for on-the-run TIPS |
   | Index Ratio | CPI index ratio = CPI(settlement) / CPI(base date) |
   | Reference CPI series | US: CPI-U (NSA); UK: RPI or CPIH; EU: HICP |

2. Pull current CPI index ratio and reference CPI series:
   - US: `fred_series` with series CPIAUCNS (CPI-U NSA)
   - UK / EU: `lseg_economic_indicators` for RPI or HICP

3. Pull the matched-maturity nominal Treasury yield:
   - `fred_yield_curve` or `fmp_treasury_rates` at the matched tenor.

4. Call `tips_analytics` with real coupon, maturity, index ratio, reference CPI, settlement date, and optional market price.
   - Outputs:
     - **CPI-adjusted principal** (index ratio × face value)
     - **Clean real price** and **dirty real price**
     - **Real YTM** (yield on CPI-adjusted cash flows)
     - **Breakeven inflation rate** (nominal yield − real YTM at matched maturity)
     - **Deflation floor value** (put option on principal; value > 0 for recently issued near-par TIPS)
   - Note: for deeply discounted TIPS (index ratio << 1 due to deflation episode), deflation floor value is material; flag accordingly.

5. Sanity check: real yield must be lower than nominal yield (breakeven > 0 in most environments). If real yield > nominal yield, breakeven is negative — this implies deflation expectations; flag explicitly.

### Phase 2 — Breakeven Curve and Inflation Expectations Analysis

6. Compute breakevens across the standard TIPS tenor curve (5Y, 7Y, 10Y, 20Y, 30Y) by repeating step 4 for each.

7. Adjust for liquidity premium:
   - **Liquidity-adjusted breakeven** = raw breakeven − estimated liquidity premium.
   - Liquidity premium proxy: 10–30 bps for US TIPS vs on-the-run Treasuries (use 15 bps as default; adjust if bid-ask spread data is available).
   - Document the liquidity premium assumption explicitly.

8. Derive the **5Y5Y forward breakeven** (inflation expected 5 years from now over the subsequent 5 years):
   5Y5Y breakeven = ((1 + 10Y breakeven)^10 / (1 + 5Y breakeven)^5)^(1/5) − 1

9. Interpret:
   - 5Y breakeven reflects near-term inflation expectations (more sensitive to current CPI prints).
   - 5Y5Y breakeven reflects long-run inflation anchoring (closer to the central bank's 2% target in anchored regimes).
   - Divergence between 5Y and 5Y5Y > 100 bps signals de-anchoring risk.

### Phase 3 — Inflation Derivative Pricing

10. For an inflation swap or cap/floor request, collect:
    - Notional
    - Tenor (standard: 1Y, 2Y, 5Y, 10Y, 30Y)
    - Floating leg: CPI index
    - Fixed rate (for a fair swap, this equals the breakeven at the tenor)
    - For caps/floors: strike inflation rate

11. Call `inflation_derivatives` with swap type (zero-coupon or year-on-year), notional, tenor, fixed rate, CPI index, and current CPI path from Phase 1.
    - Zero-coupon inflation swap:
      - Fixed leg pays: notional × [(1 + fixed)^T − 1] at maturity
      - Floating leg pays: notional × [(CPI_T / CPI_0) − 1] at maturity
      - Outputs: **fair value**, **mark-to-market PnL** if rate differs from contract rate, **inflation DV01** (change in value per 1 bp change in breakeven).
    - Year-on-year inflation swap:
      - Outputs: **fair value per payment period**, **total mark-to-market**.
    - Inflation cap/floor:
      - Outputs: **cap/floor premium**, **implied inflation volatility**, **delta** (sensitivity to breakeven move).

12. Cross-check: zero-coupon inflation swap fair value at inception should be zero if the fixed rate equals the TIPS-implied breakeven (adjusted for liquidity premium). Document any basis.

## Output Format

**TIPS Pricing**

| Metric | 5Y TIPS | 10Y TIPS | 30Y TIPS |
|---|---|---|---|
| CPI-adjusted principal ($) | X,XXX.XX | X,XXX.XX | X,XXX.XX |
| Real YTM | X.XX% | X.XX% | X.XX% |
| Breakeven inflation (raw) | X.XX% | X.XX% | X.XX% |
| Liquidity premium adjustment | −X bps | −X bps | −X bps |
| Liquidity-adjusted breakeven | X.XX% | X.XX% | X.XX% |
| Deflation floor value | $X.XX | $X.XX | $X.XX |

**Inflation Expectations Summary**

| Measure | Value | Interpretation |
|---|---|---|
| 5Y breakeven | X.XX% | Near-term inflation pricing |
| 10Y breakeven | X.XX% | Medium-term anchor |
| 5Y5Y forward breakeven | X.XX% | Long-run inflation anchor |
| Anchoring status | ANCHORED / ELEVATED / CONCERN | Based on benchmarks above |

**Inflation Derivative**

| Metric | Value |
|---|---|
| Instrument | Zero-coupon / YoY inflation swap |
| Tenor | X years |
| Fixed rate | X.XX% |
| Fair value (at inception) | $X |
| Inflation DV01 | $X per 1 bp breakeven move |

**Tool-Call Traceability Table**

| # | Tool | Key Inputs | Output |
|---|---|---|---|
| 1 | `fred_series` | CPIAUCNS | CPI-U index level |
| 2 | `fred_yield_curve` | as-of date | nominal Treasury par yields |
| 3 | `tips_analytics` | real coupon, maturity, index ratio | real YTM, breakeven, deflation floor |
| 4 | `inflation_derivatives` | swap type, tenor, fixed rate | fair value, inflation DV01 |

## Quality Gates

- [ ] CPI index ratio sourced from market data tool, not hand-keyed
- [ ] Real yield is lower than nominal yield (breakeven > 0) or deflation expectation flagged explicitly
- [ ] Liquidity premium adjustment stated and documented (default 15 bps if bid-ask unavailable)
- [ ] 5Y5Y forward breakeven computed and interpreted
- [ ] 10Y US breakeven benchmarked: 2.0–2.5% = anchored; deviations flagged
- [ ] Inflation swap fair value cross-checked to TIPS-implied breakeven (basis < 15 bps or dislocation explained)
- [ ] Deflation floor value reported for near-par TIPS issuance

## Related Skills

- `workflow-fi-bond-valuation` — nominal bond pricing for breakeven spread computation
- `workflow-fi-yield-curve-construction` — builds the nominal curve for breakeven derivation
- `workflow-fi-alm-strategy` — uses real yield curve for inflation-matching in pension portfolios
- `corp-finance-analyst-fixed-income` — deeper inflation derivatives (caps, floors, swaption overlays)
