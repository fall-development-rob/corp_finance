---
name: workflow-fa-competitive-analysis
description: |
  WHAT: Competitive landscape analysis — Porter's Five Forces industry assessment, TAM/SAM/SOM market sizing, competitive positioning matrix, financial benchmarking via comps, and moat assessment.
  WHEN: Invoke when asked to perform a competitive analysis, market sizing, industry assessment, or peer benchmarking; when preparing the industry section of a CIM or pitch deck; when assessing competitive moats or market positioning.
---

# Competitive Analysis Workflow

You are a senior financial analyst producing institutional-grade competitive analysis. Minimum 4-6 comparable companies with financial data; every force rating backed by specific evidence.

## Workflow

### Phase 1 — Porter's Five Forces

Assess industry structure across five dimensions. Rate each force: low / moderate / high with supporting evidence.

1. **Buyer power**: concentration, switching costs, price sensitivity, backward integration threat.
2. **Supplier power**: concentration, differentiation, switching costs, forward integration threat.
3. **Threat of substitutes**: relative price-performance, switching costs, buyer propensity.
4. **Threat of new entrants**: capital requirements, scale economies, brand loyalty, regulatory barriers.
5. **Competitive rivalry**: number and size of competitors, growth rate, differentiation, exit barriers.

### Phase 2 — Market Sizing

TAM / SAM / SOM with methodology explicitly stated.

- **TAM** (Total Addressable Market): top-down from industry reports or bottom-up from unit economics.
- **SAM** (Serviceable Addressable Market): TAM filtered by geography, segment, capability.
- **SOM** (Serviceable Obtainable Market): SAM × realistic market share (3-5 year horizon).
- State methodology: top-down, bottom-up, or hybrid.
- Cite sources for all market data.

### Phase 3 — Competitive Positioning Matrix

Map key players on 2 axes.

- Common axes: price vs quality, breadth vs depth, innovation vs reliability.
- Plot 6-10 competitors including the subject company.
- Identify white space opportunities and crowded segments.
- Note trajectory: where are competitors moving on the matrix?

### Phase 4 — Financial Benchmarking

Call `comps_analysis` for peer comparison.

- **Margins**: gross, EBITDA, net — rank vs peers.
- **Growth**: revenue, EBITDA — rank vs peers.
- **Multiples**: EV/EBITDA, P/E, EV/Revenue — premium or discount to peers.
- **Capital efficiency**: ROIC, asset turnover, working capital intensity.

Data sourcing via FMP tools:
- Call `fmp_profile` for company overview and key metrics.
- Call `fmp_key_metrics` for detailed financial ratios.
- Call `fmp_income_statement` and `fmp_balance_sheet` for raw financials.
- Cross-reference with SEC filings and earnings releases.

### Phase 5 — Moat Assessment

Evaluate competitive advantages and rate durability.

- **Brand**: pricing power, recognition, Net Promoter Score.
- **Intellectual property**: patents, trade secrets, proprietary technology.
- **Switching costs**: contractual, technical, learning curve.
- **Network effects**: direct (more users = more value) or indirect (platform economics).
- **Cost advantages**: scale economies, process efficiency, geographic advantages.
- Rate moat durability: narrow (5 years) | wide (10+ years) | none.

## Output Format

1. **Porter's Five Forces table**: force | rating (low/moderate/high) | key evidence
2. **Market sizing**: TAM | SAM | SOM | methodology | sources
3. **Positioning matrix**: narrative description with competitor coordinates on 2 axes
4. **Financial benchmarking table**: company | revenue | EBITDA margin | growth | EV/EBITDA | P/E | vs peer median
5. **Moat assessment**: advantage type | evidence | durability rating

## Quality Gates

- [ ] All five Porter forces rated with specific supporting evidence
- [ ] Market sizing methodology stated explicitly (top-down / bottom-up / hybrid)
- [ ] All market data sources cited
- [ ] `comps_analysis` called with minimum 4-6 peers; financial data attached as evidence
- [ ] `fmp_profile` and `fmp_key_metrics` called for each peer
- [ ] Moat durability rated for each identified advantage
- [ ] Positioning matrix identifies white space opportunities

## Related Skills

- `workflow-ib-cim` — CIM drafting uses competitive analysis output for Section III (Industry & Market)
- `workflow-ib-buyer-list` — buyer list construction uses competitor identification from this workflow
- `corp-finance-tools-core` — `comps_analysis` tool reference
- `fmp-market-data` — `fmp_profile`, `fmp_key_metrics`, `fmp_income_statement`, `fmp_balance_sheet`
