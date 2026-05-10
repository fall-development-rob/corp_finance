---
name: workflow-ib-datapack
description: |
  WHAT: Sell-side datapack assembly — 10-section VDR-ready dataset covering historical financials, KPIs, customer data, org charts, capital structure, key contracts, litigation register, IP register, headcount and compensation, and public-comp reference set; with cross-section consistency checks and version control.
  WHEN: Invoke at sell-side process kickoff to build the canonical dataset that feeds the CIM and VDR; when refreshing financials and KPIs at each quarter-end during a multi-quarter process; when organising buyer DD responses against an existing datapack post-LOI.
---

# Datapack Builder Workflow

You are a senior investment banking associate assembling the sell-side datapack. The datapack is the structured input that feeds the CIM, populates the VDR, and answers the first 80% of buyer DD requests. Every numerical artifact cites its source. Cross-section consistency is mandatory.

## Core Principles

- Data-driven: all financials sourced from FMP tools or labelled as management-provided.
- Internally consistent: a buyer reading two artifacts from different sections should not find contradictory numbers.
- Redaction discipline: clean version (internal use) vs redacted version (buyer distribution); never mix in the same VDR.
- Version control: every artifact carries `Version: vX.Y` and `Updated: YYYY-MM-DD` header.

## When to Invoke

- **Sell-side kickoff**: 4-6 weeks before launch; build datapack so CIM and management presentation reference a single canonical dataset.
- **Post-LOI DD support**: organise buyer DD responses against existing datapack rather than starting fresh.
- **Refresh cadence**: re-cut financials and KPIs at each quarter-end during a multi-quarter process; tag the version.

## Section List (10 Sections)

### Section 1 — Historical Financials (3-year)

Income statement, balance sheet, cash flow — annual for 3 years, plus latest LTM.
- Call `fmp_income_statement`, `fmp_balance_sheet`, `fmp_cash_flow` with period "annual" and limit 3 (public comp datapacks).
- Call `build_three_statement` to produce a clean reformatted three-statement view from raw inputs (private targets).
- Quality of earnings: clearly labelled adjustments table (one-time, non-recurring, run-rate) with rationale.
- Output: `datapack/01-financials/income-statement.md`, `balance-sheet.md`, `cash-flow.md`, `qoe-adjustments.md`.

### Section 2 — KPIs and Operating Metrics

- Revenue by segment, geography, product, channel; volume and price decomposition.
- Customer KPIs: count, ARPU, churn, NRR, LTV, CAC, payback.
- Operational KPIs sector-specific: utilisation, throughput, NPS.
- Monthly cadence preferred; 24-36 months of history where available.
- Output: `datapack/02-kpis/INDEX.md` and per-KPI-family files.

### Section 3 — Customer and Contract Data

- Top-25 customer table: name (or coded ID for redacted version), revenue, tenure, contract end date, renewal status.
- Concentration analysis: top-1 / top-5 / top-10 share; flag any customer >10%.
- Contract types: subscription, transactional, project-based; weighted average duration.
- Output: `datapack/03-customers/top-25.md`, `concentration.md`, `contract-mix.md`.

### Section 4 — Organisational Charts

- Company-wide org chart: functions, headcount per box, reporting lines.
- Senior team detail: name, title, tenure, prior experience, equity stake.
- Locations and headcount by site.
- Output: `datapack/04-org/org-chart.md`, `senior-team.md`, `locations.md`.

### Section 5 — Capital Structure

- Current debt schedule: tranche, principal, rate, maturity, covenants, security.
- Equity holders: ownership table, options/RSUs outstanding, vesting profiles.
- Off-balance-sheet items: leases (ASC 842), guarantees, earnouts.
- Pro forma capital structure under indicative deal scenarios.
- Output: `datapack/05-capital/debt-schedule.md`, `equity-cap-table.md`, `off-bs.md`.

### Section 6 — Key Contracts List

- Material customer contracts (top-25 plus any with change-of-control provisions).
- Supplier and vendor contracts: top-10 by spend, single-source flags.
- Real estate leases: locations, rent, term, renewal options.
- Licensing, distribution, JV, and partnership agreements.
- Each entry: counterparty, value, term, CoC clause flag, redaction tag (clean / redact-customer / redact-pricing / restricted).
- Output: `datapack/06-contracts/contracts-register.md` and per-contract abstracts.

### Section 7 — Litigation Register

- Active matters: case name, jurisdiction, counterparty, claim amount, reserved amount, status, expected resolution.
- Settled matters in last 36 months.
- Threatened or pre-litigation matters disclosed by counsel.
- Regulatory investigations and enforcement actions.
- Output: `datapack/07-litigation/active.md`, `settled.md`, `regulatory.md`.

### Section 8 — IP Register

- Patents (granted and pending), trademarks, copyrights, domain names.
- Per-asset record: jurisdiction, registration number, filing date, status, owner, expiry.
- Trade secrets and know-how schedule (categorised, not detailed).
- Inbound and outbound licences.
- Open-source dependencies and licence types (for software businesses).
- Output: `datapack/08-ip/patents.md`, `trademarks.md`, `licences.md`, `oss-bom.md` (where applicable).

### Section 9 — Employee Headcount and Compensation

- Headcount roll-forward: opening, hires, attrition, closing — by function and location, monthly for 24 months.
- Compensation bands by function and seniority.
- Equity programme: outstanding options/RSUs, vesting schedule, dilution profile.
- Key person dependencies: roles where departure would materially impair operations.
- Output: `datapack/09-people/headcount-rollforward.md`, `comp-bands.md`, `equity-program.md`, `key-persons.md`.

### Section 10 — Public-Comp Reference Set

- Select 6-10 trading peers via `comps_analysis`.
- Per peer: revenue, EBITDA, growth, margins, EV, trading multiples (EV/EBITDA, EV/Revenue, P/E).
- Precedent transactions in sector: announce date, target, acquirer, EV, multiple, deal rationale.
- Output: `datapack/10-comps/trading-comps.md`, `precedent-transactions.md`.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `fmp_income_statement` | Historical income statement (public targets) |
| `fmp_balance_sheet` | Historical balance sheet (public targets) |
| `fmp_cash_flow` | Historical cash flow statement (public targets) |
| `build_three_statement` | Reformatted three-statement view (private targets) |
| `comps_analysis` | Trading peer selection and multiples for Section 10 |

## Output Convention

```
datapack/
  INDEX.md          <- master index; links every artifact; version + last refresh
  01-financials/
  02-kpis/
  03-customers/
  04-org/
  05-capital/
  06-contracts/
    contracts-register.md
    abstracts/
  07-litigation/
  08-ip/
  09-people/
  10-comps/
```

`datapack/INDEX.md` lists every artifact with: section number, artifact title, source, last refresh date, redaction tag. Previous versions retained under `datapack/_archive/<version>/` — never overwritten.

## Quality Gates

- [ ] Every numerical artifact cites its source (FMP call signature, management report ID, or audited filing)
- [ ] Every contract entry carries a redaction tag — no untagged commercial terms
- [ ] Financial artifacts in section 01 reconcile to the same EBITDA used in section 02 KPIs and section 10 comps
- [ ] Headcount in section 09 reconciles to org chart in section 04
- [ ] Capital structure in section 05 reconciles to balance-sheet debt and equity in section 01
- [ ] Every artifact has `Version: vX.Y` and `Updated: YYYY-MM-DD` header
- [ ] Clean vs redacted versions separated; never mixed in the same VDR folder

## Related Skills

- `workflow-ib-cim` — CIM references the datapack as its canonical data source
- `workflow-ib-buyer-list` — Section 10 comps output informs the strategic buyer universe
- `workflow-ib-process-letter` — data room referenced in process letter mirrors the datapack structure
- `fmp-market-data` — FMP tool reference for Sections 01 and 10
- `corp-finance-tools-core` — `build_three_statement` and `comps_analysis` tool references
