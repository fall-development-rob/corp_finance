---
name: workflow-pe-portfolio-monitoring
description: |
  WHAT: Ongoing portfolio company monitoring — monthly KPI tracking, budget-vs-actual variance analysis, covenant compliance monitoring with early-warning flags, VCP initiative progress tracking, and quarterly board report production.
  WHEN: Invoke when producing a monthly portfolio update, quarterly board report, or covenant compliance check; when a portco's EBITDA misses plan by >5%; when covenant headroom falls below 15%.
---

# PE Portfolio Monitoring

## What this skill covers

Structured monitoring of active portfolio companies across three time horizons: monthly KPI dashboard, quarterly board report, and ongoing covenant compliance early-warning. Escalates automatically when thresholds are breached.

## Workflow

### Step 1 — Monthly KPI dashboard

For each portfolio company, track:

| Metric | Actual | Budget | Prior Year | Variance (Budget) | Variance (PY) |
|--------|--------|--------|------------|-------------------|---------------|
| Revenue ($M) | | | | | |
| EBITDA ($M) | | | | | |
| EBITDA margin (%) | | | | | |
| Free cash flow ($M) | | | | | |
| Capex ($M) | | | | | |
| Headcount | | | | | |

Flag any metric with >5% adverse variance vs budget. Investigate and document root cause.

### Step 2 — Budget vs actual variance analysis

For flagged variances (>5% adverse):
- **Revenue variance**: decompose into volume, pricing, and mix components
- **EBITDA variance**: split between revenue flow-through, cost overage, and one-time items
- Document whether variance is timing (catch-up expected) or structural (plan requires revision)
- Escalation: if EBITDA variance persists >2 consecutive months → trigger portfolio review meeting

### Step 3 — Covenant compliance

Call `credit_metrics` on the latest 12-month trailing financial data.
Call `covenant_compliance` with covenant thresholds from the credit agreement.

Key metrics to track:
- Net Leverage (Net Debt / EBITDA): covenant threshold and current headroom
- Interest Coverage (EBITDA / Interest Expense): threshold and headroom
- Fixed Charge Coverage (EBITDA - Capex) / (Interest + Scheduled Debt Repayment): threshold and headroom

Escalation triggers:
- **<15% headroom**: early-warning flag — notify GP operating partner immediately
- **<10% headroom**: covenant cure preparation required — engage lender relationship
- **<5% headroom**: potential breach — legal and restructuring counsel on standby

### Step 4 — VCP initiative progress

For each active VCP initiative:

| Initiative | Status | EBITDA Impact Target | EBITDA Captured YTD | Owner | Next Milestone |
|-----------|--------|---------------------|--------------------|----|----------------|
| | On-track / At-risk / Delayed | | | | |

Flag "At-risk" or "Delayed" items. Document blocker and recovery plan.
Assess total VCP delivery as % of target: highlight if <80% of YTD target.

### Step 5 — Quarterly board report

Structure (6 sections):

**1. Executive Summary**
- Financial performance vs plan: headline revenue and EBITDA
- Key achievements and disappointments in the quarter
- Top 3 risks for the next quarter

**2. Financial Performance**
- Revenue and EBITDA vs budget and prior year, with variance bridge
- FCF generation and cash position
- Working capital movements and DSO/DIO/DPO trends

**3. Operational KPIs**
- Sector-specific operational metrics (customer NPS, utilisation, backlog, etc.)
- Headcount and productivity metrics

**4. Strategic Update**
- M&A bolt-on pipeline: targets in discussion, NDAs signed, exclusives
- VCP initiative progress by category (revenue / cost / AI)
- Key commercial wins and losses in the quarter

**5. Risk Register**
- Top 5 risks: severity, likelihood, mitigant, owner
- Covenant headroom by facility (flagging if <15%)

**6. Forward Look**
- Next-quarter guidance: revenue and EBITDA vs budget
- Upcoming milestones: contract renewals, capex decisions, regulatory events
- Recommended IC actions: additional equity, dividend recap, refinancing, exit prep

## Output format

1. **Monthly KPI table** — per-company dashboard with variance flags
2. **Variance commentary** — bullet points for flagged items
3. **Covenant compliance table** — per-facility with headroom % and RAG status
4. **VCP tracker** — per-initiative status with EBITDA captured vs target
5. **Quarterly board report** — 6-section narrative, 5-8 pages

## Quality gates

- [ ] All KPIs populated with actuals and budget for every portfolio company
- [ ] Adverse variances >5% have root-cause commentary
- [ ] `credit_metrics` and `covenant_compliance` run on LTM financials each quarter
- [ ] Covenant headroom <15% triggers early-warning escalation immediately
- [ ] VCP initiative status current (not carried over from prior quarter without verification)
- [ ] Board report recommendation section contains actionable IC decisions

## Related skills

- `workflow-pe-value-creation-plan` — VCP initiatives tracked in Step 4
- `workflow-pe-returns-analysis` — updated returns at exit vs entry model
- `workflow-pe-ic-memo` — reference for original investment thesis and assumptions to compare actuals against
