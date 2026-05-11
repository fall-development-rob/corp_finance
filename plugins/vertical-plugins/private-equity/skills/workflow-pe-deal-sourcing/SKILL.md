---
name: workflow-pe-deal-sourcing
description: |
  WHAT: Proactive deal origination and pipeline management — define screening criteria, screen the universe via FMP tools, profile candidates, and maintain a prioritised funnel from universe to exclusive.
  WHEN: Invoke when asked to source deals, build a deal pipeline, identify acquisition targets, or produce a screened candidate list for a PE mandate.
---

# PE Deal Sourcing

## What this skill covers

Proactive deal origination: translating a fund mandate into a screened long-list of targets, profiling each company, and prioritising the funnel. Output is a ranked candidate pipeline ready for screening-memo review.

## Workflow

### Step 1 — Define screening criteria

Document the fund mandate parameters:
- Sector(s) and sub-verticals within mandate (or adjacent with explicit rationale)
- EBITDA target range (e.g., $20-100M for mid-market)
- Geography (domestic, regional, or global)
- Revenue growth minimum (e.g., >5% CAGR)
- EBITDA margin minimum (e.g., >15%)
- Entry leverage ceiling (typically 4-6x Net Debt/EBITDA)

### Step 2 — Screen the universe

Call `fmp_stock_screener` with financial filters derived from Step 1:
- Market cap range, revenue growth, EBITDA margin, leverage ratio
- Sector and geography constraints

Output: 50-100 raw candidates. Remove obvious misfits (regulated utilities, financials, micro-caps below liquidity threshold).

### Step 3 — Profile each shortlisted company

Call `fmp_company_profile` (or `fmp_profile`) for each candidate:
- Business description, sector, headquarters, employee count
- Revenue and EBITDA (most recent annual)
- Market cap, enterprise value, implied EV/EBITDA

Call `fmp_key_metrics` for margin and growth trends.

### Step 4 — Prioritise the funnel

Rank remaining candidates by attractiveness and feasibility:

| Criterion | Weight | Proxy |
|-----------|--------|-------|
| Market position / moat | 30% | Gross margin vs peers |
| Revenue growth | 25% | 3-year CAGR |
| EBITDA margin quality | 25% | EBITDA margin vs sector |
| Availability / deal probability | 20% | Analyst judgement |

Funnel stages:
- Universe (50-100) → Long list (15-25) → Short list (5-10) → Active DD (2-3) → Exclusive (1)

### Step 5 — Produce the pipeline table

| Company | Sector | Revenue ($M) | EBITDA ($M) | EV/EBITDA (x) | Attractiveness | Feasibility | Stage |
|---------|--------|-------------|-------------|---------------|---------------|------------|-------|

Include a one-sentence thesis per company on the short list.

## Output format

1. **Pipeline summary** — funnel count at each stage, sector breakdown
2. **Ranked candidate table** — one row per company with key metrics and stage
3. **Short-list profiles** — 3-5 sentences per company: business, thesis, key risk
4. **Next steps** — recommended outreach approach and timing for top 3

## Quality gates

- [ ] Screening criteria document mandate constraints explicitly
- [ ] `fmp_stock_screener` filters match mandate parameters
- [ ] Each shortlisted company profiled with revenue, EBITDA, and EV/EBITDA
- [ ] Funnel tracks stage per company with rationale for advancement
- [ ] Short list limited to 10 or fewer; every entry has a one-sentence thesis

## Related skills

- `workflow-pe-due-diligence` — next step for candidates entering active DD
- `workflow-pe-ic-memo` — formal recommendation after DD completion
- `workflow-pe-returns-analysis` — preliminary returns model to validate entry multiple
