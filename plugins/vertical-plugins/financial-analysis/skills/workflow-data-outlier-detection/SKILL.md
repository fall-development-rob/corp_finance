---
name: "workflow-data-outlier-detection"
description: |
  WHAT: Statistical outlier detection across financial datasets — univariate (z-score, IQR, p1/p99), cross-sectional (peer-relative), time-series (moving-average spike), and factor-residual methods. Outliers are flagged with a review_flag column; they are never silently deleted.
  WHEN: Invoke when a raw numeric dataset has been ingested and must be screened for implausible values before entering a financial model, or when a model run has produced implausible output and the upstream data feed is suspect.
---

# Data Outlier Detection

## What this skill covers

Systematic identification and tagging of statistical outliers in financial datasets. Covers four detection methods: univariate (distribution-based), cross-sectional (peer comparison), time-series (trend deviation), and factor-residual (model-based). All flagged values are annotated in a `review_flag` column for analyst review — the analyst decides whether to retain, exclude, or winsorise.

## Inputs

- A numeric dataset (columnar: each column is a series, each row is a period or entity)
- Target columns to screen (or "all numeric columns")
- Optional: peer/factor data for cross-sectional or factor-residual methods
- Optional: preferred detection method (default: univariate p1/p99 or 3-sigma)

## Workflow

### Step 1 — Profile the numeric columns

For each target column compute:
- min, p1, p25, p50, p75, p99, max
- mean and standard deviation
- inferred distribution shape (roughly symmetric vs skewed)

Select the appropriate threshold: use **p1/p99** for skewed distributions; use **3-sigma** for roughly symmetric distributions. Document the choice per column.

### Step 2 — Univariate outlier detection

Flag any value that falls outside the chosen threshold (p1/p99 or ±3σ). Annotate the `review_flag` column with the method and threshold:

```
review_flag = "OUTLIER_UNI | method=p99 | threshold=[value] | review_required"
```

### Step 3 — Cross-sectional outlier detection (comp datasets)

When the dataset contains multiple companies or entities for the same period, flag any entity whose metric is more than 2 sigma from the peer median. Use `comps_analysis` to derive the peer distribution.

```
review_flag = "OUTLIER_CROSS | peer_median=[x] | sigma_dev=[y] | review_required"
```

### Step 4 — Time-series outlier detection

For each series with 5+ observations, flag any single-period value that deviates more than 50% from the trailing 4-period moving average. Call `rolling_forecast` to compute the baseline trend.

```
review_flag = "OUTLIER_TS | trailing_avg=[x] | deviation=[y%] | review_required"
```

### Step 5 — Factor-residual outlier detection (optional)

When peer or benchmark factor data is available, call `factor_model` to regress the suspect series against the benchmark. Flag residuals beyond ±2 sigma.

```
review_flag = "OUTLIER_FACTOR | factor=[name] | residual=[x] | sigma_dev=[y] | review_required"
```

### Step 6 — Variance sanity check

Call `variance_analysis` on actual vs prior-period values for every flagged column. Implausibly large period-over-period swings (>50%) often indicate upstream unit errors rather than true outliers — distinguish between the two in the flag annotation.

### Step 7 — Analyst review summary

Produce a flag summary table:

| Column | Row/Entity | Period | Flag Type | Value | Threshold | Recommended Action |
|--------|-----------|--------|-----------|-------|-----------|-------------------|
| Revenue | AAPL | Q2 2024 | OUTLIER_UNI | 94,000 | p99=85,000 | Verify source unit scale |

The analyst reviews the table and marks each flag as: **retain** / **exclude** / **winsorise** / **investigate**.

## Output format

1. **Outlier flag table** — one row per flagged cell with flag type, threshold, and recommended action
2. **Annotated dataset** — original data plus `review_flag` column; unflagged cells have `review_flag = "CLEAN"`
3. **Detection method log** — per column: method chosen, threshold applied, count flagged
4. **Analyst decision column** — populated by analyst post-review: retain / exclude / winsorise

## Quality gates

- [ ] Profile computed before any flag is applied
- [ ] Detection method documented per column (p1/p99 vs 3-sigma with rationale)
- [ ] Outliers flagged with structured annotation, never silently deleted or modified
- [ ] `variance_analysis` run to distinguish unit-error outliers from true outliers
- [ ] Analyst review summary presented before output is passed downstream

## Related skills

- `workflow-data-unit-reconciliation` — unit/sign/frequency alignment (run before or in parallel with outlier detection)
- `workflow-data-lineage-tracking` — provenance annotation for all flagged and cleaned values
- `workflow-data-pre-modelling-checklist` — composite quality gate that calls this skill as one of its checks
