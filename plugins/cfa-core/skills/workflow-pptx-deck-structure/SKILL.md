---
name: "workflow-pptx-deck-structure"
description: |
  WHAT: Slide-deck structural conventions for headless pitch and research deck authoring — slide-break syntax (`---` horizontal rule), mandatory slide types (title, agenda, exec summary, company overview, appendix), deck-type templates (pitch 15-25, IC 12-18, research 10-15, board 6-10), and quality gate rules for every slide.
  WHEN: Invoke when starting any financial deck — IB pitch, PE IC presentation, equity research initiation, or board summary — to structure the slide order and apply mandatory formatting conventions before populating individual content slides.
---

# Slide Deck Structure and Conventions

## What this skill covers

The foundational structural conventions for authoring headless financial slide decks: slide-break syntax, mandatory slides and their canonical order, deck-type templates, core formatting principles, and slide-level quality gates. Numbers and content come from corp-finance-mcp tools; this skill governs structure only.

## Core principle

One slide = one purpose. Each slide makes a single argument or presents a single artefact. The title summarises the slide's takeaway, not merely the section name.

## Slide-break syntax

The horizontal rule `---` on its own line is the slide boundary. Every `---` separates exactly two slides.

```
# Title Slide

(content)

---

## Agenda

(content)

---

## Section 1: Topic

(content)
```

This convention is compatible with pandoc, marp, reveal.js, and the `office_pptx_write` MCP tool. Do not place `---` inline with text.

## Mandatory slides

Every deck type includes these slides in this order:

| # | Slide | Required level | Notes |
|---|-------|---------------|-------|
| 1 | Title slide | All decks | H1 = project name/company; bold subtitle = document type; date; confidentiality notice |
| 2 | Agenda | All decks | Numbered list matching the sections that follow |
| 3 | Executive summary | All decks | 3-5 bullets, each with a metric; drives the reader's hypothesis |
| Last | Appendix | All decks | Methodology, assumptions, tools used, legal disclaimer |

## Deck-type templates

### Pitch Deck (IB / PE) — 15-25 slides

Compute via: `dcf_model`, `lbo_model`, `comps_analysis`, `merger_model`, `sensitivity_matrix`, `scenario_analysis`.

Slide order:
1. Title
2. Agenda
3. Executive summary
4. Company overview
5. Industry and market
6. Financial profile
7. Operating model
8. DCF valuation
9. Trading comparables
10. SOTP (if applicable)
11. Bull / base / bear scenarios
12. Sensitivity analysis
13. Transaction considerations
14. Risk factors
15. Recommendation
16. Appendix (methodology, detailed financials, comp detail, assumptions)

### IC Deck (PE) — 12-18 slides

Compute via: `lbo_model`, `returns_calculator`, `waterfall_calculator`, `sensitivity_matrix`.

Slide order:
1. Title
2. Recommendation (one line)
3. Investment thesis (3-5 bullets)
4. Company snapshot
5. Industry context
6. Management team
7. Operating model
8. LBO returns (base / upside / downside)
9. Sensitivity analysis
10. Risks and mitigations
11. Diligence findings summary
12. Proposed transaction structure
13. Approval ask
14. Appendix (detailed LBO, comparable transactions, assumptions)

### Research Deck — 10-15 slides

Compute via: `dcf_model`, `comps_analysis`, `target_price`.

Slide order:
1. Title
2. Recommendation and price target
3. Investment thesis
4. Company snapshot
5. Near-term catalysts
6. Financial profile
7. Estimates vs consensus
8. DCF valuation
9. Trading comparables
10. Bull / base / bear
11. Key risks
12. Appendix

### Board Deck — 6-10 slides

Condensed format for senior-audience consumption:
1. Title
2. Executive summary
3. Recommendation
4. Key financials
5. Valuation summary
6. Risks
7. Appendix

## Standard slide header format

Each slide must begin with a heading:
- Slide 1: `# Title` (H1 for the title slide only).
- All other slides: `## Slide Title` (H2).
- The title summarises the slide's **takeaway**, not just the topic. "Revenue Growing at 18% CAGR" is better than "Revenue".

## Title slide template

```
# [Project Codename or Company Name]

**[Subtitle: Pitch Deck / IC Presentation / Research Note / Board Deck]**

[Advisor Firm / Research House]
[DD-Mon-YYYY]

CONFIDENTIAL — FOR DISCUSSION PURPOSES ONLY

---
```

## Executive summary slide template

```
## Executive Summary

- $[X] revenue, [Y]% EBITDA margin, [Z]% three-year CAGR (Source: FY[year] 10-K)
- Market leader with [n]% share of $[X]B addressable market
- [n] near-term catalysts: [event 1], [event 2], [event 3]
- Indicative valuation: $[X]B - $[Y]B ([A]x - [B]x LTM EBITDA)

---
```

## Appendix slide template

```
## Appendix: Methodology and Assumptions

**DCF/LBO Assumptions**
- Forecast period: [n] years
- Revenue growth: [range]
- EBITDA margin: [range]
- WACC: [X]% (Rf=[a]%, beta=[b], ERP=[c]%, Kd=[d]%)
- Terminal growth rate / exit multiple: [X]

**Tools Used**
- corp-finance-mcp: [list of tools called]
- FMP: fmp_income_statement, fmp_balance_sheet, fmp_key_metrics

**Legal Disclaimer**
[Per `workflow-deal-documents` standard — confidentiality notice, not investment advice]

---
```

## Footer requirement

Every slide carries a footer (rendered as the final line of the slide body, below a blank line):

```
*Source: [corp-finance-mcp tool / data provider] | CONFIDENTIAL*
```

## Quality gates

- `---` slide breaks on their own line — never inline.
- Every slide has an H1 or H2 title.
- Title summarises the takeaway, not just the topic.
- Confidentiality notice on title slide and final appendix slide at minimum.
- Every numeric value cites `corp-finance-mcp <tool>` or a stated data provider.
- Appendix carries methodology, full assumption list, and tools used.

## Related skills

- `workflow-pptx-valuation-layout` — bull/base/bear valuation slide conventions
- `workflow-pptx-comps-table` — comps table and sensitivity slide conventions
- `workflow-pptx-mcp-writer-binding` — mapping the markdown deck to `office_pptx_write` SlideDeckSpec

## Routing

**Primary agent:** `cfa-private-markets-analyst` (pitch and IC decks)
**Secondary agent:** `cfa-equity-analyst` (research decks)
