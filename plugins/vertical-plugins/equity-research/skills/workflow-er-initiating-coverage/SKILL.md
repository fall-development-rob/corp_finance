---
name: "workflow-er-initiating-coverage"
description: |
  WHAT: Five-task pipeline for producing a 30-50 page equity research initiating coverage report — company research, financial modelling (three-statement), multi-method valuation (DCF + comps + SOTP + Monte Carlo), exhibit generation, and full report assembly. Every financial figure must come from a corp-finance-mcp or FMP tool call.
  WHEN: Invoke when initiating research coverage on a new company or sector — generating the first comprehensive published report including an investment rating, price target, bull/base/bear scenarios, and detailed financial model.
---

# Equity Research: Initiating Coverage

## What this skill covers

A five-task pipeline for initiating coverage on a new company. Each task produces a specific deliverable; tasks are executed sequentially with explicit user confirmation between steps. The output is a 30-50 page institutional equity research report meeting sell-side publication standards.

## Core Rules

- Every number from a corp-finance-mcp or FMP tool call — never LLM-generated
- At least 2 independent valuation methods for any price target
- Bull/base/bear scenarios with probability weights mandatory
- Source attribution for every figure

## Task 1 — Company Research

**Output:** 6-8K word research document covering business model, competitive landscape, management, and TAM.

**Tools:**
- `fmp_company_profile` — company description, sector, industry, market cap, employees
- `fmp_income_statement` — 3-5 years historical revenue, margins, earnings
- `fmp_balance_sheet` — asset base, capital structure, working capital
- `fmp_cash_flow` — operating cash flow, capex, free cash flow generation
- `fmp_key_metrics` — revenue per share, debt/equity, ROIC, book value
- `fmp_financial_ratios` — profitability, leverage, efficiency, valuation ratios

**Sections:**
1. Business model and revenue drivers
2. Competitive landscape and market positioning
3. Management track record and capital allocation history
4. Total addressable market sizing (TAM/SAM/SOM)
5. Key risks and competitive threats

## Task 2 — Financial Modelling

**Output:** Integrated 3-statement model summary with 5-year projections.

**Tools:**
- `three_statement_model` — integrated IS/BS/CF with circular reference resolution
- `fmp_income_statement` — 3 years historical data as base
- `fmp_balance_sheet` — historical balance sheet for working capital trends
- `fmp_cash_flow` — historical capex intensity and cash conversion

**Deliverable includes:**
- Revenue build-up by segment or driver
- Margin trajectory with explicit assumptions per line item
- Capex and depreciation schedule
- Working capital assumptions (DSO, DIO, DPO trends)
- Debt schedule and interest expense
- Balance sheet integrity check (A = L + E every period)

## Task 3 — Valuation Analysis

**Output:** Valuation summary with base/bull/bear price targets and full sensitivity analysis.

**Tools:**
- `wacc_calculator` — CAPM-based cost of capital
- `dcf_model` — discounted cash flow with terminal value (both Gordon Growth and exit multiple)
- `comps_analysis` — trading multiples vs 4-6 comparable companies
- `sotp_valuation` — sum-of-the-parts for multi-segment businesses
- `target_price` — blended target from multiple methodologies
- `sensitivity_matrix` — WACC vs terminal growth; exit multiple vs EBITDA growth
- `monte_carlo_dcf` — stochastic valuation; median value and 90% confidence interval

**Rules:**
- Terminal value must be 50-75% of enterprise value — if >80%, extend the forecast period
- Trading comps on EV/EBITDA, P/E, EV/Revenue with median and mean
- Sensitivity tables: WACC ±100bps vs TGR ±50bps (minimum)

## Task 4 — Exhibit Generation

**Output:** Data tables for report inclusion.

**Tools:** FMP historical data endpoints for trend series.

**Tables required:**
- Revenue growth and margin trends (5-year historical + 5-year projected)
- Valuation multiple history (EV/EBITDA, P/E, EV/Revenue)
- Peer comparison table (multiples, growth, margins, returns)
- Free cash flow bridge (EBITDA to FCF walk)
- Capital structure evolution (debt maturity profile, leverage trend)

## Task 5 — Report Assembly

**Output:** Final 30-50 page initiating coverage report.

**Report structure:**
1. Investment Summary (1 page) — rating, price target, key thesis, upside/downside
2. Company Overview (2 pages) — business model, history, competitive position
3. Industry Analysis (2 pages) — TAM, competitive dynamics, secular trends
4. Financial Analysis (3 pages) — historical performance, projection model, key metrics
5. Valuation (3 pages) — DCF, comps, SOTP, sensitivity tables, football-field chart data
6. Risk Factors (1 page) — company, industry, and macro risks ranked by probability and impact
7. Appendix — detailed financial statements, comp sheet, methodology notes

**Prerequisites:** Task 5 requires all prior task outputs. Task 3 requires Task 2 output.

## Output format

Final report includes: investment rating (Buy/Hold/Sell), price target, key metrics table, sensitivity tables, three scenarios with probability weights.

## Quality gates

- [ ] At least 2 valuation methods used for price target
- [ ] Monte Carlo median and 90% CI produced
- [ ] Bull/base/bear scenarios with probability weights summing to 100%
- [ ] Terminal value between 50-75% of enterprise value
- [ ] Balance sheet check: A = L + E in every forecast period
- [ ] All figures cited with tool name and key assumptions

## Related skills

- `workflow-er-earnings-update` — subsequent updates once coverage is initiated
- `workflow-er-thesis-tracker` — ongoing bull/base/bear tracking framework
- `workflow-er-model-update` — model revisions triggered by new information
- `workflow-deal-citation-standards` — citation format for all tool outputs and filings
- `workflow-deal-quality-checklist` — final QC before publishing
