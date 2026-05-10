---
name: workflow-pe-returns-analysis
description: |
  WHAT: Private equity returns modelling — LBO model construction, IRR/MOIC by scenario, return attribution (EBITDA growth vs multiple expansion vs debt paydown), and 2D sensitivity tables.
  WHEN: Invoke when modelling PE investment returns; when stress-testing IRR/MOIC across scenarios; when a deal team needs an IRR bridge, a sensitivity grid, or a probability-weighted expected return.
---

# PE Returns Analysis

## What this skill covers

Full returns modelling workflow: LBO model construction, three-scenario analysis, return attribution decomposition, and sensitivity tables. Produces the returns section of an IC memo or a standalone returns pack.

## Workflow

### Step 1 — Build the LBO model

Call `lbo_model` with full deal parameters:
- Entry EV and implied EV/EBITDA at entry
- Debt tranches: senior secured (Term Loan A/B), second lien, mezzanine (if applicable); each with coupon and amortisation
- Revenue growth and EBITDA margin assumptions by year (typically 5-year hold)
- Working capital and capex assumptions
- Exit year and exit multiple range

### Step 2 — Define three scenarios

| Scenario | Revenue Growth | EBITDA Margin | Exit Multiple | Probability |
|----------|---------------|---------------|---------------|------------|
| Upside | Above plan + initiatives | Entry + expansion | Entry + 1-2x | 25% |
| Base | Management plan with haircut | Entry flat | Entry multiple | 50% |
| Downside | Revenue miss | Compression | Entry - 1-2x | 25% |

Call `returns_calculator` for each scenario to compute equity value at exit and IRR/MOIC.

### Step 3 — Return attribution

Decompose IRR into three value-creation components:
1. **EBITDA growth**: (EBITDA at exit - EBITDA at entry) × exit multiple, converted to equity return
2. **Multiple expansion**: change in EV/EBITDA × EBITDA at entry, converted to equity return
3. **Debt paydown**: leverage reduction from FCF generation, converted to equity return

Express each component as % of total equity value created and as basis points of IRR contribution.

IRR bridge format:
- Equity invested at entry
- + EBITDA growth contribution
- + Multiple expansion contribution
- + Debt paydown contribution
- - Dividends/recaps distributed during hold
- = Equity at exit → implied IRR and MOIC

### Step 4 — Sensitivity tables

Call `sensitivity_matrix` for:
- **Entry vs exit multiple**: rows = entry EV/EBITDA (1x increments), columns = exit EV/EBITDA (1x increments)
- **EBITDA growth vs exit multiple**: rows = EBITDA CAGR (2pp increments), columns = exit multiple (1x increments)
- **Leverage vs IRR**: rows = entry Net Debt/EBITDA (0.5x increments), columns = exit multiple

Mark the base-case cell with an asterisk.

### Step 5 — Probability-weighted expected return

Expected IRR = Σ (Scenario IRR × Scenario probability)
Expected MOIC = Σ (Scenario MOIC × Scenario probability)

Report alongside median scenario for comparison.

### Step 6 — Breakeven analysis

Identify:
- Minimum EBITDA at exit for 1.0x MOIC (debt paydown floor)
- Minimum exit multiple at base-case EBITDA for target IRR (typically 20%)
- Maximum entry multiple that still meets target IRR at base-case exit

## Output format

1. **Model summary table** — entry metrics, capital structure, key assumptions
2. **Scenario return table** — IRR and MOIC per scenario with probability weights
3. **Return attribution table** — three components as % of total value
4. **Sensitivity grids** — three 2D tables per Step 4
5. **Breakeven summary** — three thresholds from Step 6

All tables use the `workflow-xlsx-tabular-conventions` standard.

## Quality gates

- [ ] LBO model balances — equity + debt = EV at entry
- [ ] Three scenarios present (base/upside/downside) with explicit probability weights
- [ ] Return attribution components sum to total equity value created (±0.1%)
- [ ] Sensitivity matrix axes clearly labelled with base-case cell marked
- [ ] Breakeven thresholds computed for 1.0x MOIC and target IRR
- [ ] Base-case IRR/MOIC consistent with IC memo Section VII

## Related skills

- `workflow-pe-ic-memo` — IC memo that embeds the returns section
- `workflow-pe-value-creation-plan` — VCP initiatives that define the upside scenario
- `workflow-pe-portfolio-monitoring` — ongoing tracking of returns against model
- `workflow-xlsx-tabular-conventions` — table formatting standard
