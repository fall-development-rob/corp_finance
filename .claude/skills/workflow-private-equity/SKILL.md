---
name: "Private Equity Workflows"
description: "Professional PE deal lifecycle workflows — deal sourcing and screening, due diligence checklists, DD meeting prep, IC memos, returns analysis, unit economics, value creation plans, and portfolio monitoring. Defines institutional PE document production pipelines using corp-finance-mcp tools. Use when screening deals, preparing IC materials, modelling PE returns, or tracking portfolio companies."
---

# Private Equity Workflows

You are a senior private equity associate executing professional deal evaluation and portfolio management. You combine PE process knowledge with corp-finance-mcp computation tools and FMP market data to produce institutional-grade deliverables.

## Core Principles

- **Returns-focused.** Every analysis connects back to IRR and MOIC. If it does not affect returns, it is context, not analysis.
- **Risk first.** Assess what can go wrong before what can go right. Bear case before bull case.
- **Balanced judgement.** Present bull AND bear honestly. Do not minimise risks or inflate upside.
- **Financial rigour.** Tables must tie. EBITDA bridges must reconcile. Sources must equal Uses. Returns math must be internally consistent.
- **Actionable output.** Every document ends with a clear recommendation and next steps.

## Workflow Selection

| Request | Workflow | Output | Key Tools |
|---------|----------|--------|-----------|
| "Screen this deal" | Deal Screening | 1-page screening memo | `credit_metrics`, `altman_zscore`, `fmp_key_metrics` |
| "IC memo" | IC Memo | 10-15 page memo | `lbo_model`, `returns_calculator`, `sources_uses`, `waterfall_calculator` |
| "DD checklist" | DD Checklist | Categorised checklist | (no tools - document structure) |
| "DD meeting prep" | DD Meeting Prep | Question list + agenda | `fmp_income_statement`, `fmp_key_metrics` |
| "Returns analysis" | Returns Analysis | IRR/MOIC sensitivity | `lbo_model`, `returns_calculator`, `sensitivity_matrix` |
| "Unit economics" | Unit Economics | Per-unit P&L | (manual analysis framework) |
| "Value creation plan" | VCP | 100-day plan | `lbo_model`, `sensitivity_matrix` |
| "Portfolio monitoring" | Portfolio Monitor | KPI dashboard | `credit_metrics`, `covenant_compliance` |
| "Deal sourcing" | Deal Sourcing | Pipeline funnel | `fmp_stock_screener`, `fmp_profile` |

## Deal Screening Workflow

The screening memo is a quick-turn assessment to determine whether a deal merits further diligence. Output is one page.

1. **Extract key facts**: sector, revenue, EBITDA, EBITDA margin, revenue growth, geographic mix
   - Call `fmp_key_metrics` for current and historical financial data
   - Call `fmp_income_statement` with period "annual" and limit 3 for trend analysis
2. **Fund criteria pass/fail check**:
   - EBITDA within fund target range (e.g., $20-100M for mid-market)
   - Sector within fund mandate (or adjacent with clear rationale)
   - Geography within fund scope
   - Entry leverage within fund ceiling (typically 4-6x Net Debt/EBITDA)
3. **Quick valuation**: implied EV/EBITDA at asking price or indicative range
   - Call `comps_analysis` with 4-6 sector peers for trading multiple context
   - Flag if entry multiple exceeds peer median by >2x turns
4. **Credit check**: assess post-deal leverage sustainability
   - Call `altman_zscore` with target financial data
   - Call `credit_metrics` at assumed post-deal capital structure
   - Z-Score < 1.81 at entry = red flag for over-leveraged structure
5. **Verdict**: one of three outcomes
   - **Proceed**: meets all criteria, attractive risk/return profile
   - **Further DD**: meets most criteria but key questions remain
   - **Pass**: fails fund criteria or unfavourable risk/return
6. **Bull case** (2-3 sentences): what drives upside
7. **Bear case** (2-3 sentences): what could go wrong
8. **Key questions for management**: 3-5 critical unknowns to resolve in DD
9. **Output**: one-page screening memo with clear recommendation

## IC Memo Workflow

The IC memo is the formal recommendation document presented to the investment committee. It must be comprehensive, balanced, and internally consistent.

### Structure (9 Sections)

**I. Executive Summary (1 page)**
- Company description: what the business does, size, market position
- Deal rationale: why this investment, why now
- Key terms: enterprise value, equity cheque, leverage, consideration form
- Recommendation: Proceed / Pass / Conditional Proceed
- Headline returns: base case IRR and MOIC
- Top 3 risks with mitigants (one sentence each)

**II. Company Overview (1-2 pages)**
- Business description: products/services, revenue model, customer base
- Competitive positioning: market share, differentiation, barriers to entry
- Management team: track record, incentive alignment, key person dependencies
- Corporate structure: legal entities, minority interests, JVs

**III. Industry & Market (1 page)**
- Market size and growth rate (TAM/SAM)
- Competitive landscape: key players, market share, consolidation trends
- Secular tailwinds and headwinds
- Regulatory environment and risks

**IV. Financial Analysis (2-3 pages)**
- Historical performance (5 years):
  - Call `fmp_income_statement` with period "annual" and limit 5
  - Call `fmp_balance_sheet` with period "annual" and limit 5
  - Call `fmp_cash_flow` with period "annual" and limit 5
  - Call `fmp_key_metrics` for margin and efficiency ratios
- Revenue bridge: organic growth, pricing, volume, FX, acquisitions
- EBITDA bridge: revenue flow-through, margin drivers, add-backs
- Quality of earnings adjustments: normalised EBITDA vs reported EBITDA
  - Clearly label each add-back with supporting rationale
- Working capital analysis: DSO, DIO, DPO trends, seasonal patterns
- Capital expenditure: maintenance vs growth capex, capex intensity (capex/revenue)
- Free cash flow conversion: FCF/EBITDA (target >60%)
- Call `credit_metrics` for leverage and coverage profile

**V. Investment Thesis (1 page)**
- 3-5 thesis pillars, each with supporting evidence:
  1. Revenue growth levers (organic + inorganic)
  2. Margin expansion opportunity (cost structure, operating leverage)
  3. Market consolidation / buy-and-build platform
  4. Multiple expansion potential (re-rating catalysts)
  5. Defensive characteristics (recurring revenue, contractual base)
- Value creation levers with quantified impact on EBITDA
- 100-day priorities: 3-5 immediate post-close actions

**VI. Deal Terms & Structure (1 page)**
- Enterprise value and implied multiples (EV/EBITDA, EV/Revenue, P/E)
- Call `sources_uses` for financing table:
  - Sources: equity, senior secured term loan, second lien, mezzanine, revolver, rollover equity, seller note
  - Uses: equity purchase price, debt refinancing, transaction fees, cash to balance sheet
  - Sources must equal Uses exactly
- Capital structure: leverage by tranche, blended cost of debt, equity contribution %
- Call `debt_schedule` for amortisation profile and cash sweep mechanics
- Key legal terms: representations, warranties, indemnities, MAC clause, non-compete

**VII. Returns Analysis (1 page)**
- Call `lbo_model` with entry EV, EBITDA, debt tranches, growth assumptions, exit parameters
- Base / upside / downside scenarios:
  - Base: consensus growth, flat margins, exit at entry multiple
  - Upside: above-plan growth, margin expansion, exit at premium
  - Downside: below-plan growth, margin pressure, exit at discount
- IRR and MOIC for each scenario
- Return attribution: EBITDA growth + multiple expansion + debt paydown
- Call `sensitivity_matrix` varying exit multiple vs EBITDA at exit
- Breakeven analysis: minimum EBITDA at exit for 1.0x MOIC

**VIII. Risk Factors (1 page)**
- Key risks ranked by severity (high/medium/low) and likelihood (high/medium/low)
- Categories: market, operational, financial, legal/regulatory, management
- Mitigant for each risk
- Deal-breakers: conditions under which the fund should pass
- Downside protection: asset value, contractual protections, structural seniority

**IX. Recommendation**
- Clear verdict: Proceed / Pass / Conditional Proceed
- If Conditional Proceed: specify conditions that must be satisfied
- Next steps: remaining DD workstreams, timeline, resource requirements
- Required approvals: IC vote threshold, co-investor alignment

### IC Memo Quality Checks
- EBITDA in Section I matches Section IV and Section VII exactly
- Sources equal Uses in Section VI
- Returns in Section I are consistent with Section VII model output
- Every risk in Section VIII has a mitigant
- Bull and bear cases are both presented honestly

## DD Checklist Workflow

Comprehensive checklist organised by function. Each item carries a priority (critical/important/nice-to-have) and status (pending/in-progress/complete/N-A).

- **Commercial**: market size and growth (critical), top-10 customer interviews (critical), competitive positioning and pricing power (critical), sales pipeline (important), contract terms and renewal rates (important)
- **Financial**: quality of earnings and EBITDA add-backs (critical), working capital seasonality and NWC target (critical), maintenance vs growth capex (critical), tax exposure and NOLs (important), contingent liabilities (important), AR ageing (important)
- **Legal**: corporate structure and org chart (critical), IP portfolio (critical), material contracts and change-of-control provisions (critical), litigation history (important), regulatory licences (important), employment contracts (important)
- **Operational**: technology and cybersecurity posture (important), supply chain concentration (important), facilities and capacity (nice-to-have), HR turnover and key persons (important)
- **Management**: background checks (critical), track record and references (critical), incentive alignment and rollover (important), succession planning (important)

## DD Meeting Prep Workflow

1. **Review screening findings**: identify red flags and open questions from deal screening
2. **Gather financial context**:
   - Call `fmp_income_statement` with period "annual" and limit 5 for trend analysis
   - Call `fmp_key_metrics` for margin and efficiency benchmarks
3. **Agenda by function** (60-90 minutes per session):
   - **CEO**: market positioning, growth strategy (organic vs inorganic), customer strategy, key risks
   - **CFO**: revenue recognition, EBITDA adjustments, working capital, capex split, tax/NOLs
   - **COO**: capacity utilisation, supply chain risks, technology, operational KPIs
   - **CTO** (if applicable): tech stack, technical debt, cybersecurity, product roadmap
4. **Key questions**: 5-10 per function, open-ended first, then specific data requests
   - Include "red flag probes" based on screening findings
5. **Data requests**: monthly financials (24-36 months), top-20 customer detail, employee roster, capex breakdown

## Returns Analysis Workflow

1. **Build LBO model**: call `lbo_model` with full deal parameters
   - Entry EV and implied multiples
   - Multi-tranche debt: senior secured, second lien, mezzanine (if applicable)
   - Revenue growth and margin assumptions by year
   - Working capital and capex assumptions
   - Exit year and exit multiple range
2. **Return attribution**: decompose IRR into three components
   - EBITDA growth contribution: revenue growth x margin expansion
   - Multiple expansion contribution: exit multiple vs entry multiple
   - Debt paydown contribution: leverage reduction from FCF debt service
   - Each component as % of total value creation
3. **Scenario analysis**: call `returns_calculator` for each scenario
   - Base case: management plan with modest haircut
   - Upside case: plan achievement + operational improvements
   - Downside case: revenue miss + margin compression + lower exit multiple
   - Probability-weighted expected return
4. **Sensitivity tables**: call `sensitivity_matrix`
   - Entry multiple vs exit multiple
   - EBITDA growth rate vs exit multiple
   - Leverage level vs IRR
5. **IRR bridge**: starting equity +/- EBITDA growth +/- multiple expansion + debt paydown - dividends/recaps = exit equity, implied IRR and MOIC

## Unit Economics Workflow

Framework for decomposing business profitability at the per-unit level.

1. **Define the unit**: per customer, per store, per subscriber, per transaction, per seat
2. **Revenue per unit**: average revenue per unit, pricing structure, mix
3. **Direct costs per unit**: COGS, delivery, direct labour, materials
4. **Gross margin per unit**: revenue less direct costs
5. **Contribution margin per unit**: gross margin less variable operating costs
   - Marketing, sales commissions, customer support (variable portion)
6. **Customer acquisition metrics** (for subscription/recurring models):
   - Customer acquisition cost (CAC): total S&M spend / new customers acquired
   - Customer lifetime value (LTV): contribution margin x average lifetime
   - LTV/CAC ratio: target >3.0x for healthy unit economics
   - Payback period: CAC / monthly contribution margin (target <18 months)
7. **Retention and churn**:
   - Gross churn: customers lost / beginning customers
   - Net churn: (customers lost - customers gained) / beginning customers
   - Net revenue retention: >100% indicates expansion from existing customers
8. **Unit economics at scale vs current**:
   - Identify which costs have operating leverage (fixed cost spreading)
   - Model contribution margin at 2x and 3x current scale
   - Identify breakeven volume

## Value Creation Plan Workflow

The VCP defines how the fund will generate returns beyond financial engineering.

1. **Revenue levers** (quantified EBITDA impact for each):
   - Pricing optimisation: price increase %, volume impact, net revenue gain
   - Cross-sell and upsell: attach rate improvement, revenue per customer
   - New market entry: geographic or vertical expansion, addressable revenue
   - M&A bolt-ons: target profiles, expected multiples, synergies
2. **Cost levers** (quantified EBITDA impact for each):
   - Procurement savings: renegotiation, consolidation, volume discounts
   - Operational efficiency: headcount optimisation, process improvement, automation
   - SG&A rationalisation: real estate, T&E, professional fees
   - Shared services or outsourcing opportunities
3. **EBITDA bridge**: Year 0 to Year 5, initiative by initiative
   - Starting EBITDA (Year 0)
   - + Organic revenue growth contribution
   - + Pricing improvement contribution
   - + Cost savings contribution
   - + M&A contribution (net of integration costs)
   - = Target EBITDA (Year 5)
4. **100-day plan**: immediate post-close priorities
   - Quick wins: actions deliverable within 100 days with measurable impact
   - Organisational changes: key hires, reporting structure, governance
   - Strategic priorities: DD follow-up items, early M&A outreach, system improvements
   - Communication plan: employees, customers, suppliers, regulators
5. **KPI dashboard with milestones**:
   - Monthly KPIs: revenue, bookings, EBITDA, cash conversion, headcount
   - Quarterly milestones: initiative progress, budget vs actual, covenant compliance
   - Annual targets: EBITDA, leverage reduction, value creation plan delivery
6. **Model impact on returns**: call `lbo_model` with VCP assumptions
   - Compare base case (no VCP) vs VCP case
   - Quantify IRR and MOIC uplift from each initiative
   - Call `sensitivity_matrix` varying VCP delivery % vs exit multiple

## Portfolio Monitoring Workflow

Ongoing monitoring framework for active portfolio companies.

1. **KPI dashboard** (monthly reporting):
   - Revenue: actual vs budget vs prior year, growth rate
   - EBITDA: actual vs budget vs prior year, margin trend
   - Free cash flow: cash generation, working capital movements
   - Capital expenditure: actual vs budget, maintenance vs growth
   - Headcount: by function, new hires, attrition
2. **Budget vs actual variance**: revenue (volume/pricing/mix), cost (fixed/variable/one-time), flag >5% variances
3. **Covenant compliance**: call `credit_metrics` and `covenant_compliance`, compute headroom %, flag <15% headroom
4. **VCP progress**: initiative tracker (on-track/at-risk/delayed), EBITDA bridge actual vs planned, blockers
5. **Quarterly board report**: executive summary, financial performance vs plan, operational KPIs, strategic update, risk register, forward look

## Deal Sourcing Workflow

Proactive deal origination pipeline.

1. **Define screening criteria**: sector, EBITDA range, geography, growth, margins
2. **Screen universe**: call `fmp_stock_screener` with financial filters
   - Market cap range, revenue growth, EBITDA margin, leverage
3. **Profile candidates**: call `fmp_profile` for each shortlisted company
   - Business description, sector, headquarters, employee count
4. **Funnel**: Universe (50-100) -> Long list (15-25) -> Short list (5-10) -> Active DD (2-3) -> Exclusive (1)
5. **Prioritisation**: rank by attractiveness (market position, growth, margins) and feasibility (availability, pricing, competitive dynamics)

## AI Readiness Assessment

<!-- Adapted from: plugins/vertical-plugins/private-equity/skills/ai-readiness/SKILL.md (anthropics/financial-services) -->

Cross-portfolio diagnostic that identifies, gates, and ranks AI opportunities by annualised EBITDA impact. Run during quarterly portfolio reviews, annual planning cycles, or when the operating partner is building a value-creation initiative around technology.

Routing: `cfa-private-markets-analyst` for single-portco assessments and quarterly portfolio reviews; `cfa-chief-analyst` when the diagnostic spans the full portfolio and feeds the fund-level value creation roll-up.

### Core Principle

Rank by dollars, not excitement. A back-office automation that saves $400k at a $40m revenue company beats a flashy customer-facing chatbot every time. Hold period and data quality together determine urgency: a company exiting in 18 months needs a faster payback than one with a 5-year runway.

### Three-Gate Evaluation (per company)

Apply all three gates before scoring. A single "No" stalls the opportunity to "Wait" status — record the specific blocker so it can be re-evaluated next quarter.

| Gate | Question | Pass Condition |
|------|----------|----------------|
| **G1 — Data Readiness** | Can clean, structured inputs be sourced without a multi-month data-engineering programme? | Usable data exists today or within 30 days at negligible cost |
| **G2 — Organisational Readiness** | Is there a named manager with authority, budget, and motivation to drive this to completion? | Owner identified; role confirmed; incentive aligned to the outcome |
| **G3 — ROI Readiness** | Does a defensible EBITDA case clear the hurdle for the company's hold period and resourcing? | Pilot scoped within 30 days; payback inside hold period; impact >=$100k annualised |

A quick win with no internal owner dies in 90 days. G2 is the binding constraint in the majority of portfolio situations.

#### Concrete Pass/Fail Criteria

**G1 — Data Readiness**
- Pass: source systems identified; extracts available in CSV/SQL/API form; data dictionary exists or can be reconstructed in a week; <10 percent missing values on the input fields the model needs; refresh cadence aligns with the use case (daily for triage, monthly for forecasting)
- Fail: data lives only in PDFs/email/free-text fields; multiple disconnected systems with no common key; CFO confirms a data-warehouse build is a prerequisite; PII or contractual restrictions block downstream use without a remediation programme

**G2 — Organisational Readiness**
- Pass: named operational owner (not the CIO by default — a line manager who feels the pain); budget line item or reallocation confirmed; success metric agreed; incentive plan touches the metric; change-management capacity exists in the relevant team
- Fail: "the COO will figure it out"; no named owner; competing transformation already consuming bandwidth; recent leadership turnover in the function; union/works-council issues unresolved

**G3 — ROI Readiness**
- Pass: EBITDA case validated against `mcp__cfa-core__variance_analysis` baseline and `mcp__cfa-core__peer_benchmarking` margin reference; payback shorter than remaining hold period (with one-year buffer); incremental capex and licence cost stay within available budget; downside case still produces positive NPV
- Fail: payback exceeds remaining hold period; impact <$100k annualised on a >$10m EBITDA company (rounding error); reliant on optimistic adoption assumptions with no data point to anchor; would trigger a covenant or capex limit

### Per-Company Opportunity Scoring Rubric

Score each opportunity 1–5 on four dimensions. Multiply dimension scores to produce a raw opportunity score (max 625), then normalise across the portfolio to produce a ranking. The four dimensions deliberately include both impact magnitude and execution risk so that flashy-but-unrealistic ideas do not crowd out grindable wins.

| Dimension | 1 (Low) | 3 (Medium) | 5 (High) |
|-----------|---------|-----------|---------|
| **Annualised EBITDA Impact** | <$100k annualised | $100k–$500k | >$500k |
| **Time-to-Value** | >6 months to measurable result | 2–6 months | <60 days |
| **Hold-Period Fit** | Payback >=remaining hold | Payback <remaining hold but >50 percent of it | Payback <25 percent of remaining hold |
| **Data-Quality Dependency** | Requires significant cleansing/integration | Partial — some work required | Clean and accessible today |

When organisational ownership is uncertain, append a fifth qualitative score for **Ownership Strength** (1 = no clear owner, 3 = identified but limited mandate, 5 = named owner with budget and KPI accountability). Use it as a tiebreaker, not a multiplier.

To quantify EBITDA impact:
- Call `mcp__cfa-core__variance_analysis` to decompose the cost or revenue line the use case targets, isolating the addressable portion
- Call `mcp__cfa-core__peer_benchmarking` to anchor the achievable margin level against in-sector peers — use the gap to top-quartile as the upper bound on impact
- Call `mcp__cfa-core__calculate_dupont` to identify whether the lever flows through margin, asset turnover, or leverage so the impact is booked against the right return-driver
- Call `mcp__cfa-core__analyze_working_capital` for back-office automation use cases where the lever is days-payable / days-receivable / inventory days
- Call `mcp__cfa-core__build_sensitivity_grid` to stress-test the impact estimate across optimistic / base / conservative adoption and accuracy assumptions

Cross-reference: every opportunity that scores GO with EBITDA impact >$250k should be reflected as a line item in the company's Value Creation Plan (see Value Creation Plan Workflow above) — the AI Readiness Assessment is the discovery and prioritisation step; the VCP is where it gets resourced and committed.

### Operational Focus Areas

Prioritise opportunities in these zones, roughly in order of typical payback speed:

1. **Back-office automation**: invoice processing, contract abstraction, expense coding, GL reconciliation — fastest to pilot, easiest to measure
2. **Revenue-facing workflows**: proposal drafting, ticket triage, call summarisation, lead scoring
3. **Sector-specific operations**: field-service scheduling, code generation (software portfolio), clinical documentation (healthcare), demand forecasting (consumer/retail)
4. **Strategic and planning processes**: board reporting automation, market intelligence aggregation, competitor monitoring

### Common Opportunity Archetypes

The same handful of patterns recur across portfolios. Use this as a starter list; do not invent novel use cases until the obvious ones are gated and ranked.

| Archetype | One-Line Description | Typical Annualised EBITDA Impact | Data Prerequisites |
|-----------|----------------------|----------------------------------|--------------------|
| **Sales productivity** | LLM-assisted proposal drafting, call summarisation, CRM hygiene, lead scoring on existing CRM data | $200k–$1.5m on a $50m revenue base; scales with sales-team size | CRM with >=12 months clean activity data; call-recording corpus or notes; product/pricing catalogue |
| **Customer support automation** | Tier-1 ticket deflection, agent assist, knowledge-base search, sentiment routing | $300k–$2m; scales with ticket volume and current cost-per-contact | Ticket history (>=90 days); resolution outcomes; KB articles or runbooks |
| **Finance / back-office automation** | AP invoice capture, expense coding, GL recon, contract abstraction, vendor-master de-duplication | $150k–$800k; scales with transaction volume and clerical FTE count | Invoice scans (PDF/image); chart of accounts; vendor master; AP system export |
| **Predictive maintenance** | Failure prediction on industrial / fleet / facility assets to shift from reactive to planned downtime | $400k–$3m on asset-heavy companies; near-zero on asset-light | Telemetry / IoT / SCADA history (>=6 months); maintenance-event log; asset register |
| **Fraud / anomaly detection** | Transaction-level scoring for chargeback, expense, returns, claims, or supplier fraud | $200k–$1.5m at firms with >$500m of throughput; lower at smaller scale | Transaction ledger (>=18 months); known-fraud labels; identity / device data where available |
| **Demand forecasting** | SKU-level forecasts to drive inventory, staffing, or capacity decisions in consumer / retail / hospitality | $250k–$1.5m on a $100m revenue base; depends on current forecast error | POS / sales history (>=24 months); promotional calendar; weather/seasonal exogenous data |
| **Code generation (software portfolio)** | Developer copilot, test generation, code-review automation in software / tech-enabled portcos | 10–25 percent engineering productivity uplift; flows through to release velocity, not always EBITDA | Repo access; CI/CD telemetry; ticket-to-PR linkage |

### EBITDA-Impact Ranking Output Table

Produce one summary table covering all portfolio companies assessed. Sort descending by Annualised EBITDA Impact.

```
| Company | Use Case | Gate Status | EBITDA Impact (ann.) | Speed to Result | Score | Priority |
|---------|----------|-------------|----------------------|-----------------|-------|----------|
| Co A    | AP automation | GO       | $420k                | 45 days         | 189   | 1        |
| Co B    | Ticket triage | GO       | $310k                | 60 days         | 150   | 2        |
| Co C    | Demand forecast | WAIT — G1 | $550k             | 90 days         | —     | Blocked  |
```

Gate status: **GO** (all three gates pass), **WAIT** (one or more gates fail — record blocker), **PASS** (opportunity assessed and declined — record reason).

### Cross-Portfolio Diagnostic Workflow

Designed to scan 5–20 portfolio companies inside a single quarter without the operating-partner team disappearing into bespoke per-company exercises. Run on a fixed cadence so quarter-on-quarter movement is visible.

**Quarter-long cadence**

- Week 1: scope and intake. Confirm the company list, pull the latest quarterly financials, request a one-page operating-team memo per portco listing 3–6 candidate use cases. Pre-load `mcp__cfa-core__variance_analysis` and `mcp__cfa-core__peer_benchmarking` baselines for every company so analysts are not waiting on data when they score.
- Weeks 2–6: per-company gating and scoring. Allocate a half-day analyst slot per portco (5 portcos: 2.5 days; 20 portcos: 10 days plus buffer). Each session applies the three gates, scores GO opportunities on the four-dimension rubric, and runs `mcp__cfa-core__build_sensitivity_grid` on the top 1–3 use cases for that company. Do not score WAIT or PASS opportunities — record the blocker and move on.
- Weeks 7–8: portfolio synthesis. Rank every GO opportunity across every company. Identify replay candidates (next subsection). Pick the top three for deep-dive memos.
- Week 9: top-three deep dives. For each of the three highest-scoring opportunities, produce a 2–3 page memo with: scoped pilot design, owner, success metric, 30/60/90-day plan, sensitivity table, and explicit linkage into the company's VCP.
- Weeks 10–11: review and challenge. Operating partner / IC challenge session; revise scores where the challenge surfaces new information. Lock the ranking.
- Week 12: deliverable lock and next-quarter rollover. Publish the portfolio roll-up. Move WAIT items to a watchlist with an owner-by date for the blocker. Schedule next-quarter re-gate.

**Output format**

Two artifacts:

1. **Ranked portfolio table** (single page; the headline). One row per scored opportunity, sorted by Annualised EBITDA Impact descending. Columns: Company, Use Case, Gate Status, EBITDA Impact (annualised), Speed to Result, Score, Priority, Owner, Capture Probability. WAIT and PASS items appear at the bottom for completeness with blocker / reason populated.
2. **Top-three deep-dive memos** (2–3 pages each). For the three highest-ranked opportunities, the memo covers: business context, scoped pilot, data prerequisites and current state, owner and resourcing, 30/60/90-day plan, sensitivity grid output, downside scenarios, dependencies on other VCP initiatives, and the success metric the operating partner will track at the next board meeting.

A quarterly diagnostic produces a single ranked table even if 80 percent of opportunities are unchanged from the prior quarter — the value is in tracking which WAIT items resolved, which scores moved, and which deep-dives shipped on time.

### Cross-Portfolio Replay Identification

After completing individual company assessments, scan for patterns where the same solution can be deployed across multiple companies with minimal incremental effort.

Replay criteria:
- Two or more companies in the same sector or with comparable workflow structures
- Same tooling or vendor can serve both with configuration-only changes
- Combined EBITDA impact justifies a portfolio-wide negotiated contract or shared implementation resource

Document each replay candidate with: (a) companies included, (b) combined EBITDA impact, (c) shared implementation cost saving vs individual deployments, (d) recommended sequencing (lead company first, followers within 60–90 days).

Call `mcp__cfa-core__analyze_strategy` to frame the replay as a portfolio-level operational lever alongside financial value creation initiatives.

### Deliverable Structure (Operating Partner)

Produce a one-page executive summary plus supporting detail. Structure:

**One-Page Summary**
- Portfolio snapshot: number of companies assessed, GO / WAIT / PASS breakdown
- Top 3 ranked opportunities with headline EBITDA impact and owner name
- Replay candidates with aggregate impact
- Recommended next step for each GO opportunity (kickoff date, pilot scope, success metric)

**Supporting Detail (one section per company)**
- Gate assessment with pass/fail rationale
- Scored opportunity table (all use cases considered, not just top-ranked)
- Implementation sketch: tooling, data requirements, owner, 30-day pilot design
- Risk flags: data privacy constraints, IT change-freeze windows, integration complexity

**Portfolio Roll-Up**
- Total addressable EBITDA impact if all GO opportunities are executed
- Expected capture rate (apply 60–70% probability to GO opportunities; 0% to WAIT)
- Impact on portfolio EBITDA bridge alongside VCP initiatives
- Call `mcp__cfa-core__analyze_working_capital` and `mcp__cfa-core__build_sensitivity_grid` to validate the roll-up figures against each company's financial baseline

### Failure Modes

The diagnostic itself fails predictably. Watch for these patterns and stop the work before producing a deliverable that misleads the IC.

- **Data-quality blockers presented as solvable**: an operating team marks G1 as Pass on the strength of a "we'll clean it up" plan. Force the question: which named person, on which budget, by which date. If any answer is missing, downgrade to WAIT regardless of how attractive the impact looks.
- **Change-management debt**: the company has lived through a recent ERP migration, leadership turnover, or carve-out. Adoption capacity is exhausted. A new initiative competes for the same scarce attention and quietly stalls. Apply a haircut to capture probability (not score) when this is present.
- **Vendor lock-in**: the impact case requires committing to a single proprietary platform, a multi-year minimum, or a data-egress restriction that prevents migration. Score the lock-in as a downside scenario in the sensitivity grid and require an exit clause in the contract.
- **Model-drift risk**: the use case depends on a model whose accuracy degrades as the underlying data distribution shifts (fraud, demand forecasting, lead scoring). Without a monitoring regime and a refresh budget, the impact decays inside the hold period. Require a monitoring KPI before scoring above 3 on Time-to-Value.
- **EBITDA-impact double-counting**: the same headcount or cost line appears as a saving in two different VCP initiatives. Reconcile the AI Readiness ranking against the active VCP at the synthesis stage; flag any overlapping initiatives to the operating partner.
- **Pilot-to-production gap**: a successful 30-day pilot does not equal a production rollout. The capture probability on the portfolio roll-up should reflect this — the 60–70 percent default exists for a reason.
- **Discovery theatre**: the diagnostic produces a ranked list every quarter and nothing ever ships. Track shipped vs ranked across quarters; if the ratio falls below one in three for two consecutive quarters, the diagnostic is generating heat without light and the operating model needs a reset.

### Workflow Selection Integration

| Request | Workflow | Output | Key Tools |
|---------|----------|--------|-----------|
| "AI readiness scan" | AI Readiness Assessment | 1-page summary + per-company detail | `variance_analysis`, `peer_benchmarking`, `calculate_dupont`, `analyze_working_capital`, `build_sensitivity_grid`, `analyze_strategy` |
| "Which portfolio companies are ready for AI?" | AI Readiness Assessment | Gate assessment + ranked table | `variance_analysis`, `peer_benchmarking`, `build_sensitivity_grid` |
| "Find AI replays across the portfolio" | AI Readiness Assessment (Replay step) | Replay candidates with combined impact | `analyze_strategy` |
| "Quarterly portfolio AI review" | Cross-Portfolio Diagnostic | Ranked table + top-3 deep-dive memos | `variance_analysis`, `peer_benchmarking`, `build_sensitivity_grid`, `analyze_strategy` |

---

## Quality Standards

- LBO returns: target 20-25% IRR / 2.5-3.0x MOIC for a typical mid-market buyout
- Z-Score < 1.81 at entry leverage = red flag for over-leveraged deal structure
- Sources must equal Uses in every S&U table, verified to the penny
- IC memo: financial tables must be internally consistent across all sections
- Screening memo: always include both bull AND bear case with honest assessment
- Returns analysis: always show at least 3 scenarios (base/upside/downside)
- VCP: every initiative must be quantified with estimated EBITDA impact
- Portfolio monitoring: covenant headroom <15% triggers early warning escalation

## Output Standards

All private equity output should:
1. State the investment question being answered
2. Lead with the recommendation and headline returns (inverted pyramid)
3. Show methodology, key assumptions, and sensitivity analysis
4. Present bull and bear cases with equal rigour
5. Flag risks before opportunities
6. Be auditable: someone can follow the logic, check the math, and verify against tool output
7. End with a clear recommendation and actionable next steps
