# Fund Operations

Run a fund administration workflow using the `workflow-fund-admin` skill.

## What It Does

Executes one of six period-end fund administration sub-workflows based on the `<workflow-type>` argument. All workflows produce controller-ready deliverables: break reports, NAV validation reports, accrual schedules with draft JEs, roll-forward schedules, and variance commentary. Schedules must foot and all unexplained variances are disclosed, never suppressed.

## Agent

Routes to `cfa-esg-regulatory-analyst`.

## Key Tools

`reconcile_accounting`, `calculate_nav`, `calculate_gp_economics`, `calculate_investor_net_returns`, `analyze_working_capital`, `build_three_statement`, `build_rolling_forecast`, `analyze_variance`

## Usage

```
/fund-ops <workflow-type>
```

Where `<workflow-type>` is one of:

| Argument | Sub-Workflow | Output |
|----------|-------------|--------|
| `gl-recon` | GL Reconciliation | Break report table + matched % summary |
| `break-trace` | Break Trace | Root-cause statement per break, owner, action |
| `nav-tieout` | NAV Tie-Out | LP validation table, pass/fail per LP |
| `accruals` | Accrual Schedule | Accrual table + draft journal entries |
| `roll-forward` | Roll-Forward | Period bridge schedule with "ties to" references |
| `variance` | Variance Commentary | Commentary table + 3-5 sentence period narrative |

## Pipeline

1. **Identify sub-workflow** from the `<workflow-type>` argument. If the argument is missing or unrecognised, list the six options above and ask the user to specify.

2. **Collect inputs** specific to the chosen workflow:
   - `gl-recon`: GL extract, subledger extract, reconciliation key, period, tolerance (default 0.01)
   - `break-trace`: Break row(s) from a prior `gl-recon` output, GL journal entry detail, subledger transaction detail
   - `nav-tieout`: Draft LP capital account statements, fund NAV pack, commitment register, prior close package, fee and carry schedule
   - `accruals`: Accrual register, GL account mapping, period dates, supporting document references
   - `roll-forward`: Account/account group, opening balance source, period transaction detail, period-end GL balance
   - `variance`: Current-period actuals, prior-period comparatives, budget/forecast (if available), materiality threshold (default 5% or $50,000)

3. **Execute the sub-workflow** following the step-by-step pipeline in `workflow-fund-admin`. Call the relevant MCP tools:
   - `gl-recon`: `reconcile_accounting`, `analyze_working_capital`
   - `break-trace`: `reconcile_accounting`
   - `nav-tieout`: `calculate_nav`, `calculate_gp_economics`, `calculate_investor_net_returns`
   - `accruals`: `analyze_working_capital`, `build_three_statement`
   - `roll-forward`: `build_rolling_forecast`, `build_three_statement`
   - `variance`: `analyze_variance`, `build_rolling_forecast`

4. **Apply quality checks** from the relevant quality checklist in `workflow-fund-admin` before delivering output.

5. **Deliver the output** clearly labelled as "Draft — Pending Controller Review" or "Ready for Distribution" depending on whether all quality checks passed.

## Notes

- `break-trace` requires a prior `gl-recon` output. If no break rows are available, run `gl-recon` first.
- `nav-tieout` blocks LP statement distribution if any LP fails the 0.01 tolerance check. State this explicitly in the output.
- `accruals` produces draft journal entries only. Nothing is posted to the ledger by this workflow.
- `roll-forward` must disclose any unexplained variance — it is never suppressed.
- All numbers come from MCP tool outputs, never from LLM-generated arithmetic.
