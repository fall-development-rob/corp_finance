---
name: workflow-wm-portfolio-rebalancing
description: |
  WHAT: Portfolio rebalancing — asset allocation drift detection, mean-variance or Black-Litterman optimisation, lot-level trade list generation with tax-efficient lot selection, wash-sale check, and pre/post risk comparison.
  WHEN: Invoke when an asset class drifts >3-5% from strategic target; when adding a significant new contribution; when a client requests a rebalance; when optimising the portfolio against updated return expectations.
---

# Wealth Management — Portfolio Rebalancing

## What this skill covers

Full rebalancing workflow: detect allocation drift, generate an optimised target, produce a lot-level trade list with tax-efficient lot selection, and validate the post-rebalance risk profile.

## Workflow

### Step 1 — Document current allocation vs strategic target

| Asset Class | Target (%) | Current ($) | Current (%) | Drift (pp) | Flag |
|-------------|-----------|-------------|-------------|------------|------|
| US Large Cap | | | | | |
| US Mid Cap | | | | | |
| US Small Cap | | | | | |
| International Developed | | | | | |
| Emerging Markets | | | | | |
| Investment Grade Fixed Income | | | | | |
| High Yield | | | | | |
| TIPS | | | | | |
| Real Estate (REITs) | | | | | |
| Commodities | | | | | |
| Alternatives | | | | | |
| Cash and Equivalents | | | | | |

### Step 2 — Drift detection

Flag any asset class where:
- Absolute drift (current % − target %) exceeds ±3%
- Total portfolio drift (sum of absolute deviations ÷ 2) exceeds 5%

Rebalancing is triggered when either condition is met. If no class breaches threshold, consider combining with a new contribution to reduce transaction count.

### Step 3 — Optimise the target allocation

Select approach based on client mandate:

**Option A — Mean-variance optimisation**: call `mean_variance_optimization`
- Efficient frontier construction
- Select portfolio at target Sharpe or target volatility
- Output: optimal weights for each asset class

**Option B — Black-Litterman**: call `black_litterman_portfolio`
- Incorporate advisor views on expected returns
- Output: BL-adjusted weights blending market equilibrium and views

**Option C — Risk parity**: call `risk_parity`
- Equal risk contribution from each asset class
- Output: risk-parity weights (typically over-weights fixed income)

Document the chosen approach and rationale.

### Step 4 — Generate lot-level trade list

For each required trade (buy or sell):
- Identify specific tax lots to use (specific identification method, not FIFO/LIFO)
- Prioritise lots for selling in this order:
  1. Lots with unrealised losses (harvest loss while rebalancing)
  2. Lots with long-term gains (lower tax rate than short-term)
  3. Lots with short-term gains (sell last — highest tax cost)
- Combine rebalance trades with new contributions to minimise transaction count

| Security | Action | Lots | Units | Cost Basis | Current Value | Gain/(Loss) | Holding Period |
|----------|--------|------|-------|-----------|---------------|-------------|---------------|

### Step 5 — Tax impact assessment

- Total realised gains: short-term and long-term separately
- Total realised losses: short-term and long-term separately
- Net taxable impact of the rebalance
- Estimated tax cost at client's marginal rates
- Losses harvested during rebalance (reference `workflow-wm-tax-loss-harvesting` if dedicated TLH pass needed)
- Confirm no wash-sale violations (30-day rule across all household accounts)

### Step 6 — Post-rebalance risk validation

Call `risk_metrics` on proposed (post-rebalance) portfolio:
- Compare Sharpe ratio, VaR, max drawdown before and after
- Verify proposed portfolio meets client's stated risk tolerance constraints
- Flag any concentrated single-security positions (>10% of portfolio)
- Confirm portfolio-level beta, duration, and sector concentration are within policy limits

## Output format

1. **Drift analysis table** — current vs target with flags
2. **Optimised target weights** — with methodology note
3. **Trade list** — lot-level detail with tax impact per trade
4. **Tax impact summary** — net gain/loss, short-term vs long-term, estimated tax cost
5. **Pre/post risk comparison** — Sharpe, VaR, drawdown side by side

## Quality gates

- [ ] Drift detected using both absolute (3%) and portfolio-level (5%) thresholds
- [ ] Optimisation methodology documented with rationale for approach chosen
- [ ] Lot selection uses specific identification (not FIFO)
- [ ] Tax-lot priority order followed: losses first, then long-term gains, then short-term gains
- [ ] Wash-sale compliance checked across all household accounts
- [ ] `risk_metrics` run on post-rebalance portfolio before trade list is finalised

## Related skills

- `workflow-wm-tax-loss-harvesting` — dedicated TLH pass if losses are material
- `workflow-wm-client-meeting-prep` — rebalance recommendation discussed in client meeting
- `workflow-wm-financial-planning` — strategic target allocation set in the financial plan
