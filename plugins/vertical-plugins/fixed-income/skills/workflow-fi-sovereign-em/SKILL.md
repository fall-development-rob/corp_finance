---
name: "workflow-fi-sovereign-em"
description: |
  WHAT: Sovereign bond analysis with country risk premium overlay; EM hard- vs local-currency comparison; capital-controls cost estimation.
  WHEN: EM sovereign credit research, hard-vs-local-currency allocation, country risk premium derivation.
---

# Fixed Income: Sovereign and Emerging-Market Bond Analysis

## What this skill covers

A four-phase analytical pipeline for sovereign fixed income: sovereign yield decomposition (credit, liquidity, FX), country risk premium derivation, EM hard- vs local-currency comparison, and capital-controls adjustment. Covers both investment-grade sovereigns and sub-investment-grade EM issuers. Every figure traces to an explicit tool call.

## Core Rules

- Sovereign spread = sovereign yield − risk-free reference (US Treasury or relevant G10 benchmark at matched tenor).
- Country risk premium (CRP) = market-implied sovereign default spread × (σ_equity / σ_bond); use `country_risk_premium` for derivation.
- Hard-currency bonds priced in USD (or EUR); local-currency bonds carry additional FX risk — both must be compared on a currency-hedged and unhedged basis.
- Capital-controls discount applies where repatriation restrictions limit realised yield; use `capital_controls` to quantify.
- EM bond analysis benchmarks against EMBI Global spread (USD-denominated EM sovereign debt index, typically 250–450 bps over US Treasuries).

## Workflow

### Phase 1 — Sovereign Bond Pricing and Spread Decomposition

1. Collect sovereign bond inputs:
   - Issuer (country), bond type (Eurobond / local-currency / Brady), ISIN or CUSIP, coupon, maturity, settlement date, current yield or price.
   - Credit rating (Moody's / S&P / Fitch): retrieve from `moodys_credit_rating` or `sp_credit_rating` if subscription available; else from public rating source.

2. Pull the risk-free reference yield:
   - USD-denominated sovereign: use US Treasury at matched tenor from `fred_yield_curve` or `fmp_treasury_rates`.
   - EUR-denominated sovereign: use German Bund at matched tenor from `lseg_yield_curve`.

3. Call `sovereign_bond_analysis` with sovereign yield, risk-free yield, country rating, and bond characteristics.
   - Outputs:
     - **Sovereign spread** (sovereign yield − risk-free, in bps)
     - **Credit component** (rating-implied expected default loss)
     - **Liquidity component** (residual above credit component; thinly traded sovereigns carry 30–100 bps liquidity premium)
     - **FX component** (for local-currency bonds only; quantified by CDS-implied credit vs yield spread gap)
   - Decomposition check: credit + liquidity + FX component = sovereign spread ± 5 bps.

4. Compare to EMBI Global or EMBIG-D benchmark spread: `fred_spread` (series BAMLEMCBPIOAS for EM corporate, or equivalent EMBI series) or `lseg_credit_spreads`.

### Phase 2 — Country Risk Premium Derivation

5. Call `country_risk_premium` with:
   - Country sovereign CDS spread (5Y USD CDS) — source from `lseg_credit_spreads` or `sp_credit_rating`
   - Local equity market volatility (σ_equity) — annualised; source from market data
   - Local bond market volatility (σ_bond) — annualised; source from `lseg_bond_pricing` or historical price series
   - Base equity risk premium (e.g., US ERP from `fmp_market_risk_premium`)
   - Outputs: **country risk premium (CRP)**, **total equity risk premium for country** = Base ERP + CRP.

6. Interpret CRP:
   - Investment-grade EM (e.g., Chile, Mexico): CRP typically 100–200 bps
   - Sub-investment-grade EM (e.g., Nigeria, Pakistan): CRP typically 300–600 bps
   - Distressed / default-adjacent: CRP > 800 bps; flag credit event risk

7. Document political risk and macro context:
   - Pull governance indicators: `wb_governance` (World Bank WGI — political stability, rule of law, control of corruption)
   - Pull macro fundamentals: `wb_country_indicators` (GDP growth, current account, external debt / GDP, FX reserves)
   - Note any ACLED conflict events: `acled_country_summary` (conflict frequency and fatalities as political risk proxy)

### Phase 3 — Hard-Currency vs Local-Currency Comparison

8. For the same issuer with both hard (USD) and local-currency bonds:

   a. Hard-currency bond: price with `sovereign_bond_analysis` as in Phase 1 — yield is USD yield with no FX exposure to the investor (if held to maturity); FX exposure only affects mark-to-market if sovereign devalues and bond trades at a discount.

   b. Local-currency bond: call `em_bond_analysis` with:
      - Local currency bond yield (in local currency terms)
      - Current FX spot rate (local / USD)
      - Forward FX rate at bond maturity (from `fx_forward` if available, else compute from interest rate parity: F = S × (1 + USD rate) / (1 + local rate))
      - Outputs: **local-currency YTM**, **USD-hedged YTM** (YTM adjusted for forward FX cost), **carry advantage or disadvantage**, **break-even FX depreciation** (max depreciation where local-currency bond still matches hard-currency bond return).

9. Present a hard-vs-local comparison table (see Output format).

10. Capital-controls adjustment (if applicable):
    - Call `capital_controls` with country, instrument type, and restriction type (repatriation limits, withholding tax).
    - Outputs: **effective yield haircut from capital controls** (bps), **onshore vs offshore yield differential**, **break-even holding period**.
    - If capital controls apply: local-currency effective yield = local YTM − capital controls haircut.

### Phase 4 — Investment Recommendation Structure

11. Summarise with a structured view:
    - Relative value vs EMBI benchmark (cheap / rich / fair)
    - Hard vs local currency recommendation with rationale
    - Key macro and political risks with quantified impact (spread widening scenario from `scenario_analysis`)
    - Near-term catalysts: rate decisions, IMF review dates, election calendar from `wb_country` or `fmp_economic_calendar`

## Output Format

**Sovereign Bond Summary**

| Metric | Value |
|---|---|
| Sovereign yield | XX.XX% |
| Risk-free reference yield | XX.XX% |
| Sovereign spread | +XXX bps |
| Credit component | +XXX bps |
| Liquidity component | +XX bps |
| Country risk premium (CRP) | +XXX bps |
| EMBI benchmark spread | +XXX bps |
| Relative to EMBI | +/− XX bps (Xth pctile) |

**Hard vs Local Currency Comparison**

| Metric | Hard (USD) | Local (hedged) | Local (unhedged) |
|---|---|---|---|
| Yield | XX.XX% | XX.XX% | XX.XX% |
| FX hedge cost | — | −X.XX% | — |
| Capital controls haircut | — | −X.XX% | −X.XX% |
| Net effective yield | XX.XX% | XX.XX% | XX.XX% |
| Break-even FX depreciation | — | — | −XX.X% |

**Country Risk Profile**

| Indicator | Value | Percentile |
|---|---|---|
| WB Political Stability | X.XX | XX th |
| WB Rule of Law | X.XX | XX th |
| External debt / GDP | XX% | — |
| FX reserves (months import cover) | X.X | — |
| CRP | +XXX bps | — |

**Tool-Call Traceability Table**

| # | Tool | Key Inputs | Output |
|---|---|---|---|
| 1 | `fred_yield_curve` | as-of date | US risk-free curve |
| 2 | `sovereign_bond_analysis` | sovereign yield, risk-free, rating | spread decomposition |
| 3 | `country_risk_premium` | CDS spread, σ_equity, σ_bond | CRP |
| 4 | `em_bond_analysis` | local yield, FX spot/forward | hedged/unhedged yield |
| 5 | `capital_controls` | country, instrument | effective yield haircut |
| 6 | `wb_governance` | country | WGI scores |
| 7 | `acled_country_summary` | country, date range | conflict events |

## Quality Gates

- [ ] Spread decomposition: credit + liquidity ± FX = sovereign spread (±5 bps)
- [ ] CRP derivation uses tool output; σ_equity and σ_bond stated with source
- [ ] Hard-vs-local comparison on same as-of date with consistent FX forward rate
- [ ] Capital controls discount applied where restrictions exist; zero stated explicitly if no restrictions
- [ ] Break-even FX depreciation computed for local-currency bond
- [ ] EMBI benchmark spread cited and relative positioning stated
- [ ] Political risk and macro fundamentals documented (WB WGI + macro ratios)

## Related Skills

- `workflow-fi-bond-valuation` — plain-vanilla bond pricing prior to spread computation
- `workflow-fi-credit-spreads` — corporate credit spread methodology (same spread decomposition framework)
- `workflow-fi-yield-curve-construction` — US risk-free curve construction
- `corp-finance-analyst-fixed-income` — deeper EM analysis (political risk, capital controls, EM equity premium)
- `corp-finance-analyst-regulatory` — country risk assessment for regulatory capital (Basel III sovereign risk weights)
