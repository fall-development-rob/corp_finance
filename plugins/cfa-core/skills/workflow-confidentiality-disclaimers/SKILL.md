---
name: "workflow-confidentiality-disclaimers"
description: |
  WHAT: Standard confidentiality notices, NDA language, legal disclaimers, distribution controls, and recipient tracking for institutional financial deliverables — CIM/teaser cover pages, footer banners, final-page legal riders, and copy-control procedures.
  WHEN: Invoke when producing any client-facing financial document (CIM, pitch deck, equity research report, IC memo, client proposal) that requires confidentiality marking before distribution. Must be applied before any document leaves an internal workflow.
---

# Confidentiality Disclaimers and Distribution Control

## What this skill covers

Defines the confidentiality language, legal disclaimer text, and distribution control procedures that must appear on every institutional financial deliverable. Covers three tiers: (1) the per-page running confidentiality banner, (2) the cover-page confidentiality block for documents distributed externally (CIM, teaser, client memo), and (3) the final-page or footer legal disclaimer. Also covers physical and electronic copy control.

## Workflow

### Step 1 — Classify the document tier

| Document type | Confidentiality tier |
|---------------|---------------------|
| Internal model, working draft | Tier 1 — banner only |
| Client pitch, IC memo, credit committee memo | Tier 2 — cover block + banner |
| CIM, teaser, information memorandum, offering document | Tier 3 — cover block + banner + legal rider |
| Equity research (institutional distribution) | Tier 3 — regulatory disclaimer + banner |

### Step 2 — Apply per-page running banner (all tiers)

Add to the footer of every page:

```
CONFIDENTIAL — FOR DISCUSSION PURPOSES ONLY
```

For Tier 2 and Tier 3, extend the banner to include the recipient name or firm (for electronic distribution with watermarking):

```
CONFIDENTIAL — FOR DISCUSSION PURPOSES ONLY — PREPARED FOR [RECIPIENT FIRM]
```

### Step 3 — Cover-page confidentiality block (Tier 2 and 3)

Place immediately below the document title on the cover page:

> This document has been prepared by [Advisor/Firm] on behalf of [Client] and is strictly confidential. Distribution or reproduction of this document, in whole or in part, without the prior written consent of [Advisor/Firm] is prohibited.

For CIM / teaser, also include the process instruction:

> Recipients are requested to maintain strict confidentiality regarding the existence of this process and the information contained herein.

### Step 4 — Legal disclaimer block (Tier 3)

Place on the final page or as a footer on the first substantive page for offering documents:

> This presentation does not constitute an offer to sell or a solicitation of an offer to buy any securities. The information contained herein is preliminary and subject to change. [Advisor/Firm] makes no representation or warranty, express or implied, as to the accuracy or completeness of the information contained herein. Prospective investors should conduct their own due diligence and consult their legal, tax, and financial advisors before making any investment decision.

For equity research, replace with the applicable regulatory disclaimer per the firm's compliance template (MiFID II / FINRA / SEC Rule 17a-3 as applicable).

### Step 5 — Distribution control

**Physical distribution:**
- Number each copy sequentially (Copy 1 of N, Copy 2 of N)
- Record recipient name and copy number in the distribution log
- Instruct recipients to return or destroy upon request

**Electronic distribution:**
- Watermark each PDF with recipient name and date
- Restrict printing and copying where the distribution platform supports it
- Record the recipient email, document version, and distribution timestamp in the distribution log

### Step 6 — Distribution log entry

For every document distributed, record:

| Field | Value |
|-------|-------|
| Document name | [title] |
| Version | [v1.0, vFinal] |
| Distribution date | [DD-Mon-YYYY] |
| Recipient name | [name] |
| Recipient firm | [firm] |
| Copy number | [N of M] (physical) or n/a (electronic) |
| Watermark applied | yes / no |
| Returned / destroyed | yes / no / pending |

## Output format

1. **Classified document tier** — with rationale
2. **Confidentiality text blocks** — populated with advisor/client/recipient names, ready for insertion
3. **Distribution log entry** — one row per recipient

## Quality gates

- [ ] Per-page banner applied to every page of the document
- [ ] Cover-page block present for Tier 2 and 3 documents
- [ ] Legal rider present for Tier 3 documents
- [ ] Distribution log entry created before document is transmitted
- [ ] Recipient watermark applied for electronic Tier 3 distribution

## Related skills

- `workflow-deal-formatting-conventions` — number formatting, table standards, and date conventions that accompany the confidentiality marks
- `workflow-deal-quality-checklist` — final pre-delivery QC that verifies confidentiality marks are present
- `workflow-deal-citation-standards` — source footnote conventions used alongside the legal disclaimer
