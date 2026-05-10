---
name: "workflow-data-unit-reconciliation"
description: |
  WHAT: Unit scale, sign convention, frequency, and currency normalisation for financial datasets — converting 000s/millions/billions, aligning income statement sign conventions, aggregating/disaggregating time series to a uniform frequency, translating multi-currency series to a single base, and explicitly tagging period stubs.
  WHEN: Invoke when merging data from multiple sources (FMP, EDGAR, vendor feeds, manual entry) that may report in different unit scales, currencies, or periods, or when a three-statement or comps model requires a uniform dataset as input.
---

# Data Unit and Sign Reconciliation

## What this skill covers

Standardises raw financial datasets on four dimensions before they enter any model: (1) unit scale, (2) sign convention, (3) reporting frequency, and (4) currency. Also handles period stubs — partial periods arising from fiscal year changes, mid-year acquisitions, or LTM constructions — with explicit labelling, never silent annualisation.

## Inputs

- A columnar financial dataset (one column per period or entity; rows are line items or series)
- Target unit convention (typically $M with one decimal)
- Target frequency (annual, quarterly, or monthly — driven by the downstream model)
- Base currency (entity's reporting currency or fund base currency)
- FX rate source and reference date if translating currencies

## Workflow

### Step 1 — Detect unit scale per column

Scan column magnitudes. A column with values in the hundreds alongside a sibling column with values in the millions is mis-scaled. Common patterns:
- SEC EDGAR XBRL: USD integers (full dollars) vs model convention ($M)
- FMP income statement: typically in absolute USD — verify via `fmp_income_statement` metadata
- Manual entries: 000s vs millions ambiguity

Flag any column where the inferred scale does not match the target. Provide a conversion factor (e.g., ÷ 1,000,000 to reach $M).

### Step 2 — Standardise to target unit

Apply the conversion factor. Standard model conventions:
- Financial statement line items: $M with one decimal
- Percentages: 12.3 not 0.123 (multiply by 100 if stored as decimal)
- Multiples: absolute numeric (8.5 not "8.5x" text)
- Basis points: for spreads <1%, convert from % (0.25% = 25bps)

Call `variance_analysis` after conversion to detect implausible period-over-period swings that may indicate a residual scale error.

### Step 3 — Sign convention audit

Verify consistency across the dataset. Standard convention:
- **Revenue, gross profit, operating income, net income:** positive
- **Expenses (COGS, SGA, RD, interest, tax):** positive when shown as line items being subtracted; model subtracts them explicitly
- **Cash flow — outflows (capex, acquisitions, debt repayment):** negative; inflows positive
- **Working capital changes:** negative for use of cash (receivables increase), positive for source

Flag any column where mixed sign conventions are detected across periods (a critical error). Mixed-convention findings require analyst confirmation before any fix is applied.

### Step 4 — Frequency alignment

Identify each series' native frequency (daily, monthly, quarterly, semi-annual, annual). Choose the target frequency based on the downstream model (annual for three-statement, monthly for cash management, daily for rate models).

**Aggregation (high to low frequency):**
- Flow variables (revenue, expenses, cash flow): sum across the period
- Stock variables (assets, debt, equity): take period-end value
- Rates and ratios: compute from aggregated flows and stocks — never average the rates directly

**Disaggregation (low to high frequency):**
- Flows: divide evenly across sub-periods (or apply a known seasonality vector if available)
- Stocks: linear interpolation between period-ends
- Mark every disaggregated value as derived in the lineage column (`workflow-data-lineage-tracking`)

Flag any series where the inferred frequency does not match the column header or stated metadata.

### Step 5 — Currency normalisation

Identify each series' native currency. A clean dataset never silently mixes currencies.

**Translation rules:**
- Income statement (flow): use period-average FX rate
- Balance sheet (stock): use period-end FX rate
- Cash flow: period-average for operating; period-end for closing cash

**FX trail per line item:**
- Source currency
- FX rate used
- FX rate source (e.g., WM/R 4pm, ECB reference, Bloomberg close)
- Rate date

CTA (cumulative translation adjustment): if translating equity, carry CTA separately and flag if absent.

### Step 6 — Period stub handling

Identify any period covering less than the standard reporting frequency (e.g., 9-month fiscal year stub, partial-year acquisition contribution).

- Label column header explicitly: "FY2025 (9M)" not "FY2025"
- Flow variables: do not annualise the stub itself; if a comparable annual figure is needed, produce it in a derived column with explicit methodology noted
- Stock variables: use stub period-end value as-is
- Ratios from a stub period: tag as "stub-period ratio" and exclude from trend analysis unless explicitly handled
- LTM construction: LTM = full-year + interim - prior-year-interim; document the algebra in the lineage column

## Output format

1. **Conversion log** — per column: original unit, target unit, conversion factor applied
2. **Sign audit report** — flags for any mixed or non-standard sign conventions, with analyst approval status
3. **Frequency alignment record** — per series: native frequency, target frequency, aggregation/disaggregation method used
4. **FX trail table** — per series translated: source currency, FX rate, source, date
5. **Stub register** — list of stub periods with label, length, and modelling treatment applied
6. **Standardised dataset** — ready for model consumption, with lineage column entries per `workflow-data-lineage-tracking`

## Quality gates

- [ ] Unit scale verified per column before conversion; `variance_analysis` run post-conversion
- [ ] Sign convention audited; mixed conventions require explicit analyst approval before fix
- [ ] Aggregation method (sum vs period-end) applied correctly per variable type
- [ ] Rates and ratios computed from aggregated flows/stocks — never by averaging rates
- [ ] FX trail recorded per translated line item
- [ ] Stub periods labelled explicitly; no silent annualisation
- [ ] Output dataset has uniform unit, currency, and frequency

## Related skills

- `workflow-data-outlier-detection` — run after unit reconciliation so outlier thresholds are computed on correctly scaled data
- `workflow-data-lineage-tracking` — records every transformation applied in Steps 2-5
- `workflow-data-pre-modelling-checklist` — composite quality gate that calls this skill as one of its checks
