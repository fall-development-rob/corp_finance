# One-Pager

Draft a one-page deal summary for distribution to MDs using the `workflow-investment-banking` skill (Strip Profile and Pitch Deck conventions).

## What It Does
Produces a single-page deal summary suitable for senior partner review: (1) target overview (name, sector, geography, business description, ownership), (2) deal terms (transaction type, EV / equity value, consideration, financing, timeline), (3) financial snapshot (3-year history + 2-year projection: revenue, EBITDA, margins, FCF), (4) valuation summary (entry multiple vs peer median, vs precedent transactions, vs LBO floor), (5) recommendation (proceed / pass / conditional with key conditions), (6) key risks and mitigants (3-5 bullets).

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-investment-banking` skill.

## Key Tools
`fmp_profile`, `fmp_key_metrics`, `comps_analysis`, `credit_metrics`, `dcf_model`, `lbo_model`

## Usage
Provide the target (ticker or company name), deal context (transaction type, indicative EV, consideration mix, financing assumption), and intended audience (e.g. group MD, sector head). The agent will pull financials, run the comparable analysis, and assemble the one-pager.

## Output
Single-page markdown summary with five panels in a fixed layout: target overview header, deal terms table, financial snapshot table (3y history + 2y projection), valuation football-field summary (DCF range, comps range, precedents, LBO floor), and recommendation with risk/mitigant bullets. Designed to fit on one printed page.
