# Debug Model

Audit and debug a financial model using the `workflow-model-audit` skill (alias for `/cfa:model-audit`). Combines link tracing, hardcode detection, balance-sheet integrity checks, and three-statement tie-out.

## What It Does
Performs a structured model audit: (1) balance-sheet integrity (Assets = Liabilities + Equity at every period, zero tolerance), (2) circular reference detection and convergence assessment, (3) formula audit (hardcode detection, broken-row patterns, shadow hardcodes), (4) sign-convention check, (5) growth-rate sanity (revenue, margin expansion, terminal growth vs GDP), (6) cross-statement linkage (NI to retained earnings, D&A to PP&E, WC changes to BS, debt issuance to BS), (7) credit-metric stress (`credit_metrics` on projected financials), (8) terminal-value reasonableness (50-75% of EV), (9) re-derivation of headline outputs against `corp-finance-mcp` computation tools.

## Agent
Routes to `cfa-chief-analyst` with `workflow-model-audit` and `workflow-financial-analysis` skills.

## Key Tools
`credit_metrics`, `dcf_model`, `lbo_model`, `three_statement_model`, `sensitivity_matrix`

## Usage
Provide the model file or a structured description (sheets, key inputs, outputs being verified). Optionally specify scope (full audit vs targeted section) and severity threshold (critical-only vs full report).

## Output
Markdown audit checklist grouped by severity: critical (blocks use), major (material impact), minor (cosmetic). Each finding has category (structural / mathematical / logical / formatting), location (sheet, cell or section), description of the issue, expected vs actual value, and recommended fix. Sign-off block: "Critical issues remaining: N | Cleared for use: yes / no".
