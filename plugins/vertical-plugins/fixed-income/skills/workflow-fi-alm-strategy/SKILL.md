---
name: "workflow-fi-alm-strategy"
description: |
  WHAT: Asset-liability matching for pension / insurance balance sheets; duration matching, key-rate duration matching, and immunization.
  WHEN: Pension / insurance ALM construction, LDI strategy formulation, surplus risk minimization.
---

# Fixed Income: Asset-Liability Management and LDI Strategy

## What this skill covers

A five-phase pipeline for constructing and validating an asset-liability management strategy for pension funds and insurance companies. Covers liability duration profiling, asset-side duration matching, key-rate gap analysis, immunization portfolio composition, and hedge effectiveness testing. All duration and DV01 figures come from `bond_duration`; ALM and LDI analytics from `alm_analysis`, `pension_funding`, and `ldi_strategy`; hedge validation from `hedge_effectiveness`.

## Core Rules

- Duration-matching is necessary but not sufficient: key-rate duration gaps must also be closed to eliminate curve-twist risk.
- Key-rate DV01 sum must equal total DV01 within ±1%.
- Immunization requires: asset PV ≥ liability PV, asset duration = liability duration, and asset convexity ≥ liability convexity.
- Funded status = asset market value / PV of liabilities; fully funded = 100%; underfunded < 100%.
- Surplus = asset MV − liability PV; surplus DV01 = asset DV01 − liability DV01 (minimize for immunized portfolio).

## Workflow

### Phase 1 — Liability Profiling

1. Collect liability cash-flow schedule:
   - Pension: benefit payment projections by year from actuarial valuation.
   - Insurance: policy reserve cash flows (claims, surrender values) by year.
   - If user provides only summary statistics (PV, duration, convexity), document source and as-of date.

2. Call `pension_funding` with liability cash flows, discount rate (spot curve or flat rate), and actuarial assumptions.
   - Inputs: projected benefit payments by year, discount curve (from `bootstrap_spot_curve` or `fred_yield_curve`), inflation assumption (for CPI-linked benefits).
   - Outputs:
     - **Present value of liabilities (PVL)**
     - **Liability duration** (weighted average maturity of PV-weighted cash flows)
     - **Liability convexity**
     - **Liability DV01** per $1M PVL
     - **Funded status** if current asset market value is provided
     - **Funding shortfall / surplus** in dollar terms

3. Construct the liability key-rate duration profile at standard nodes: 2Y, 5Y, 10Y, 20Y, 30Y.
   - Sum of key-rate DV01s must equal total liability DV01 ±1%.
   - Long-dated pension liabilities typically have the majority of DV01 at the 20Y–30Y nodes; insurance may be more concentrated at 10Y–20Y.

### Phase 2 — Asset-Side Analytics

4. For each asset in the current (or proposed) portfolio, call `bond_duration`:
   - Inputs: bond pricing inputs (coupon, maturity, price, day count).
   - Outputs: key-rate durations, total DV01.

5. Aggregate asset key-rate DV01s at the same nodes (2Y, 5Y, 10Y, 20Y, 30Y).

6. Compute asset-liability key-rate gap:
   - Gap at node n = asset DV01(n) − liability DV01(n)
   - Positive gap: asset DV01 > liability DV01 at that node — over-hedged.
   - Negative gap: asset DV01 < liability DV01 — under-hedged (rate rise hurts funded status).

### Phase 3 — LDI Strategy Construction

7. Call `ldi_strategy` with:
   - Liability PV, duration, convexity, key-rate DV01 profile
   - Current asset DV01 profile
   - Target hedge ratio (e.g., 80% duration match = partial immunization)
   - Available instruments (long Treasuries, STRIPS, Treasury futures, interest rate swaps, LDI credit overlay)
   - Outputs:
     - **Recommended hedge instruments and notionals** to close key-rate gaps
     - **Hedge ratio achieved** at each key-rate node
     - **Residual surplus DV01 after hedging**
     - **Carry cost** of the hedge (if swaps or futures used)

8. Call `alm_analysis` for broader balance sheet impact:
   - Inputs: asset PV, liability PV, surplus, duration gap, funding ratio.
   - Outputs: **surplus volatility** (standard deviation of surplus change per 100 bps rate move), **VaR of surplus** at 95% and 99% confidence, **optimal hedge ratio** to minimize surplus volatility.

9. Immunization check (if full immunization is the goal):
   - Condition 1: asset PV ≥ liability PV (check funded status ≥ 100%).
   - Condition 2: asset total DV01 = liability total DV01 (duration match within 1%).
   - Condition 3: asset convexity ≥ liability convexity (convexity cushion).
   - If any condition fails, document which and quantify the gap.

### Phase 4 — Instrument Structuring

10. For recommended LDI instruments, size the positions:
    - **Long Treasury STRIPS** at the 20Y–30Y nodes: DV01 per $1M STRIPS = approx 0.20 per year of maturity; use `bond_duration` to compute exact DV01.
    - **Receive-fixed interest rate swap** (via `interest_rate_swap` if available): net DV01 equivalent to Treasury of same tenor at lower balance sheet cost.
    - **Treasury futures** (not captured in MCP; flag as out-of-scope; estimate notional from DV01 of CTD bond).

11. Document carry cost:
    - STRIPS: no coupon; negative carry vs liability discount rate if asset yield < liability discount rate.
    - Swaps: SOFR floating payment received, fixed payment made; net carry if SOFR < fixed rate.

### Phase 5 — Hedge Effectiveness Testing

12. Call `hedge_effectiveness` with:
    - Pre-hedge surplus DV01 (from Phase 2)
    - Post-hedge surplus DV01 (after implementing LDI instruments from Phase 3)
    - Prospective effectiveness test: percentage of liability DV01 hedged at each key-rate node.
    - Retrospective test: if historical data is available, compare actual surplus P&L vs predicted P&L from hedge.
    - Outputs: **prospective hedge effectiveness** (%), **residual basis risk**, **pass / fail** (ASC 815 / IAS 39 threshold: ≥ 80% effectiveness required for hedge accounting).

## Output Format

**Liability Profile**

| Metric | Value |
|---|---|
| Present value of liabilities | $XXXm |
| Liability duration | XX.X years |
| Liability convexity | XX.X |
| Total liability DV01 | $XXX per $1M PVL |
| Funded status | XXX% |
| Surplus / (Deficit) | $XXXm |

**Key-Rate Duration Gap**

| Tenor | Asset DV01 ($) | Liability DV01 ($) | Gap ($) | Gap Direction |
|---|---|---|---|---|
| 2Y | X,XXX | X,XXX | +/−XXX | Over / Under |
| 5Y | X,XXX | X,XXX | +/−XXX | — |
| 10Y | X,XXX | X,XXX | +/−XXX | — |
| 20Y | X,XXX | X,XXX | +/−XXX | — |
| 30Y | X,XXX | X,XXX | +/−XXX | — |
| **Total** | **X,XXX** | **X,XXX** | **+/−XXX** | |

**LDI Instrument Recommendation**

| Instrument | Tenor | Notional | DV01 added | Gap closed |
|---|---|---|---|---|
| Treasury STRIPS | 30Y | $XXm | $XXX | 20Y−30Y node |
| Receive-fixed swap | 10Y | $XXm | $XXX | 10Y node |

**Immunization Status**

| Condition | Status | Detail |
|---|---|---|
| Asset PV ≥ Liability PV | PASS / FAIL | Funded at XXX% |
| Duration match | PASS / FAIL | Asset dur: XX.X; Liability dur: XX.X |
| Convexity cushion | PASS / FAIL | Asset conv: XX.X ≥ Liability conv: XX.X |

**Hedge Effectiveness**

| Node | Pre-hedge hedge % | Post-hedge hedge % | Target |
|---|---|---|---|
| 10Y | XX% | XX% | ≥ 80% |
| 20Y | XX% | XX% | ≥ 80% |
| 30Y | XX% | XX% | ≥ 80% |

**Tool-Call Traceability Table**

| # | Tool | Key Inputs | Output |
|---|---|---|---|
| 1 | `pension_funding` | benefit cash flows, discount curve | PVL, duration, DV01 |
| 2 | `bond_duration` | per-asset pricing inputs | asset key-rate DV01 |
| 3 | `ldi_strategy` | asset/liability DV01 profile, instruments | hedge notionals, residual DV01 |
| 4 | `alm_analysis` | surplus, DV01 gap | surplus VaR, optimal hedge ratio |
| 5 | `hedge_effectiveness` | pre/post DV01 | effectiveness %, pass/fail |

## Quality Gates

- [ ] Liability key-rate DV01 sum = total liability DV01 (±1%)
- [ ] Asset key-rate DV01 sum = total asset DV01 (±1%)
- [ ] Post-hedge key-rate gap < 5% of total liability DV01 at each node
- [ ] Immunization conditions 1 (PV), 2 (duration), 3 (convexity) all assessed
- [ ] Hedge effectiveness ≥ 80% at all nodes targeted for LDI accounting
- [ ] Carry cost of hedge documented
- [ ] Surplus DV01 before and after hedge stated

## Related Skills

- `workflow-fi-bond-valuation` — provides key-rate DV01 for individual bond positions
- `workflow-fi-yield-curve-construction` — spot curve used to discount liability cash flows
- `workflow-fi-inflation-linked` — TIPS allocation for CPI-linked liability hedging
- `corp-finance-analyst-fixed-income` — deeper LDI (pension accounting, IFRS 19, ASC 715)
- `corp-finance-analyst-regulatory` — insurance ALM (Solvency II SCR, NSFR)
