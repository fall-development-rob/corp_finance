---
name: "Slide Deck Authoring Workflows"
description: "Markdown-with-slide-breaks deck authoring conventions for headless pitch and research decks — slide-break syntax, title slide, agenda, exec summary, valuation bull/base/bear layout, comps table, sensitivity, and appendix slides. The CFA agent stack does not have a PPTX writer; this skill defines the markdown deck format so downstream tooling (or the recipient) can convert to PowerPoint, Keynote, or PDF. Routes to cfa-private-markets-analyst for IB/PE decks and cfa-equity-analyst for research decks."
---

# Slide Deck Authoring Workflows

You are producing pitch decks and research decks in a headless environment. The CFA agent stack does not have a PPTX writer (no python-pptx, no Office JS). This skill defines the markdown-with-slide-breaks convention so the deck can be converted to PowerPoint or Keynote by downstream tooling, or read directly as a slide-shaped markdown document.

## Core Principles

- **Slide break is the horizontal rule `---`.** Every `---` on its own line is a slide boundary.
- **One slide = one purpose.** Each slide makes a single argument or presents a single artefact.
- **Title before content.** Every slide starts with an H1 or H2 title. The title summarises the slide's takeaway, not the section.
- **Numbers come from corp-finance-mcp.** Decks reference tool output; LLM-generated figures are not permitted.
- **Footer on every slide.** Confidentiality notice and source citation belong on every slide, rendered as a footnote line.
- **Appendix carries detail.** The main deck is selective; supporting tables, methodology notes, and assumptions live in the appendix.

## When to Invoke

- An IB pitch, sell-side teaser-extension, or buy-side presentation deck is needed
- A PE IC presentation deck (separate from the IC memo) is needed
- An equity research initiation or thematic deck is needed
- A board-level summary deck is needed and must be convertible to PowerPoint downstream

## Slide-Break Syntax

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

The horizontal rule on its own line is the slide boundary. Downstream conversion tools (pandoc, marp, reveal.js, custom converters) all recognise this convention.

## Workflow Selection

| Request | Workflow | Output |
|---------|----------|--------|
| "Build a pitch deck" | Pitch Deck | 15-25 slide markdown |
| "Build an IC deck" | IC Deck | 12-18 slide markdown |
| "Build a research deck" | Research Deck | 10-15 slide markdown |
| "Build a board summary deck" | Board Deck | 6-10 slide markdown |

## Standard Slide Templates

### Title Slide

```
# [Project Codename or Company Name]

**[Subtitle: Pitch Deck / IC Memo / Research Note]**

[Advisor Firm / Research House]
[DD-Mon-YYYY]

CONFIDENTIAL — FOR DISCUSSION PURPOSES ONLY

---
```

H1 carries the project codename or company name. Subtitle indicates document type. Confidentiality notice is mandatory.

### Agenda Slide

```
## Agenda

1. Executive Summary
2. Company Overview
3. Industry & Market
4. Financial Profile
5. Valuation
6. Transaction Considerations
7. Appendix

---
```

Numbered list. The slide that follows the agenda begins section 1.

### Executive Summary Slide

Three to five bullets, each one line, each backed by a metric:

```
## Executive Summary

- $1.2B revenue, 22% EBITDA margin, 18% three-year CAGR (Source: FY2025 10-K)
- Market leader with 35% share of $4B addressable market
- Three near-term catalysts: [event 1], [event 2], [event 3]
- Indicative valuation: $4.5B - $5.2B (8.5x - 10.0x LTM EBITDA)

---
```

### Company Overview Slide

Two-column structure: business description on left, headline metrics box on right.

```
## Company Overview

**Business**
[2-3 sentence description: what the business does, where it operates, how it makes money]

**Differentiators**
- Proprietary technology or process
- Customer concentration / contract structure
- Geographic footprint

**Headline Metrics**

| Metric | LTM | Source |
|--------|-----|--------|
| Revenue | $1,200M | tool: fmp_income_statement |
| EBITDA | $264M | tool: fmp_income_statement |
| Margin | 22.0% | derived |
| Customers | 1,850 | filing |

Source: corp-finance-mcp + FMP, as of DD-Mon-YYYY

---
```

### Valuation Slide — Bull / Base / Bear Layout

```
## Valuation: Bull / Base / Bear

| Scenario | Equity Value | EV/EBITDA | Implied per Share | Probability |
|----------|--------------|-----------|-------------------|-------------|
| Bull     | $5,800M      | 11.0x     | $58.00            | 25%         |
| **Base** | **$4,800M**  | **9.1x**  | **$48.00**        | **55%**     |
| Bear     | $3,500M      | 6.6x      | $35.00            | 20%         |

Probability-weighted: $46.20

**Key drivers**
- Bull: revenue growth 25% sustained, margin expansion to 28%
- Base: revenue growth 15%, margin holds at 22%
- Bear: growth deceleration to 5%, margin compression to 18%

Source: corp-finance-mcp dcf_model, scenario_analysis

---
```

### Comps Table Slide

Title `## Trading Comparables`. Markdown table per `workflow-xlsx-author` conventions: ticker, market cap, EV, EV/Revenue, EV/EBITDA, P/E, revenue growth, EBITDA margin. Add Median and Target rows at the bottom (separator-preceded, bold). Single-sentence positioning takeaway below the table. Footer cites `Source: corp-finance-mcp comps_analysis, as of DD-Mon-YYYY`.

### Sensitivity Slide

Title `## Sensitivity: <metric>`. 2D markdown grid per `workflow-xlsx-author` Sensitivity Tabular convention. Mark the base case row/column header in bold. One-line takeaway with the sensitivity range below the grid. Footer cites `Source: corp-finance-mcp sensitivity_matrix`.

### Appendix Slide

Title `## Appendix: Methodology and Assumptions`. Bullet block of DCF/LBO assumptions with key inputs (forecast period, growth rates, WACC components, terminal method). Tools Used line citing every corp-finance-mcp tool referenced in the deck. Closing legal disclaimer per `workflow-deal-documents` standard.

## Workflow Outlines

### Pitch Deck (IB / PE) — 15-25 slides

Compute via `dcf_model`, `lbo_model`, `comps_analysis`, `merger_model`, `sensitivity_matrix`, `scenario_analysis`. Slide order: title, agenda, exec summary, company overview, industry and market, financial profile, operating model, DCF, comps, SOTP (if applicable), bull/base/bear, sensitivity, transaction considerations, risk factors, recommendation, appendix (methodology, detailed financials, comp detail, assumptions).

### IC Deck (PE) — 12-18 slides

Compute via `lbo_model`, `returns_calculator`, `waterfall_calculator`, `sensitivity_matrix`. Partners-only audience. Slide order: title, recommendation (one line), thesis (3-5 bullets), company snapshot, industry context, management, operating model, LBO returns (base/upside/downside), sensitivity, risks and mitigations, diligence findings, proposed structure, approval ask, appendix (detailed LBO, comparable transactions).

### Research Deck — 10-15 slides

Compute via `dcf_model`, `comps_analysis`, `target_price`. Sales/PM audience. Slide order: title, recommendation and price target, thesis, company snapshot, catalysts, financial profile, estimates vs consensus, DCF, comps, bull/base/bear, risks, appendix.

### Board Deck — 6-10 slides

Condensed senior-audience format: title, executive summary, recommendation, key financials, valuation summary, risks, appendix.

## Tool References

| Tool | Use |
|------|-----|
| `dcf_model` | Valuation slides |
| `lbo_model` | LBO returns slides (PE/IC decks) |
| `comps_analysis` | Trading comps slide |
| `merger_model` | Pro forma slides (IB pitches) |
| `sensitivity_matrix` | Sensitivity slide |
| `scenario_analysis` | Bull/base/bear slide |
| `target_price` | Research deck price target |
| `returns_calculator` | IC deck returns slide |
| `waterfall_calculator` | IC deck distribution slide |

## Output Standard

A deck deliverable is structured as:

1. **One markdown file** with `---` slide separators
2. **First slide is title**, last slide is appendix
3. **Every slide carries** a title, content, and footer (source + confidentiality)
4. **Tables follow** `workflow-deal-documents` formatting standards (negatives in parentheses, multiples with x, percentages with one decimal)
5. **Every numeric claim** carries a `Source: corp-finance-mcp <tool>` or `Source: <data provider>` footer

## Adaptation Note

This is the headless replacement for a PPTX writer. We do not have python-pptx or Office JS. The contract with downstream tooling: feed the markdown to a converter (pandoc, marp, reveal.js, or a custom CFA agent converter), splitting on `---` to produce one slide per section. Every slide is self-contained — title, body, footer — so the conversion is one-to-one.

For recipients without a converter: the markdown reads as a slide-shaped document where each `---` boundary is a visual break. A human reviewer can navigate it directly.

## Quality Standards

- Slide breaks (`---`) on their own line, never inline
- Every slide has a title (H1 or H2)
- Every numeric value cites `corp-finance-mcp <tool>` or a stated data provider
- Negatives in parentheses, multiples with x, percentages with one decimal
- Confidentiality notice on title and final appendix slide at minimum
- Bull/base/bear slide includes probability weights summing to 100%
- Sensitivity slide marks the base case explicitly
- Appendix carries methodology and assumptions; main deck is selective

## Routing

**Primary agent:** `cfa-private-markets-analyst` (default for pitch and IC decks)
**Secondary agent:** `cfa-equity-analyst` (for research decks)

Pitch and IC decks fall under the private-markets analyst because IB pitches and PE IC presentations are deal-execution artefacts. Research decks route to the equity analyst because they communicate research conclusions to sales and PMs.
