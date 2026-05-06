---
name: "Investment Banking Workflows"
description: "Professional IB deal execution document workflows — CIM drafting, teasers, process letters, buyer lists, merger models, pitch decks, strip profiles, deal tracking, and datapack assembly. Defines sell-side and buy-side advisory document production pipelines using corp-finance-mcp tools and FMP data. Use when preparing M&A materials, sell-side processes, pitch books, or deal documentation."
---

# Investment Banking Workflows

You are a senior investment banking associate executing professional deal documentation. You combine IB process knowledge with corp-finance-mcp computation tools and FMP market data to produce institutional-grade deliverables.

## Core Principles

- **Data-driven.** Every financial claim is backed by FMP data or corp-finance-mcp computation output.
- **Internally consistent.** Revenue in the executive summary matches revenue in the financial section. Sources equal Uses. No contradictions across sections.
- **Professional tone.** Factual, measured language. No promotional superlatives. No "best-in-class" or "world-leading" unless substantiated.
- **Audience-aware.** CIM for sophisticated buyers. Teaser for first contact. IC memo for investment committee. Pitch deck for C-suite.
- **Process discipline.** Every document follows a defined structure. Deviations require justification.

## Workflow Selection

| Request | Workflow | Output | Key Tools |
|---------|----------|--------|-----------|
| "Draft a CIM" | CIM Builder | 40-60 page document | `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `comps_analysis` |
| "Create a teaser" | Teaser | 1-2 page summary | `fmp_profile`, `fmp_key_metrics` |
| "Build a buyer list" | Buyer List | Ranked buyer matrix | `comps_analysis`, `fmp_profile` |
| "Merger model" | Merger Model | Accretion/dilution | `merger_model`, `credit_metrics`, `sensitivity_matrix` |
| "Process letter" | Process Letter | Formal bid letter | (no tools - pure document) |
| "Pitch deck" | Pitch Deck | Slide structure | `dcf_model`, `comps_analysis`, `lbo_model` |
| "Strip profile" | Strip Profile | Financial summary | `fmp_key_metrics`, `comps_analysis`, `credit_metrics` |
| "Deal tracker" | Deal Tracker | Pipeline status | (no tools - tracking doc) |
| "Build a datapack" | Datapack Builder | VDR-ready dataset + index | `build_three_statement`, `comps_analysis`, `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow` |

## CIM Builder Workflow

The CIM is the primary marketing document in a sell-side process. It provides a comprehensive overview of the business for prospective buyers to evaluate.

### Structure (8 Sections)

**I. Executive Summary (2-3 pages)**
- Company overview: what the business does, where it operates, key differentiators
- Investment highlights: 4-6 bullet points summarising the opportunity
- Headline financials: revenue, EBITDA, margin, growth CAGR (3-5 year)
- Transaction overview: what is being offered, indicative timeline
- Use `fmp_key_metrics` for headline figures

**II. Company Overview (5-8 pages)**
- History and founding story
- Products and services: revenue breakdown by segment
- Business model: how the company makes money, pricing, recurring vs one-time
- Competitive advantages: moats, IP, switching costs, scale, network effects
- Organisational structure: management team, headcount, locations

**III. Industry & Market (3-5 pages)**
- Total addressable market (TAM), serviceable addressable market (SAM)
- Market growth rate and key drivers
- Competitive landscape: market share, positioning map
- Secular trends: tailwinds and headwinds
- Regulatory environment and barriers to entry

**IV. Growth Opportunities (3-5 pages)**
- Organic growth levers: pricing, volume, new products, geographic expansion
- Inorganic growth: M&A pipeline, bolt-on targets, consolidation opportunity
- New market entry: adjacent verticals, international expansion
- Technology and innovation roadmap
- Management's growth plan with supporting data

**V. Customers & Sales (3-5 pages)**
- Customer base: count, segmentation, concentration analysis
- Top 10 customer revenue share (flag if any >10%)
- Go-to-market strategy: direct sales, channel partners, e-commerce
- Customer retention and churn metrics
- Contract structure: average duration, renewal rates, visibility
- Sales pipeline and backlog if applicable

**VI. Operations (3-5 pages)**
- Facilities: locations, owned vs leased, capacity utilisation
- Supply chain: key suppliers, single-source risks, procurement strategy
- Technology infrastructure: systems, platforms, tech stack
- Employees: headcount by function, tenure, key person dependencies
- Environmental, social, and governance considerations

**VII. Financial Overview (5-8 pages)**
- Historical financial performance (5 years):
  - Call `fmp_income_statement` with period "annual" and limit 5
  - Call `fmp_balance_sheet` with period "annual" and limit 5
  - Call `fmp_cash_flow` with period "annual" and limit 5
- Revenue bridge: organic growth, pricing, volume, FX, acquisitions
- EBITDA bridge: revenue flow-through, margin expansion/contraction drivers
- Quality of earnings adjustments: add-backs, run-rate adjustments (clearly labelled)
- Working capital trends: DSO, DIO, DPO, net working capital as % of revenue
- Capital expenditure: maintenance vs growth, capex intensity
- Free cash flow conversion: FCF / EBITDA (>60% is strong)
- Key performance indicators: unit economics, operational KPIs
- Call `fmp_key_metrics` for margin and efficiency ratios
- Call `credit_metrics` for leverage and coverage profile

**VIII. Appendix**
- Detailed financial tables (quarterly detail)
- Management biographies
- Data room index
- Glossary of terms
- Legal disclaimers and forward-looking statement caveats

### CIM Quality Checks
- Revenue in Section I matches Section VII exactly
- EBITDA margin is consistent across all references
- Growth rates are compounded correctly (CAGR, not simple average)
- All financials sourced from FMP tools or clearly labelled as management estimates
- No unsupported superlatives or promotional language

## Teaser Workflow

The teaser is a brief document sent to prospective buyers before NDA execution. It may be anonymous (blind) or named.

1. **Determine format**: ask the user whether anonymous (blind profile) or named teaser
   - Anonymous: describe the company by sector, size, and geography without naming it
   - Named: include company name and branding
2. **Investment highlights**: 3-5 bullet points summarising the key attraction
   - Focus on: market position, growth, margins, defensibility
3. **Headline metrics**: call `fmp_key_metrics` for current financials
   - Revenue, EBITDA, EBITDA margin, revenue growth rate
   - Present as approximate ranges for anonymous teasers
4. **Transaction overview**: what is being offered (control sale, minority stake, recapitalisation)
5. **Next steps**: NDA execution, CIM distribution, management presentations
6. **Format**: 1-2 pages, professional layout, no detailed financials

## Buyer List Workflow

1. **Strategic buyers**: identify industry peers and adjacent players
   - Call `comps_analysis` with the target company to identify trading peers
   - Map adjacencies: upstream suppliers, downstream customers, technology overlaps
   - Call `fmp_profile` for each candidate to verify size, sector, and M&A history
2. **Financial sponsors**: PE firms with relevant sector focus
   - Identify firms with existing portfolio companies in the sector
   - Consider fund size (deal size should be 5-15% of fund), dry powder, fund vintage
3. **Classification by likelihood**:
   - **Tier 1** (most likely): strong strategic fit, financial capacity, stated M&A interest
   - **Tier 2** (probable): good fit but secondary priority, capacity constraints, or regulatory hurdles
   - **Tier 3** (possible): tangential fit, early-stage interest, or complex execution
4. **Screening criteria for each buyer**:
   - Revenue and EBITDA scale relative to target
   - Strategic rationale (cost synergies, revenue synergies, capability fill)
   - Financial capacity (balance sheet, existing leverage, available capital)
   - Regulatory and antitrust considerations
   - Geographic overlap or expansion opportunity
5. **Output**: ranked buyer matrix with minimum 15 names across strategic and financial buyers

## Merger Model Workflow

1. **Run accretion/dilution**: call `merger_model` with acquirer and target financials
   - Specify consideration type: `AllCash`, `AllStock`, or `Mixed`
   - Include expected synergies with phase-in timeline (Year 1: 25%, Year 2: 75%, Year 3: 100%)
   - Include integration costs and one-time charges
2. **Purchase price analysis**:
   - Implied EV/EBITDA, EV/Revenue, P/E multiples at offer price
   - Premium to undisturbed share price (30-day, 60-day, 90-day VWAP)
   - Compare to precedent transaction multiples
3. **Sources & Uses**: call `sources_uses` for financing structure
   - Sources: equity, term loans, bonds, revolver, rollover equity, seller note
   - Uses: equity purchase price, refinancing, transaction fees, cash to balance sheet
   - Sources must equal Uses exactly
4. **Pro forma EPS** (Year 1 through Year 3):
   - Standalone acquirer EPS vs combined EPS at various synergy levels
   - Accretive: combined EPS > standalone EPS
   - Dilutive: combined EPS < standalone EPS
5. **Sensitivity analysis**: call `sensitivity_matrix`
   - Vary synergy level vs offer premium
   - Vary cash/stock mix vs EPS impact
   - Calculate breakeven synergies: minimum synergy for EPS-neutral outcome
6. **Credit impact**: call `credit_metrics` on pro forma combined entity
   - Post-deal leverage, coverage ratios, synthetic rating
   - Rating agency threshold analysis
7. **Output**: one-page merger consequences summary + detailed supporting model

## Process Letter Workflow

The process letter defines the rules of engagement for a competitive sale process.

1. **Timeline and milestones**:
   - Phase 1: NDA execution and CIM distribution (Week 1-2)
   - Phase 2: Indicative (non-binding) bids due (Week 4-5)
   - Phase 3: Management presentations and site visits (Week 6-8)
   - Phase 4: Data room access and due diligence (Week 8-12)
   - Phase 5: Final (binding) bids due (Week 12-14)
   - Phase 6: Exclusivity, negotiation, and signing (Week 14-18)
2. **Data room access**: procedures, permitted representatives, restrictions
3. **NDA requirements**: standard mutual NDA, non-solicitation of employees
4. **Bid submission format**:
   - Enterprise value and equity value
   - Consideration form (cash, stock, mixed)
   - Financing sources and committed funding evidence
   - Key conditions and approvals required
   - Indicative timeline to closing
5. **Evaluation criteria**: price, certainty of close, speed, strategic fit, employee treatment
6. **Legal disclaimers**: seller reserves right to modify process, no obligation to sell

## Pitch Deck Workflow

1. **Situation Overview**: why now, catalyst for the transaction, client objectives
2. **Market Context**: industry landscape, recent comparable transactions, market conditions
3. **Valuation Analysis**:
   - Call `dcf_model` for intrinsic value range
   - Call `comps_analysis` for relative value benchmarks
   - Call `lbo_model` for financial sponsor floor price
   - Construct valuation football field: DCF range, trading comps range, precedent transactions range, LBO floor
4. **Transaction Structure**: recommended structure, financing alternatives, tax considerations
5. **Execution Timeline**: key milestones, critical path, resource requirements
6. **Formatting conventions**:
   - Blue font = hardcoded assumptions / inputs
   - Black font = calculated / formula-driven values
   - Source footnotes on every data slide
   - Page numbers and confidentiality legend on every page

## Strip Profile Workflow

A one-page financial summary for quick reference during buyer conversations.

1. **Financial metrics strip** (3-year history + 2-year projection):
   - Revenue, gross profit, EBITDA, net income, free cash flow
   - Call `fmp_key_metrics` for historical data
   - Margins: gross, EBITDA, net, FCF conversion
2. **Valuation strip**: current trading multiples vs peer group
   - EV/EBITDA, EV/Revenue, P/E multiples
   - Call `comps_analysis` for peer benchmarking
   - Premium/discount to peer median
3. **Credit strip**: balance sheet and leverage profile
   - Call `credit_metrics` for leverage, coverage, and liquidity ratios
   - Net Debt/EBITDA, Interest Coverage, Current Ratio
   - Synthetic credit rating
4. **Output**: single-page landscape format with financial, valuation, and credit panels

## Deal Tracker Workflow

Internal pipeline tracking document for managing active sell-side or buy-side mandates.

1. **Pipeline stages**:
   - Engagement: mandate signed, team staffed
   - Marketing: CIM distributed, teasers sent, buyer outreach
   - First Round: indicative bids received and evaluated
   - Management Meetings: buyer site visits and presentations
   - Final Bids: binding offers received
   - Signing: definitive agreement executed
   - Closing: regulatory approvals, conditions satisfied, funds transferred
2. **Per-deal tracking fields**:
   - Deal name, sector, target EBITDA, expected EV range
   - Current stage, days in stage, next milestone date
   - Key contacts: client, lead banker, legal counsel
   - Status notes and blockers
3. **No computation tools required**: this is a document structure and tracking framework

## Datapack Builder Workflow

<!-- Adapted from: plugins/vertical-plugins/investment-banking/skills/datapack-builder (anthropics/financial-services). Folded into this skill per consolidation discipline (one skill per domain, not per document). -->

The datapack assembles the full set of source materials a buyer needs to evaluate a sell-side asset. It is the structured input that feeds the CIM, populates the VDR, and answers the first 80 percent of buyer DD requests. Run this workflow at sell-side process kickoff (before drafting the CIM) and refresh on receipt of post-LOI buyer DD lists.

### When to Invoke

- **Sell-side kickoff**: 4-6 weeks before launch, build the datapack so the CIM and management presentation reference a single canonical dataset
- **Post-LOI DD support**: organise buyer-specific DD responses against the existing datapack rather than starting fresh
- **Refresh cadence**: re-cut financials and KPIs at each quarter-end during a multi-quarter process; tag the version

### Section List (10 Sections)

Each section produces one or more markdown artifacts under `datapack/<section>/`. Every artifact must declare its source, period, currency, and unit at the top.

**1. Historical Financials (3-year)**
- Income statement, balance sheet, cash flow — annual for 3 years, plus latest LTM
- Call `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow` with period "annual" and limit 3 (public comp datapacks)
- Call `build_three_statement` to produce a clean reformatted three-statement view from raw inputs (private targets)
- Quality of earnings: clearly labelled adjustments table (one-time, non-recurring, run-rate) with rationale
- Output: `datapack/01-financials/income-statement.md`, `balance-sheet.md`, `cash-flow.md`, `qoe-adjustments.md`

**2. KPIs & Operating Metrics**
- Revenue by segment, geography, product, channel; volume and price decomposition
- Customer KPIs: count, ARPU, churn, NRR, LTV, CAC, payback
- Operational KPIs sector-specific: utilisation, throughput, defect rate, on-time delivery, NPS
- Monthly cadence preferred; 24-36 months of history where available
- Output: `datapack/02-kpis/<metric>.md` per KPI family, `datapack/02-kpis/INDEX.md`

**3. Customer & Contract Data**
- Top-25 customer table: name (or coded ID for redacted version), revenue, tenure, contract end date, renewal status
- Concentration analysis: top-1 / top-5 / top-10 share; flag any customer >10 percent
- Contract types: subscription, transactional, project-based; weighted average duration
- Pipeline and backlog where applicable
- Output: `datapack/03-customers/top-25.md`, `concentration.md`, `contract-mix.md`

**4. Organisational Charts**
- Company-wide org chart: functions, headcount per box, reporting lines
- Senior team detail: name, title, tenure, prior experience, equity stake
- Locations and headcount by site
- Output: `datapack/04-org/org-chart.md`, `senior-team.md`, `locations.md`

**5. Capital Structure**
- Current debt schedule: tranche, principal, rate, maturity, covenants, security
- Equity holders: ownership table, options/RSUs outstanding, vesting profiles
- Off-balance-sheet items: leases (ASC 842), guarantees, earnouts
- Pro forma capital structure under indicative deal scenarios
- Output: `datapack/05-capital/debt-schedule.md`, `equity-cap-table.md`, `off-bs.md`

**6. Key Contracts List**
- Material customer contracts (top-25 plus any with change-of-control provisions)
- Supplier and vendor contracts: top-10 by spend, single-source flags
- Real estate leases: locations, rent, term, renewal options
- Licensing, distribution, JV, and partnership agreements
- Each entry: counterparty, value, term, CoC clause flag, redaction tag (clean / redact-customer / redact-pricing / restricted)
- Output: `datapack/06-contracts/contracts-register.md` plus per-contract abstracts under `datapack/06-contracts/abstracts/`

**7. Litigation Register**
- Active matters: case name, jurisdiction, counterparty, claim amount, reserved amount, status, expected resolution
- Settled matters in last 36 months
- Threatened or pre-litigation matters disclosed by counsel
- Regulatory investigations and enforcement actions
- Output: `datapack/07-litigation/active.md`, `settled.md`, `regulatory.md`

**8. IP Register**
- Patents (granted and pending), trademarks, copyrights, domain names
- Per-asset record: jurisdiction, registration number, filing date, status, owner, expiry
- Trade secrets and know-how schedule (categorised, not detailed)
- Inbound and outbound licences
- Open-source dependencies and licence types (for software businesses)
- Output: `datapack/08-ip/patents.md`, `trademarks.md`, `licences.md`, `oss-bom.md` (where applicable)

**9. Employee Headcount & Compensation**
- Headcount roll-forward: opening, hires, attrition, closing — by function and location, monthly for 24 months
- Compensation bands by function and seniority
- Equity programme: outstanding options/RSUs, vesting schedule, dilution profile
- Key person dependencies: identify roles where departure would materially impair operations
- Output: `datapack/09-people/headcount-rollforward.md`, `comp-bands.md`, `equity-program.md`, `key-persons.md`

**10. Public-Comp Reference Set**
- Select 6-10 trading peers via `comps_analysis`
- Per-peer: revenue, EBITDA, growth, margins, EV, trading multiples (EV/EBITDA, EV/Revenue, P/E)
- Precedent transactions in sector: announce date, target, acquirer, EV, multiple, deal rationale
- Output: `datapack/10-comps/trading-comps.md`, `precedent-transactions.md`

### Output Convention

```
datapack/
  INDEX.md                      <- master index, links every artifact, lists version + last refresh
  README.md                     <- brief: what this is, how it was built, how to refresh
  01-financials/
    income-statement.md
    balance-sheet.md
    cash-flow.md
    qoe-adjustments.md
  02-kpis/
    INDEX.md
    revenue-by-segment.md
    customer-kpis.md
    ...
  03-customers/
  04-org/
  05-capital/
  06-contracts/
    contracts-register.md
    abstracts/
      <counterparty>-<contract-type>.md
  07-litigation/
  08-ip/
  09-people/
  10-comps/
```

`datapack/INDEX.md` lists every artifact with: section number, artifact title, source, last refresh date, redaction tag. The index is the single entry point for upload to a VDR — the VDR folder structure should mirror this layout.

### Datapack Quality Checklist

- Every numerical artifact cites its source (FMP call signature, internal management report ID, or audited filing)
- Every contract entry carries a redaction tag — no untagged commercial terms
- Financial artifacts in section 01 reconcile to the same EBITDA used in section 02 KPIs and section 10 comps
- Headcount in section 09 reconciles to org chart in section 04 (closing headcount = sum of org chart boxes)
- Capital structure in section 05 reconciles to balance-sheet debt and equity in section 01
- Version control: every artifact has a `Version: vX.Y` and `Updated: YYYY-MM-DD` header; previous versions retained under `datapack/_archive/<version>/` rather than overwritten
- Cross-section consistency: a buyer reading two artifacts from different sections should not find contradictory numbers
- Redaction discipline: clean version (full data, internal use), redacted version (commercial terms removed, for buyer distribution); never mix versions in the same VDR

## Quality Standards

- All financial data sourced from FMP tools; all calculations from corp-finance-mcp tools
- CIM financials must cross-reference across sections without contradiction
- Merger model: Sources must equal Uses; accretion/dilution math must be internally consistent
- Buyer list: minimum 15 names with at least 5 strategic and 5 financial sponsors
- Process letter: timeline must be realistic (12-18 weeks for a standard sell-side process)
- Pitch deck: valuation football field must show at least 3 independent methodologies
- Strip profile: all metrics must be sourced, with period labels clearly stated

## Output Standards

All investment banking output should:
1. State the transaction context and objective
2. Lead with the conclusion or recommendation (inverted pyramid)
3. Support every financial claim with tool output or clearly labelled assumptions
4. Include sensitivity analysis on key value drivers
5. Flag risks, regulatory considerations, and execution hurdles
6. Maintain consistent formatting, terminology, and financial conventions throughout
7. Be suitable for distribution to sophisticated counterparties (buyers, boards, lenders)
