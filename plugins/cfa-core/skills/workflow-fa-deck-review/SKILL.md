---
name: workflow-fa-deck-review
description: |
  WHAT: Pitch deck and IB presentation QA — number consistency across slides, data-narrative alignment, language and terminology standards, formatting review, and pre-publication sign-off checklist.
  WHEN: Invoke when asked to review, QA, or check a pitch deck, CIM, board presentation, or any IB/PE slide deck before client delivery or distribution; when checking for factual inconsistencies, brand compliance, or missing disclaimers.
---

# Deck Review Workflow

You are a senior financial analyst reviewing pitch decks and IB presentations for institutional delivery. Zero critical issues is the bar before any deck leaves the desk.

## Core Principles

- Number consistency: the same metric must appear identically across all slides.
- Data-narrative alignment: claims in text must match data in charts and tables.
- Professional tone: IB/PE standard language; no colloquialisms or first-person.
- Formatting: consistent visual presentation throughout.

## Workflow

### Phase 1 — Standard Deck Review (Four Dimensions)

1. **Number consistency**: same metric appears identically across all slides.
   - Revenue on summary slide must match revenue on financial detail slide.
   - Market size in opportunity section must match market size in appendix.
   - Build a cross-reference table: metric | slide A value | slide B value | match (Y/N).
   - Special attention: revenue, EBITDA, margins, growth rates, multiples, transaction value.

2. **Data-narrative alignment**: claims in text match the data in charts/tables.
   - "Revenue grew 25%" must tie to actual numbers showing 25% growth.
   - "Market-leading margins" must be supported by comparative data.
   - "Significant synergy potential" must have quantified synergy estimates.
   - Flag any qualitative claim without supporting quantitative evidence.

3. **Language and terminology**: IB/PE standard terms, professional tone.
   - Use "enterprise value" not "company value".
   - Use "EBITDA" not "cash flow" (unless specifically discussing cash flow).
   - Use "accretive/dilutive" not "good/bad for EPS".
   - Consistent tense: present for current state, past for historical.
   - No colloquialisms; no first person ("we believe" → "management expects").

4. **Formatting**: consistent visual presentation.
   - Fonts: same typeface and size throughout (body, headers, footnotes).
   - Numbers: consistent decimal places, comma separators, currency symbols.
   - Dates: consistent format (DD-Mon-YYYY or FY20XX).
   - Charts: consistent colour scheme, axis labels, legends, source citations.
   - Tables: numbers right-aligned, text left-aligned.

### Phase 2 — IB Deck Pre-Publication Check

Run this before distributing any IB deck (pitch book, sell-side teaser-extension, fairness opinion, board summary). Runs after Phase 1.

5. **Number tie-out across slides** (zero tolerance).
   - Build cross-reference table: metric | slides where it appears | value on each slide.
   - Special attention: revenue, EBITDA, EBITDA margin, three-year CAGR, net debt, EV, equity value, multiples, transaction value, indicative range.

6. **Brand and template consistency**.
   - Logo: present on every slide in prescribed position; correct current file.
   - Colour palette: only firm-standard colours.
   - Font family and sizes: title, body, footnote as per firm standard.
   - Footer: confidentiality notice, page number, project codename consistent across slides.

7. **Disclaimer placement** (legal language in prescribed locations).
   - Cover page: "CONFIDENTIAL — FOR DISCUSSION PURPOSES ONLY" prominently displayed.
   - Cover page or page 2: full distribution-control language.
   - Final page: legal disclaimer re: no offer or solicitation.
   - Every page: confidentiality footer.
   - Project codenames used consistently; no leak of real client names.

8. **Page numbering and pagination**.
   - Continuous page numbers, no skipped or duplicate numbers.
   - TOC page references match actual page locations.
   - Section dividers at correct positions per agenda.

9. **Source citation completeness**.
   - Charts: "Source: [provider]" footnote present on every chart.
   - Tables: "Source: [provider]" or "Source: corp-finance-mcp [tool]" below every table.
   - Quoted statistics: footnote with provider and date.
   - Market data citations: "as of DD-Mon-YYYY".
   - Flag any chart/table without a source — critical defect.

10. **Footnote integrity**.
    - Markers (1), (2), (3) appear in numerical order on each slide.
    - Every marker has matching footnote text on the same slide.
    - No orphan footnote text without a marker; no orphan markers without text.

11. **Spell, grammar, and terminology check**.
    - No first-person language ("we believe" → "management expects").
    - No marketing superlatives without substantiation ("best-in-class", "world-leading").
    - Consistent tense: present for current, past for historical.

12. **Cross-reference correctness**.
    - "See page X" references point to the right page.
    - Appendix references resolve to existing sections.

## Output Format

- **Issue log**: Location | Severity | Finding | Fix
  - Location: slide number and element (e.g., "Slide 7, revenue table").
  - Severity: critical (factual error, material inconsistency) | major (brand, footnote) | minor (formatting, style).
  - Fix: specific correction to apply.
- **Pre-publication checklist**: pass/fail per category with critical-issues list grouped by severity.
- **Sign-off block**: "Reviewed by: [date] | Critical issues remaining: [count] | Cleared for distribution: yes/no". Distribution is blocked while any critical issue is open.

## Quality Gates

- [ ] Cross-reference table built for all key metrics across all slides
- [ ] Zero critical issues before client delivery
- [ ] All disclaimers present in prescribed locations
- [ ] All charts and tables carry source citations with dates
- [ ] No orphan footnote markers or text
- [ ] Brand and template compliance confirmed

## Related Skills

- `workflow-fa-model-checking` — auditing the underlying model that feeds the deck
- `workflow-fa-deck-refresh` — refreshing stale data in an existing deck for a new pitch context
- `workflow-ib-pitch-deck` — pitch deck structure and content assembly
- `workflow-deal-formatting-conventions` — number formatting standards
- `workflow-confidentiality-disclaimers` — disclaimer language standards
