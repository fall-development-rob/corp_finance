---
description: Fund migration and redomiciliation feasibility analysis
requires_tools:
  - compare_jurisdictions
  - migration_feasibility
---
Analyze fund redomiciliation feasibility and cost-benefit for: $ARGUMENTS

Use the following analytical framework:

1. **Migration feasibility** — Use `migration_feasibility` to assess legal mechanism (continuation, redomiciliation, scheme of arrangement), regulatory approval requirements, and timeline
2. **Cost-benefit analysis** — Use `migration_feasibility` to compute NPV of migration decision including setup costs, ongoing savings, and tax consequences
3. **Source vs target comparison** — Use `compare_jurisdictions` to compare current and target jurisdictions side-by-side on cost, substance, distribution, and regulatory dimensions
4. **Tax consequence analysis** — Evaluate exit tax, step-up basis, and per-investor impact across investor types and geographies
5. **Migration timeline** — Build a phased implementation plan with regulatory milestones and critical path
6. **Go/No-Go recommendation** — Quantitative threshold-based recommendation with sensitivity to key assumptions

**Output a migration feasibility report** with legal mechanism, regulatory requirements, cost-benefit NPV, tax consequences, phased timeline, and Go/No-Go recommendation.

Use tools from: `corp-finance-tools-regulatory`
Route to: `cfa-private-markets-analyst`
