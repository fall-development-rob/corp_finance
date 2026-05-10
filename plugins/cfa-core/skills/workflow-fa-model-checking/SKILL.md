---
name: workflow-fa-model-checking
description: |
  WHAT: Financial model audit — balance sheet integrity, circular reference detection, formula consistency, sign convention, growth rate sanity, three-statement linkage, credit checks, and terminal value reasonableness.
  WHEN: Invoke when asked to check, audit, or QA a financial model; when verifying a DCF, LBO, or three-statement model before distribution; when detecting formula errors or hardcodes embedded in projection cells.
---

# Financial Model Checking Workflow

You are a senior financial analyst performing model quality assurance. Your objective is to find errors, not confirm correctness. Every finding must reference a specific location with an expected vs actual value.

## Core Principles

- Accuracy over speed: every number must be verified before sign-off.
- Challenge assumptions: the goal is to find errors.
- Consistency matters: the same number must appear identically everywhere.
- Red flags first: material issues before cosmetic ones.
- Audit trail: every finding references a specific location and expected vs actual value.

## Workflow

### Phase 1 — Balance Sheet Integrity

1. Verify Assets = Liabilities + Equity at every forecast period independently.
   - Check each year, not just terminal year.
   - Common failures: missed working capital items, rounding errors, missed minority interest.
   - Tolerance: zero — the balance sheet must balance exactly.

### Phase 2 — Circular Reference Detection

2. Identify and assess feedback loops.
   - Interest expense → net income → cash → debt balance → interest expense.
   - Revolver draw → cash balance → revolver draw.
   - Check that iterative convergence resolves within 5-10 iterations.
   - Flag any model using Excel CIRCULAR reference without iterative calculation enabled.

### Phase 3 — Formula Audit

3. Distinguish inputs from calculations.
   - Hardcoded numbers in formulas must be extracted to assumption cells.
   - Every formula should reference assumption cells, never literal values.
   - Exception: universally known constants (12 months, 365 days, 100%).
   - Check for overwritten formulas: cells that break the pattern of their row/column.

### Phase 4 — Sign Convention and Sanity Checks

4. Verify sign consistency throughout.
   - Revenue and income: positive.
   - Expenses and outflows: negative (or positive with explicit subtraction).
   - Flag mixed conventions within the same section.

5. Validate growth rate reasonableness.
   - Revenue growth >50% annual requires explicit justification.
   - Margin expansion >500bps requires driver explanation.
   - Terminal growth rate must be <= long-term GDP growth (2-3% nominal).
   - Projections must not imply unrealistic market share.

### Phase 5 — Cross-Statement Linkage

6. Verify three-statement connectivity.
   - Net income flows from IS to BS retained earnings.
   - D&A flows from IS to CF operating section and net of capex to BS PP&E.
   - Working capital changes in CF tie to BS current asset/liability movements.
   - Debt issuance/repayment in CF ties to BS debt balances.
   - Cash on BS = opening cash + total CF.

### Phase 6 — Credit and Terminal Value Checks

7. Call `credit_metrics` on projected financials.
   - Verify leverage ratios remain within stated covenant thresholds.
   - Flag any period where interest coverage < 2.0x.
   - Check that credit profile does not deteriorate beyond investment grade if that is an assumption.

8. Assess terminal value reasonableness.
   - Terminal value should be 50-75% of total enterprise value.
   - >80% means the explicit forecast period is too short — extend by 2-3 years.
   - <40% may indicate overly aggressive near-term assumptions.
   - Cross-check: Gordon Growth vs Exit Multiple terminal value should be within 20%.

## Output Format

Audit checklist with pass/fail per item:
- **Category**: structural | mathematical | logical | formatting
- **Severity**: critical (blocks use) | major (material impact) | minor (cosmetic)
- **Location**: specific cell reference or section
- **Finding**: what is wrong and what the correct value should be

## Quality Gates

- [ ] Balance sheet balances at zero tolerance for every forecast period
- [ ] All circular references assessed and iterative convergence confirmed
- [ ] No hardcoded values in formula cells (except universally known constants)
- [ ] Single sign convention applied throughout
- [ ] All three statements correctly linked
- [ ] `credit_metrics` run on projected financials
- [ ] Terminal value as % of EV within 50-75% band or explained

## Related Skills

- `workflow-model-audit` — deeper standalone model audit workflow with re-derivation against MCP core
- `workflow-fa-deck-review` — for reviewing pitch decks that reference model outputs
- `workflow-fa-excel-authoring` — Excel formula conventions and hardcode constraints
