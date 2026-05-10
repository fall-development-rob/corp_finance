---
name: workflow-fund-break-trace
description: |
  WHAT: Root-cause tracing of individual GL reconciliation breaks — audit trail from GL journal entry to subledger transaction, attribute diffing, single-sentence root-cause statements, and owner/action assignment.
  WHEN: Invoke after GL Reconciliation has classified one or more breaks; when needing to trace the audit trail back to the originating journal entry and subledger transaction for a specific break; when preparing root-cause documentation for controller review.
---

# Break Trace Workflow

You are a senior fund controller tracing reconciliation breaks to root cause. This workflow diagnoses — it does not post adjustments. Every break receives a single-sentence root-cause statement in a prescribed form.

## Core Principles

- Source of truth is the NAV pack. Subledger extracts and supporting documents are untrusted inputs.
- Diagnose, do not post: root-cause only; only the controller posts adjustments.
- Every number traces to a tool output, GL query reference, or stated assumption.

## Prerequisite

Run `workflow-fund-gl-reconciliation` first. This workflow operates on individual break rows from that output.

## Inputs Required

Per break row from GL Reconciliation:
- Key, GL values, subledger values, bucket, classified cause.
- GL journal entry detail: entry ID, posting date, source system, batch ID, preparer.
- Subledger transaction detail: trade ID, trade/settle dates, counterparty, source feed, FX rate used.

## Workflow

### Step 1 — Pull GL Side

Query the journal entry or posting that produced this GL line. Record: entry ID, posting date, source system, batch ID, preparer.

### Step 2 — Pull Subledger Side

Query the matching subledger transaction. Record: trade ID, trade/settle dates, counterparty, source feed, FX rate and date used.

### Step 3 — Isolate the Break

Call `reconcile_accounting` on the isolated break pair to confirm the differing attribute (posting date, FX rate, account mapping, quantity sign, amount sign).

### Step 4 — Diff Attributes

Compare sides attribute by attribute. The attribute with the largest discrepancy is the root cause.

### Step 5 — Draft Root-Cause Statement

Single sentence in the form: `[side] [did what] because [reason]`, with expected clear date if timing:

- **Timing**: "GL posted on settle date (T+2) while subledger posted on trade date — timing break, will clear on [date]."
- **FX rate**: "Subledger used WM/R 4pm rate; GL used Bloomberg close — FX break of [bps] on the base amount."
- **Mapping**: "Security [ID] maps to GL account [A] in the mapping table but the subledger fed [B] — mapping break, raise to reference-data."
- **Duplicate**: "Subledger posted trade [ID] twice ([ID1] and [ID2] are duplicates) — duplicate post, suppress [ID2]."

### Step 6 — Assign Owner and Action

Owner: `ops | reference-data | accounting | upstream-system`
Action: `monitor | adjust | raise-ticket | suppress`

## MCP Tools

| Tool | Purpose |
|------|---------|
| `reconcile_accounting` | Confirms the specific differing attribute on the isolated break pair |

## Output Format

```json
{
  "key": "...",
  "root_cause": "one sentence per the pattern above",
  "owner": "ops | reference-data | accounting | upstream-system",
  "expected_clear_date": "YYYY-MM-DD or null",
  "action": "monitor | adjust | raise-ticket | suppress"
}
```

One object per traced break. Aggregate into a break-trace log ordered by action severity (raise-ticket first, then adjust, monitor, suppress).

## Quality Gates

- [ ] GL and subledger sides both pulled and documented for every traced break
- [ ] Root-cause statement follows prescribed sentence form (side / did what / because)
- [ ] Owner and action assigned for every break
- [ ] Expected clear date populated for all timing breaks
- [ ] No adjustments posted by this workflow — diagnose only, controller posts

## Related Skills

- `workflow-fund-gl-reconciliation` — upstream: produces the break rows that this workflow traces
- `workflow-fund-nav-tieout` — downstream: break-free GL is a prerequisite for NAV tie-out
