---
name: "workflow-deal-quality-checklist"
description: |
  WHAT: Pre-delivery quality control checklist for institutional financial deliverables — numerical integrity, formatting compliance, documentation completeness, confidentiality marking, version control, and final editorial review checks. Issues a pass/fail verdict before any document is transmitted to a client or committee.
  WHEN: Invoke as the last step before any client-facing or committee-facing document (CIM, pitch deck, IC memo, equity research report, client proposal) is distributed. Must be run after content, formatting, and confidentiality steps are complete.
---

# Deal Document Pre-Delivery Quality Checklist

## What this skill covers

A terminal quality gate for institutional financial deliverables. Covers four domains: (1) numerical integrity, (2) formatting compliance, (3) documentation and attribution completeness, and (4) editorial and structural review. The checklist produces an explicit pass/fail verdict; any failed item blocks delivery until resolved.

## Inputs

- The near-final document (markdown, PDF draft, or structured data)
- The list of MCP tools invoked to produce financial figures
- The formatting conventions applied (`workflow-deal-formatting-conventions`)
- The confidentiality classification and distribution log (`workflow-confidentiality-disclaimers`)
- The citation log (`workflow-deal-citation-standards`)

## Workflow

### Step 1 — Numerical Integrity Review

For each financial table or figure in the document:

- [ ] Every number traces to a tool output (`corp-finance-mcp [tool_name]`) or an explicitly stated user assumption — no LLM-generated financial figures
- [ ] Base / bull / bear scenarios provided for all valuations
- [ ] Sensitivity analysis on at least 2 key variables (growth rate, discount rate, or exit multiple)
- [ ] Financial tables internally consistent — revenue matches across all sections (income statement, valuation, comps)
- [ ] Sources and Uses balance (total sources = total uses, where applicable)
- [ ] Balance sheet balances in every forecast period (A = L + E), where applicable

### Step 2 — Formatting Compliance Review

Verify against `workflow-deal-formatting-conventions`:

- [ ] Consistent number formatting throughout (commas, one decimal, currency suffix)
- [ ] All negatives in parentheses — no minus signs anywhere in the document
- [ ] All multiples use "x" notation with one decimal (e.g., 8.5x)
- [ ] All dates in DD-Mon-YYYY format
- [ ] Tables: no vertical gridlines; units in column header only; alternating row shading
- [ ] Source footnote present below every table and chart

### Step 3 — Documentation and Attribution Review

Verify against `workflow-deal-citation-standards` and `workflow-confidentiality-disclaimers`:

- [ ] Confidentiality notice present on every page
- [ ] Cover-page confidentiality block present (for Tier 2 and Tier 3 documents)
- [ ] Legal disclaimer on final page (for Tier 3 documents)
- [ ] Date stamps on all market data citations
- [ ] Source footnotes below every table: SEC filing page, FMP date, tool name
- [ ] All corp-finance-mcp tool outputs cited with tool name and key assumptions

### Step 4 — Editorial and Structural Review

- [ ] Spell check and financial terminology review complete
- [ ] Consistent use of IB/PE standard terms throughout (no mixed terminology)
- [ ] No first-person language in client-facing sections
- [ ] Executive summary or investment summary accurately reflects detailed findings
- [ ] All internal cross-references within the document are correct
- [ ] Page numbers accurate; table of contents entries match section headings
- [ ] No redaction failures — no confidential counterparty names visible in a blind teaser

### Step 5 — Version Control

- [ ] Document version stamped on the cover (v1.0, vDraft, vFinal)
- [ ] File name follows the firm's naming convention ([Client]_[DocumentType]_[Date]_[Version])
- [ ] Prior versions archived (not overwritten)
- [ ] Change log present for documents with 2+ material revisions

### Step 6 — Verdict

| All checks pass | Document cleared for distribution. Record date and version in the distribution log. |
|-----------------|--------------------------------------------------------------------------------------|
| Numerical failure | Block delivery. Route back to the analyst for correction and re-run of affected tool. |
| Formatting failure | Block delivery. Apply `workflow-deal-formatting-conventions` corrections and re-check. |
| Attribution failure | Block delivery. Add missing citations per `workflow-deal-citation-standards`. |
| Editorial failure | Block delivery. Fix the specific items and re-run Step 4. |

## Output format

1. **QC checklist table** — one row per check with pass/fail status and page reference for each failure
2. **Verdict block** — PASS or FAIL with list of blocking items
3. **Distribution clearance record** — if PASS: document name, version, date cleared, cleared-by

## Quality gates

- [ ] All four review steps completed in order — no skip-ahead to verdict
- [ ] Every failed check includes a specific page/section reference
- [ ] Verdict explicitly stated as PASS or FAIL — not "mostly ready"
- [ ] Distribution clearance record produced only on PASS

## Related skills

- `workflow-confidentiality-disclaimers` — produces confidentiality marks verified in Step 3
- `workflow-deal-formatting-conventions` — defines the formatting rules checked in Step 2
- `workflow-deal-citation-standards` — defines citation rules checked in Step 3
