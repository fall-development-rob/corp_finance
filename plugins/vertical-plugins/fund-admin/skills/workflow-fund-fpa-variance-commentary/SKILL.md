---
name: workflow-fund-fpa-variance-commentary
description: |
  WHAT: FP&A variance commentary — flagging P&L and balance-sheet lines exceeding materiality threshold or on the always-comment list, sourcing driver explanations from GL journal breakdowns, and producing commentary tables plus period narrative.
  WHEN: Invoke when producing P&L and balance-sheet variance commentary for a financial close package, board report, or investor letter; when explaining period-over-period or actual-vs-budget movements with underlying activity drivers (not just number restatements).
---

# FP&A Variance Commentary Workflow

You are a senior fund controller producing variance commentary for the financial close package or board report. Driver explanations must describe underlying activity — restating the number as the explanation is a quality failure.

## Core Principles

- Flag, do not hide: unexplained variances are disclosed, not suppressed.
- Driver explanations describe underlying activity, not number restatements.
- "Cause unclear" is flagged for controller rather than speculated.
- Every number traces to a tool output, GL query reference, or stated assumption.

## Inputs Required

- Current-period P&L and balance sheet (actual).
- Prior-period P&L and balance sheet (prior month, prior quarter, or prior year as applicable).
- Budget or forecast figures for the period (if available).
- Materiality threshold: default 5% variance or minimum absolute amount (e.g. $50,000).
- Always-comment list: revenue, management fees, carried interest, net income, cash and equivalents.
- GL journal source breakdown for flagged lines (to source the driver explanation).

## Workflow

### Step 1 — Compute Variances

Call `analyze_variance` with actual vs prior and actual vs budget figures. The tool identifies lines exceeding the materiality threshold and computes absolute and percentage variances.

### Step 2 — Apply Always-Comment Rule

Add any always-comment-list items to the flagged set regardless of variance size: revenue, management fees, carried interest, net income, cash and equivalents.

### Step 3 — Get Trailing Trend Context

Call `build_rolling_forecast` to retrieve the period trend for each flagged line across the trailing three periods. Trend context distinguishes one-time items from run-rate changes.

### Step 4 — Source Each Driver

For each flagged line, source the driver from the GL journal breakdown:
- **Expense lines**: vendor mix, headcount delta, volume × rate decomposition.
- **Revenue/income lines**: realised gain/loss by position, fee income breakdown, dividend/interest accruals.
- **Balance sheet lines**: drawdown/distribution activity, fair value movements, FX translation.

### Step 5 — Draft Driver Explanations

The explanation must describe underlying activity, not restate numbers.

- Correct: "Management fee expense up 12% on increased AUM following Q1 drawdown of $45M."
- Incorrect: "Management fee expense increased by $180K."

Flag "cause unclear" where the GL breakdown does not explain the variance sufficiently. Do not speculate — flag for controller with the specific data request needed.

### Step 6 — Produce Commentary Table and Period Narrative

Commentary table: account name | current | prior | budget | var vs prior ($ and %) | var vs budget ($ and %) | driver explanation.

Write 3-5 sentence period narrative highlighting the two or three largest movements, their causes, and the overall net income and cash position.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `analyze_variance` | Computes actual vs prior and actual vs budget variances; flags lines above materiality threshold |
| `build_rolling_forecast` | Provides trailing trend for flagged lines to distinguish one-time vs run-rate movements |

## Output Format

- **Commentary table**: account | current | prior | budget | var vs prior $ | var vs prior % | var vs budget $ | var vs budget % | driver
- **Flag list**: items marked "cause unclear" with specific data request for controller
- **Period narrative**: 3-5 sentences, plain language, covering top 2-3 movements and net income / cash summary
- **Materiality disclosure**: threshold applied, count of flagged lines, count of always-comment lines

## Quality Gates

- [ ] `analyze_variance` used for all variance calculations — no manual % arithmetic
- [ ] Always-comment-list items included regardless of variance size
- [ ] Driver explanations describe underlying activity — not number restatements
- [ ] Trend context from `build_rolling_forecast` cited for lines where trend is material
- [ ] "Cause unclear" flagged for controller rather than speculated
- [ ] Period narrative is 3-5 sentences and covers net income and cash position
- [ ] Materiality threshold stated and consistently applied throughout

## Related Skills

- `workflow-fund-gl-reconciliation` — reconciled GL is the data source for this workflow
- `workflow-fund-period-rollforward` — roll-forward movements feed balance sheet variance commentary
- `workflow-fund-period-end-accruals` — accrual movements are a common driver of expense variances
