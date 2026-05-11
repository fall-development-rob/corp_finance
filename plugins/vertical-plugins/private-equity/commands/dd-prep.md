# DD Meeting Prep

Prepare for a due-diligence meeting using the DD Meeting Prep section of the `workflow-private-equity` skill.

## What It Does
Builds a structured prep pack ahead of management, customer reference, or expert calls: agenda construction sequenced by workstream (commercial, financial, operational, legal/regulatory), targeted question list per workstream, red-flag list to probe (margin volatility, customer concentration, working-capital swings, covenant headroom), and reconciliation against prior call notes / data-room findings to avoid duplication.

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-private-equity` skill.

## Key Tools
`fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow`, `credit_metrics`, `altman_zscore`, `peer_benchmarking`, `variance_analysis`

## Usage
Provide the target name, meeting type (management / customer / expert), and any prior call notes or red-flag list. The agent returns the agenda, prioritised questions, and a reconciliation memo.
