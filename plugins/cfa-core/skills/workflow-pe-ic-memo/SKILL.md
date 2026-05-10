---
name: workflow-pe-ic-memo
description: |
  WHAT: Investment committee memo production — 9-section institutional document covering executive summary, company overview, industry, financial analysis, investment thesis, deal terms and structure, returns analysis, risk factors, and recommendation.
  WHEN: Invoke when preparing an IC memo for a formal investment committee presentation; when a deal has completed DD and requires a formal vote recommendation; when producing a structured deal recommendation document.
---

# PE Investment Committee Memo

## What this skill covers

Production of a complete, internally consistent IC memo: 9 sections, 10-15 pages, covering the full investment case from company overview through to a clear proceed/pass recommendation. All financial tables must reconcile across sections.

## Workflow

### Section I — Executive Summary (1 page)

- Company description: what the business does, size, market position
- Deal rationale: why this investment, why now
- Key terms: enterprise value, equity cheque, leverage, consideration form
- Recommendation: Proceed / Pass / Conditional Proceed
- Headline returns: base case IRR and MOIC
- Top 3 risks with one-sentence mitigant each

### Section II — Company Overview (1-2 pages)

- Products/services, revenue model, customer base
- Competitive positioning: market share, differentiation, barriers to entry
- Management team: track record, incentive alignment, key-person dependencies
- Corporate structure: legal entities, minority interests, JVs

### Section III — Industry and Market (1 page)

- Market size and growth (TAM/SAM with source)
- Competitive landscape: key players, market share, consolidation trends
- Secular tailwinds and headwinds
- Regulatory environment and risks

### Section IV — Financial Analysis (2-3 pages)

Pull historical data:
- Call `fmp_income_statement` with period "annual" and limit 5
- Call `fmp_balance_sheet` with period "annual" and limit 5
- Call `fmp_cash_flow` with period "annual" and limit 5
- Call `fmp_key_metrics` for margin and efficiency ratios

Produce:
- Revenue and EBITDA bridge (5-year trend, organic vs acquisition)
- Quality of earnings: normalised EBITDA vs reported EBITDA, each add-back labelled
- Working capital analysis: DSO, DIO, DPO trends and seasonality
- Capex: maintenance vs growth, capex intensity (capex/revenue)
- FCF conversion: FCF/EBITDA (target >60%)
- Call `credit_metrics` for leverage and coverage profile

### Section V — Investment Thesis (1 page)

Three to five thesis pillars, each with supporting evidence and quantified EBITDA impact:
1. Revenue growth levers (organic + inorganic)
2. Margin expansion opportunity (cost structure, operating leverage)
3. Market consolidation / buy-and-build platform potential
4. Multiple expansion catalysts (sector re-rating, ESG, profitability)
5. Defensive characteristics (recurring revenue, contractual base)

100-day priorities: 3-5 immediate post-close actions.

### Section VI — Deal Terms and Structure (1 page)

- Enterprise value and implied multiples (EV/EBITDA, EV/Revenue, P/E)
- Call `sources_uses` for financing table:
  - Sources: equity, senior secured term loan, second lien, mezzanine, revolver, rollover equity, seller note
  - Uses: equity purchase price, debt refinancing, transaction fees, cash to balance sheet
  - Sources must equal Uses exactly
- Capital structure: leverage by tranche, blended cost of debt, equity contribution %
- Call `debt_schedule` for amortisation profile and cash sweep mechanics
- Key legal terms: reps/warranties, indemnities, MAC clause, non-compete

### Section VII — Returns Analysis (1 page)

- Call `lbo_model` with entry EV, EBITDA, debt tranches, growth assumptions, exit parameters
- Three scenarios (base / upside / downside) with IRR and MOIC each
- Return attribution: EBITDA growth + multiple expansion + debt paydown
- Call `sensitivity_matrix` varying exit multiple vs EBITDA at exit
- Breakeven: minimum EBITDA at exit for 1.0x MOIC

### Section VIII — Risk Factors (1 page)

| Risk | Severity | Likelihood | Mitigant |
|------|----------|-----------|----------|

Categories: market, operational, financial, legal/regulatory, management. Include deal-breaker conditions under which the fund should pass.

### Section IX — Recommendation

- Clear verdict: Proceed / Pass / Conditional Proceed
- If conditional: specify conditions with owners and deadlines
- Next steps: remaining workstreams, timeline, approvals required

## Internal consistency checks

- EBITDA in Section I = Section IV = Section VII (exact match)
- Sources = Uses in Section VI (to the penny)
- Returns in Section I are consistent with Section VII model output
- Every risk in Section VIII has a mitigant
- Bull and bear cases both presented with equal rigour

## Output format

Structured markdown document, 10-15 pages. One H2 per section. Financial tables use the `workflow-xlsx-tabular-conventions` standard (header row, units row, source column, negatives in parentheses).

## Quality gates

- [ ] EBITDA consistent across Sections I, IV, VII
- [ ] Sources = Uses in Section VI
- [ ] All financial data traced to `fmp_*` calls or stated assumptions
- [ ] Minimum 3 scenarios (base/upside/downside) in returns analysis
- [ ] Every risk has a mitigant; at least one deal-breaker condition stated
- [ ] Recommendation is explicit: Proceed / Pass / Conditional

## Related skills

- `workflow-pe-returns-analysis` — detailed returns modelling used in Section VII
- `workflow-pe-due-diligence` — upstream DD that populates Sections II-IV
- `workflow-pe-value-creation-plan` — VCP initiatives that feed Section V thesis pillars
- `workflow-xlsx-tabular-conventions` — table formatting for financial schedules
