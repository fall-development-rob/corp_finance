---
name: workflow-wm-financial-planning
description: |
  WHAT: Comprehensive financial plan — client profile, cash flow analysis, retirement projection, goal-specific analysis (education, estate, major purchases), Monte Carlo probability of success, insurance review, tax optimisation, and priority-ranked recommendations.
  WHEN: Invoke when producing a full financial plan for a new or existing client; when a client asks for a retirement analysis; when reviewing whether a client is on track for stated goals; when a life event (marriage, inheritance, retirement) requires a plan update.
---

# Wealth Management — Financial Planning

## What this skill covers

End-to-end financial plan production: profile the client, analyse cash flows and goals, project retirement outcomes, run Monte Carlo simulations, and produce a priority-ranked action plan. Output is a 15-25 page comprehensive financial plan.

## Workflow

### Step 1 — Client profile

Gather and document:
- **Demographics**: age, marital status, dependents, target retirement age
- **Income**: salary, bonus, investment income, pension, Social Security estimate
- **Assets**: taxable accounts, 401(k)/403(b), IRA/Roth IRA, real estate, business interests, other
- **Liabilities**: mortgage, student loans, other debt; document interest rates and terms
- **Risk tolerance**: separate capacity (financial ability) from willingness (emotional comfort); rate conservative / moderate / aggressive
- **Goals**: retirement (age, spending level), education funding (children's ages, target schools), home purchase, legacy/estate

### Step 2 — Cash flow analysis

- Annual gross income vs total expenses
- **Savings rate**: current savings ÷ gross income (target >15% for retirement readiness)
- **Emergency fund**: current liquid assets ÷ monthly expenses (target 3-6 months)
- **Debt service ratio**: total annual debt payments ÷ gross income (flag if >36%)
- Identify discretionary spending that can be reallocated to savings if needed

### Step 3 — Retirement projection

Call `retirement_planning` with:
- Current total retirement savings
- Annual contribution (employee + employer)
- Expected annual return (real rate, net of inflation)
- Inflation assumption (typically 2.5-3.0%)
- Target retirement age and spending in today's dollars

Gap analysis: projected assets at retirement vs required assets (spending × safe withdrawal rate multiple, typically 25-33x annual spending).

Integrate Social Security / pension income where applicable.

### Step 4 — Goal-specific analysis

**Education funding**:
- Current 529 balance and annual contributions
- Target college cost in today's dollars, escalated at 5-6% annual tuition inflation
- Projected gap at matriculation

**Estate planning**:
- Call `estate_planning` for transfer tax analysis
- Applicable exclusions (federal and state)
- Recommended structures (trusts, gifting strategy, charitable vehicles)

**Major purchases (home upgrade, vacation property)**:
- Savings timeline and funding source (taxable account, home equity, liquidation)
- Impact on cash flow and investment portfolio

### Step 5 — Monte Carlo simulation

Call `monte_carlo_simulation` with:
- Asset allocation, expected return, return volatility, inflation
- Annual spending in retirement, length of retirement (mortality assumption)
- At least 1,000 simulations

Report:
- Probability of meeting each goal independently
- Probability of meeting all goals jointly
- Median outcome and 10th / 90th percentile range
- Stress scenario: if returns are 2pp below expected, new probability

### Step 6 — Insurance and risk management review

- **Life insurance**: 10-12x income rule-of-thumb vs needs-based analysis
- **Disability**: 60-70% income replacement target
- **Long-term care**: assess need based on age (onset typically 60+), family history, asset level

Flag any coverage gaps with recommended action.

### Step 7 — Tax optimisation

- **Asset location**: bonds and TIPS in tax-deferred accounts; growth equities in taxable; Roth for highest-growth assets
- **Roth conversion ladder**: optimal annual conversion amounts in low-income years (retirement gap before Social Security, early retirement)
- **Tax-loss harvesting**: identify current opportunities (reference `workflow-wm-tax-loss-harvesting`)
- **RMD planning**: first RMD age, annual RMD schedule, impact on Medicare premiums (IRMAA)

### Step 8 — Priority-ranked recommendations

Prioritise by urgency and impact:

| Priority | Action | Rationale | Impact | Timeline |
|----------|--------|-----------|--------|----------|
| 1 | | | | Immediate (0-30 days) |
| 2 | | | | Near-term (1-6 months) |
| 3 | | | | Long-term (6-12+ months) |

Each recommendation: rationale, expected quantitative impact, implementation steps.

## Output format

Structured document, 15-25 pages:
1. Client summary and goals (1-2 pages)
2. Current financial snapshot — assets, liabilities, cash flow (2-3 pages)
3. Retirement projection with gap analysis (2-3 pages)
4. Goal-specific analysis (2-4 pages)
5. Monte Carlo results — probability of success (1-2 pages)
6. Insurance review (1 page)
7. Tax optimisation opportunities (1-2 pages)
8. Recommendations — priority-ranked with timeline (2-3 pages)

## Quality gates

- [ ] Client risk capacity and willingness separately assessed and documented
- [ ] Retirement projection uses real (inflation-adjusted) return; gap analysis included
- [ ] Monte Carlo uses ≥1,000 simulations; reports median and 10th/90th percentiles
- [ ] Estate planning tool called if estate >$1M or client age >55
- [ ] Asset location recommendation covers all account types
- [ ] All recommendations ranked by urgency with explicit implementation steps

## Related skills

- `workflow-wm-portfolio-rebalancing` — implement allocation changes from Step 7
- `workflow-wm-tax-loss-harvesting` — capture losses identified in Step 7
- `workflow-wm-client-report` — ongoing performance reporting once plan is in place
- `workflow-wm-client-meeting-prep` — meeting prep that references the financial plan progress
