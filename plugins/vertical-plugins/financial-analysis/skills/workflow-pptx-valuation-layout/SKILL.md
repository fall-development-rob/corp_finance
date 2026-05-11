---
name: "workflow-pptx-valuation-layout"
description: |
  WHAT: Valuation slide templates for financial decks — bull/base/bear scenario table with probability weights, company overview headline-metrics box, and financial-profile slide layout; all numbers sourced from corp-finance-mcp tools (`dcf_model`, `scenario_analysis`, `lbo_model`, `returns_calculator`, `target_price`).
  WHEN: Invoke when authoring the valuation or scenario slide(s) in any IB pitch, PE IC deck, or equity research deck. The bull/base/bear layout is mandatory for any deck that presents a valuation recommendation.
---

# Slide Deck — Valuation and Scenario Slide Layouts

## What this skill covers

Standardised markdown templates for the valuation-facing slides in financial decks: the bull/base/bear scenario table, company overview with headline metrics, financial profile, and LBO returns slides. All number cells must reference tool output — LLM-generated figures are not permitted.

## Tool references

| Tool | Slide |
|------|-------|
| `dcf_model` | DCF valuation column in bull/base/bear table |
| `scenario_analysis` | Probability weights and scenario assumptions |
| `lbo_model` | LBO returns slide (PE/IC decks) |
| `returns_calculator` | IRR, XIRR, MOIC, cash-on-cash in IC deck |
| `waterfall_calculator` | LP/GP distribution in IC deck |
| `target_price` | Research deck price target |
| `comps_analysis` | EV/EBITDA range for valuation cross-check |
| `fmp_income_statement` | Revenue and EBITDA actuals for headline metrics |

## Valuation slide — bull / base / bear layout

This is the mandatory format for any deck that presents a valuation recommendation.

```
## Valuation: Bull / Base / Bear

| Scenario | Equity Value | EV/EBITDA | Implied per Share | Probability |
|----------|--------------|-----------|-------------------|-------------|
| Bull     | $[X]M        | [A]x      | $[a]              | [P1]%       |
| **Base** | **$[Y]M**    | **[B]x**  | **$[b]**          | **[P2]%**   |
| Bear     | $[Z]M        | [C]x      | $[c]              | [P3]%       |

Probability-weighted value: $[W]

**Key drivers**
- Bull: [revenue growth assumption], [margin assumption], [exit multiple]
- Base: [revenue growth assumption], [margin assumption], [exit multiple]
- Bear: [revenue growth assumption], [margin assumption], [exit multiple]

*Source: corp-finance-mcp dcf_model, scenario_analysis | CONFIDENTIAL*

---
```

Rules:
- Probability weights must sum to 100% — verify before publishing.
- Base case row is bolded.
- Equity value in $M (or $B if >$1B); EV/EBITDA in x to one decimal; per-share to two decimals.
- Negative values in parentheses: ($123).
- Probability-weighted value = Σ(equity value × probability weight).

## Company overview slide — headline metrics box

```
## Company Overview

**Business**
[2-3 sentence description: what the business does, where it operates, how it makes money]

**Differentiators**
- [Proprietary technology, process, or IP]
- [Customer concentration / contract structure]
- [Geographic footprint or market position]

**Headline Metrics**

| Metric | LTM | Source |
|--------|-----|--------|
| Revenue | $[X]M | tool: fmp_income_statement |
| EBITDA | $[Y]M | tool: fmp_income_statement |
| Margin | [Z]% | derived |
| Net Debt | $[A]M | tool: fmp_balance_sheet |
| EV | $[B]M | tool: fmp_enterprise_values |

*Source: corp-finance-mcp + FMP, as of [DD-Mon-YYYY] | CONFIDENTIAL*

---
```

The Headline Metrics table must cite the specific tool or data provider for every row. "Derived" is acceptable only for calculated metrics (margins, ratios) where the inputs are already cited.

## LBO returns slide — IC deck format

```
## LBO Returns

| Scenario | Entry EV | Exit EV | Equity Multiple (MOIC) | IRR | Hold Period |
|----------|----------|---------|------------------------|-----|-------------|
| Upside   | $[X]M    | $[A]M   | [M1]x                  | [R1]% | [Y] years |
| **Base** | **$[X]M**| **$[B]M**| **[M2]x**             | **[R2]%** | **[Y] years** |
| Downside | $[X]M    | $[C]M   | [M3]x                  | [R3]% | [Y] years |

**Key LBO assumptions (base case)**
- Entry multiple: [A]x LTM EBITDA
- Debt / EBITDA: [B]x (total leverage); [C]x (senior); [D]x (sub/mezz)
- Exit multiple: [E]x (year [N])
- Revenue CAGR: [F]%
- EBITDA margin at exit: [G]%

*Source: corp-finance-mcp lbo_model, returns_calculator | CONFIDENTIAL*

---
```

## Financial profile slide

```
## Financial Profile

| ($M) | [Year-2] | [Year-1] | LTM | [Year+1]E | [Year+2]E |
|------|----------|----------|-----|-----------|-----------|
| Revenue | [A] | [B] | [C] | [D] | [E] |
| YoY growth | — | [x]% | — | [x]% | [x]% |
| EBITDA | [A] | [B] | [C] | [D] | [E] |
| EBITDA margin | [x]% | [x]% | [x]% | [x]% | [x]% |
| Capex | ([A]) | ([B]) | ([C]) | ([D]) | ([E]) |
| Free Cash Flow | [A] | [B] | [C] | [D] | [E] |

Actuals: FMP fmp_income_statement | Estimates: corp-finance-mcp dcf_model (base case)

*Source: FMP, corp-finance-mcp | CONFIDENTIAL*

---
```

Rules:
- Negatives in parentheses: (123).
- Growth rates to one decimal: 12.3%.
- Multiples to one decimal: 8.5x.
- Currency in $M unless EV >$1B, then $B.
- Estimates column headers carry an "E" suffix.
- Actuals and estimates distinguished by source citation.

## Quality gates

- Bull/base/bear probability weights sum to exactly 100%.
- Base case is visually distinguished (bold row).
- Every number in the Headline Metrics and Financial Profile tables cites a tool or data provider.
- Negative values use parentheses — no minus signs.
- LBO returns slide includes entry/exit multiple, leverage, and hold period — no returns without context.

## Related skills

- `workflow-pptx-deck-structure` — master deck structure and slide order
- `workflow-pptx-comps-table` — comps table and sensitivity slide (referenced from valuation slides as cross-check)
- `workflow-pptx-mcp-writer-binding` — converting the markdown valuation slides to SlideDeckSpec for `office_pptx_write`

## Routing

**Primary agent:** `cfa-private-markets-analyst` (pitch and IC decks)
**Secondary agent:** `cfa-equity-analyst` (research decks)
