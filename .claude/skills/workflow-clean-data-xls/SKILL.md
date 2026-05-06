---
name: "Data Hygiene Workflows"
description: "Pre-modelling data hygiene workflows — outlier detection, unit and sign reconciliation, frequency alignment (monthly/quarterly/annual), unit conversion (000s/millions/billions), currency normalisation, period-stub handling, and data lineage tracking before any number enters a financial model. Use when ingesting raw extracts from FMP, EDGAR, vendor feeds, or CSV uploads, and before any modelling step that consumes them. Routes to cfa-chief-analyst for data-quality sign-off."
---

# Data Hygiene Workflows

You are a senior analyst preparing raw data for modelling. Garbage in, garbage out — every modelling deliverable is only as good as the data hygiene step that precedes it. This skill defines the standard pre-modelling data preparation pipeline.

## Core Principles

- **Profile before you touch.** Always characterise the data (types, distribution, frequency) before applying any fix.
- **Non-destructive first.** Helper columns and tagged adjustments precede in-place overwrites.
- **One unit, one currency, one frequency.** A clean dataset has uniform unit, currency, and period frequency before it enters a model.
- **Outliers are flagged, not deleted.** Statistical outliers may be real; the analyst decides, the workflow surfaces.
- **Lineage is mandatory.** Every transformed value records source extract, transformation applied, and timestamp.
- **Period stubs are explicit.** A partial period is labelled and treated as such — never silently annualised.

## When to Invoke

- A new dataset has just been ingested (FMP pull, EDGAR XBRL, CSV upload, vendor feed)
- Different sources (FMP + EDGAR + manual) are being merged for a single company
- Multi-currency or multi-frequency data is being unified for a comp set
- A model run produced implausible output and the data feed is suspect
- Before any modelling workflow (DCF, LBO, three-statement, comps) consumes the dataset

## Workflow Selection

| Request | Workflow | Output |
|---------|----------|--------|
| "Clean this raw extract" | Profile and Repair | Hygiene report + cleaned dataset |
| "Reconcile units and signs" | Unit / Sign Reconciliation | Standardised dataset |
| "Align frequencies" | Frequency Alignment | Frequency-uniform dataset |
| "Normalise currencies" | Currency Normalisation | Single-currency dataset with FX trail |
| "Handle period stubs" | Period Stub Handling | Stub-tagged dataset |

## Profile and Repair Workflow

### Step 1 — Scope and Profile

Confirm the target range with the user (full sheet, named range, or explicit cell range). For each column:

- Dominant data type (numeric, text, date, mixed)
- Cells deviating from dominant type (count, percentage)
- Distribution summary for numeric columns: min, p25, p50, p75, max
- Date columns: min date, max date, frequency inferred (daily/monthly/quarterly/annual)

### Step 2 — Issue Detection

Scan and classify into categories. Report counts before proposing fixes.

| Category | Detection signal |
|----------|------------------|
| Whitespace | Leading/trailing/non-breaking spaces |
| Inconsistent casing | Multiple case variants of same token |
| Number-as-text | Numeric value stored as string (left-aligned, prefixed) |
| Date format inconsistency | Mixed ISO/US/EU/text formats |
| Duplicates | Exact or near-duplicate rows on the key columns |
| Blanks in mandatory column | Empty cells where data is required |
| Mixed-type column | Numeric/text mix exceeding 5% minority |
| Error tokens | `#REF!`, `#DIV/0!`, `#VALUE!`, `#N/A`, `NaN`, `Inf` |
| Encoding artefacts | Replacement chars, smart quotes in numeric fields |
| Outlier | Beyond p1/p99 or 3 sigma from mean (flag for review) |

### Step 3 — Per-Category Confirmation Gate

Present a summary table grouped by category. Obtain explicit user confirmation per category before applying fixes. Do not batch-approve all categories.

### Step 4 — Non-Destructive Fix Application

Prefer helper columns over in-place overwrite. Standard helpers:

| Issue | Helper |
|-------|--------|
| Whitespace | TRIM |
| Casing | UPPER / PROPER |
| Number-as-text | VALUE conversion |
| Date inconsistency | DATEVALUE or TEXT to YYYY-MM-DD |
| Error suppression | IFERROR with "REVIEW" sentinel |

In-place overwrite, row deletion, and column deletion require a second explicit confirmation even if the category was already approved.

### Step 5 — Per-Category Report

For each fix applied: rows affected, sample before -> sample after, count of unresolved cases requiring manual review.

## Unit and Sign Reconciliation Workflow

1. **Detect unit scale per column:** scan magnitudes. A column with values in 100s alongside values in 1,000,000s is mis-scaled. Common patterns: filings reported in 000s vs millions vs billions, percentages reported as 0.05 vs 5.
2. **Standardise to one unit:** typical model conventions — financial statements in $M (one decimal), percentages as 12.3 not 0.123, multiples in absolute "x" units (8.5 not 8.5x as text), basis points for sub-percentage spreads.
3. **Sign convention check:** confirm consistency. Standard model convention:
   - Revenue, gross profit, operating income, net income: positive
   - All expenses (COGS, SG&A, R&D, interest, tax): negative when subtracted, positive when shown as line items being subtracted
   - Cash flow: inflows positive, outflows negative
   - Working capital change: negative use of cash, positive source
4. **Flag mixed conventions:** any section using both signs for the same concept is critical.
5. **Magnitude sanity:** call `variance_analysis` on actual vs prior period — implausibly large variances often indicate unit errors (e.g., a column flipped from 000s to millions mid-series).

## Frequency Alignment Workflow

1. **Identify each series' native frequency:** daily, monthly, quarterly, semi-annual, annual.
2. **Choose target frequency:** typically driven by the modelling step. Three-statement models default to annual; cash-management models to monthly; rate models to daily.
3. **Aggregation rules (high-frequency to low-frequency):**
   - Flow variables (revenue, expenses, cash flow): sum across the period
   - Stock variables (assets, debt, equity): take the period-end value
   - Rates and ratios (margin, leverage): compute from the aggregated flow and stock values, never average the rates directly
4. **Disaggregation rules (low-frequency to high-frequency):**
   - Flows: divide evenly across sub-periods (or apply a known seasonality vector)
   - Stocks: linear interpolate between period-ends
   - Mark every disaggregated value as derived in the lineage column
5. **Mismatch detection:** flag any series where the inferred frequency does not match the column header or stated frequency.

## Currency Normalisation Workflow

1. **Identify each series' native currency.** A clean dataset never silently mixes currencies.
2. **Choose base currency:** typically the entity's reporting currency (model base) or the fund's base currency.
3. **Translation:**
   - Income statement (flow): use the period-average FX rate
   - Balance sheet (stock): use the period-end FX rate
   - Cash flow: use period-average for operating, period-end for closing-cash translation
4. **FX trail:** record per-line the source currency, FX rate used, FX rate source (e.g., WM/R 4pm, ECB reference, Bloomberg close), and rate date.
5. **CTA handling:** if translating equity, the cumulative translation adjustment must be carried separately. Flag if not.

## Period Stub Handling Workflow

1. **Identify stub periods:** any period covering less than the standard reporting frequency (e.g., fiscal year change creates a 9-month stub; mid-year acquisition creates a partial-period contribution).
2. **Tag explicitly:** label the column header with the stub length (e.g., "FY2025 (9M)" not "FY2025"). Never silently annualise.
3. **Modelling rules:**
   - Flow variables: do not annualise the stub itself; if a comparable annual figure is needed, annualise in a derived column with explicit methodology
   - Stock variables: use the stub period-end as-is
   - Ratios computed from a stub: flag as "stub-period ratio" and exclude from trend analysis unless explicitly handled
4. **LTM construction:** when computing last-twelve-months figures across a stub, the standard formula is LTM = full-year + interim - prior-year-interim. Document the algebra.

## Outlier Detection Workflow

1. **Univariate outlier detection:** for each numeric column, flag values beyond p1/p99 or beyond 3 standard deviations from the mean. Use either depending on distribution shape.
2. **Cross-sectional outlier:** for comp datasets, flag a company whose multiple, margin, or growth rate is more than 2 sigma from the peer median.
3. **Time-series outlier:** flag a single-period spike >50% deviation from the trailing 4-period moving average.
4. **Optional factor-residual outlier (call `factor_model`):** when peer/factor data is available, regress the suspect series against a benchmark. Residuals beyond +/- 2 sigma are factor-residual outliers.
5. **Flag, never delete:** outliers are tagged for analyst review with a "review_flag" column. The analyst decides whether to retain, exclude, or windsorise.

## Tool References

| Tool | Use |
|------|-----|
| `variance_analysis` | Detect implausible period-over-period swings (often unit errors) |
| `working_capital` | Sanity-check DSO/DIO/DPO consistency across cleaned periods |
| `factor_model` | Cross-sectional residual analysis for outlier identification |
| `rolling_forecast` | Trend baseline for time-series outlier detection |

## Output Standard

The hygiene deliverable is:

1. **Profile summary:** column-by-column type, distribution, frequency, currency
2. **Detected issues table:** category, columns, count, proposed fix, approval status
3. **Cleaned dataset:** with non-destructive helper columns and a lineage column
4. **Lineage column entries:** source extract, transformation applied, FX rate (if any), timestamp
5. **Outstanding flags:** outliers and unresolved cases requiring analyst review
6. **Sign-off block:** "Hygiene applied by: [date] | Source extracts: [list] | Approved for modelling: yes/no"

## Quality Standards

- Profile produced before any fix is applied
- Per-category user confirmation captured before destructive operations
- Helper-column approach used for all reversible transforms
- Every transformed value carries a lineage entry
- Outliers flagged, never silently deleted
- Stub periods labelled explicitly; never silently annualised
- Mixed currencies and mixed frequencies disallowed in the output dataset

## Routing

**Primary agent:** `cfa-chief-analyst`

Data hygiene is a cross-domain pre-modelling step. The chief analyst owns the standard because every downstream skill (valuation, credit, comps, attribution, fund admin) consumes the cleaned dataset and the integrity guarantee must be uniform across domains.
