---
name: "workflow-data-pre-modelling-checklist"
description: |
  WHAT: Composite quality gate that aggregates the outputs of outlier detection, unit reconciliation, and lineage tracking into a single pass/fail checklist before a dataset is consumed by a financial model. Routes to cfa-chief-analyst for data-quality sign-off.
  WHEN: Invoke as the final step before a cleaned dataset enters any modelling workflow (DCF, LBO, three-statement, comps, attribution). Should be run after workflow-data-outlier-detection, workflow-data-unit-reconciliation, and workflow-data-lineage-tracking have all been applied.
---

# Pre-Modelling Data Quality Checklist

## What this skill covers

A terminal quality gate that aggregates the outputs of the three upstream data hygiene skills into a single checklist and produces a pass/fail verdict. When this checklist passes, the dataset is cleared for modelling. When it fails, the checklist identifies which dimension requires remediation and routes back to the appropriate upstream skill.

## Inputs

- Cleaned dataset with `lineage` and `review_flag` columns populated
- Lineage manifest and sign-off block from `workflow-data-lineage-tracking`
- Flag summary table from `workflow-data-outlier-detection`
- Conversion log and frequency alignment record from `workflow-data-unit-reconciliation`

## Workflow

### Step 1 — Structural profile

Confirm the dataset scope matches expectations:
- Row count and column count match what was ingested
- Target date range is complete (no silent gaps in the time series)
- All mandatory columns are present (as defined by the downstream model's input schema)

### Step 2 — Unit and sign check

Verify the unit reconciliation output:
- [ ] All columns are in the target unit ($M, %, bps — per model convention)
- [ ] No column has a residual scale anomaly (validated via `variance_analysis` post-conversion)
- [ ] Sign conventions are uniform and match model convention
- [ ] Frequency is uniform across all series at the target frequency
- [ ] No silent mixed currencies in the output dataset
- [ ] Stub periods are labelled and modelling treatment is documented

### Step 3 — Outlier disposition check

Verify the outlier detection output:
- [ ] All `review_flag` entries have been reviewed by the analyst (no open flags)
- [ ] Each flag has a recorded disposition: retain / exclude / winsorise / investigate
- [ ] No flagged value has been silently deleted (deletions require explicit approval)
- [ ] `factor_model` or `rolling_forecast` residual flags are resolved if raised

### Step 4 — Lineage completeness check

Verify the lineage tracking output:
- [ ] Every cell in the dataset has a non-empty `lineage` entry
- [ ] Every destructive transformation has an `approval` record
- [ ] Lineage manifest lists all source extracts with pull timestamps
- [ ] Sign-off block is present and `Approved for modelling` is set to "yes"

### Step 5 — Cross-source consistency

When the dataset merges multiple sources, verify:
- Call `working_capital` to sanity-check DSO/DIO/DPO consistency across cleaned periods
- Verify that the same line item from two sources (e.g., FMP revenue vs EDGAR revenue) agrees within a rounding tolerance
- Flag any residual discrepancy >0.1% as unresolved (requires analyst review before pass)

### Step 6 — Verdict and routing

| All checks pass | Dataset is cleared. Produce the sign-off summary and pass to the downstream model. |
|-----------------|-------------------------------------------------------------------------------------|
| Unit/sign failure | Route to `workflow-data-unit-reconciliation` for remediation. |
| Open outlier flags | Route to `workflow-data-outlier-detection` for analyst review. |
| Lineage incomplete | Route to `workflow-data-lineage-tracking` to complete provenance annotation. |
| Cross-source mismatch | Escalate to `cfa-chief-analyst` for resolution. |

## Output format

1. **Checklist table** — one row per check, with pass/fail status and remediation action if failed
2. **Verdict block** — PASS or FAIL with list of failing dimensions
3. **Model-ready dataset** — the cleaned dataset with sign-off block, ready for consumption

```
DATA QUALITY SIGN-OFF
---------------------
Dataset: [name / description]
Date: [ISO date]
Source extracts: [list]
Unit convention: $M (1dp) | Frequency: Annual | Currency: USD
Outstanding flags: 0
Approved for modelling: YES
Signed: cfa-chief-analyst
```

## Quality gates

- [ ] All six unit/sign checks pass before proceeding
- [ ] Zero open `review_flag` entries in the outlier log
- [ ] Lineage sign-off block present and marked "yes"
- [ ] Cross-source discrepancies resolved within tolerance
- [ ] Verdict explicitly stated as PASS or FAIL — not "mostly clean"

## Related skills

- `workflow-data-outlier-detection` — upstream; provides the outlier flag table consumed in Step 3
- `workflow-data-unit-reconciliation` — upstream; provides the conversion log consumed in Step 2
- `workflow-data-lineage-tracking` — upstream; provides the lineage manifest consumed in Step 4
