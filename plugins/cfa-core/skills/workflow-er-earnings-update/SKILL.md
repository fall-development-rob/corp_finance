---
name: "workflow-er-earnings-update"
description: |
  WHAT: Two-mode earnings workflow — (1) post-release earnings update note (8-12 pages) covering beat/miss analysis, guidance revision impact, estimate revisions, thesis assessment, and price target update; (2) pre-release earnings preview note (3-5 pages) covering consensus expectations, key metrics to watch, historical surprise patterns, and scenario analysis.
  WHEN: Invoke after a quarterly or annual earnings release to produce an earnings update note, or ahead of a scheduled earnings release to produce an earnings preview. Requires an existing coverage position (use workflow-er-initiating-coverage first).
---

# Equity Research: Earnings Analysis (Update and Preview)

## What this skill covers

Two workflows for earnings-driven research notes: the post-release earnings update (8-12 pages) and the pre-release earnings preview (3-5 pages). Both produce tool-sourced analysis with updated estimates, thesis assessment, and revised price target.

## Inputs

- Company ticker and reporting period
- Existing coverage: current price target, rating, bull/base/bear scenarios
- Mode: **update** (post-release) or **preview** (pre-release)

## Earnings Update Workflow (post-release, 8-12 pages)

### Step 1 — Beat/miss summary

Call `fmp_earnings` — compare actual EPS, revenue vs consensus estimates:
- Magnitude of surprise: % beat/miss on revenue and EPS
- Quality of beat: operating vs below-the-line items (tax rate benefit, share buybacks)
- Headline vs adjusted: confirm which metric management highlighted

### Step 2 — Guidance revision impact

Call `fmp_analyst_estimates` — pull forward estimates and compare to prior guidance:
- Revenue guide: raised / maintained / lowered vs street
- Margin commentary: input cost trends, pricing power, product mix shift
- Quantify the guidance delta vs prior-period guide and vs street consensus

### Step 3 — Updated estimates

Revise revenue, EBITDA, and EPS forecasts:
- Show old vs new estimates side-by-side with change rationale
- Flow through to updated `three_statement_model` if revision is material (>5% change in any year)
- Update `dcf_model` inputs with revised projections

### Step 4 — Thesis impact assessment

Does this quarter change the bull/base/bear framework?
- **Thesis confirming:** results in-line or better — maintain rating; note supporting data points
- **Thesis challenging:** negative surprise on a key driver — reassess case weights
- **Thesis breaking:** core assumption proven wrong — trigger downgrade review

Document which scenario's probability weight changes and by how much.

### Step 5 — Price target update

Call `target_price` with revised inputs:
- Updated DCF value from `dcf_model`
- Updated comps implied value from `comps_analysis`
- Revised blended target
- Updated `sensitivity_matrix`: WACC ±100bps vs TGR ±50bps

### Report structure (8-12 pages)

1. Headline (rating, old vs new PT, recommendation)
2. Quarterly results summary (beat/miss table)
3. Key themes from the quarter (3-5 bullets)
4. Estimate revision table (old vs new by year)
5. Thesis assessment (confirming / challenging / breaking)
6. Updated valuation summary
7. Updated sensitivity tables
8. Risk factors (unchanged or updated)

## Earnings Preview Workflow (pre-release, 3-5 pages)

### Step 1 — Consensus expectations

Call `fmp_analyst_estimates` — current street estimates for revenue, EPS, key segment metrics:
- Consensus revenue, EBITDA, EPS for the upcoming quarter
- Implied growth rates vs prior-year quarter
- Estimate revision trend (up / down / stable over last 30 days)

### Step 2 — Key items to watch

Identify 3-5 metrics that will drive the stock reaction:
- Margins: gross margin trajectory, SGA leverage, operating margin trend
- Guidance: forward quarter and full-year outlook — will it be raised, maintained, or lowered?
- Segment mix: growth vs mature segment contribution
- Special items: restructuring charges, M&A commentary, FX headwinds

### Step 3 — Historical surprise pattern

Call `fmp_earnings` for last 4-8 quarters of beat/miss history:
- Directional bias: does the company consistently beat or guide conservatively?
- Typical surprise magnitude in % terms (revenue and EPS)
- Most recent surprise and stock reaction

### Step 4 — Scenario analysis

Model beat/miss/inline impacts on price target using `sensitivity_matrix`:
- Beat scenario: estimate + 1 standard deviation; implied stock reaction and revised PT
- Miss scenario: estimate - 1 standard deviation; implied stock reaction and revised PT
- Inline scenario: numbers in-line but guidance tone drives the reaction

### Report structure (3-5 pages)

1. Preview summary (what we expect, key debates)
2. Consensus expectations table
3. Key items to watch (3-5 bullets with context)
4. Historical surprise pattern summary
5. Pre-earnings scenario table (beat / inline / miss)
6. Positioning recommendation

## Output format

- **Update note:** 8-12 pages with beat/miss table, old vs new estimate table, updated PT summary, updated sensitivity tables
- **Preview note:** 3-5 pages with consensus table, key items, scenario table

## Quality gates

- [ ] Actuals vs consensus sourced from `fmp_earnings` and `fmp_analyst_estimates` — not manually entered
- [ ] Price target revision uses `target_price` and `dcf_model` — not narrative judgment
- [ ] Three scenarios explicitly assessed (confirming / challenging / breaking)
- [ ] Old vs new estimates shown side-by-side with change rationale
- [ ] `sensitivity_matrix` updated for new inputs

## Related skills

- `workflow-er-initiating-coverage` — produces the baseline PT, rating, and scenarios updated here
- `workflow-er-thesis-tracker` — thesis evolution framework that tracks scenario drift across multiple earnings
- `workflow-er-model-update` — full model revision when the earnings change is material
- `workflow-er-morning-note` — same-day post-earnings brief for the trading desk
