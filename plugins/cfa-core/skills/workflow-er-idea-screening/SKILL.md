---
name: "workflow-er-idea-screening"
description: |
  WHAT: Quantitative and thematic investment idea generation — applying Piotroski F-Score, Beneish M-Score, FCF yield, ROIC vs WACC, and revenue growth filters to a broad universe, then overlaying thematic qualitative criteria to produce a ranked list of deep-dive candidates with one-paragraph thesis summaries.
  WHEN: Invoke when generating new investment ideas for a coverage universe or sector, performing quantitative stock screening, or responding to a mandate such as "find quality growth names in [sector]" or "screen for value with fundamental momentum".
---

# Equity Research: Investment Idea Screening

## What this skill covers

A two-stage funnel — quantitative filters followed by thematic overlay — that narrows a broad universe to a ranked shortlist of deep-dive candidates. Each surviving candidate receives a one-paragraph thesis summary and a composite score.

## Inputs

- Universe definition: broad index, sector, or custom ticker list
- Thematic mandate (optional): sector focus, factor tilt (quality, value, growth, momentum), or ESG constraint
- Screening stringency: standard (Piotroski >= 7, Beneish < -1.78) or custom thresholds

## Workflow

### Step 1 — Define the universe

Start with the broadest feasible universe. Options:
- A sector via `fmp_stock_screener` (filter by sector, market cap floor)
- An index via `fmp_index_constituents`
- A custom ticker list provided by the user

Record the starting universe count.

### Stage 1 — Quantitative screens

Apply hard filters sequentially. Record pass/fail count at each gate.

**Gate 1: Fundamental quality — Piotroski F-Score**

Call `piotroski_fscore` for each company. Pass criterion: score >= 7 (strong fundamentals on profitability, leverage, and operating efficiency dimensions). Fail: score < 7.

**Gate 2: Earnings integrity — Beneish M-Score**

Call `beneish_mscore` for each passing company. Pass criterion: M-Score < -1.78 (no manipulation signal). Fail: M-Score >= -1.78 (potential manipulation — exclude or flag for extra scrutiny).

**Gate 3: Value creation — ROIC vs WACC**

Call `fmp_key_metrics` for ROIC. Call `wacc_calculator` for WACC estimate. Pass criterion: ROIC > WACC (company destroys or creates value). Fail: ROIC <= WACC.

**Gate 4: Cash generation — FCF yield and revenue growth**

Call `fmp_key_metrics` for FCF yield. Call `fmp_financial_ratios` for revenue growth vs sector median. Pass criteria: FCF yield > 5% AND revenue growth > sector median. Fail: either criterion misses.

Record the universe count after Stage 1: [N] companies passed all four gates.

### Stage 2 — Thematic overlay (qualitative filters)

For each Stage 1 survivor, apply the thematic mandate:

1. **Sector trends:** is the company operating in an industry with secular tailwinds (not just cyclical recovery)?
2. **TAM growth:** is the addressable market expanding >5% annually? (use industry sources or World Bank data)
3. **Regulatory tailwinds:** are pending policy changes a net positive for this company's model?
4. **Management alignment:** recent insider buying or low turnover at senior level (check `fmp_insider_latest`)

Each criterion scores 0 or 1. Minimum thematic score: 2 of 4 to advance to the shortlist.

### Stage 3 — Composite ranking

Compute a composite score for each shortlisted company:

```
composite = (piotroski_fscore / 9 × 0.35) + (fcf_yield_normalised × 0.30) + (roic_spread_normalised × 0.20) + (thematic_score / 4 × 0.15)
```

Rank all shortlisted companies by composite score. Top 5-10 become deep-dive candidates.

### Step 4 — One-paragraph thesis per idea

For each deep-dive candidate, write a one-paragraph thesis:
- Ticker, market cap (from `fmp_market_cap`), sector
- Key fundamental strengths (from Piotroski sub-scores)
- Valuation hook (FCF yield, ROIC spread)
- Thematic alignment
- Single-sentence investment thesis

## Output format

1. **Funnel table** — universe start count and pass count at each gate
2. **Shortlist table** — ranked by composite score with sub-scores per dimension
3. **Thesis summaries** — one paragraph per deep-dive candidate
4. **Next steps** — which candidates are recommended for full `workflow-er-initiating-coverage`

## Quality gates

- [ ] All Piotroski scores from `piotroski_fscore` — not manual calculation
- [ ] All Beneish scores from `beneish_mscore`
- [ ] WACC from `wacc_calculator`; ROIC from `fmp_key_metrics`
- [ ] Composite score formula applied consistently to all shortlisted candidates
- [ ] Shortlist limited to 5-10 candidates — not a padded list

## Related skills

- `workflow-er-initiating-coverage` — full coverage initiation for shortlisted candidates
- `workflow-er-sector-overview` — sector context that informs thematic criteria in Stage 2
- `workflow-er-thesis-tracker` — thesis framework applied once a candidate advances to coverage
