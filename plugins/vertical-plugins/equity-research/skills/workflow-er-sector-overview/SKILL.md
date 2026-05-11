---
name: "workflow-er-sector-overview"
description: |
  WHAT: 10-15 page sector overview report covering competitive landscape and market structure (HHI), sector performance trends, key player profiling, growth/headwind/regulatory environment analysis, and a valuation heat map (EV/EBITDA ranges, PEG analysis, historical bands). Produced using FMP sector data and corp-finance-mcp comps tools.
  WHEN: Invoke when initiating sector coverage, preparing a sector rotation recommendation, or responding to a mandate such as "give me a sector overview of [industry]" or "where are the best opportunities in [sector]?"
---

# Equity Research: Sector Overview

## What this skill covers

A comprehensive 10-15 page sector report covering market structure, competitive dynamics, performance trends, key player profiles, growth and risk drivers, and a full valuation heat map. Provides the thematic context that underpins stock-specific recommendations.

## Inputs

- Sector or industry name (e.g., "US Large-Cap Technology", "European Financials")
- Peer set: top 5-10 companies by market cap (or user-specified list)
- Coverage scope: full sector report or targeted sub-segment

## Workflow

### Step 1 — Market landscape and competitive structure

Call `comps_analysis` for the full peer group:
- Market share distribution by revenue (or AUM for financials)
- Herfindahl-Hirschman Index (HHI) to classify concentration: <1,500 (fragmented), 1,500-2,500 (moderate), >2,500 (concentrated)
- Competitive moats analysis: scale economies, network effects, switching costs, IP/patents, regulatory barriers
- Recent market share shifts (who is gaining vs losing and why)

### Step 2 — Sector performance trends

Call `fmp_sector_performance` for historical data:
- Absolute sector total return (1M, 3M, 6M, 1Y, 3Y)
- Relative performance vs broad market benchmark (S&P 500 or regional equivalent)
- Sector rotation context: early/mid/late economic cycle positioning
- Beta relative to market: cyclical (beta >1) or defensive (beta <1)

### Step 3 — Key player profiling

For each of the top 5-10 companies, call `fmp_company_profile` and `fmp_key_metrics`:
- Revenue scale and 3-year CAGR
- EBITDA margin and margin trajectory
- ROIC and capital allocation track record
- Management tenure and recent strategic decisions
- Valuation: EV/EBITDA, P/E, EV/Revenue at current prices

Present as a comparative table with all metrics standardised.

### Step 4 — Growth drivers, headwinds, and regulatory environment

Structured analysis of the sector's forward outlook:

**Demand drivers:**
- Demographic tailwinds (ageing populations, urbanisation, workforce trends)
- Technology adoption curves and disruption risk
- Policy-driven demand (infrastructure spend, energy transition, defence)

**Headwinds:**
- Input cost sensitivity (labour, commodities, energy)
- Competitive intensity: new entrants, pricing pressure, substitution risk
- Cyclicality: revenue volatility vs GDP; operating leverage

**Regulatory landscape:**
- Current regulatory framework and pending changes
- Sector-specific risks: antitrust, environmental, consumer protection
- Regulatory tailwinds: subsidies, procurement, mandated adoption

### Step 5 — Valuation heat map

Produce a valuation map across the sector:

**EV/EBITDA ranges by sub-segment:**
- Premium names (structural growers, moat leaders): EV/EBITDA range
- Core / mid-tier: EV/EBITDA range
- Value / discount names (cyclical, challenged): EV/EBITDA range

**PEG analysis (P/E vs growth):**
- Call `comps_analysis` for P/E and consensus growth rates
- PEG < 1.0: potentially undervalued relative to growth; PEG > 2.0: expensive relative to growth

**Historical valuation bands:**
- Current EV/EBITDA vs 5-year average and standard deviation
- Premium/discount to historical mean: is the sector cheap or expensive in context?

## Report structure (10-15 pages)

1. Sector snapshot (market cap, revenue, key growth rate — 1 page)
2. Competitive landscape (HHI, moat classification, market share table — 2 pages)
3. Performance trends (sector vs market, rotation context — 1 page)
4. Key player profiles (comparative table — 2-3 pages)
5. Growth drivers and headwinds (structured analysis — 2 pages)
6. Regulatory environment (current + pending — 1-2 pages)
7. Valuation heat map (EV/EBITDA by tier, PEG, historical bands — 2 pages)
8. Investment conclusions (top picks and avoids with one-line rationale)

## Output format

- 10-15 pages (markdown with structured tables)
- All financial metrics sourced from FMP tools with as-of date
- Comps and valuation from `comps_analysis` with tool citation

## Quality gates

- [ ] HHI computed from market share data — not estimated
- [ ] Sector performance from `fmp_sector_performance` with date
- [ ] Key player metrics from `fmp_key_metrics` — standardised to the same period
- [ ] Historical valuation bands include at least 3 years of data
- [ ] Valuation heat map covers all three tiers (premium / core / value)

## Related skills

- `workflow-er-initiating-coverage` — single-stock deep dives within the sector context
- `workflow-er-idea-screening` — quantitative screening of sector names for investment candidates
- `workflow-er-morning-note` — sector rotation signals used in morning briefs
