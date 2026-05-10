---
name: "workflow-er-model-update"
description: |
  WHAT: Financial model revision and accompanying analyst note when material new information arrives — quarterly earnings, guidance change, M&A announcement, or macro shift. Produces an updated three-statement model, revised DCF and price target, and a side-by-side old-vs-new estimate table with change rationale.
  WHEN: Invoke when new information is material enough to change model assumptions by >5% in any forecast year — following an earnings release with a guidance change, an M&A announcement affecting the company, or a significant macro shift (interest rate change, FX move, commodity price shock).
---

# Equity Research: Financial Model Update

## What this skill covers

A structured model revision workflow triggered by material new information. Isolates which assumptions changed, updates the three-statement model, recalculates valuation, and produces a concise revision note with side-by-side estimates and a rating decision.

## Inputs

- Company ticker and the triggering event (earnings, guidance change, M&A, macro shift)
- Prior model assumptions (from the most recent `workflow-er-initiating-coverage` or previous model update)
- Updated inputs provided by the triggering event

## Workflow

### Step 1 — Identify changed assumptions

Pull latest actuals or announcements:
- For earnings: call `fmp_income_statement` with latest period
- For M&A: pull transaction details and synergy assumptions
- For macro: note interest rate, FX, or commodity change and its P&L impact

Compare actual vs prior model assumptions. Identify:
- Which line items changed (revenue, margin, capex, WACC, other)
- Direction and magnitude of each change
- Whether the change is one-time or persistent (affects forecast run-rate)

Materiality threshold: changes >5% to any forecast-year revenue, EBITDA, or EPS trigger a full model update. Changes <5% trigger a note only.

### Step 2 — Update the three-statement model

Call `three_statement_model` with revised assumptions:
- Adjust revenue growth, gross margin, operating margin, capex, working capital as warranted
- Re-solve the interest expense / revolver draw circular reference
- Verify: A = L + E in every forecast period after update

### Step 3 — Recalculate valuation

Call `dcf_model` with updated projections:
- Revised DCF value with updated free cash flow stream
- If WACC changes (due to macro shift): recompute via `wacc_calculator` first

Call `target_price` with updated DCF and comps:
- Call `comps_analysis` if peer multiples have shifted materially
- Produce the revised blended price target

Call `sensitivity_matrix` with updated inputs:
- WACC ±100bps vs TGR ±50bps minimum
- Add the changed variable as a third sensitivity axis if it is the primary driver

### Step 4 — Rating review

Assess whether the model revision changes the rating:
- **Maintain:** new PT within ±10% of prior PT, thesis unchanged
- **Upgrade:** new bull/base PT materially above prior, thesis improving
- **Downgrade:** new bear/base PT materially below prior, thesis deteriorating

If a rating change is warranted, trigger `workflow-er-thesis-tracker` to update scenario weights.

### Step 5 — Revision note

Produce the model update note:

1. **Headline:** old PT → new PT, rating maintained/changed, event trigger
2. **What changed:** bullet list of revised assumptions with direction and magnitude
3. **Old vs new estimates table:**

| Metric | FY[prior] actual | FY[n] old est | FY[n] new est | Change | FY[n+1] old | FY[n+1] new | Change |
|--------|-----------------|--------------|--------------|--------|-------------|-------------|--------|
| Revenue ($M) | | | | | | | |
| EBITDA ($M) | | | | | | | |
| EBITDA margin (%) | | | | | | | |
| EPS ($) | | | | | | | |

4. **Valuation update:** old PT, new PT, methodology (DCF weight, comps weight)
5. **Updated sensitivity tables**
6. **Rating decision with rationale**

## Output format

- Revision note: 2-4 pages (concise, focused on what changed and why)
- Updated estimate table (old vs new side-by-side)
- Updated sensitivity matrix
- Rating decision with explicit rationale

## Quality gates

- [ ] Materiality check documented (>5% change triggers full update)
- [ ] `three_statement_model` re-run with revised inputs — not adjusted manually
- [ ] `dcf_model` and `target_price` re-run with updated free cash flows
- [ ] A = L + E verified post-update
- [ ] Old vs new estimates table complete for at least 2 forward years
- [ ] Rating decision explicitly stated: maintain / upgrade / downgrade

## Related skills

- `workflow-er-earnings-update` — earnings notes that frequently trigger model updates
- `workflow-er-thesis-tracker` — scenario weight revisions when a rating change is warranted
- `workflow-er-initiating-coverage` — original model that this skill updates
- `workflow-deal-citation-standards` — citation format for revised tool outputs in the revision note
