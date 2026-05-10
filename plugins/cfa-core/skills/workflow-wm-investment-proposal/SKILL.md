---
name: workflow-wm-investment-proposal
description: |
  WHAT: Investment proposal for a wealth management client — opportunity overview, expected return analysis with DCF and comps, risk assessment, portfolio fit, comparison to alternatives, and a clear conviction-rated recommendation with position sizing.
  WHEN: Invoke when recommending a new investment to an existing client; when a client asks for analysis of a specific security or strategy; when producing a formal written proposal for review before execution.
---

# Wealth Management — Investment Proposal

## What this skill covers

A structured investment proposal tailored to a specific client: thesis, quantitative return analysis, risk assessment, portfolio fit, and a clear conviction-rated recommendation with position size and funding source. Output is 3-5 pages, client-appropriate language.

## Core principles

- **Suitability first**: confirm the investment is appropriate for this client's risk tolerance, time horizon, and objectives before producing the analysis
- **Three scenarios**: base, bull, and bear return cases with explicit probability weights
- **Portfolio impact**: show how adding this position changes portfolio-level metrics, not just the security in isolation
- **Fiduciary standard**: conflicts of interest (if any) must be disclosed

## Workflow

### Step 1 — Suitability check

Before analysis, verify:
- Risk tolerance: is the investment's expected volatility within the client's stated tolerance?
- Time horizon: is the expected holding period consistent with the client's liquidity needs?
- Concentration: will this position create a single-name or sector overweight beyond policy limits?
- Regulatory: any restrictions (client-imposed, SMA mandate, ESG exclusion list)?

Document the suitability assessment. If any check fails, do not proceed without advisor sign-off.

### Step 2 — Opportunity overview

- Investment thesis: 2-3 sentences explaining what the opportunity is and why it is attractive now
- Catalyst: what specific event or trend will drive returns and over what timeframe?
- Fit: why this investment is suitable for this client specifically (not just generically attractive)
- Suggested holding period and expected exit/review trigger

### Step 3 — Expected return analysis

**For equities with financial statements:**
- Call `dcf_model` for intrinsic value estimate using analyst assumptions
- Call `comps_analysis` for relative valuation vs 4-6 comparable peers (EV/EBITDA, P/E, EV/Revenue)
- Implied upside: (intrinsic value − current price) / current price

**Scenario analysis:**

| Scenario | Probability | Key Assumption | Expected Return | Expected Holding Period |
|----------|-------------|---------------|-----------------|------------------------|
| Bull | 25% | | | |
| Base | 50% | | | |
| Bear | 25% | | | |

Probability-weighted expected return = Σ (probability × return).

**For alternatives, funds, or structured products:** document expected return basis (historical, manager projection, stress-test) with explicit limitations.

### Step 4 — Risk assessment

Call `risk_metrics` on the security (or proxy) for historical volatility, maximum drawdown, and correlation to benchmark.

Call `sensitivity_matrix` varying 2-3 key assumptions:
- For equities: revenue growth rate vs discount rate (WACC)
- For bonds: yield vs credit spread
- For alternatives: expected return vs volatility

| Risk | Category | Severity | Mitigant |
|------|----------|---------|----------|
| | Market / credit / liquidity / operational | High / Medium / Low | |

**Downside scenario**: maximum expected loss in a severe adverse outcome (3-sigma event). What would need to happen, and how does the client's overall portfolio behave in that scenario?

### Step 5 — Portfolio fit

If added at proposed position size (X% of portfolio):
- How does total portfolio asset allocation change?
- Correlation with top 3 existing holdings (does it diversify or concentrate)?
- Impact on portfolio Sharpe ratio, VaR, and max drawdown
- Does any single-security position exceed 10% of portfolio after addition?
- Does any sector weighting exceed 25% of portfolio after addition?

Call `risk_metrics` on the combined (current + proposed) portfolio to quantify changes.

### Step 6 — Comparison to alternatives

Consider 2-3 alternative investments that serve the same client need:

| Alternative | Expected Return | Risk | Liquidity | Tax Efficiency | Rationale for/against |
|-------------|----------------|------|----------|---------------|----------------------|

Explain why the proposed investment is preferred: superior risk-adjusted return, better liquidity, lower tax cost, or stronger thesis conviction.

State explicitly what would cause the advisor to prefer an alternative.

### Step 7 — Recommendation

- **Conviction level**: High / Moderate / Low (with rationale)
- **Suggested position size**: % of total portfolio (e.g., 2-3%)
- **Funding source**: which position(s) to trim to fund the purchase; preferred lot selection for tax efficiency
- **Entry strategy**: immediate full position vs scaling in over 2-3 tranches
- **Review trigger**: what specific event, price level, or time horizon would prompt reassessment or exit

## Output format

1. **Suitability confirmation** — documented in the advisor file, not necessarily in the client copy
2. **Opportunity overview** — thesis, catalyst, fit (1 page)
3. **Return analysis** — valuation, scenario table, probability-weighted return (1-2 pages)
4. **Risk assessment** — risk table, sensitivity grid, downside scenario (1 page)
5. **Portfolio fit** — allocation change, diversification impact (half page)
6. **Alternatives considered** — comparison table with rationale (half page)
7. **Recommendation** — conviction, position size, funding source, entry, trigger (half page)

## Quality gates

- [ ] Suitability check completed and documented before analysis begins
- [ ] At least three scenarios (bull/base/bear) with explicit probability weights
- [ ] Probability-weighted expected return computed
- [ ] `risk_metrics` run on the combined portfolio (not just the individual security)
- [ ] At least two alternatives compared with clear rationale for preference
- [ ] Conviction level, position size, funding source, and exit trigger all specified

## Related skills

- `workflow-wm-portfolio-rebalancing` — execute portfolio changes once proposal is accepted
- `workflow-wm-tax-loss-harvesting` — coordinate funding source selection with tax efficiency
- `workflow-wm-client-meeting-prep` — investment proposals surfaced in client review meetings
