---
name: workflow-ib-pitch-deck
description: |
  WHAT: Investment banking pitch deck assembly — situation overview, market context, valuation football field (DCF, comps, LBO), transaction structure, execution timeline, and strip profile for quick-reference buyer conversations.
  WHEN: Invoke when assembling an IB pitch deck (sell-side or buy-side advisory); when building a valuation football field showing DCF range, trading comps, precedent transactions, and LBO floor; when producing a one-page strip profile for buyer conversations.
---

# Pitch Deck and Strip Profile Workflow

You are a senior investment banking associate assembling an IB pitch deck. The valuation football field must show at least three independent methodologies. Every data slide carries source footnotes. Confidentiality legend and page numbers on every page.

## Core Principles

- Data-driven: valuations backed by corp-finance-mcp tools; market data from FMP.
- Internally consistent: revenue, EBITDA, and multiples match across all slides.
- Professional tone: factual, no promotional language.
- Formatting conventions: blue font = hardcoded inputs; black font = formula outputs; source footnotes on every data slide; page numbers and confidentiality legend on every page.

## Pitch Deck Workflow

### Slide 1 — Situation Overview

- Why now: catalyst for the transaction (strategic review, ownership change, market window).
- Client objectives: price maximisation, speed, certainty, employee considerations.
- Recommended approach: sell-side, buy-side, or strategic alternatives.

### Slide 2 — Market Context

- Industry landscape: sector growth, key trends, regulatory environment.
- Recent comparable transactions: announce date, acquirer, target, EV, implied multiple.
- Market conditions: financing environment, deal activity, valuation levels vs prior periods.

### Slide 3 — Valuation Analysis (Football Field)

Construct the valuation football field using at least three methodologies:

1. **DCF (intrinsic value range)**:
   - Call `dcf_model` with base, bull, and bear assumptions.
   - Show WACC sensitivity and terminal growth rate sensitivity.

2. **Trading Comps (relative value)**:
   - Call `comps_analysis` to retrieve peer multiples.
   - Apply peer EV/EBITDA and EV/Revenue ranges to subject company.

3. **LBO (financial sponsor floor)**:
   - Call `lbo_model` to compute implied entry price at target sponsor returns (typically 20-25% IRR).
   - LBO floor represents the minimum price a PE buyer would pay.

4. **Precedent Transactions** (if applicable):
   - Sector transaction multiples for the trailing 3-5 years.
   - Apply to subject company metrics to derive implied value range.

Football field format: one horizontal bar per methodology, showing low-high range. All values in EV and equity value per share (or total equity value).

### Slide 4 — Transaction Structure

- Recommended structure: full sale, partial sale, recapitalisation, SPAC, or IPO.
- Financing alternatives: all-cash, stock-for-stock, mixed, earnout.
- Tax considerations: asset sale vs stock sale implications.
- Key conditions: regulatory approvals, board approval, shareholder vote.

### Slide 5 — Execution Timeline

- Key milestones: teaser distribution, NDA, CIM, management presentations, bids, exclusivity, signing, closing.
- Critical path: longest-lead items (regulatory filings, financing commitments).
- Resource requirements: deal team, legal counsel, advisors.

## Strip Profile Workflow

A one-page financial summary for quick reference during buyer conversations.

### Financial Metrics Strip (3-year history + 2-year projection)

Call `fmp_key_metrics` for historical data. Metrics to include:
- Revenue, gross profit, EBITDA, net income, free cash flow.
- Margins: gross, EBITDA, net, FCF conversion.

### Valuation Strip

Call `comps_analysis` for peer benchmarking:
- EV/EBITDA, EV/Revenue, P/E multiples: subject vs peer median.
- Premium or discount to peer median.

### Credit Strip

Call `credit_metrics` for balance sheet and leverage profile:
- Net Debt/EBITDA, interest coverage, current ratio.
- Synthetic credit rating.

Output: single-page landscape format with financial, valuation, and credit panels. All metrics carry period labels clearly stated.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `dcf_model` | Intrinsic value range for football field |
| `comps_analysis` | Trading comp multiples for football field and valuation strip |
| `lbo_model` | Financial sponsor floor price for football field |
| `fmp_key_metrics` | Historical financial metrics for strip profile |
| `credit_metrics` | Leverage and coverage ratios for credit strip |

## Output Format

- **Pitch deck**: slide-by-slide structure per sections above. Source footnotes on every data slide. Confidentiality legend and page numbers on every page.
- **Strip profile**: single-page landscape with financial / valuation / credit panels. All metrics sourced and period-labelled.

## Quality Gates

- [ ] Football field shows minimum 3 independent valuation methodologies
- [ ] `dcf_model`, `comps_analysis`, and `lbo_model` all called for football field
- [ ] Revenue, EBITDA, and multiples consistent across all slides
- [ ] Source footnotes on every data slide
- [ ] Confidentiality legend on every page
- [ ] Strip profile: all metrics sourced; `fmp_key_metrics`, `comps_analysis`, and `credit_metrics` called
- [ ] Blue font for hardcoded inputs; black font for formula outputs throughout

## Related Skills

- `workflow-ib-merger-model` — merger model feeds transaction analysis slides
- `workflow-fa-deck-review` — deck QA and pre-publication check after assembly
- `workflow-fa-deck-refresh` — refreshing the deck for a new pitch context
- `workflow-pptx-author` — PPTX formatting conventions and office_pptx_write tool
- `workflow-confidentiality-disclaimers` — disclaimer and confidentiality language
- `corp-finance-tools-core` — `dcf_model`, `comps_analysis`, `lbo_model`, `credit_metrics` tool references
