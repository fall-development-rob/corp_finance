---
name: workflow-pe-ai-readiness
description: |
  WHAT: Cross-portfolio AI opportunity diagnostic — three-gate evaluation (data, organisational, ROI readiness), four-dimension opportunity scoring, EBITDA-impact ranking table, quarterly cross-portfolio synthesis, and deep-dive memos for the top-ranked opportunities.
  WHEN: Invoke when the operating partner wants to identify AI value-creation opportunities across the portfolio; when conducting a quarterly AI readiness review; when evaluating a specific AI use case at a single portco; when building the AI component of a value creation plan.
---

# PE AI Readiness Assessment

## What this skill covers

A structured diagnostic that identifies, gates, and ranks AI opportunities across PE portfolio companies by annualised EBITDA impact. Runs as a single-company assessment or a full cross-portfolio diagnostic on a quarterly cadence.

**Core principle**: rank by dollars, not excitement. A back-office automation saving $400k beats a flashy customer chatbot. Hold period and data quality together determine urgency.

## Three-Gate Evaluation

Apply all three gates before scoring. A single "No" stalls the opportunity to "Wait" status.

| Gate | Question | Pass Condition |
|------|----------|----------------|
| G1 — Data Readiness | Can clean, structured inputs be sourced without a multi-month data-engineering programme? | Usable data exists today or within 30 days at negligible cost |
| G2 — Organisational Readiness | Is there a named manager with authority, budget, and motivation to drive this to completion? | Owner identified; role confirmed; incentive aligned to outcome |
| G3 — ROI Readiness | Does a defensible EBITDA case clear the hurdle for hold period and resourcing? | Payback inside hold period; impact >=$100k annualised |

G2 is the binding constraint in the majority of portfolio situations — a quick win with no internal owner dies in 90 days.

## Opportunity Scoring Rubric

Score 1-5 on four dimensions. Multiply scores to produce raw score (max 625).

| Dimension | 1 (Low) | 3 (Medium) | 5 (High) |
|-----------|---------|-----------|---------|
| Annualised EBITDA Impact | <$100k | $100k–$500k | >$500k |
| Time-to-Value | >6 months | 2–6 months | <60 days |
| Hold-Period Fit | Payback ≥ remaining hold | Payback < remaining hold | Payback < 25% of remaining hold |
| Data-Quality Dependency | Significant cleansing required | Partial — some work needed | Clean and accessible today |

Use Ownership Strength (1-5) as a tiebreaker, not a multiplier.

## EBITDA Impact Quantification

To compute the EBITDA impact estimate:
- Call `variance_analysis` to decompose the cost or revenue line the use case targets
- Call `peer_benchmarking` to anchor achievable margin level (gap to top-quartile = upper bound on impact)
- Call `dupont_analysis` to identify whether the lever flows through margin, asset turnover, or leverage
- Call `working_capital` for back-office automation use cases (impact on DSO/DIO/DPO)
- Call `sensitivity_matrix` to stress-test impact across optimistic/base/conservative adoption assumptions

## Common Opportunity Archetypes

| Archetype | Typical EBITDA Impact | Data Prerequisites |
|-----------|----------------------|--------------------|
| Sales productivity (proposal drafting, CRM, lead scoring) | $200k–$1.5m | CRM with ≥12 months clean activity data |
| Customer support automation (ticket deflection, agent assist) | $300k–$2m | Ticket history ≥90 days; resolution outcomes |
| Finance / back-office automation (AP, GL, contract abstraction) | $150k–$800k | Invoice scans; chart of accounts; AP system export |
| Predictive maintenance (industrial/fleet assets) | $400k–$3m | Telemetry ≥6 months; maintenance log; asset register |
| Fraud / anomaly detection | $200k–$1.5m | Transaction ledger ≥18 months; known-fraud labels |
| Demand forecasting (consumer/retail) | $250k–$1.5m | POS history ≥24 months; promotional calendar |
| Code generation (software portcos) | 10-25% engineering productivity uplift | Repo access; CI/CD telemetry |

## Cross-Portfolio Diagnostic Cadence

**Quarter-long cadence (5-20 companies)**:
- Week 1: scope, intake, confirm company list, pull quarterly financials, request operating-team use-case memos
- Weeks 2-6: per-company gating and scoring (half-day per company)
- Weeks 7-8: portfolio synthesis, rank GO opportunities, identify replay candidates
- Week 9: top-3 deep-dive memos (2-3 pages each)
- Weeks 10-11: challenge session, revise scores
- Week 12: deliverable lock, WAIT-item watchlist, next-quarter rollover

**Replay identification**: two or more companies in the same sector where the same tooling can be deployed with configuration-only changes. Document combined EBITDA impact and shared implementation cost saving.

## Output format

**Ranked portfolio table** (one row per GO opportunity):

| Company | Use Case | Gate Status | EBITDA Impact (ann.) | Speed | Score | Priority | Owner |
|---------|----------|-------------|----------------------|-------|-------|----------|-------|

Gate status: **GO** (all three gates pass), **WAIT** (one or more gates fail — record blocker), **PASS** (declined — record reason).

**Top-3 deep-dive memos** (2-3 pages each): business context, scoped pilot, data prerequisites, owner and resourcing, 30/60/90-day plan, sensitivity grid output, success metric.

**Portfolio roll-up**: total addressable EBITDA impact if all GO opportunities execute; apply 60-70% capture probability to GO items; 0% to WAIT.

## Quality gates

- [ ] Three-gate evaluation documented for every assessed opportunity
- [ ] Scores computed from four-dimension rubric (not qualitative judgement alone)
- [ ] EBITDA impact anchored to `peer_benchmarking` and `variance_analysis` outputs
- [ ] Replay candidates identified where applicable
- [ ] GO items with EBITDA impact >$250k reflected as VCP line items
- [ ] Failure modes checked before finalising the ranked table

## Common failure modes to check

- Data-quality blockers presented as solvable (force specific owner/date/budget)
- Change-management debt (recent ERP, leadership turnover) — haircut capture probability
- Vendor lock-in without exit clause — score lock-in as downside scenario
- Pilot-to-production gap — apply 60-70% capture probability, not 100%
- EBITDA double-counting with active VCP initiatives — reconcile before submission

## Related skills

- `workflow-pe-value-creation-plan` — AI opportunities scoring GO with EBITDA >$250k become VCP line items
- `workflow-pe-portfolio-monitoring` — tracks GO opportunity delivery quarter-by-quarter
- `workflow-pe-ic-memo` — AI readiness assessment cited as a VCP lever in Section V (Investment Thesis)
