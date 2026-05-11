---
name: "workflow-er-morning-note"
description: |
  WHAT: 2-4 page morning brief for the trading desk or portfolio managers — coverage universe movers, earnings calendar, sector rotation signals, macro catalyst events, and 3-5 actionable trade ideas or risk alerts. Produced at market open or pre-open using live FMP data.
  WHEN: Invoke to produce a daily or event-driven morning note for a coverage universe. Typically generated pre-market open or immediately after a significant overnight event (earnings release, macro data, geopolitical event).
---

# Equity Research: Morning Note

## What this skill covers

A rapid-turnaround 2-4 page morning brief that synthesises overnight and pre-market information across a coverage universe into actionable trade ideas and risk alerts. Relies exclusively on live market data tools — no LLM-generated price levels or estimates.

## Inputs

- Coverage universe watchlist (list of tickers)
- Date and relevant overnight events (optional — if omitted, the workflow discovers them via tools)
- Focus area if applicable (e.g., "focus on tech earnings", "focus on rates impact")

## Workflow

### Step 1 — Coverage universe movers

Call `fmp_batch_quote` for the full coverage universe watchlist:
- Flag any stock moving >2% pre-market or after-hours
- Note price, volume, and change driver for flagged names
- Call `fmp_biggest_gainers` and `fmp_biggest_losers` for broad market movers beyond the coverage universe

### Step 2 — Earnings calendar

Call `fmp_earnings_calendar` for the current week:
- Highlight coverage names reporting within the next 5 trading days
- Note consensus estimates (EPS, revenue) for each upcoming report
- Flag names where the estimate revision trend has shifted in the past 7 days

### Step 3 — Sector rotation signals

Call `fmp_sector_performance` for sector-level trends:
- Identify sectors with momentum divergence from the prior week
- Flag defensive vs cyclical rotation patterns
- Note sectors approaching 52-week highs or lows

### Step 4 — Key data releases and catalyst events

Identify upcoming macro and regulatory events:
- Central bank meetings, CPI/PPI releases, employment data — note consensus and expected market impact
- FDA approvals, FCC decisions, EPA rulings relevant to coverage
- Management conferences, investor days, analyst meetings scheduled this week

### Step 5 — Actionable summary

Produce 3-5 bullet points with specific trade ideas or risk alerts:
- Each bullet: ticker, action (buy the dip / reduce / avoid / monitor), rationale, and entry level or stop
- Distinguish between: event-driven (catalyst in <5 days), technical (price/volume signal), and thesis-driven (new fundamental information)
- Note if any idea conflicts with current coverage rating (flag for compliance review)

## Report structure

1. **Market movers** — coverage universe pre-market moves >2%; broad market movers
2. **Earnings calendar** — this week's coverage reports with consensus and estimate trend
3. **Sector rotation** — sector performance table, rotation signal
4. **Catalyst events** — macro releases and regulatory events for the week
5. **Actionable ideas** — 3-5 bullets with ticker, action, rationale

## Output format

- 2-4 pages (markdown or formatted brief)
- All price data sourced from `fmp_batch_quote` with as-of timestamp
- Sector data sourced from `fmp_sector_performance` with as-of date
- No manually entered price levels

## Quality gates

- [ ] All price and quote data from `fmp_batch_quote` or `fmp_quote` — with as-of timestamp
- [ ] Earnings calendar sourced from `fmp_earnings_calendar`
- [ ] Sector data sourced from `fmp_sector_performance`
- [ ] Actionable ideas limited to 3-5 — not a laundry list
- [ ] Any idea conflicting with coverage rating flagged for compliance review
- [ ] Total note length: 2-4 pages — brevity is the standard

## Related skills

- `workflow-er-earnings-update` — deeper analysis when a coverage name has just reported
- `workflow-er-thesis-tracker` — context for thesis-driven trade ideas in the actionable summary
- `workflow-er-idea-screening` — quantitative screening that can surface new ideas for inclusion
