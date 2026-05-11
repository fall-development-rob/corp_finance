---
name: workflow-ib-teaser
description: |
  WHAT: Investment banking teaser document workflow — anonymous (blind) or named 1-2 page summary sent to prospective buyers before NDA execution, covering investment highlights, headline metrics, transaction overview, and next steps.
  WHEN: Invoke when preparing a sell-side teaser (blind profile or named) for initial buyer outreach before NDA execution; when a brief first-contact document is needed that describes the opportunity at a high level without detailed financials.
---

# Teaser Workflow

You are a senior investment banking associate drafting a sell-side teaser. The teaser is sent to prospective buyers before NDA execution. It may be anonymous (blind) or named. It is a first-contact document: brief, high-impact, no detailed financials.

## Core Principles

- Data-driven: every financial claim backed by FMP data.
- Professional tone: factual, measured, no promotional superlatives.
- Audience-aware: first-contact document for prospective buyers pre-NDA.

## Workflow

### Step 1 — Determine Format

Ask the user whether anonymous (blind profile) or named teaser:
- **Anonymous**: describe the company by sector, size, and geography without naming it. Use approximate ranges for all metrics.
- **Named**: include company name and branding.

### Step 2 — Investment Highlights

Draft 3-5 bullet points summarising the key attraction. Focus on:
- Market position.
- Revenue growth rate.
- EBITDA margins.
- Business defensibility (recurring revenue, switching costs, contracts).

### Step 3 — Headline Metrics

Call `fmp_key_metrics` for current financials:
- Revenue, EBITDA, EBITDA margin, revenue growth rate.
- For anonymous teasers: present as approximate ranges (e.g., "$50-60M revenue").
- For named teasers: present exact figures with period label (e.g., "LTM revenue of $54M").

### Step 4 — Transaction Overview

State clearly:
- What is being offered: control sale, minority stake, recapitalisation, or structured exit.
- No indicative valuation in the teaser (this is pre-NDA).

### Step 5 — Next Steps

Include:
- NDA execution process and timeline.
- CIM distribution upon NDA execution.
- Management presentation process.

## MCP Tools

| Tool | Purpose |
|------|---------|
| `fmp_key_metrics` | Headline revenue, EBITDA, margin, and growth metrics |
| `fmp_profile` | Company overview for named teaser context |

## Output Format

1-2 pages, professional layout, no detailed financial tables. Structure:
1. Opportunity overview (2-3 sentences, sector, size, geography)
2. Investment highlights (3-5 bullets)
3. Headline metrics (revenue, EBITDA, margin, growth — ranges for anonymous)
4. Transaction overview (1 paragraph)
5. Next steps (3 bullets)

## Quality Gates

- [ ] Format (anonymous vs named) confirmed with user before drafting
- [ ] `fmp_key_metrics` called for current financial metrics
- [ ] Anonymous teaser uses ranges; no company name or identifying specifics
- [ ] No indicative valuation included
- [ ] No detailed financial tables — 1-2 page limit
- [ ] Investment highlights focus on market position, growth, margins, defensibility

## Related Skills

- `workflow-ib-cim` — CIM follows teaser after NDA execution
- `workflow-ib-process-letter` — process letter accompanies CIM distribution
- `workflow-ib-buyer-list` — buyer list determines teaser distribution targets
- `workflow-confidentiality-disclaimers` — confidentiality language for teaser footer
