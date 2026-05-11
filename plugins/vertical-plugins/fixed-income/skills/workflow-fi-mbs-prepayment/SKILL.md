---
name: "workflow-fi-mbs-prepayment"
description: |
  WHAT: Model MBS prepayment (PSA ramp, CPR), compute OAS, effective duration adjusted for prepayment risk, and convexity adjustment.
  WHEN: MBS portfolio MTM, prepayment-risk hedging, agency RMBS / CMBS investment analysis.
---

# Fixed Income: MBS Prepayment and Analytics

## What this skill covers

A three-phase pipeline for modelling mortgage-backed securities: prepayment speed estimation, MBS pass-through cash-flow analytics (OAS, WAL, effective duration), and convexity-adjustment interpretation. Captures negative convexity — the defining structural feature of mortgage securities — and sizes hedge requirements.

## Core Rules

- 100% PSA is the standard prepayment ramp; 150–200% PSA during strong rate rallies; 50–75% PSA in rising-rate environments.
- Agency MBS OAS benchmarks: 30–80 bps for on-the-run agency pass-throughs; flag OAS outside this range.
- Negative convexity must be documented: callable/prepayable securities always show convexity < 0 for premium-priced bonds.
- WAC (weighted average coupon), WAM (weighted average maturity), and servicing fee must be stated before any tool call.

## Workflow

### Phase 1 — Security Specification and Prepayment Modelling

1. Collect MBS security characteristics:

   | Field | Notes |
   |---|---|
   | Pool type | Agency (FNMA / FHLMC / GNMA) or non-agency |
   | Coupon (pass-through rate) | Net of servicing fee |
   | WAC | Gross coupon of underlying mortgages |
   | Servicing fee | WAC − pass-through rate |
   | WAM | Weighted average remaining maturity (months) |
   | Original term | 30Y, 15Y, etc. |
   | Pricing speed assumption | PSA or CPR |

2. Call `prepayment_analysis` with pool characteristics and current mortgage rate environment.
   - Inputs: WAC, WAM, current 30Y mortgage rate, model type (PSA or CPR), base PSA assumption.
   - Outputs:
     - **PSA speed** (% of standard ramp)
     - **CPR** (conditional prepayment rate, annual)
     - **SMM** (single monthly mortality = 1 − (1 − CPR)^(1/12))
     - **Refinancing incentive** (WAC vs current mortgage rate spread)
     - **Burnout adjustment** — seasoned pools show lower sensitivity (apply when WAM << original term)
   - Justify the PSA assumption: document rate environment, pool seasoning, loan size, FICO distribution if available.

3. Sensitivity: run `prepayment_analysis` at PSA −50%, PSA base, PSA +50% to bracket prepayment uncertainty.

### Phase 2 — MBS Cash Flow Analytics

4. Call `mbs_analytics` with pool characteristics and prepayment output from Phase 1.
   - Inputs: pass-through rate, WAC, WAM, face balance, prepayment speed (SMM or CPR), discount curve (government spot or OAS-shifted).
   - Outputs:
     - **Projected monthly cash flows** (principal + interest schedule)
     - **WAL** (weighted average life in years)
     - **OAS** (option-adjusted spread over the benchmark spot curve)
     - **Effective duration** (OAS-based; accounts for prepayment optionality)
     - **Effective convexity** (negative for premium MBS)
     - **DV01** per $1M face

5. Interpret OAS:
   - Agency MBS: OAS 30–80 bps = fairly priced; OAS < 20 bps = rich (prepayment risk underpriced); OAS > 100 bps = cheap or structural/liquidity issue.
   - Non-agency: wider OAS reflects credit risk premium in addition to prepayment optionality.

6. Interpret negative convexity:
   - Discount MBS (price < par): positive convexity — prepayments are slow; duration extends as rates rise.
   - Par MBS: approximately zero convexity.
   - Premium MBS (price > par): negative convexity — prepayments accelerate as rates fall, capping price appreciation.
   - Quantify: "For a −100 bps rate move, negative convexity reduces price appreciation by approximately [Convexity × (0.01)² / 2] × 100% of face."

### Phase 3 — Hedge Sizing

7. Compute the duration-equivalent Treasury hedge to neutralise rate risk:
   - Target: match the MBS effective DV01 with a short Treasury position.
   - Call `bond_duration` on the hedge Treasury (matched to MBS WAL).
   - Hedge ratio = MBS DV01 / Treasury DV01.
   - Note: the convexity mismatch (negative MBS convexity vs positive Treasury convexity) must be managed dynamically; flag the gamma requirement.

8. Convexity hedge (if required): an options overlay (cap / floor or swaption) is required to neutralise negative convexity. State the notional and tenor of the recommended options overlay. Refer to `workflow-fi-inflation-linked` or derivatives skill for pricing.

## Output Format

**Prepayment Assumptions**

| Parameter | Value | Notes |
|---|---|---|
| PSA speed | XXX% | Justified by rate environment |
| CPR | XX.X% | Annual |
| SMM | X.XX% | Monthly |
| Refinancing incentive | +XX bps | WAC vs current mortgage rate |
| Burnout applied | Yes / No | Pool seasoning |

**MBS Analytics**

| Metric | Discount PSA | Base PSA | Premium PSA |
|---|---|---|---|
| WAL (years) | XX.X | XX.X | XX.X |
| OAS (bps) | +XXX | +XXX | +XXX |
| Effective duration | X.XX | X.XX | X.XX |
| Effective convexity | +X.XX | −X.XX | −X.XX |
| DV01 ($M face) | $XXX | $XXX | $XXX |

**Convexity Adjustment Summary**

For ±100 bps rate shock, price appreciation drag from negative convexity: approximately −XX bps vs a duration-equivalent Treasury.

**Hedge**

| Instrument | Notional | DV01 match | Convexity |
|---|---|---|---|
| Short Treasury (WAL-matched) | $XXM | = MBS DV01 | Positive |
| Swaption overlay (if needed) | $XXM | — | Offsets negative Δ |

**Tool-Call Traceability Table**

| # | Tool | Key Inputs | Output |
|---|---|---|---|
| 1 | `prepayment_analysis` | WAC, WAM, current rate, PSA | PSA speed, CPR, SMM |
| 2 | `mbs_analytics` | pass-through, WAC, WAM, CPR, spot curve | WAL, OAS, eff duration, convexity |
| 3 | `bond_duration` | hedge Treasury pricing inputs | DV01, modified duration |

## Quality Gates

- [ ] PSA assumption justified with reference to rate environment and pool seasoning
- [ ] Prepayment run at three PSA speeds (base, base−50%, base+50%)
- [ ] Agency MBS OAS within 30–80 bps or deviation explained
- [ ] Convexity sign correct: negative for premium MBS, positive for discount MBS
- [ ] DV01 from `mbs_analytics` agrees with effective duration × price × 0.0001 (±2%)
- [ ] Hedge ratio computed and stated
- [ ] Gamma / convexity mismatch flagged if convexity hedge not in place

## Related Skills

- `workflow-fi-bond-valuation` — plain-vanilla bond duration for comparison baseline
- `workflow-fi-yield-curve-construction` — spot curve used as OAS discount reference
- `workflow-fi-credit-spreads` — for non-agency MBS with credit component in OAS
- `corp-finance-analyst-fixed-income` — deeper CLO / structured credit analytics
