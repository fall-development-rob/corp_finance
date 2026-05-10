---
name: workflow-pe-due-diligence
description: |
  WHAT: Due diligence execution — comprehensive DD checklist by function (commercial, financial, legal, operational, management) and structured management meeting preparation with question lists by C-suite function.
  WHEN: Invoke when a deal has passed screening and entered active DD; when preparing for management meetings, site visits, or advisor calls; when tracking DD workstream status.
---

# PE Due Diligence

## What this skill covers

Two linked workflows: (1) a comprehensive DD checklist to track open items by function and priority, and (2) management meeting preparation with agenda and question sets tailored to each C-suite function.

## DD Checklist Workflow

### Checklist Structure

Each item carries:
- **Priority**: critical / important / nice-to-have
- **Status**: pending / in-progress / complete / N-A
- **Owner**: deal team member responsible
- **Due date**: aligned with exclusivity timeline

### Functional Categories

**Commercial**
- Market size and growth — TAM/SAM with source (critical)
- Top-10 customer interviews and concentration analysis (critical)
- Competitive positioning, pricing power, switching costs (critical)
- Sales pipeline quality and conversion rates (important)
- Contract terms: duration, renewal rates, change-of-control provisions (important)

**Financial**
- Quality of earnings — EBITDA add-backs with auditor support (critical)
- Working capital: seasonality, NWC target, intra-year swings (critical)
- Maintenance vs growth capex split by asset category (critical)
- Tax exposure: NOLs, deferred tax assets, transfer pricing, open years (important)
- Contingent liabilities: litigation, environmental, product warranty (important)
- AR ageing: DSO trend, bad debt reserve adequacy (important)

**Legal**
- Corporate structure and entity org chart (critical)
- IP portfolio: ownership, registration, freedom-to-operate (critical)
- Material contracts with change-of-control triggers (critical)
- Litigation history and current disputes (important)
- Regulatory licences and permits (important)
- Employment and non-compete agreements (important)

**Operational**
- Technology stack and cybersecurity posture (important)
- Supply chain concentration: top-5 suppliers as % of COGS (important)
- Facilities: ownership vs lease, capacity utilisation, deferred maintenance (nice-to-have)
- HR: attrition rate, key-person dependencies, succession plans (important)

**Management**
- Background checks and reference calls (critical)
- Track record: prior roles, successes, failures (critical)
- Equity rollover commitment and incentive alignment (important)
- Succession planning for CEO, CFO, COO (important)

## DD Meeting Prep Workflow

### Step 1 — Review screening findings

Re-read the screening memo. Identify red flags and unresolved questions that must be addressed in management meetings.

### Step 2 — Gather financial context

Call `fmp_income_statement` with period "annual" and limit 5 for 5-year trend.
Call `fmp_key_metrics` for margin and efficiency benchmarks.
Flag material anomalies (margin swings, revenue step-changes) to probe.

### Step 3 — Build agenda by function

Each session: 60-90 minutes. Structure: intro context-setting → open-ended exploration → specific data probes.

| Function | Focus areas |
|----------|-------------|
| CEO | Market positioning, growth strategy (organic vs M&A), customer strategy, top risks |
| CFO | Revenue recognition, EBITDA adjustments and add-backs, working capital management, capex split, tax/NOLs |
| COO | Capacity utilisation, supply chain risks, technology systems, operational KPIs |
| CTO | Tech stack, technical debt, cybersecurity posture, product roadmap |
| CSO/Head of Sales | Pipeline quality, win/loss rates, customer concentration, pricing strategy |

### Step 4 — Question sets (5-10 per function)

Lead with open-ended questions; follow with specific data requests.
Include "red flag probes" derived from screening memo anomalies.

Example probes for CFO:
- "Walk us through the $X add-back on the QoE — what supporting documentation is available?"
- "How does the working capital requirement vary across your seasonal peak? What is the peak-to-trough NWC swing?"
- "What is the split between maintenance and growth capex in the most recent three years?"

### Step 5 — Data requests

Compile a single data request list:
- Monthly financials (24-36 months, P&L and balance sheet)
- Top-20 customer detail: revenue, tenure, contract length, renewal status
- Employee roster by function and cost centre
- Capex project breakdown (maintenance/growth/IT) for last 3 years
- Legal entity org chart and ownership structure

## Output format

1. **DD checklist** — functional table with priority, status, owner, due date
2. **Meeting prep pack** — one section per function: agenda, open-ended questions, red-flag probes, data requests
3. **Open items tracker** — summary of critical items pending with escalation owners

## Quality gates

- [ ] All critical DD items have an owner and a due date within exclusivity window
- [ ] Financial anomalies from screening memo appear as specific meeting probes
- [ ] Data requests list is consolidated (no duplicates across functions)
- [ ] Red-flag probes are specific (not generic) — each references a number from the financial analysis

## Related skills

- `workflow-pe-deal-sourcing` — upstream sourcing and screening
- `workflow-pe-ic-memo` — downstream IC memo production after DD completion
- `workflow-pe-returns-analysis` — preliminary returns model to cross-check deal economics
