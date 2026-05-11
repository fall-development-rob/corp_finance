---
name: workflow-wm-client-report
description: |
  WHAT: Quarterly client report production — period performance with benchmark comparison, Brinson attribution, factor attribution, holdings changes, market commentary, and forward-looking action items — formatted for client distribution.
  WHEN: Invoke when producing a formal quarterly or annual client report; when a client requests a written performance summary with attribution; when generating a period-end document for distribution.
---

# Wealth Management — Client Report

## What this skill covers

Formal quarterly (or annual) written report for client distribution. Covers period returns, benchmark comparison, attribution analysis, holdings activity, market commentary, and recommended next steps. Output is 5-8 pages, client-ready.

## Workflow

### Step 1 — Performance summary

Compute returns for the reporting period:

| Period | Portfolio Return (Gross) | Portfolio Return (Net) | Benchmark Return | Alpha |
|--------|------------------------|----------------------|-----------------|-------|
| Quarter | | | | |
| YTD | | | | |
| 1-Year | | | | |
| 3-Year (ann.) | | | | |
| 5-Year (ann.) | | | | |
| Inception (ann.) | | | | |

Call `risk_adjusted_returns` for Sharpe ratio, Information Ratio, Treynor ratio for the full available history.

**Always report net-of-fee returns alongside gross returns.**

### Step 2 — Benchmark comparison

- Primary benchmark (e.g., 60/40 blended — 60% S&P 500 / 40% Bloomberg Agg)
- Secondary benchmark (peer universe, if available)
- Attribution of quarterly outperformance/underperformance (summary — detail in Step 3)
- Rolling 12-month alpha: compute last 4 quarters to show consistency of outperformance

### Step 3 — Attribution analysis

Call `brinson_attribution` for the quarter:
- **Allocation effect**: over/underweight by sector in basis points
- **Selection effect**: stock-picking alpha by sector in basis points
- **Interaction effect**: combined impact in basis points
- Total active return reconciliation: allocation + selection + interaction = portfolio return − benchmark return

Call `factor_attribution` for factor-based decomposition:
- Market (beta), size (small vs large), value, momentum, quality, low-volatility
- Factor contribution in basis points; residual alpha

Attribution table:

| Sector | Portfolio Weight | Benchmark Weight | Portfolio Return | Benchmark Return | Allocation Effect | Selection Effect | Total |
|--------|-----------------|-----------------|-----------------|-----------------|-------------------|-----------------|-------|

### Step 4 — Holdings changes

Document all portfolio activity during the reporting period:

**Purchases:**
| Security | Date | Amount ($) | Rationale |
|----------|------|-----------|-----------|

**Sales:**
| Security | Date | Proceeds ($) | Realised Gain/(Loss) | Rationale |
|----------|------|-------------|---------------------|-----------|

**Income received:** dividends, interest, capital gain distributions

### Step 5 — Market commentary

Contextualise performance with key market events:
- 2-3 sentences on the market environment during the period (rates, equities, credit spreads)
- 2-3 sentences on how portfolio positioning responded to or anticipated market conditions
- Sector and style performance context (growth vs value, quality vs momentum)
- Any tactical tilts taken or reversed during the quarter

Keep this section accessible to a non-professional investor — no jargon.

### Step 6 — Next steps and recommendations

Forward-looking action items for the next quarter:

- **Rebalancing needs**: flag any asset class currently outside target range (reference `workflow-wm-portfolio-rebalancing`)
- **Tax planning**: TLH opportunities ahead of year-end; RMDs due; Roth conversion window
- **Upcoming financial events**: contract renewals, large cash needs, estate milestones
- **Market outlook implications**: any tactical positioning changes being considered

| Action | Rationale | Priority | Timeline |
|--------|-----------|----------|----------|

## Output format

Client-ready document, 5-8 pages:
1. Cover page — client name, reporting period, advisor name, date
2. Performance summary table
3. Benchmark comparison and rolling alpha
4. Attribution analysis (Brinson + factor)
5. Portfolio activity (purchases, sales, income)
6. Market commentary
7. Next steps and recommendations
8. Disclosures and notes (fee schedule, benchmark description, calculation methodology)

**Plain language in the body; technical attribution detail in an appendix if needed.**

## Quality gates

- [ ] Net-of-fee returns reported alongside gross returns for all periods
- [ ] Brinson attribution terms sum to total active return (allocation + selection + interaction = total alpha)
- [ ] Factor attribution residual documented (unexplained alpha)
- [ ] All holdings changes documented with rationale
- [ ] Market commentary written in plain language suitable for a retail investor
- [ ] Forward-looking recommendations include at least one actionable item with a timeline

## Related skills

- `workflow-wm-client-meeting-prep` — the client report is the written complement to the meeting prep pack
- `workflow-wm-portfolio-rebalancing` — triggered if Step 6 identifies allocation drift
- `workflow-wm-tax-loss-harvesting` — triggered if Step 6 surfaces year-end TLH opportunity
- `workflow-xlsx-tabular-conventions` — table formatting standard for all financial tables in the report
