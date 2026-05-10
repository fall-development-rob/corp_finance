---
name: "workflow-data-lineage-tracking"
description: |
  WHAT: Source annotation and provenance metadata for every value in a financial dataset — recording the originating extract, transformation chain, FX rate (if translated), analyst approval, and timestamp so that any downstream model cell can be traced back to its raw source without ambiguity.
  WHEN: Invoke alongside any data cleaning or transformation step (outlier detection, unit reconciliation, currency normalisation) to ensure that every transformed value in the dataset carries auditable provenance before the dataset enters a model.
---

# Data Lineage Tracking

## What this skill covers

Mandates and structures a `lineage` column (or metadata sidecar) that records the full transformation history for every cell value in a financial dataset. Lineage is not optional: every value that enters a model must have a traceable path from raw source to cleaned input. This skill defines the lineage schema, population rules, and audit procedures.

## Inputs

- A dataset undergoing cleaning or transformation (may be mid-pipeline from `workflow-data-unit-reconciliation` or `workflow-data-outlier-detection`)
- The list of source extracts consumed (e.g., "FMP fmp_income_statement pull 2026-05-10", "EDGAR XBRL CIK 0000320193 10-K FY2025")
- Any transformations already applied (from sibling skills)

## Workflow

### Step 1 — Define the lineage schema

Each dataset must have a `lineage` column (or a sidecar JSON file keyed by column+row coordinates). The standard lineage entry is a pipe-delimited string:

```
source_extract | transformation | fx_rate | approval | timestamp
```

| Field | Description | Example |
|-------|-------------|---------|
| `source_extract` | Origin data pull — tool name, company, period | `fmp_income_statement AAPL FY2024` |
| `transformation` | Ordered list of transforms applied | `÷1e6 (to $M) > sign_flip > stub_tag(9M)` |
| `fx_rate` | FX rate used if translated (or "n/a") | `USDEUR 0.9213 ECB 2026-05-09` |
| `approval` | Analyst sign-off if a destructive op was applied | `approved_by_analyst 2026-05-10` |
| `timestamp` | ISO 8601 datetime of last transformation | `2026-05-10T14:32:00Z` |

### Step 2 — Populate lineage for raw (untransformed) values

Any value that passes through with no transformation applied still gets a lineage entry:

```
fmp_income_statement AAPL FY2024 | none | n/a | n/a | 2026-05-10T14:32:00Z
```

This establishes the source even for clean values.

### Step 3 — Append transformations in order

Each transformation step performed by `workflow-data-unit-reconciliation` or `workflow-data-outlier-detection` appends to the `transformation` field in chronological order, separated by ` > `:

```
fmp_income_statement AAPL FY2024 | ÷1e6 > pct_decimal_x100 | n/a | n/a | 2026-05-10T14:35:00Z
```

### Step 4 — Record analyst approvals

When a destructive operation was applied (in-place overwrite, row deletion, winsorisation), append the approval record:

```
fmp_income_statement AAPL FY2024 | ÷1e6 > winsorise_p99 | n/a | approved_by_analyst 2026-05-10 | 2026-05-10T14:40:00Z
```

No destructive transformation may have a blank `approval` field.

### Step 5 — Produce the lineage manifest

At dataset close, produce a lineage manifest (markdown table or JSON) listing:
- All source extracts consumed (tool, entity, period, pull timestamp)
- All transformation types applied and their scope (columns affected)
- Count of analyst-approved destructive operations
- Count of outstanding `review_flag` entries (from `workflow-data-outlier-detection`) still awaiting resolution

### Step 6 — Sign-off block

The final lineage entry in the manifest is the sign-off block:

```
Hygiene applied by: [analyst name or "cfa-chief-analyst"]
Date: [ISO date]
Source extracts: [comma-separated list]
Transformations applied: [summary]
Outstanding flags: [count]
Approved for modelling: yes / no / conditional on [action]
```

No dataset may enter a model with `Approved for modelling: no`.

## Output format

1. **Annotated dataset** — `lineage` column populated for every row/column coordinate
2. **Lineage manifest** — source extracts, transformation summary, approval log
3. **Sign-off block** — analyst or agent attestation that the dataset is model-ready

## Quality gates

- [ ] Every cell in the cleaned dataset has a non-empty `lineage` entry
- [ ] No destructive operation has a blank `approval` field
- [ ] Lineage manifest lists every source extract with tool name and pull date
- [ ] Outstanding `review_flag` entries from outlier detection are counted and disclosed
- [ ] Sign-off block produced; `Approved for modelling` field is explicitly set

## Related skills

- `workflow-data-outlier-detection` — populates `review_flag` entries that lineage tracking discloses in the manifest
- `workflow-data-unit-reconciliation` — produces the transformations that this skill records in the `lineage` column
- `workflow-data-pre-modelling-checklist` — consumes the sign-off block to verify the dataset is model-ready
