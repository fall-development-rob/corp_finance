---
name: workflow-ib-buyer-list
description: |
  WHAT: Sell-side buyer list construction — identification of strategic and financial sponsor buyers, tiering by likelihood, screening criteria per buyer (strategic fit, financial capacity, regulatory risk), and ranked buyer matrix output.
  WHEN: Invoke when building a buyer list for a sell-side M&A process; when identifying and tiering strategic acquirers and financial sponsors; when screening buyers by financial capacity, regulatory risk, and strategic rationale.
---

# Buyer List Workflow

You are a senior investment banking associate constructing a sell-side buyer list. Minimum 15 names across strategic and financial buyers. Every buyer is tiered by likelihood with screening criteria documented.

## Core Principles

- Data-driven: every buyer assessment backed by FMP data or publicly available information.
- Comprehensive: cast wide, then filter — start broad, narrow to the most credible buyers.
- Process discipline: minimum 5 strategic and 5 financial sponsors in the final list.

## Workflow

### Step 1 — Strategic Buyers

Identify industry peers and adjacent players.

- Call `comps_analysis` with the target company to identify trading peers.
- Map adjacencies: upstream suppliers, downstream customers, technology overlaps.
- Call `fmp_profile` for each candidate to verify size, sector, and M&A history.

Strategic buyer screening criteria:
- Revenue and EBITDA scale relative to target.
- Strategic rationale: cost synergies, revenue synergies, capability fill, geographic expansion.
- Financial capacity: balance sheet, existing leverage, available capital (call `credit_metrics` if public).
- Regulatory and antitrust considerations (geographic overlap, market share post-merger).

### Step 2 — Financial Sponsors

Identify PE firms with relevant sector focus.

- Identify firms with existing portfolio companies in the sector.
- Consider fund size: deal size should be 5-15% of fund; verify latest fundraise and estimated dry powder.
- Fund vintage: early-fund vs late-fund positioning affects appetite.
- Platform vs add-on: is a portfolio company already active in the sector?

Financial sponsor screening criteria:
- Sector focus alignment.
- Fund size and dry powder estimate.
- Existing sector platform (platform buy vs add-on).
- Hold period preference vs seller timeline.

### Step 3 — Tiering by Likelihood

Classify each buyer:
- **Tier 1** (most likely): strong strategic fit, financial capacity, stated M&A interest, no material regulatory hurdles.
- **Tier 2** (probable): good fit but secondary priority, capacity constraints, or regulatory considerations.
- **Tier 3** (possible): tangential fit, early-stage interest, or complex execution.

### Step 4 — Build Ranked Buyer Matrix

Minimum 15 names total: at least 5 strategic and 5 financial sponsors. For each buyer:
- Name, type (strategic / financial sponsor), tier.
- Strategic rationale (1-2 sentences).
- Financial capacity assessment.
- Regulatory risk flag (low / medium / high).
- Key contact(s) if known.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `comps_analysis` | Identifies trading peers as strategic buyer candidates |
| `fmp_profile` | Company overview, M&A history, and size verification per buyer |
| `credit_metrics` | Financial capacity check for public strategic buyers |

## Output Format

Ranked buyer matrix:
- Sorted by tier (Tier 1, then 2, then 3), then alphabetically within tier.
- Columns: tier | buyer name | type | strategic rationale | financial capacity | regulatory risk | key contact
- Summary: total strategic | total financial sponsors | total Tier 1 | total list count

## Quality Gates

- [ ] Minimum 15 buyers on the list
- [ ] At least 5 strategic buyers with rationale documented
- [ ] At least 5 financial sponsors with sector fit documented
- [ ] `comps_analysis` called to identify peer set as strategic buyer base
- [ ] `fmp_profile` called for each candidate to verify size and M&A history
- [ ] Every buyer assigned a tier with justification
- [ ] Regulatory risk flag assessed for all Tier 1 strategic buyers

## Related Skills

- `workflow-ib-cim` — CIM is distributed to buyers on the Tier 1 and Tier 2 list
- `workflow-ib-teaser` — teaser is sent to broader list before NDA execution
- `workflow-ib-process-letter` — process letter accompanies CIM to buyer list
- `workflow-fa-competitive-analysis` — competitive analysis produces the peer universe used as the strategic buyer base
