---
name: workflow-wm-tax-loss-harvesting
description: |
  WHAT: Tax-loss harvesting — scan taxable holdings for unrealised losses, assess gain/loss budget, identify and rank actionable candidates, select replacement securities maintaining market exposure, verify wash-sale compliance across all household accounts, and document lot-level detail for tax reporting.
  WHEN: Invoke when reviewing a taxable account for TLH opportunities; when a client has realised gains that should be offset; when a market drawdown creates harvesting opportunities; when year-end tax planning requires a loss review.
---

# Wealth Management — Tax-Loss Harvesting

## What this skill covers

Systematic identification and execution of tax-loss harvesting opportunities in taxable accounts. Covers loss scanning, gain/loss budget assessment, candidate ranking, replacement security selection, wash-sale compliance, and tax reporting documentation.

## Core rules

- **Wash-sale rule**: the same or substantially identical security cannot be repurchased within 30 days before or after the sale in any household account (taxable, IRA, Roth IRA, 401(k), DRIP)
- **Short-term losses offset short-term gains first** (both taxed at ordinary income rates); **long-term losses offset long-term gains first**
- Remaining net losses offset the other category; any net capital loss after offsetting gains reduces ordinary income up to $3,000/year; excess carries forward

## Workflow

### Step 1 — Scan holdings for unrealised losses

Review all taxable account positions:

| Security | Lot Date | Cost Basis | Current Value | Unrealised Gain/(Loss) | Holding Period |
|----------|----------|-----------|---------------|----------------------|---------------|
| | | | | | Short-term / Long-term |

Separate short-term (<1 year) from long-term (>1 year) losses. Short-term losses are more valuable because they offset income taxed at ordinary rates.

### Step 2 — Gain/loss budget assessment

| Category | Amount |
|----------|--------|
| Realised short-term gains YTD | |
| Realised long-term gains YTD | |
| Carryforward losses (short-term) | |
| Carryforward losses (long-term) | |
| Ordinary income offset capacity remaining | up to $3,000 |

Priority order for harvested losses:
1. Offset short-term gains (highest-value use)
2. Offset long-term gains
3. Offset ordinary income (up to $3,000)
4. Add to loss carryforward for future years

### Step 3 — Candidate identification

Filter for actionable harvesting opportunities:
- Unrealised loss exceeds $1,000 absolute OR >5% of position value
- Position is large enough to justify transaction cost (commission + spread)
- Holding period considered: short-term losses prioritised over long-term
- Fundamental view: would the client continue to hold this security? (If yes — sell and replace; if no — this is an exit, not a harvest)

Call `risk_metrics` on each candidate to assess volatility and correlation for replacement screening.

### Step 4 — Replacement security selection

Select a replacement that:
- Maintains market exposure (same asset class, similar beta, comparable sector)
- Is NOT substantially identical to the sold security (avoid wash sale)
- Common approaches:
  - Individual stock → sector ETF for that industry
  - One broad index fund → a comparable index tracking a different index
  - Bond fund → comparable duration/quality fund with different issuer composition

Verify replacement with `risk_metrics`: correlation to sold security should be >0.80 to maintain exposure, but instruments must not be substantially identical.

Document each replacement:
| Sold Security | Replacement | Rationale | Correlation | Substantially Identical? |
|--------------|-------------|-----------|------------|------------------------|
| | | | | No |

### Step 5 — Wash-sale compliance verification

For EVERY proposed harvest:
- Check 30 days before the sale date: was this security (or any substantially identical) purchased?
- Check 30 days after the planned sale: are there any pending or automatic purchases (DRIPs, 401(k) contributions)?
- Check ALL household accounts: taxable, IRA, Roth IRA, 401(k), 403(b), 529

Flag any wash-sale risk. If a wash sale is identified, either defer the harvest 31 days or exclude that lot from the plan.

### Step 6 — Priority-ranked execution plan

Rank harvesting trades by tax benefit (largest first), then by holding period (short-term first):

| Priority | Security | Lots | Units | Loss Amount | Replacement | Tax Benefit at Marginal Rate |
|----------|----------|------|-------|------------|-------------|------------------------------|

Total estimated tax benefit: sum of (loss amount × applicable tax rate) for each trade.

### Step 7 — Lot-level documentation

For each executed harvest, record:
- Security sold: name, CUSIP, lot date, cost basis, sale price, loss realised
- Replacement purchased: name, CUSIP, purchase date, purchase price
- Holding period of the loss: short-term / long-term
- Wash-sale clearance date: 30 days after sale (replacement cannot be sold before this date either)
- Cost basis method: specific identification

Estimated tax savings = loss amount × client's marginal tax rate (short-term or long-term as applicable).

## Output format

1. **Unrealised loss scan** — all positions with losses >$500
2. **Gain/loss budget** — YTD realised gains, carryforwards, remaining capacity
3. **Candidate list** — ranked by tax benefit with fundamentals note
4. **Replacement security table** — with correlation and wash-sale clearance
5. **Execution plan** — priority-ranked trades with estimated tax benefit
6. **Wash-sale compliance log** — per-trade clearance documentation

## Quality gates

- [ ] ALL household accounts checked for wash-sale compliance (not just taxable account)
- [ ] Short-term vs long-term losses separated and offset against appropriate gain type
- [ ] Replacement securities verified as not substantially identical to sold securities
- [ ] DRIP and 401(k) automatic contributions checked for 30-day window
- [ ] Specific identification method documented for each lot
- [ ] Total estimated tax benefit computed and reported to client

## Related skills

- `workflow-wm-portfolio-rebalancing` — coordinate with rebalancing to harvest losses while rebalancing
- `workflow-wm-financial-planning` — annual TLH strategy defined in the tax optimisation section
- `workflow-wm-client-meeting-prep` — TLH opportunities surfaced during client review meetings
