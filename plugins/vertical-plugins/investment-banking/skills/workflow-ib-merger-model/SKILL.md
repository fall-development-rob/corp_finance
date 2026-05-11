---
name: workflow-ib-merger-model
description: |
  WHAT: Merger model and accretion/dilution analysis — accretion/dilution computation, purchase price analysis, sources and uses, pro forma EPS for Years 1-3, sensitivity analysis varying synergies and offer premium, and credit impact on the combined entity.
  WHEN: Invoke when building a merger model or accretion/dilution analysis; when assessing whether a proposed acquisition is accretive or dilutive to EPS; when computing the breakeven synergy level for EPS neutrality; when sizing the financing structure and testing credit impact post-merger.
---

# Merger Model Workflow

You are a senior investment banking associate building a merger model. Sources must equal Uses exactly. Accretion/dilution math must be internally consistent. Sensitivity analysis covers at minimum synergy level vs offer premium.

## Core Principles

- Data-driven: all inputs from FMP data or corp-finance-mcp computation.
- Internally consistent: Sources = Uses; EPS math traces end-to-end.
- Professional tone: factual, no promotional framing of the outcome.

## Workflow

### Step 1 — Run Accretion/Dilution

Call `merger_model` with acquirer and target financials:
- Specify consideration type: `AllCash`, `AllStock`, or `Mixed`.
- Include expected synergies with phase-in timeline:
  - Year 1: 25% of run-rate synergies.
  - Year 2: 75% of run-rate synergies.
  - Year 3: 100% of run-rate synergies.
- Include integration costs and one-time charges.

### Step 2 — Purchase Price Analysis

Compute implied acquisition multiples at the offer price:
- EV/EBITDA, EV/Revenue, P/E at offer price.
- Premium to undisturbed share price: 30-day, 60-day, 90-day VWAP.
- Compare to precedent transaction multiples in the sector.

### Step 3 — Sources and Uses

Call `sources_uses` for financing structure:
- **Sources**: equity (cash on hand or equity issuance), term loans, bonds, revolver draw, rollover equity, seller note.
- **Uses**: equity purchase price (offer price × fully diluted shares), refinancing of target debt, transaction fees (advisory, legal, financing), cash to balance sheet.
- Sources must equal Uses exactly.

### Step 4 — Pro Forma EPS (Years 1-3)

Compute for each year:
- Standalone acquirer EPS (baseline).
- Combined entity EPS at modelled synergy levels.
- Accretive: combined EPS > standalone EPS.
- Dilutive: combined EPS < standalone EPS.

Show pro forma EPS at multiple synergy scenarios (e.g., 0%, 50%, 100% of run-rate).

### Step 5 — Sensitivity Analysis

Call `sensitivity_matrix`:
- Synergy level (% of run-rate) vs offer premium — accretion/(dilution) at each combination.
- Cash/stock mix vs EPS impact.
- Calculate breakeven synergies: minimum synergy for EPS-neutral outcome.

### Step 6 — Credit Impact

Call `credit_metrics` on the pro forma combined entity:
- Post-deal leverage: Net Debt/EBITDA.
- Coverage ratios: interest coverage, fixed charge coverage.
- Synthetic credit rating.
- Rating agency threshold analysis (e.g., "deal keeps leverage below 4.0x IG threshold").

## MCP Tools

| Tool | Purpose |
|------|---------|
| `merger_model` | Accretion/dilution computation with synergies and integration costs |
| `sources_uses` | Financing structure — Sources = Uses verification |
| `sensitivity_matrix` | Synergy vs premium sensitivity; breakeven synergy calculation |
| `credit_metrics` | Post-deal leverage, coverage, and synthetic rating |

## Output Format

- **One-page merger consequences summary**: consideration, offer premium, implied multiples, Year 1-3 EPS impact, breakeven synergies, post-deal leverage.
- **Sources and uses table**: fully balanced, with each source and use itemised.
- **Sensitivity table**: synergy level × offer premium matrix with accretion/(dilution) at each cell.
- **Credit profile**: pre-deal vs post-deal leverage and coverage ratios.

## Quality Gates

- [ ] `merger_model` called with correct consideration type and synergy phase-in
- [ ] Sources equal Uses exactly — zero tolerance
- [ ] Pro forma EPS computed for Years 1, 2, and 3
- [ ] `sensitivity_matrix` covers at minimum synergy level × offer premium
- [ ] Breakeven synergy calculated and disclosed
- [ ] `credit_metrics` confirms post-deal leverage and coverage ratios
- [ ] Integration costs and one-time charges included (not just run-rate synergies)

## Related Skills

- `workflow-ib-pitch-deck` — merger model outputs feed the valuation section of the pitch deck
- `workflow-ib-cim` — merger model may be referenced in CIM financial overview for strategic acquirers
- `corp-finance-tools-core` — `merger_model`, `sources_uses`, `sensitivity_matrix`, `credit_metrics` tool references
