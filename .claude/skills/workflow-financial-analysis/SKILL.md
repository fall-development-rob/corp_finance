---
name: "Financial Analysis Workflows"
description: "Quality assurance and competitive analysis workflows — model checking and auditing, presentation/deck review, competitive analysis frameworks, and document formatting standards. Use when reviewing financial models for errors, checking pitch deck quality, performing competitive landscape analysis, or validating calculations."
---

# Financial Analysis Workflows

You are a senior financial analyst performing quality assurance and competitive analysis. You combine rigorous analytical frameworks with corp-finance-mcp computation tools to deliver institutional-grade review output.

## Core Principles

- **Accuracy over speed.** Every number must be verified before sign-off.
- **Challenge assumptions.** The goal is to find errors, not confirm correctness.
- **Consistency matters.** The same number must appear identically everywhere it is referenced.
- **Red flags first.** Material issues are reported before cosmetic ones.
- **Audit trail.** Every finding references a specific location and expected vs actual value.

## Workflow Selection

| Request | Workflow | Key Checks |
|---------|----------|------------|
| Check this model | Model Audit | BS balance, circular refs, formula consistency |
| Review this deck | Deck Review | Number consistency, formatting, terminology |
| Competitive analysis | Competitive Analysis | Porter's 5 Forces, market sizing, positioning |

## Analysis Workflows

### Model Audit Workflow

1. **Balance sheet balance**: verify Assets = Liabilities + Equity at every period
   - Check each forecast year independently, not just the terminal year
   - Common failures: missed working capital items, rounding errors, missed minority interest
   - Tolerance: zero — the balance sheet must balance exactly
2. **Circular reference detection**: identify and assess feedback loops
   - Interest expense -> net income -> cash -> debt balance -> interest expense
   - Revolver draw -> cash balance -> revolver draw
   - Check that iterative convergence resolves within 5-10 iterations
   - Flag any model that uses Excel CIRCULAR reference without iterative calculation enabled
3. **Formula audit**: distinguish inputs from calculations
   - Hardcoded numbers embedded in formulas must be extracted to assumption cells
   - Every formula should reference assumption cells, never contain literal values
   - Exception: universally known constants (12 months, 365 days, 100%)
   - Check for overwritten formulas: cells that break the pattern of their row/column
4. **Sign convention**: verify consistency throughout the model
   - Revenue and income: positive
   - Expenses and outflows: negative (or positive with explicit subtraction)
   - The entire model must use one convention consistently
   - Flag mixed conventions within the same section
5. **Growth rate sanity**: validate reasonableness of projections
   - Revenue growth >50% annual requires explicit justification (acquisition, new market)
   - Margin expansion >500bps requires driver explanation
   - Terminal growth rate must be <= long-term GDP growth (2-3% nominal)
   - Check that projections do not imply unrealistic market share
6. **Cross-statement linkage**: verify the three statements connect correctly
   - Net income flows from IS to BS retained earnings
   - D&A flows from IS to CF operating section and net of capex to BS PP&E
   - Working capital changes in CF tie to BS current asset/liability movements
   - Debt issuance/repayment in CF ties to BS debt balances
   - Cash on BS = opening cash + total CF
7. **Credit check**: call `credit_metrics` on projected financials
   - Verify leverage ratios remain within stated covenant thresholds
   - Flag any period where interest coverage < 2.0x
   - Check that credit profile does not deteriorate beyond investment grade if that is an assumption
8. **Terminal value check**: assess reasonableness
   - Terminal value should be 50-75% of total enterprise value
   - >80% means the explicit forecast period is too short — extend by 2-3 years
   - <40% may indicate overly aggressive near-term assumptions
   - Cross-check: Gordon Growth terminal value vs Exit Multiple terminal value should be within 20%
9. **Output**: audit checklist with pass/fail per item
   - Category: structural, mathematical, logical, formatting
   - Severity: critical (blocks use), major (material impact), minor (cosmetic)
   - Location: specific cell reference or section
   - Finding: what is wrong and what the correct value should be

### Deck Review Workflow

Four-dimension review framework:

1. **Number consistency**: same metric appears identically across all slides
   - Revenue on the summary slide must match revenue on the financial detail slide
   - Market size in the opportunity section must match market size in the appendix
   - Create a cross-reference table: metric, slide A value, slide B value, match (Y/N)
   - Special attention to: revenue, EBITDA, margins, growth rates, multiples, transaction value
2. **Data-narrative alignment**: claims in text match the data in charts/tables
   - "Revenue grew 25%" must tie to actual numbers showing 25% growth
   - "Market-leading margins" must be supported by comparative data
   - "Significant synergy potential" must have quantified synergy estimates elsewhere
   - Flag any qualitative claim without supporting quantitative evidence
3. **Language and terminology**: IB/PE standard terms, professional tone
   - Use "enterprise value" not "company value"
   - Use "EBITDA" not "cash flow" (unless specifically discussing cash flow)
   - Use "accretive/dilutive" not "good/bad for EPS"
   - Consistent tense: present tense for current state, past tense for historical
   - No colloquialisms, no first person ("we believe" -> "management expects")
4. **Formatting**: consistent visual presentation
   - Fonts: same typeface and size throughout (body, headers, footnotes)
   - Numbers: consistent decimal places, comma separators, currency symbols
   - Dates: consistent format throughout (DD-Mon-YYYY or FY20XX)
   - Charts: consistent colour scheme, axis labels, legends, source citations
   - Tables: consistent alignment (numbers right-aligned, text left-aligned)

- Flag issues with structured format:
  - **Location**: slide number and element (e.g., "Slide 7, revenue table")
  - **Severity**: critical (factual error, material inconsistency) or minor (formatting, style)
  - **Finding**: what is wrong
  - **Fix**: specific correction to apply
- **Output**: annotated review checklist, grouped by severity

### Pitch Deck Context Refresh Workflow

Use this workflow when an existing pitch deck is being repurposed for a new pitch context (new target audience, new pitch date, new market environment) without rewriting the underlying analysis. The objective is to detect every element of the deck whose freshness has expired and produce a structured refresh list before any edit is made.

1. **Date-stamp scan**: identify every dated element in the deck.
   - Cover-page date and any "as of" subtitles
   - Footer "as of" qualifiers on charts and tables
   - Market data date stamps (e.g., "Source: FMP as of DD-Mon-YYYY")
   - Footnote dates on comps tables and trading benchmarks
   - "Last twelve months" / "next twelve months" labels (LTM, NTM dates implicitly shift)
   - Flag any date older than 30 days for refresh; older than 90 days mandates refresh
2. **Rate-environment-dependent slides**: identify slides whose conclusions assume a specific yield-curve, credit-spread, or FX environment.
   - WACC and DCF slides — risk-free rate stale if 10Y Treasury moved >25bps
   - LBO returns slides — financing rate stale if SOFR or credit spreads moved >50bps
   - Bond / fixed income slides — entire yield curve dependency
   - Cross-border slides — FX rate dependency
   - For each: state the input that has shifted and the expected directional impact on the slide's conclusion
3. **Comp-set staleness check**: every comparable trades on a moving market.
   - Pull latest market caps, EVs, and multiples for every name in the comp set via `comps_analysis`
   - Flag any comp whose multiple moved >15% since the deck was last refreshed
   - Flag any comp that announced a transaction (acquired, going private, restructuring) since the last refresh — those are no longer comparable
   - Identify any new entrant that has IPO'd or entered the public market in the interim
4. **Market context updates**: identify slides describing the macro or sector environment.
   - Sector performance, sector rotation commentary, GDP/inflation context
   - Recent transaction commentary (M&A activity, IPO activity)
   - Regulatory developments (any new legislation, agency action since the deck was written)
   - Competitive landscape changes (new entrants, exits, rebrands)
5. **Recipient-context dependencies**: identify slides whose framing was tailored to the prior recipient.
   - Strategic-buyer pitch vs financial-buyer pitch — synergy framing differs
   - PE vs sovereign-wealth recipient — risk framing differs
   - Investor-day vs internal IC framing — depth and disclosure differ
   - For each, note where the framing must change vs where the underlying analysis stands
6. **Refresh list**: structured table with slide number, element, current value or framing, freshness status (current / stale / superseded), recommended action (no change / refresh data / rewrite framing).
7. **Approval gate**: present the full refresh list to the user. Do not edit any slide until the user has approved the per-element refresh actions. Number-level edits within the approved scope follow the standard Deck Refresh Workflow below.

### IB Deck Pre-Publication Check Workflow

Use this workflow before publishing or distributing any IB deck (pitch book, sell-side teaser-extension, fairness opinion deck, board-level summary). The objective is to catch every issue that would reflect poorly on the firm if it reached the recipient unaddressed. This workflow runs after the Deck Review Workflow and before the deck leaves the desk.

1. **Number tie-out across slides**: every metric appears identically across every slide it is referenced on.
   - Build a cross-reference table: metric, slides where it appears, value on each slide
   - Flag any inconsistency (e.g., revenue $1,200M on summary slide, $1,210M on financial detail)
   - Special attention to: revenue, EBITDA, EBITDA margin, three-year CAGR, net debt, EV, equity value, multiples, transaction value, indicative range
   - Tolerance: zero — same number, same precision, every appearance
2. **Brand and template consistency**: visual elements match the firm template.
   - Logo: present on every slide in the prescribed position; correct file (current logo, not deprecated)
   - Colour palette: only firm-standard colours used; no leftover accent colours from prior templates
   - Font family and sizes: title font, body font, footnote font as per firm standard
   - Footer position: confidentiality notice, page number, project codename consistent across slides
   - Header / divider slides: match firm template
3. **Disclaimer placement**: legal language appears in the prescribed locations.
   - Cover page: confidentiality notice ("CONFIDENTIAL — FOR DISCUSSION PURPOSES ONLY") prominently displayed
   - Cover page or page 2: full distribution-control language ("This document has been prepared by [Advisor] on behalf of [Client] and is strictly confidential...")
   - Final page: legal disclaimer ("This presentation does not constitute an offer to sell or a solicitation of an offer to buy any securities...")
   - Every page: confidentiality footer
   - Project-codename references: code names used consistently; no leak of real client names where confidentiality is in force
4. **Page numbering and pagination**:
   - Continuous page numbers starting from 1 (cover) or 2 (post-cover) per firm convention
   - No skipped numbers, no duplicate numbers
   - Table of contents page references match the actual page locations
   - Section dividers appear at the correct positions per the agenda
   - Total page count matches the deck's stated length
5. **Source citation completeness**: every chart, table, and quoted figure carries a source.
   - Charts: "Source: [provider]" footnote present on every chart
   - Tables: "Source: [provider]" or "Source: corp-finance-mcp [tool]" below every table
   - Quoted statistics: footnote with provider and date
   - Date stamps: market data citations include "as of DD-Mon-YYYY"
   - Tool attribution: corp-finance-mcp tool names cited where the figure was tool-derived
   - Flag any chart/table without a source — that is a critical defect
6. **Footnote integrity**:
   - Footnote markers (1), (2), (3) appear in numerical order on each slide
   - Every marker has a matching footnote text on the same slide (or in the appendix if the firm convention permits)
   - No orphan footnote text without a marker
   - No orphan markers without text
7. **Spell, grammar, and terminology check**:
   - Spell check complete; flagged technical terms accepted intentionally
   - Consistent terminology: "enterprise value" not "company value"; "EBITDA" not "cash flow"; "accretive" not "good for EPS"
   - Consistent tense: present for current state, past for historical
   - No first-person language ("we believe" -> "management expects")
   - No marketing superlatives without substantiation ("best-in-class", "world-leading", "premier")
8. **Cross-reference correctness**:
   - "See page X" references actually point to the right page
   - Appendix references resolve to existing appendix sections
   - Internal hyperlinks (if used in PDF output) target the correct slide
9. **Output**: pre-publication checklist with pass/fail per category, plus a critical-issues list grouped by severity.
   - Critical (blocks publication): factual error, missing disclaimer, missing source, page-number mismatch on TOC
   - Major (must fix before sending): brand inconsistency, footnote orphan, terminology slip
   - Minor (fix if time allows): formatting nit, alignment, small typography issue
10. **Sign-off block**: "Reviewed by: [date] | Critical issues remaining: 0 | Cleared for distribution: yes/no". Distribution is blocked while any critical issue is open.

### Competitive Analysis Workflow

1. **Porter's Five Forces**: assess industry structure
   - **Buyer power**: concentration, switching costs, price sensitivity, backward integration threat
   - **Supplier power**: concentration, differentiation, switching costs, forward integration threat
   - **Threat of substitutes**: relative price-performance, switching costs, buyer propensity
   - **Threat of new entrants**: capital requirements, scale economies, brand loyalty, regulatory barriers
   - **Competitive rivalry**: number and size of competitors, growth rate, differentiation, exit barriers
   - Rate each force: low / moderate / high with supporting evidence
2. **Market sizing**: TAM / SAM / SOM with methodology
   - **TAM** (Total Addressable Market): top-down from industry reports or bottom-up from unit economics
   - **SAM** (Serviceable Addressable Market): TAM filtered by geography, segment, capability
   - **SOM** (Serviceable Obtainable Market): SAM x realistic market share (3-5 year horizon)
   - State methodology explicitly: top-down, bottom-up, or hybrid
   - Cite sources for all market data
3. **Competitive positioning matrix**: map key players on 2 axes
   - Common axes: price vs quality, breadth vs depth, innovation vs reliability
   - Plot 6-10 competitors including the subject company
   - Identify white space opportunities and crowded segments
   - Note trajectory: where are competitors moving on the matrix?
4. **Financial benchmarking**: call `comps_analysis` for peer comparison
   - Margins: gross, EBITDA, net — rank vs peers
   - Growth: revenue, EBITDA — rank vs peers
   - Multiples: EV/EBITDA, P/E, EV/Revenue — premium or discount to peers
   - Capital efficiency: ROIC, asset turnover, working capital intensity
5. **Moat assessment**: evaluate competitive advantages
   - **Brand**: pricing power, recognition, Net Promoter Score
   - **Intellectual property**: patents, trade secrets, proprietary technology
   - **Switching costs**: contractual, technical, learning curve
   - **Network effects**: direct (more users = more value) or indirect (platform economics)
   - **Cost advantages**: scale economies, process efficiency, geographic advantages
   - Rate moat durability: narrow (5 years), wide (10+ years), or none
6. **Data sourcing**: use FMP tools for competitor financials
   - Call `fmp_profile` for company overview and key metrics
   - Call `fmp_key_metrics` for detailed financial ratios
   - Call `fmp_income_statement` and `fmp_balance_sheet` for raw financials
   - Cross-reference with SEC filings and earnings releases

## Quality Standards

- Model audit: all formulas traceable, no unresolved circular references, balance sheet balances exactly
- Deck review: zero critical issues before client delivery; minor issues documented but not blocking
- Competitive analysis: minimum 4-6 comparable companies with financial data
- All findings reference specific locations (slide number, cell reference, section heading)
- Severity classification applied consistently: critical = factual/material, minor = cosmetic/style
- Every recommendation includes a specific corrective action, not just identification of the problem

## Output Standards

All financial analysis output should:
1. State the scope and objective of the review
2. Summarise critical findings upfront (inverted pyramid)
3. Provide detailed findings with location, severity, and recommended fix
4. Distinguish between errors of fact and matters of judgement
5. Flag assumptions that require external validation
6. Be actionable — every finding has a clear resolution path

<!-- Adapted from: plugins/vertical-plugins/financial-analysis/skills/dcf-model/SKILL.md, plugins/vertical-plugins/financial-analysis/skills/lbo-model/SKILL.md, plugins/vertical-plugins/financial-analysis/skills/3-statement-model/SKILL.md (https://github.com/anthropics/financial-services) -->
## Excel Authoring Environment

### Runtime Selection

Before writing any cell, identify the authoring environment:

| Environment | Detection | Write method |
|---|---|---|
| **Office JS** (live Excel session) | Add-in context present, `Office.context` available | `range.formulas = [["=formula_string"]]` |
| **openpyxl** (standalone .xlsx) | No live session, generating a file for download | `cell.value = "=formula_string"` then run `recalc.py` |

Confirm the environment at the start of every Excel authoring task. Never silently assume one mode.

### Computation Source: cfa-core MCP Tools

**Key adaptation for this project.** Upstream patterns compute financial math inside Python/openpyxl. In this codebase, all financial math lives in the Rust core, exposed as MCP tools. The correct flow is:

1. **Compute** via cfa-core MCP tools (`mcp__cfa-core__build_dcf`, `mcp__cfa-core__build_lbo`, `mcp__cfa-core__build_three_statement`, etc.).
2. **Receive** structured JSON output containing schedule rows, rates, and derived values.
3. **Write to Excel** using Office JS or openpyxl purely as the output medium — translating MCP results into formula strings and cell ranges.

Never re-implement DCF discounting, LBO debt scheduling, or three-statement linkage in Python. That math already runs in 128-bit Decimal precision inside Rust. openpyxl is for layout and formula wiring only.

### Merged-Cell Pitfall and Fix Pattern

Office JS raises `InvalidArgument` when you attempt to write an array of values onto a range that contains a merged cell because the array dimensions do not match the merged region.

**Fix pattern (always apply when writing near merged cells):**
1. Unmerge the target range before writing: `range.unmerge()`.
2. Write the value or formula to the top-left single cell only.
3. Re-merge to the desired span: `range.merge(Excel.MergeBy.across)`.
4. Never pass a multi-element array to a merged range.

### Formulas-Over-Hardcodes Constraint

Every projection cell, roll-forward, linkage, and sensitivity output **must** be a live Excel formula — not a pre-computed value pasted in as a literal. This is a hard constraint, not a style preference.

**Hardcodes are permitted only for:**
- Raw historical actuals sourced directly from filings.
- Discrete assumption drivers in a named assumptions block.
- Universal constants (12, 365, 100).

**Anti-pattern detection heuristics — flag any cell that:**
- Is numeric but its neighbours in the same row/column are formulas (broken formula row).
- Contains a value that equals a formula result elsewhere but has no `=` prefix (shadow hardcode).
- Embeds a literal rate, multiple, or growth figure inside a formula string (e.g., `=B5*0.25` where `0.25` is an assumption, not a constant).
- Is in a projection column (FYxxE / FYxxF) but has no formula bar content.

All hardcoded inputs must use blue font; all formula outputs must use black font; cross-sheet references must use green font. This two-layer colour system is the primary visual audit trail.

### Step-by-Step User Confirmation Gates

Work section-by-section and pause for user confirmation before advancing. Rebuilding downstream sections because an upstream assumption was wrong is expensive.

| Gate | What to show before proceeding |
|---|---|
| 1. Raw inputs | Assumption block with sources and cell addresses |
| 2. Revenue / operating projections | Revenue build, growth rates, margin schedule |
| 3. FCF or EBITDA schedule | Full bridge from revenue to free cash flow |
| 4. WACC / returns calculation | Cost of equity, cost of debt, weights, blended rate |
| 5. Valuation bridge | EV → equity value → per-share or MOIC/IRR |
| 6. Sensitivity / scenario tables | All populated cells, confirming formulas not literals |

Do not proceed past any gate unless the user explicitly confirms. Record confirmation in the chat before continuing.

<!-- Adapted from: plugins/vertical-plugins/financial-analysis/skills/deck-refresh/SKILL.md (https://github.com/anthropics/financial-services) -->
## Deck Refresh Workflow

### Phase 1 — Data Collection

Ask the user how updated numbers are arriving:
- Pasted key-value mapping (e.g., "Revenue: $510M was $485M")
- Uploaded revised model or data file
- Inline values stated in the chat

Confirm each mapping explicitly before searching. Ask whether derived metrics (growth rates, margins, coverage ratios) should recalculate automatically from the updated base numbers or remain as-is.

### Phase 2 — Comprehensive Instance Search

Locate every occurrence of each target value across the entire deck. A single metric can appear in multiple surface forms — treat all of the following as the same underlying number and flag every instance:

| Surface form | Example (485 million) |
|---|---|
| Short-form with symbol | $485M |
| Long-form with abbreviation | $485MM |
| Written out | $485 million |
| Axis label or tick | 485 (on a chart axis, implied millions by axis title) |
| Table cell | 485.0 |
| Footnote or callout | "(1) Revenue of $485M as of FY2024" |

Search locations: text boxes, table cells, chart data labels, underlying chart data series values, footnotes, speaker notes.

### Phase 3 — Derived-Number Staleness Detection

After identifying base-number changes, automatically scan for derived numbers that depend on the updated values and flag them as potentially stale.

**Detection heuristics:**
- A growth rate percentage sitting near two revenue figures — if the revenue changed, the growth rate should change.
- A margin figure adjacent to an EBITDA and a revenue figure — if either changed, the margin may be stale.
- A "vs prior year" delta that does not equal the arithmetic difference between the updated and old values.
- Index or rebased values (e.g., 100 = FY2020) that shift when the base-year number changes.

Present stale-derived candidates as a separate list. The user decides whether to update them.

### Phase 4 — Approval Gate Before Edit

Before making any change, present the complete planned edit list in a structured table:

| Slide | Element | Current value | Proposed value | Basis |
|---|---|---|---|---|
| 3 | Summary revenue callout | $485M | $510M | User mapping |
| 3 | Revenue growth rate | 8.4% | 5.2% | Derived — confirm? |
| 7 | Revenue table row | $485M | $510M | User mapping |
| 7 | Chart axis max | 600 | 650 | Derived — confirm? |

Flag every derived/stale candidate in the "Basis" column. Do not proceed until the user approves the full list.

### Phase 5 — Execution Standards

- Edit only the value content; preserve all formatting (font, size, colour, weight, alignment).
- For chart data: update the underlying data series values, not just the visible labels.
- After each edit, perform a visual check for text overflow (truncation, line-wrap, overlap).
- Do not rewrite narrative or restructure slides. If a narrative sentence becomes logically inconsistent with the new numbers, flag it as a finding rather than silently rewriting it.

### Scope Limits

This workflow does not: rebuild slides, recalculate values the user did not request, modify formatting conventions, or alter document structure.

<!-- Adapted from: plugins/vertical-plugins/financial-analysis/skills/clean-data-xls/SKILL.md (https://github.com/anthropics/financial-services) -->
## Structured Data Cleaning (XLSX)

### Step 1 — Scope Definition

Before touching any data, confirm the target range with the user:
- Full used range of the sheet, or
- A specific named range or explicit cell range (e.g., A1:M500).

Profile each column: identify dominant data type (numeric, text, date, mixed) and note how many cells deviate from the dominant type.

### Step 2 — Issue Detection Categories

Scan the target range and classify all findings into the following categories. Report counts per category before proposing any fix.

| Category | Description | Detection signal |
|---|---|---|
| **Whitespace** | Leading/trailing spaces, non-breaking spaces, double spaces | `LEN(TRIM(A2)) <> LEN(A2)` |
| **Inconsistent casing** | Mixed upper/lower/title case in categorical columns | Multiple case variants of the same token |
| **Number-stored-as-text** | Numeric values with a text prefix mark or left-aligned in a numeric column | `ISNUMBER(VALUE(A2))` returns TRUE but cell is text |
| **Date format inconsistency** | Mixed ISO, US, EU, or text date representations in one column | Multiple distinct `TEXT(A2,"YYYY-MM-DD")` patterns |
| **Duplicates** | Exact or near-duplicate rows (same key columns) | Row hash comparison or `COUNTIFS` |
| **Blank cells** | Empty cells in columns that should be fully populated | `ISBLANK(A2)` in mandatory columns |
| **Mixed-type columns** | A column containing both numbers and text (excluding intentional headers) | Type distribution: >5% minority type |
| **Error values** | `#REF!`, `#DIV/0!`, `#VALUE!`, `#N/A` present in data range | `ISERROR(A2)` |
| **Encoding artefacts** | Replacement characters, smart quotes, em-dashes in numeric fields | Character code scan |

### Step 3 — Per-Category Confirmation Gates

Present a summary table of all detected issues grouped by category. Obtain explicit user confirmation for each category before applying any fix. Do not batch-approve all categories in one step.

Example summary table:

| Category | Column(s) | Count | Proposed fix | Approve? |
|---|---|---|---|---|
| Whitespace | A, C, F | 142 | Helper column: `=TRIM(A2)` | Pending |
| Number-as-text | D | 67 | Helper column: `=VALUE(D2)` | Pending |
| Date inconsistency | G | 38 | Helper column: `=DATEVALUE(G2)` | Pending |
| Duplicates | A+B key | 12 rows | Flag in helper column, user removes | Pending |

### Step 4 — Fix Application: Formula-Based Helper Columns

**Prefer non-destructive helper columns over in-place overwrite.** Insert a new column adjacent to the source column containing a formula that produces the clean value. The user pastes-as-values and deletes the original only after verifying the helper output.

Standard helper formulas:

| Issue | Helper formula |
|---|---|
| Whitespace | `=TRIM(A2)` |
| Uppercase normalisation | `=UPPER(A2)` / `=PROPER(A2)` |
| Number-as-text conversion | `=VALUE(A2)` |
| Date standardisation | `=DATEVALUE(A2)` or `=TEXT(A2,"YYYY-MM-DD")` |
| Error suppression (inspect first) | `=IFERROR(A2,"REVIEW")` |

**Destructive operations** (in-place overwrite, row deletion, column deletion) require a separate explicit confirmation even if the category was already approved at Step 3.

### Step 5 — Per-Category Report

After applying fixes for each approved category, report a before/after summary:

- Rows affected
- Sample before value → sample after value
- Any rows where the formula could not resolve (manual review required)
