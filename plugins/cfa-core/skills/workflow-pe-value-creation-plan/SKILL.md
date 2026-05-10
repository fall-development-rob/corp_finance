---
name: workflow-pe-value-creation-plan
description: |
  WHAT: Value creation plan (VCP) for PE portfolio companies — quantified revenue and cost levers, EBITDA bridge from Year 0 to exit, 100-day plan, KPI dashboard with milestones, and impact on fund returns.
  WHEN: Invoke when building a post-acquisition value creation plan, when the IC needs a detailed 100-day plan, when quantifying the EBITDA uplift from operational initiatives, or when updating the VCP at a quarterly portfolio review.
---

# PE Value Creation Plan

## What this skill covers

A structured plan for generating returns beyond financial engineering. Covers revenue growth levers, cost levers, an EBITDA bridge from Year 0 to exit, a 100-day quick-win plan, a KPI dashboard, and the impact on LBO returns. Every initiative is quantified — unquantified initiatives are deferred to the watchlist.

## Core principle

Returns-focused. Every initiative connects back to its EBITDA impact and the resulting IRR/MOIC uplift. If an initiative cannot be quantified, it is categorised as "exploratory" and excluded from the returns model until a defensible number is available.

## Workflow

### Step 1 — Revenue levers

For each lever, specify: initiative, responsible owner, estimated EBITDA impact ($M), implementation timeline, and confidence level (high/medium/low).

| Lever | Description | Est. EBITDA Impact |
|-------|-------------|-------------------|
| Pricing optimisation | % price increase, assumed volume impact, net revenue | computed |
| Cross-sell / upsell | Attach rate improvement × revenue per customer | computed |
| New market entry | Geographic or vertical expansion, addressable TAM | computed |
| M&A bolt-ons | Target profiles, expected multiples, synergies | computed |

### Step 2 — Cost levers

| Lever | Description | Est. EBITDA Impact |
|-------|-------------|-------------------|
| Procurement savings | Renegotiation, consolidation, volume discounts | computed |
| Operational efficiency | Headcount, process improvement, automation | computed |
| SG&A rationalisation | Real estate, T&E, professional fees | computed |
| Outsourcing / shared services | Function, expected saving, timeline | computed |

### Step 3 — EBITDA bridge

Year 0 to Year 5 (or exit year), initiative by initiative:

| Item | FY0 | FY1 | FY2 | FY3 | FY4 | FY5 |
|------|-----|-----|-----|-----|-----|-----|
| Starting EBITDA | actual | | | | | |
| + Organic revenue growth | | | | | | |
| + Pricing improvement | | | | | | |
| + Cost savings | | | | | | |
| + M&A contribution | | | | | | |
| − Integration costs | | | | | | |
| = Target EBITDA | | | | | | |

Each row is a separate VCP initiative, allowing attribution of total EBITDA uplift.

### Step 4 — 100-day plan

**Quick wins** (deliverable within 100 days with measurable impact):
- Specific action, owner, success metric, target date

**Organisational changes** (key hires, reporting structure, governance):
- Role, rationale, recruitment timeline, interim coverage

**Strategic priorities** (DD follow-up items, early M&A outreach, system improvements):
- Action, owner, timeline, dependencies

**Communication plan** (employees, customers, suppliers, regulators):
- Audience, key message, medium, timing

### Step 5 — KPI dashboard with milestones

**Monthly KPIs**:
- Revenue: actual vs budget vs prior year
- EBITDA: actual vs budget, margin trend
- Free cash flow: generation, WC movements
- Capex: actual vs budget, maintenance vs growth
- Headcount: by function, new hires, attrition

**Quarterly milestones**:
- Initiative progress: on-track / at-risk / delayed
- Budget vs actual variance with bridge
- Covenant compliance headroom

**Annual targets**:
- EBITDA target vs plan, leverage reduction, VCP delivery %

### Step 6 — Model impact on returns

Call `lbo_model` with VCP assumptions (the "VCP case"):
- EBITDA growing per the VCP bridge
- Compare base case (no VCP initiatives) vs VCP case
- Quantify IRR and MOIC uplift from each initiative category
- Call `sensitivity_matrix` varying VCP delivery % (100% / 75% / 50%) vs exit multiple

## Output format

1. **Revenue lever table** — with EBITDA impact and owner
2. **Cost lever table** — with EBITDA impact and timeline
3. **EBITDA bridge** — year-by-year, initiative-by-initiative
4. **100-day plan** — quick wins, org changes, priorities, comms
5. **KPI dashboard** — monthly/quarterly/annual metrics with targets
6. **Returns impact** — VCP case vs base case IRR and MOIC; sensitivity grid

## Quality gates

- [ ] Every initiative has a quantified EBITDA impact (no unquantified items in the bridge)
- [ ] EBITDA bridge ties from Year 0 entry EBITDA to Year 5 target EBITDA
- [ ] 100-day plan items have owners and dates; quick wins are genuinely achievable in 100 days
- [ ] KPI dashboard metrics are measurable from existing reporting systems
- [ ] VCP case modelled in LBO and compared to base case; uplift quantified
- [ ] VCP EBITDA bridge consistent with IC memo Section V thesis pillars

## Related skills

- `workflow-pe-ic-memo` — Section V (Investment Thesis) draws on VCP revenue and cost levers
- `workflow-pe-returns-analysis` — VCP case feeds the upside scenario in the returns model
- `workflow-pe-portfolio-monitoring` — tracks VCP initiative delivery quarter-by-quarter
- `workflow-pe-ai-readiness` — AI opportunities scored and ranked, top items promoted to VCP
