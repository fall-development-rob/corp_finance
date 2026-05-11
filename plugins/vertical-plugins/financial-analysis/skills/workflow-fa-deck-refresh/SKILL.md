---
name: workflow-fa-deck-refresh
description: |
  WHAT: Pitch deck context refresh and data update — date-stamp scanning, rate-environment staleness detection, comp-set staleness checks, market context updates, recipient-context dependencies, structured refresh list, and number update execution with approval gate.
  WHEN: Invoke when an existing pitch deck is being repurposed for a new pitch context (new target audience, new date, new market environment); when updating stale numbers in a deck; when detecting which elements need refreshing before any edits are made.
---

# Deck Refresh Workflow

You are a senior financial analyst refreshing an existing pitch deck for a new pitch context. Detect every stale element first; obtain approval; then execute updates with discipline — no narrative rewrites, no structural changes.

## Phase 1 — Context Refresh Scan

Use when repurposing a deck for a new audience or date without rewriting the underlying analysis.

1. **Date-stamp scan**: identify every dated element.
   - Cover-page date and "as of" subtitles.
   - Footer "as of" qualifiers on charts and tables.
   - Market data date stamps.
   - Footnote dates on comps tables and trading benchmarks.
   - "Last twelve months" / "next twelve months" labels (LTM, NTM dates implicitly shift).
   - Flag any date older than 30 days for refresh; older than 90 days mandates refresh.

2. **Rate-environment-dependent slides**: identify slides whose conclusions assume a specific yield-curve, credit-spread, or FX environment.
   - WACC and DCF slides — risk-free rate stale if 10Y Treasury moved >25bps.
   - LBO returns slides — financing rate stale if SOFR or credit spreads moved >50bps.
   - Bond / fixed income slides — entire yield curve dependency.
   - Cross-border slides — FX rate dependency.
   - For each: state the input that has shifted and expected directional impact on the slide's conclusion.

3. **Comp-set staleness check**: call `comps_analysis` to pull latest market caps, EVs, and multiples.
   - Flag any comp whose multiple moved >15% since the deck was last refreshed.
   - Flag any comp that announced a transaction (acquired, going private, restructuring) since last refresh — no longer comparable.
   - Identify any new entrant that has IPO'd or entered the public market in the interim.

4. **Market context updates**: identify slides describing the macro or sector environment.
   - Sector performance, sector rotation commentary, GDP/inflation context.
   - Recent M&A and IPO activity commentary.
   - Regulatory developments since the deck was written.
   - Competitive landscape changes (new entrants, exits, rebrands).

5. **Recipient-context dependencies**: identify slides whose framing was tailored to the prior recipient.
   - Strategic-buyer vs financial-buyer pitch — synergy framing differs.
   - PE vs sovereign-wealth recipient — risk framing differs.
   - Investor-day vs internal IC framing — depth and disclosure differ.
   - For each: note where framing must change vs where underlying analysis stands.

6. **Refresh list**: structured table before any edit.
   - Columns: slide number | element | current value or framing | freshness status (current / stale / superseded) | recommended action (no change / refresh data / rewrite framing).

7. **Approval gate**: present full refresh list to user. Do not edit any slide until the user has approved per-element refresh actions.

## Phase 2 — Data Collection for Number Updates

Ask the user how updated numbers are arriving:
- Pasted key-value mapping (e.g., "Revenue: $510M was $485M").
- Uploaded revised model or data file.
- Inline values stated in the chat.

Confirm each mapping explicitly before searching. Ask whether derived metrics (growth rates, margins, coverage ratios) should recalculate automatically or remain as-is.

## Phase 3 — Comprehensive Instance Search

Locate every occurrence of each target value. A single metric can appear in multiple surface forms — flag all instances:

| Surface form | Example (485 million) |
|---|---|
| Short-form with symbol | $485M |
| Long-form with abbreviation | $485MM |
| Written out | $485 million |
| Axis label or tick | 485 (on a chart axis, implied millions by axis title) |
| Table cell | 485.0 |
| Footnote or callout | "(1) Revenue of $485M as of FY2024" |

Search locations: text boxes, table cells, chart data labels, underlying chart data series values, footnotes, speaker notes.

## Phase 4 — Derived-Number Staleness Detection

After identifying base-number changes, automatically scan for derived numbers that may have become stale.

Detection heuristics:
- A growth rate percentage sitting near two revenue figures — if revenue changed, growth rate should change.
- A margin figure adjacent to EBITDA and revenue — if either changed, margin may be stale.
- A "vs prior year" delta that does not equal arithmetic difference between updated and old values.
- Index or rebased values that shift when the base-year number changes.

Present stale-derived candidates as a separate list. The user decides whether to update them.

## Phase 5 — Approval Gate Before Edit

Before making any change, present the complete planned edit list:

| Slide | Element | Current value | Proposed value | Basis |
|---|---|---|---|---|
| 3 | Summary revenue callout | $485M | $510M | User mapping |
| 3 | Revenue growth rate | 8.4% | 5.2% | Derived — confirm? |
| 7 | Revenue table row | $485M | $510M | User mapping |
| 7 | Chart axis max | 600 | 650 | Derived — confirm? |

Flag every derived/stale candidate in the "Basis" column. Do not proceed until the user approves the full list.

## Phase 6 — Execution Standards

- Edit only value content; preserve all formatting (font, size, colour, weight, alignment).
- For chart data: update the underlying data series values, not just visible labels.
- After each edit, check for text overflow (truncation, line-wrap, overlap).
- Do not rewrite narrative or restructure slides. If a narrative sentence becomes logically inconsistent with new numbers, flag it as a finding rather than silently rewriting it.

### Scope Limits

This workflow does not: rebuild slides, recalculate values the user did not request, modify formatting conventions, or alter document structure.

## Output Format

- **Context refresh list**: slide | element | current value/framing | freshness status | recommended action
- **Edit approval table**: slide | element | current value | proposed value | basis
- **Execution log**: slide | element | old value | new value | confirmed

## Quality Gates

- [ ] Date-stamp scan complete; all elements older than 30 days flagged
- [ ] Rate environment checked; WACC and LBO slides flagged if benchmarks moved >25bps / >50bps
- [ ] `comps_analysis` called; comps with >15% multiple movement flagged
- [ ] Full refresh list presented to user before any edits begin
- [ ] Full edit approval table presented before any number changes made
- [ ] Only value content changed — no formatting, narrative rewrites, or structural changes

## Related Skills

- `workflow-fa-deck-review` — QA review of the deck after refresh; pre-publication check
- `workflow-ib-pitch-deck` — pitch deck structure and content assembly
- `corp-finance-tools-core` — `comps_analysis` tool reference
