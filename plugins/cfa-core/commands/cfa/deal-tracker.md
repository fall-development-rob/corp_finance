# Deal Tracker

Produce a live deal-tracking dashboard using the Deal Tracker workflow from `workflow-investment-banking`.

## What It Does
Generates an internal pipeline tracking document for active sell-side or buy-side mandates: (1) pipeline-stage view (Engagement, Marketing, First Round, Management Meetings, Final Bids, Signing, Closing), (2) per-deal record (deal name, sector, target EBITDA, expected EV range, current stage, days in stage, next milestone date, key contacts, status notes, blockers), (3) materials-status flag (CIM, teaser, model, IC memo: drafted / reviewed / distributed), (4) last-touch and next-touch fields per counterparty.

## Agent
Routes to `cfa-private-markets-analyst` with `workflow-investment-banking` skill.

## Key Tools
None required (tracking document; no computation).

## Usage
Provide either an existing deal log to refresh or the active mandate list (deal name, stage, counterparty, last update). Optionally specify scope filters (sector, stage, MD owner).

## Output
Markdown dashboard containing: top-line pipeline summary table (stage, deal count, aggregate EV), per-deal detail table (one row per active deal with all tracking fields), materials-status matrix (deal x deliverable), and exception list (deals stalled in stage > N days, missed milestones, open blockers).
