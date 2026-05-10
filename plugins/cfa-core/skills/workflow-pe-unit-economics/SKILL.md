---
name: workflow-pe-unit-economics
description: |
  WHAT: Per-unit profitability analysis for portfolio companies — define the unit, decompose revenue and costs to the unit level, compute gross/contribution margin, LTV/CAC for subscription businesses, and model unit economics at scale.
  WHEN: Invoke when analysing a portco's business model economics, when assessing LTV/CAC for subscription or recurring-revenue businesses, when benchmarking unit-level margins against peers, or when the IC needs evidence that the business is sound at the unit level.
---

# PE Unit Economics

## What this skill covers

Decomposition of business profitability at the per-unit level. Covers all business models: transactional, subscription/SaaS, retail/consumer, B2B services, and industrials. The unit is defined by the business model; the framework adapts to each.

## Workflow

### Step 1 — Define the unit

Select the economically meaningful unit for this business:
- Per customer (professional services, SaaS, insurance)
- Per subscriber (media, software, telecom)
- Per transaction (payments, marketplace, e-commerce)
- Per store / location (retail, restaurant, branch)
- Per seat / licence (enterprise software)
- Per unit shipped (manufacturing, consumer goods)

Document the unit definition and confirm it is consistent with how management reports.

### Step 2 — Revenue per unit

- Average revenue per unit (ARPU/ARPC) — current and trended
- Pricing structure: fixed vs variable, bundle vs a-la-carte
- Mix: premium vs standard vs entry-level proportion
- Upsell and cross-sell revenue captured per unit

### Step 3 — Direct costs per unit

- Cost of goods sold (COGS) per unit
- Direct labour per unit
- Materials and delivery cost per unit
- → Gross margin per unit = Revenue per unit − Direct costs per unit

### Step 4 — Contribution margin per unit

Contribution margin = Gross margin − variable operating costs per unit:
- Variable S&M: commissions, co-marketing, channel fees
- Variable customer support: cost per ticket × tickets per unit
- Variable delivery/fulfilment costs

### Step 5 — Customer acquisition metrics (subscription / recurring models)

- **CAC**: total S&M spend ÷ new units acquired in the period
- **LTV**: Contribution margin per unit × (1 ÷ monthly churn rate) for subscriptions; or estimated average lifetime × annual contribution margin for others
- **LTV/CAC ratio**: target >3.0x for healthy unit economics; <1.5x requires urgent attention
- **Payback period**: CAC ÷ monthly contribution margin (target <18 months for growth-stage; <12 months for mature)

### Step 6 — Retention and churn

- **Gross churn**: units lost ÷ beginning units (quarterly or annual)
- **Net churn**: (units lost − new units gained) ÷ beginning units
- **Net revenue retention (NRR)**: (Revenue from existing units at end of period) ÷ (Revenue from same units at start). NRR >100% = expansion from existing units
- Flag if gross churn >15% annually for SaaS; >10% for professional services

### Step 7 — Unit economics at scale

Model contribution margin at 2x and 3x current scale:
- Fixed costs that spread across more units (tech infrastructure, management, compliance)
- Identify the breakeven volume: units required for the contribution to cover allocated fixed costs
- Model the economics at management's plan volume and at 70% of plan (downside)

## Output format

| Metric | Current | 2x Scale | 3x Scale |
|--------|---------|----------|----------|

Separate tables for:
1. P&L per unit (revenue, COGS, gross margin, variable opex, contribution margin)
2. CAC/LTV/payback table (subscription businesses only)
3. Churn and retention analysis
4. Breakeven and scale economics

## Quality gates

- [ ] Unit definition documented and confirmed consistent with management reporting
- [ ] Gross margin and contribution margin clearly distinguished
- [ ] LTV/CAC computed for all recurring-revenue business models
- [ ] Churn analysis distinguishes gross from net churn
- [ ] Breakeven volume computed and compared to current run-rate
- [ ] Scale economics modelled at 2x and 3x current volume

## Related skills

- `workflow-pe-ic-memo` — unit economics feed Section V (Investment Thesis) and Section IV (Financial Analysis)
- `workflow-pe-value-creation-plan` — VCP initiatives that improve unit-level CAC, margin, and NRR
- `workflow-pe-ai-readiness` — AI opportunities that improve unit economics (e.g., support automation → lower cost per ticket)
