---
name: workflow-wm-client-meeting-prep
description: |
  WHAT: Client review meeting preparation — portfolio performance summary, asset allocation drift analysis, attribution, risk metrics, market outlook commentary, and prior action item status — assembled into a pre-meeting briefing.
  WHEN: Invoke when preparing for a client portfolio review meeting, quarterly check-in, or annual review; when a client asks for a performance summary ahead of a call.
---

# Wealth Management — Client Meeting Prep

## What this skill covers

Pre-meeting briefing for a client portfolio review. Covers current-period performance, benchmark comparison, attribution, risk metrics, market context, and a status check on prior action items. Output is a 3-5 page briefing the advisor reads before the meeting and shares selectively with the client.

## Workflow

### Step 1 — Portfolio performance summary

Compute total return for the review period (MTD, QTD, YTD, ITD):
- Call `risk_adjusted_returns` for time-weighted return, Sharpe ratio, information ratio, Treynor ratio
- Net-of-fee returns alongside gross returns (always report both)
- Benchmark-relative performance: alpha and tracking error

### Step 2 — Asset allocation drift analysis

Document current vs strategic target allocation for each asset class:

| Asset Class | Target (%) | Current (%) | Drift (pp) | Action |
|-------------|-----------|-------------|------------|--------|
| US Large Cap | | | | |
| International Developed | | | | |
| Emerging Markets | | | | |
| Investment Grade Fixed Income | | | | |
| High Yield | | | | |
| Alternatives | | | | |
| Cash | | | | |

Flag any class drifting >5% from strategic target. Recommend rebalancing trade if drift threshold is breached (cross-reference `workflow-wm-portfolio-rebalancing`).

### Step 3 — Attribution analysis

Call `brinson_attribution` for the review period:
- **Allocation effect**: over/underweight impact by sector
- **Selection effect**: specific holding alpha within each sector
- **Interaction effect**: combined impact

Call `factor_attribution` for factor-based decomposition:
- Market (beta), size, value, momentum, quality, low-volatility factor contributions

### Step 4 — Risk metrics

Call `risk_metrics` for the current portfolio:
- Sharpe ratio, Sortino ratio, max drawdown and recovery time
- VaR at 95% and 99% confidence, CVaR (tail risk)
- Concentrated positions: flag any single security >10% of portfolio

Compare to prior-period risk metrics to identify risk drift.

### Step 5 — Market outlook summary

- Current macro themes: rates, inflation, earnings growth, geopolitical
- How current portfolio positioning aligns with or diverges from the macro outlook
- Key risks for the next 90 days
- Sectors or asset classes with tactical tilts to discuss

### Step 6 — Prior action item status

| Action Item | Date Assigned | Status | Notes |
|-------------|--------------|--------|-------|
| | | Completed / In Progress / Deferred | |

Flag any overdue items. Document revised timeline.

### Step 7 — Agenda for meeting

1. Portfolio performance review (10 min)
2. Asset allocation and rebalancing discussion (10 min)
3. Risk and attribution (5 min)
4. Market outlook and positioning (10 min)
5. Financial planning update (15 min — if applicable)
6. New recommendations and action items (10 min)

## Output format

1. **Performance summary** — table with returns by period vs benchmark
2. **Asset allocation table** — current vs target with drift flags
3. **Attribution summary** — allocation, selection, interaction in basis points
4. **Risk metrics table** — Sharpe, VaR, drawdown vs prior period
5. **Market commentary** — 3-5 bullet points
6. **Action item tracker** — prior items + new items from this meeting
7. **Meeting agenda** — time-boxed agenda ready to share

## Quality gates

- [ ] Returns computed for MTD, QTD, YTD, and ITD periods
- [ ] Net-of-fee returns included alongside gross returns
- [ ] Attribution runs cover both Brinson and factor decomposition
- [ ] Allocation drift flagged for any class >5% from target
- [ ] All prior action items have a status
- [ ] Risk metrics include VaR and drawdown, not just Sharpe ratio

## Related skills

- `workflow-wm-portfolio-rebalancing` — if drift threshold is breached in Step 2
- `workflow-wm-tax-loss-harvesting` — if unrealised losses surfaced in Step 2 review
- `workflow-wm-client-report` — for the formal written quarterly report (longer form)
- `workflow-wm-investment-proposal` — for new position recommendations surfaced in Step 5
