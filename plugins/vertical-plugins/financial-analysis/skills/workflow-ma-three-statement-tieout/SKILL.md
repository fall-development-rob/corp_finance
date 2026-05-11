---
name: "workflow-ma-three-statement-tieout"
description: |
  WHAT: Three-statement model articulation audit — IS/BS/CF tie-out using corp-finance-mcp `three_statement_model` as ground truth, balance sheet balance check (Assets = Liabilities + Equity at zero tolerance), the four canonical cross-statement linkage rules, and working capital sanity via `working_capital` and `dupont_analysis`.
  WHEN: Invoke when auditing a three-statement model, a DCF or LBO with an integrated balance sheet, or any model where IS/BS/CF articulation must be confirmed before the output is used in a deliverable.
---

# Model Audit — Three-Statement Tie-Out and Balance-Sheet Integrity

## What this skill covers

Verification that a three-statement model (Income Statement, Balance Sheet, Cash Flow) articulates correctly. Uses `three_statement_model` as the 128-bit Decimal ground truth and checks the four canonical linkage rules, balance-sheet integrity at every period, and working capital / ROE trajectory plausibility.

## Tool references

| Tool | Use |
|------|-----|
| `three_statement_model` | IS/BS/CF articulation ground truth |
| `working_capital` | DSO/DIO/DPO/CCC sanity check |
| `variance_analysis` | Projection vs trailing actuals variance |
| `dupont_analysis` | ROE decomposition for trajectory plausibility |

## Workflow

### Step 1 — Independent rebuild via three_statement_model

Call `three_statement_model` with the model's stated assumptions (revenue growth, margin structure, capex, D&A, working capital drivers, interest rate, tax rate, dividends, debt amortisation). The tool resolves circular references in 128-bit Decimal and is the ground truth for IS/BS/CF articulation.

Compare line by line for every forecast period:
- Income Statement: Revenue, EBITDA, EBIT, Interest, EBT, Tax, Net Income.
- Balance Sheet: all asset and liability categories, equity components.
- Cash Flow: operating, investing, financing sections; net change in cash.

Any material discrepancy (>0.1% or >$1k, whichever is lower) is a finding.

### Step 2 — Balance sheet balance check

For every forecast period, verify:

```
Total Assets = Total Liabilities + Total Equity
```

**Tolerance: zero.** The `three_statement_model` tool produces a perfectly balanced balance sheet. Any model period where Assets ≠ Liabilities + Equity is a **Critical** finding — the model is broken for that period.

### Step 3 — Four canonical articulation rules

Verify each rule for every forecast period:

| Rule | Check |
|------|-------|
| 1. Net income → retained earnings | Net income on IS = change in retained earnings on BS (before dividends) |
| 2. D&A → PP&E | D&A on IS = D&A in CF operating section; ending PP&E on BS = opening PP&E + Capex - D&A |
| 3. Working capital changes → BS | Each working capital line on CF ties to the corresponding BS current asset/liability movement |
| 4. Ending cash → BS | Opening cash + total net CF = Ending cash on BS, exactly |

A failure in any rule for any period is a **Critical** finding.

### Step 4 — Working capital sanity check

Call `working_capital` with projected balance sheet figures to compute DSO, DIO, DPO, and Cash Conversion Cycle (CCC) for each forecast year.

Flag any metric that moves >10 days vs the trailing historical average without an explicit driver explanation in the model. Plausible drivers (new payment terms, geographic mix shift, procurement programme) must be documented; undocumented movement of >10 days is a **Major** finding.

### Step 5 — DuPont decomposition

Call `dupont_analysis` on each forecast year's projected ROE. Decompose into:
- Net margin × Asset turnover × Leverage multiplier.

Flag implausible trajectories:
- Net margin expansion >500bps with no stated driver: **Major**.
- Asset turnover improvement without capex reduction or revenue acceleration: **Major**.
- Leverage multiplier change without corresponding debt movement: **Major**.

### Step 6 — Variance check against trailing actuals

Call `variance_analysis` comparing the model's projections to trailing actuals. Flag every line where the Y1 projection implies a >5% variance from the trailing actuals trend without a driver explanation in the model.

## Output format

```
THREE-STATEMENT TIE-OUT REPORT
--------------------------------
Model: [name / file reference]

BALANCE SHEET INTEGRITY: PASS / FAIL
| Period | Assets | Liabilities + Equity | Delta | Status |
|--------|--------|-----------------------|-------|--------|
| FY2025 | 4,823 | 4,823 | 0 | PASS |
| FY2026 | 5,102 | 5,105 | (3) | CRITICAL FAIL |

ARTICULATION RULES: PASS / FAIL per rule per period
| Rule | FY2025 | FY2026 | FY2027 |
|------|--------|--------|--------|
| 1. NI → Retained Earnings | PASS | PASS | PASS |
| 2. D&A → PP&E | PASS | FAIL | PASS |
| 3. WC changes | PASS | PASS | PASS |
| 4. Ending cash | PASS | FAIL | PASS |

WORKING CAPITAL SANITY
| Metric | Historical avg | FY2026 | FY2027 | Finding |
|--------|---------------|--------|--------|---------|
| DSO | 42 days | 44 days | 55 days | MAJOR: +13 days FY2027, no driver |

FINDINGS SUMMARY
| # | Finding | Rule/Step | Period | Severity |
|---|---------|-----------|--------|----------|
| 1 | BS does not balance (delta $3k) | Step 2 | FY2026 | Critical |
| 2 | D&A → PP&E articulation broken | Rule 2 | FY2026 | Critical |
| 3 | DSO +13 days without driver | WC sanity | FY2027 | Major |
```

## Quality gates

- Balance sheet checked at zero tolerance — no rounding tolerance accepted.
- All four articulation rules checked for every forecast period — no periods skipped.
- Working capital metrics computed for every period with ≥1 historical period for comparison.
- DuPont computed for every period where ROE data is available.
- Findings table sorted: Critical first.

## Related skills

- `workflow-ma-rederivation` — re-derives operating model numbers against corp-finance-mcp tools; three-statement tie-out confirms structural integrity, rederivation validates the numbers
- `workflow-ma-link-tracing` — the dependency map identifies which cells feed the BS/CF and explains articulation breaks
- `workflow-ma-formula-consistency` — formula breaks are a common cause of articulation failures

## Routing

**Primary agent:** `cfa-chief-analyst`
