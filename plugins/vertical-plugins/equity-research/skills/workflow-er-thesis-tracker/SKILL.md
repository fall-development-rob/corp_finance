---
name: "workflow-er-thesis-tracker"
description: |
  WHAT: Investment thesis memo (3-5 pages) with bull/base/bear scenario definitions, quantified price targets per case, catalyst milestones with dates and expected impact, probability weights, and a quarterly drift-detection update cadence. Tracks whether incoming data confirms, challenges, or breaks the thesis.
  WHEN: Invoke when establishing or updating an investment thesis framework for a covered company — either at initiation (alongside workflow-er-initiating-coverage) or as a standalone quarterly review when new catalysts have resolved.
---

# Equity Research: Investment Thesis Tracker

## What this skill covers

Structures the bull/base/bear thesis framework with explicit scenario definitions, price targets, probability weights, and milestone catalysts. Defines the update protocol for quarterly thesis drift detection — flagging when actual results diverge from the base case and triggering a re-rating review when two or more catalysts resolve in the same direction.

## Inputs

- Company ticker and current coverage rating
- Existing price target and valuation model output (from `workflow-er-initiating-coverage` or `workflow-er-model-update`)
- List of known upcoming catalysts with dates

## Workflow

### Step 1 — Bull case definition

Define the upside scenario with:
- 3-5 specific catalysts that drive the bull case (e.g., margin expansion, market share gain, product launch success)
- Bull case revenue, EBITDA, EPS assumptions (explicit; not narrative)
- Call `dcf_model` with bull case inputs to derive the bull price target
- Quantify upside: bull PT vs current price, implied return %
- Assign probability weight (typically 20-30%)

### Step 2 — Base case definition

Define the central estimate:
- Core assumptions for revenue growth, margin, multiple — most likely outcome
- Call `dcf_model` with base case inputs to confirm the published price target
- Call `comps_analysis` for trading multiple cross-check
- This is the published price target and the current rating's basis
- Assign probability weight (typically 50-60%)

### Step 3 — Bear case definition

Define the downside scenario with:
- 3-5 specific risks that drive the bear case (e.g., competitive entry, margin compression, end-market demand weakness)
- Bear case revenue, EBITDA, EPS assumptions (explicit)
- Call `dcf_model` with bear case inputs to derive the bear price target
- Quantify downside: bear PT vs current price, implied loss %
- Assign probability weight (typically 15-25%)

**Validation:** bull + base + bear probability weights must sum to 100%.

### Step 4 — Probability-weighted price target

Compute: `PT_weighted = (bull_PT × bull_weight) + (base_PT × base_weight) + (bear_PT × bear_weight)`

The published price target is the base case PT; the probability-weighted PT is reported alongside for reference.

### Step 5 — Catalyst milestone register

For each catalyst (from all three cases), create a register entry:

| Catalyst | Expected date | Scenario relevance | Expected impact | Resolved? |
|----------|--------------|-------------------|-----------------|-----------|
| Q2 margin guidance | Q2 2026 earnings | Base / Bull | ±150bps on EBITDA | No |
| FDA approval, drug X | H1 2026 | Bull only | +20% revenue in FY2027 | No |

Track resolution quarterly. A catalyst resolves as: **confirming** (aligns with base or bull), **neutral**, or **challenging** (aligns with bear).

### Step 6 — Quarterly drift detection

At each quarterly update:
1. Mark resolved catalysts in the register
2. Tally: how many resolved as confirming vs challenging?
3. Trigger rule: if 2+ catalysts resolve in the same direction (both confirming or both challenging), initiate a scenario re-rating review
4. Call `sensitivity_matrix` to quantify the scenario weight shift
5. Document thesis status: **on-track** / **drifting** / **broken**

When thesis is **broken** (core assumption definitively refuted), initiate a rating change review and call `target_price` with updated case weights.

## Output format

1. **Thesis memo (3-5 pages)** — structured as: bull case, base case, bear case, probability weights, probability-weighted PT
2. **Catalyst milestone register** — table as in Step 5, updated each quarter
3. **Quarterly update log** — per quarter: catalysts resolved, thesis status, weight changes

## Quality gates

- [ ] All three scenario price targets derived from `dcf_model` — not narrative estimates
- [ ] Probability weights sum to 100%
- [ ] At least 3 catalysts per scenario defined with specific dates
- [ ] Drift detection rule documented and applied each quarter
- [ ] Rating change review triggered when 2+ catalysts break in the same direction

## Related skills

- `workflow-er-initiating-coverage` — produces the initial model and valuation that seed the thesis cases
- `workflow-er-earnings-update` — quarterly earnings notes trigger catalyst resolution updates
- `workflow-er-model-update` — model revision invoked when thesis drifts materially
- `workflow-er-idea-screening` — provides quantitative backing for new thesis ideas
