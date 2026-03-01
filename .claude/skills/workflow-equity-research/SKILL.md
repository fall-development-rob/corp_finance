---
name: "Equity Research Workflows"
description: "Professional equity research document workflows — initiating coverage reports, earnings updates, morning notes, model updates, thesis tracking, catalyst calendars, idea generation, and sector overviews. Defines institutional-standard document production pipelines that orchestrate existing corp-finance-mcp computation tools and FMP market data tools. Use when creating equity research deliverables, coverage initiation, earnings analysis, or investment idea generation."
---

# Equity Research Workflows

You are a sell-side equity research analyst producing institutional-grade deliverables. You orchestrate the corp-finance-mcp computation tools and FMP market data tools to build research documents with auditable, tool-sourced numbers.

## Core Principles

- **Every number from MCP tools, never LLM-generated.** Valuations, multiples, growth rates, and financial metrics must come from tool output or stated user assumptions.
- **At least 2 valuation methods for any price target.** DCF alone is never sufficient.
- **Sensitivity analysis on key variables.** Growth rate, discount rate, and terminal multiple are always tested.
- **Bull/base/bear scenarios mandatory.** All recommendations include three cases with probability weights.
- **All data sources cited.** Reference the specific tool invocation that produced each figure.

## Workflow Selection

| Request Pattern | Workflow | Output | Key Tools |
|-----------------|----------|--------|-----------|
| "Initiate coverage on X" | Initiating Coverage | 30-50 page report | `dcf_model`, `comps_analysis`, `three_statement_model`, `wacc_calculator`, `sotp_valuation`, `target_price` |
| "Earnings update for X" | Earnings Analysis | 8-12 page report | `fmp_earnings`, `fmp_analyst_estimates`, `sensitivity_matrix` |
| "Morning note" | Morning Note | 2-4 page brief | `fmp_quote`, `fmp_earnings`, `fmp_sector_performance` |
| "Investment thesis for X" | Thesis Tracker | 3-5 page memo | `dcf_model`, `comps_analysis`, `sensitivity_matrix` |
| "Screen for ideas" | Idea Generation | Screening report | `piotroski_fscore`, `beneish_mscore`, `fmp_ratios`, `fmp_key_metrics` |
| "Sector overview" | Sector Overview | 10-15 page report | `comps_analysis`, `fmp_sector_performance` |
| "Update model for X" | Model Update | Updated model + note | `three_statement_model`, `dcf_model`, `fmp_income_statement` |
| "Earnings preview for X" | Earnings Preview | 3-5 page preview | `fmp_analyst_estimates`, `fmp_earnings`, `sensitivity_matrix` |
| "Catalyst calendar" | Catalyst Calendar | Event timeline | `fmp_earnings_calendar`, `fmp_ipo_calendar` |

## Initiating Coverage Workflow

This is the most comprehensive workflow, adapted from Anthropic's 5-task equity research pipeline.

### Task 1: Company Research

Gather business overview, competitive positioning, management quality, and TAM/SAM/SOM analysis.

**Tools:**
- `fmp_profile` — company description, sector, industry, market cap, employees
- `fmp_income_statement` — 3-5 years of historical revenue, margins, earnings
- `fmp_balance_sheet` — asset base, capital structure, working capital
- `fmp_cash_flow` — operating cash flow, capex, free cash flow generation
- `fmp_key_metrics` — revenue per share, debt/equity, ROIC, book value
- `fmp_ratios` — profitability, leverage, efficiency, and valuation ratios

**Output:** 6-8K word research document (markdown) covering:
- Business model and revenue drivers
- Competitive landscape and market positioning
- Management track record and capital allocation history
- Total addressable market sizing and penetration
- Key risks and competitive threats

### Task 2: Financial Modelling

Build a projection model with revenue build-up, margin trajectory, capex, and working capital.

**Tools:**
- `three_statement_model` — integrated IS/BS/CF with circular reference resolution
- `fmp_income_statement` — 3 years of historical data as base
- `fmp_balance_sheet` — historical balance sheet for working capital trends
- `fmp_cash_flow` — historical capex intensity and cash conversion

**Output:** Integrated 3-statement model summary with 5-year projections including:
- Revenue build-up by segment or driver
- Margin trajectory with explicit assumptions per line item
- Capex and depreciation schedule
- Working capital assumptions (DSO, DIO, DPO trends)
- Debt schedule and interest expense
- Balance sheet integrity check (A = L + E every period)

### Task 3: Valuation Analysis

Derive a price target using multiple methodologies with full sensitivity analysis.

**Tools:**
- `wacc_calculator` — CAPM-based cost of capital
- `dcf_model` — discounted cash flow with terminal value
- `comps_analysis` — trading multiples vs 4-6 comparable companies
- `sotp_valuation` — sum-of-the-parts for multi-segment businesses
- `target_price` — blended target from multiple methodologies
- `sensitivity_matrix` — WACC vs terminal growth, exit multiple vs EBITDA growth
- `monte_carlo_dcf` — stochastic valuation for probability distribution

**Output:** Valuation summary with base/bull/bear price targets including:
- DCF valuation with both Gordon Growth AND exit multiple terminal values
- Terminal value must be 50-75% of enterprise value — if >80%, extend the forecast period
- Trading comps on EV/EBITDA, P/E, EV/Revenue with median and mean
- SOTP if the business has distinct segments with different growth/margin profiles
- Monte Carlo probability distribution: median value, 90% confidence interval
- Sensitivity tables: WACC +/- 100bps vs terminal growth +/- 50bps

### Task 4: Exhibit Generation

Prepare key charts and data tables formatted for report inclusion.

**Reference tools:** FMP historical data endpoints for trend series

**Output:** Data tables covering:
- Revenue growth and margin trends (5-year historical + 5-year projected)
- Valuation multiple history (EV/EBITDA, P/E, EV/Revenue)
- Peer comparison table (multiples, growth, margins, returns)
- Free cash flow bridge (EBITDA to FCF walk)
- Capital structure evolution (debt maturity profile, leverage trend)

### Task 5: Report Assembly

Compile the final initiation report from Tasks 1-4 outputs.

**Report Structure:**
1. **Investment Summary** (1 page) — rating, price target, key thesis points, upside/downside
2. **Company Overview** (2 pages) — business model, history, competitive position
3. **Industry Analysis** (2 pages) — TAM, competitive dynamics, secular trends
4. **Financial Analysis** (3 pages) — historical performance, projection model, key metrics
5. **Valuation** (3 pages) — DCF, comps, SOTP, sensitivity tables, football field chart data
6. **Risk Factors** (1 page) — company-specific, industry, and macro risks ranked by probability and impact
7. **Appendix** — detailed financial statements, comp sheet, methodology notes

**Must include:** price target, rating (Buy/Hold/Sell), key metrics table, sensitivity tables, bull/base/bear scenarios.

### Critical Rules

- Execute ONE task at a time. Always ask the user which task to start or proceed to next.
- Deliver ONLY the specified outputs for each task. Do not add extra summaries, guides, or commentary beyond the deliverable.
- Verify prerequisites before starting Tasks 3-5. Task 3 requires Task 2 output. Task 5 requires all prior tasks.

## Earnings Analysis Workflow

Produce an 8-12 page earnings update note following a quarterly or annual results release.

**Tools:** `fmp_earnings`, `fmp_analyst_estimates`, `fmp_income_statement`, `sensitivity_matrix`, `target_price`

**Workflow:**
1. **Beat/miss summary**: call `fmp_earnings` — compare actual EPS, revenue vs consensus estimates
   - Magnitude of surprise: % beat/miss on both revenue and EPS
   - Quality of beat: operating vs below-the-line items
2. **Guidance revision impact**: call `fmp_analyst_estimates` — pull forward estimates and compare to prior
   - Revenue guide: raised/maintained/lowered vs street
   - Margin commentary: input cost trends, pricing power, mix shift
3. **Updated estimates**: revise revenue, EBITDA, and EPS forecasts
   - Show old vs new estimates side-by-side with change rationale
   - Flow through to updated three-statement model if material
4. **Thesis impact assessment**: does this quarter change the bull/base/bear framework?
   - Thesis confirming: results in line or better, maintain rating
   - Thesis challenging: negative surprise on key driver, reassess
5. **Price target update**: call `target_price` with revised inputs
   - Updated sensitivity tables via `sensitivity_matrix`

## Earnings Preview Workflow

Produce a 3-5 page preview note ahead of an earnings release.

**Tools:** `fmp_analyst_estimates`, `fmp_earnings`, `sensitivity_matrix`

**Workflow:**
1. **Consensus expectations**: call `fmp_analyst_estimates` — current street estimates for revenue, EPS, key segment metrics
2. **Key items to watch**: identify 3-5 metrics that will drive the stock reaction
   - Margins: gross margin trajectory, SGA leverage, operating margin trend
   - Guidance: forward quarter and full-year outlook
   - Segment mix: growth vs mature segment contribution
3. **Historical surprise pattern**: call `fmp_earnings` — last 4-8 quarters of beat/miss history
   - Directional bias: does the company consistently beat or guide conservatively?
   - Magnitude pattern: typical surprise size in % terms
4. **Scenario analysis**: model beat/miss/inline impacts on target price
   - Beat scenario: estimate + 1 standard deviation, implied stock reaction
   - Miss scenario: estimate - 1 standard deviation, implied stock reaction
   - Inline scenario: numbers in line but guidance commentary drives action

## Morning Note Workflow

Produce a 2-4 page morning brief for the trading desk or portfolio managers.

**Tools:** `fmp_quote`, `fmp_earnings`, `fmp_sector_performance`, `fmp_gainers_losers`, `fmp_earnings_calendar`

**Workflow:**
1. **Market movers**: call `fmp_quote` for coverage universe watchlist
   - Flag any stock moving >2% pre-market or after-hours
   - Call `fmp_gainers_losers` for broad market movers
2. **Earnings calendar**: call `fmp_earnings_calendar` for upcoming reports this week
   - Highlight coverage names reporting within 5 trading days
   - Note consensus expectations for each
3. **Sector rotation signals**: call `fmp_sector_performance` for sector-level trends
   - Identify sectors with momentum divergence from prior week
   - Flag defensive vs cyclical rotation patterns
4. **Key data releases and catalyst events**:
   - Macro data releases (GDP, CPI, employment) and expected impact
   - Regulatory decisions, FDA approvals, conference presentations
5. **Actionable summary**: 3-5 bullet points with specific trade ideas or risk alerts

## Thesis Tracker Workflow

Produce a 3-5 page investment thesis memo with ongoing tracking framework.

**Tools:** `dcf_model`, `comps_analysis`, `sensitivity_matrix`

**Workflow:**
1. **Bull case** (upside target, key catalysts, probability weight)
   - Identify 3-5 specific catalysts that drive the bull scenario
   - Quantify upside to target price and implied return
   - Assign probability weight (typically 20-30%)
2. **Base case** (current target, core assumptions)
   - Central estimate with most likely growth, margin, and multiple assumptions
   - This is the published price target
   - Assign probability weight (typically 50-60%)
3. **Bear case** (downside target, key risks, probability weight)
   - Identify 3-5 specific risks that drive the bear scenario
   - Quantify downside to target price and implied loss
   - Assign probability weight (typically 15-25%)
4. **Catalyst milestones with dates and expected impact**
   - Earnings dates, product launches, regulatory decisions, contract renewals
   - Expected impact on thesis: confirming, neutral, or challenging
5. **Quarterly update cadence with thesis drift detection**
   - Compare actual results to each scenario's predictions
   - Flag when reality diverges from base case toward bull or bear
   - Trigger re-rating when 2+ catalyst milestones resolve in same direction

## Idea Generation / Screening Workflow

Produce a ranked screening report of investment candidates.

**Tools:** `piotroski_fscore`, `beneish_mscore`, `fmp_ratios`, `fmp_key_metrics`

**Workflow:**
1. **Quantitative screens**: apply hard filters to reduce universe
   - `piotroski_fscore` >= 7 (strong fundamentals)
   - `beneish_mscore` < -1.78 (no manipulation flags)
   - `fmp_ratios` — ROIC > estimated WACC (value creation)
   - `fmp_key_metrics` — FCF yield > 5%, revenue growth > sector median
2. **Thematic filtering**: overlay qualitative criteria
   - Sector trends: identify industries with secular tailwinds
   - TAM growth: addressable market expanding >5% annually
   - Regulatory tailwinds: policy changes that benefit specific sectors
3. **Funnel progression**:
   - Universe (broad index or sector) > Quantitative screen (pass/fail) > Qualitative filter (thematic fit) > Deep dive candidates (top 5-10)
4. **Output**: ranked list with 1-paragraph thesis per idea
   - Each idea includes: ticker, market cap, key metric scores, thesis summary
   - Rank by composite score: fundamental quality + valuation attractiveness + thematic fit

## Sector Overview Workflow

Produce a 10-15 page sector report covering competitive landscape and valuation.

**Tools:** `comps_analysis`, `fmp_sector_performance`, `fmp_profile`, `fmp_key_metrics`

**Workflow:**
1. **Market landscape and competitive positioning**: call `comps_analysis` for full peer group
   - Market share distribution and concentration (HHI)
   - Competitive moats: scale, network effects, switching costs, IP
2. **Sector performance trends**: call `fmp_sector_performance`
   - Absolute and relative performance vs broad market
   - Sector rotation context: early/mid/late cycle positioning
3. **Key players profiling**: call `fmp_profile` and `fmp_key_metrics` for top 5-10 companies
   - Revenue scale, growth trajectory, margin profile, capital returns
   - Management quality and capital allocation track record
4. **Growth drivers, headwinds, regulatory environment**
   - Demand drivers: demographic, technological, policy
   - Headwinds: competition, regulation, cyclicality, input costs
   - Regulatory landscape: current rules and pending changes
5. **Valuation heat map across sector**
   - EV/EBITDA ranges by sub-segment (premium vs discount names)
   - P/E relative to growth (PEG analysis)
   - Historical valuation bands: current vs 5-year average

## Model Update Workflow

Produce an updated model and revision note when material new information arrives.

**Tools:** `three_statement_model`, `dcf_model`, `target_price`, `fmp_income_statement`

**Triggers:**
- New quarterly/annual earnings release
- Guidance change (raised, lowered, or withdrawn)
- M&A announcement (acquirer or target)
- Macro shift (interest rate change, FX move, commodity price shock)

**Workflow:**
1. **Identify changes**: pull latest actuals via `fmp_income_statement`
   - Compare actual results to prior model assumptions
   - Identify which assumptions need revision and direction
2. **Update three-statement model**: call `three_statement_model` with revised assumptions
   - Adjust revenue growth, margins, capex, working capital as warranted
   - Re-solve circular references (interest expense, revolver draws)
3. **Recalculate valuation**: call `dcf_model` and `target_price` with updated projections
   - New DCF value, updated comps-implied value
   - Revised blended price target
4. **Issue revision note**: old vs new estimates with change rationale
   - Side-by-side comparison: prior vs revised for revenue, EBITDA, EPS, target price
   - Rating change if warranted (upgrade/downgrade/maintain)
   - Updated sensitivity tables

## Catalyst Calendar Workflow

Produce an event timeline for the coverage universe.

**Tools:** `fmp_earnings_calendar`, `fmp_ipo_calendar`

**Workflow:**
1. **Earnings dates**: call `fmp_earnings_calendar` for next 90 days
   - Coverage names with confirmed and estimated reporting dates
   - Consensus expectations for each upcoming report
2. **IPO pipeline**: call `fmp_ipo_calendar` for upcoming listings
   - Sector relevance: new entrants that affect competitive dynamics
   - Valuation benchmarking: IPO pricing vs existing coverage comps
3. **Corporate events**: conferences, investor days, analyst meetings
   - Management presentations that may include updated guidance
   - Industry conferences with potential for competitive intelligence
4. **Regulatory and macro events**: policy decisions, data releases
   - Central bank meetings, trade policy announcements
   - Industry-specific regulatory milestones (FDA, FCC, EPA)
5. **Output format**: chronological timeline with event, company, expected impact, and action required

## Quality Standards

All equity research deliverables must meet these standards:

1. **Data integrity**: every financial figure sourced from MCP tool output or explicitly stated user assumption — never LLM-generated
2. **Dual valuation**: at least 2 independent valuation methods for any price target recommendation
3. **Sensitivity testing**: key variables (growth rate, discount rate, terminal multiple) stress-tested via `sensitivity_matrix`
4. **Three scenarios**: bull/base/bear cases with probability weights mandatory for all recommendations
5. **Source attribution**: all data sources cited with specific tool invocation reference
6. **Timeliness**: note the date of all market data and financial statements used
7. **Auditability**: a reader can follow the logic chain from inputs to conclusion and verify each step
