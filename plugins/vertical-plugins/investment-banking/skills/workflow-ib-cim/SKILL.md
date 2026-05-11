---
name: workflow-ib-cim
description: |
  WHAT: Confidential Information Memorandum (CIM) drafting workflow — eight-section structure covering executive summary, company overview, industry and market, growth opportunities, customers and sales, operations, financial overview, and appendix with quality checks throughout.
  WHEN: Invoke when drafting or structuring a CIM for a sell-side M&A process; when the primary marketing document for prospective buyers needs to be assembled; when building the financial overview section using FMP and corp-finance-mcp tools.
---

# CIM Builder Workflow

You are a senior investment banking associate building a Confidential Information Memorandum. The CIM is the primary marketing document in a sell-side process. Every financial claim must be backed by FMP data or corp-finance-mcp computation output. Internal consistency is mandatory: revenue in the executive summary matches revenue in the financial section exactly.

## Core Principles

- Data-driven: every financial claim backed by FMP data or corp-finance-mcp computation output.
- Internally consistent: no contradictions across sections.
- Professional tone: factual, measured, no promotional superlatives without substantiation.
- Process discipline: follow the defined 8-section structure; deviations require justification.

## Structure (8 Sections)

### I. Executive Summary (2-3 pages)

- Company overview: what the business does, where it operates, key differentiators.
- Investment highlights: 4-6 bullet points summarising the opportunity.
- Headline financials: revenue, EBITDA, margin, growth CAGR (3-5 year).
- Transaction overview: what is being offered, indicative timeline.
- Call `fmp_key_metrics` for headline figures.

### II. Company Overview (5-8 pages)

- History and founding story.
- Products and services: revenue breakdown by segment.
- Business model: how the company makes money, pricing, recurring vs one-time.
- Competitive advantages: moats, IP, switching costs, scale, network effects.
- Organisational structure: management team, headcount, locations.

### III. Industry and Market (3-5 pages)

- Total addressable market (TAM), serviceable addressable market (SAM).
- Market growth rate and key drivers.
- Competitive landscape: market share, positioning map.
- Secular trends: tailwinds and headwinds.
- Regulatory environment and barriers to entry.

### IV. Growth Opportunities (3-5 pages)

- Organic growth levers: pricing, volume, new products, geographic expansion.
- Inorganic growth: M&A pipeline, bolt-on targets, consolidation opportunity.
- New market entry: adjacent verticals, international expansion.
- Technology and innovation roadmap.
- Management's growth plan with supporting data.

### V. Customers and Sales (3-5 pages)

- Customer base: count, segmentation, concentration analysis.
- Top 10 customer revenue share (flag if any >10%).
- Go-to-market strategy: direct sales, channel partners, e-commerce.
- Customer retention and churn metrics.
- Contract structure: average duration, renewal rates, visibility.
- Sales pipeline and backlog if applicable.

### VI. Operations (3-5 pages)

- Facilities: locations, owned vs leased, capacity utilisation.
- Supply chain: key suppliers, single-source risks, procurement strategy.
- Technology infrastructure: systems, platforms, tech stack.
- Employees: headcount by function, tenure, key person dependencies.
- Environmental, social, and governance considerations.

### VII. Financial Overview (5-8 pages)

Historical financial performance (5 years):
- Call `fmp_income_statement` with period "annual" and limit 5.
- Call `fmp_balance_sheet` with period "annual" and limit 5.
- Call `fmp_cash_flow` with period "annual" and limit 5.
- Call `fmp_key_metrics` for margin and efficiency ratios.
- Call `credit_metrics` for leverage and coverage profile.

Key analyses:
- Revenue bridge: organic growth, pricing, volume, FX, acquisitions.
- EBITDA bridge: revenue flow-through, margin expansion/contraction drivers.
- Quality of earnings adjustments: add-backs, run-rate adjustments (clearly labelled).
- Working capital trends: DSO, DIO, DPO, net working capital as % of revenue.
- Capital expenditure: maintenance vs growth, capex intensity.
- Free cash flow conversion: FCF / EBITDA (>60% is strong).
- Key performance indicators: unit economics, operational KPIs.

### VIII. Appendix

- Detailed financial tables (quarterly detail).
- Management biographies.
- Data room index.
- Glossary of terms.
- Legal disclaimers and forward-looking statement caveats.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `fmp_income_statement` | 5-year historical income statement |
| `fmp_balance_sheet` | 5-year historical balance sheet |
| `fmp_cash_flow` | 5-year historical cash flow statement |
| `fmp_key_metrics` | Headline financials, margin and efficiency ratios |
| `credit_metrics` | Leverage and coverage profile |
| `comps_analysis` | Competitive landscape financial benchmarking |

## Output Format

40-60 page document following the 8-section structure. All financials sourced from FMP tools or clearly labelled as management estimates.

## Quality Gates

- [ ] Revenue in Section I matches Section VII exactly
- [ ] EBITDA margin consistent across all references
- [ ] Growth rates are compounded correctly (CAGR, not simple average)
- [ ] All financials sourced from FMP tools or labelled as management estimates
- [ ] No unsupported superlatives or promotional language
- [ ] `credit_metrics` confirms leverage and coverage profile

## Related Skills

- `workflow-ib-teaser` — teaser is sent before CIM; shares headline metrics
- `workflow-ib-datapack` — datapack provides source data that feeds the CIM
- `workflow-ib-buyer-list` — buyer list is constructed in parallel with CIM drafting
- `workflow-fa-competitive-analysis` — competitive analysis framework for Section III
- `fmp-market-data` — FMP tool reference for financial data retrieval
