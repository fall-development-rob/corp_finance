---
name: "workflow-kyc-pep-screening"
description: |
  WHAT: Politically Exposed Person (PEP) screening of natural persons — classification into FATF categories 1-4, domestic vs foreign PEP distinction, time-horizon assessment for former PEPs, EDD trigger determination, and source-of-wealth evidence requirements.
  WHEN: Invoke for every natural person associated with a customer or counterparty during onboarding — UBOs, directors, authorised signatories — and during periodic KYC refresh. A confirmed PEP triggers EDD regardless of all other risk factors.
---

# KYC — PEP Screening

## What this skill covers

Identification and classification of Politically Exposed Persons (PEPs) among the natural persons associated with a customer. Covers FATF PEP categories 1-4, the domestic vs foreign PEP distinction, time-horizon treatment for former PEPs, mandatory EDD trigger, and source-of-wealth evidence requirements for confirmed PEPs.

## Core principle

PEP status is an absolute EDD trigger. A confirmed PEP triggers Enhanced Due Diligence regardless of the individual's risk score, the customer's risk score, or any other mitigating factor.

## FATF PEP categories

| Category | Role examples |
|----------|--------------|
| 1 | Heads of state, heads of government, ministers, deputy/assistant ministers, senior ruling-party officials |
| 2 | Members of parliament or equivalent legislative body; supreme court / constitutional court / other high-level judiciary judges |
| 3 | Senior military officials (general officer equivalent); central bank governors and deputy governors; senior diplomats (ambassador, high commissioner); members of governing boards of international organisations |
| 4 | Family members of PEPs in categories 1-3: spouse/partner, children and their spouses, parents; close associates of PEPs in categories 1-3: individuals with joint beneficial ownership of entities, close personal or business associates |

## Domestic vs foreign PEP

The FATF distinction is relative to the jurisdiction of the financial institution performing the screening:
- **Foreign PEP**: the person holds (or held) a PEP role in a country other than the institution's jurisdiction. FATF treats foreign PEPs as higher inherent risk — they are subject to automatic EDD.
- **Domestic PEP**: the person holds (or held) a PEP role in the same jurisdiction as the institution. Risk-based approach applies; many institutions treat domestic PEPs as CDD-minimum with risk-based uplift, though EDD is required where risk factors combine.

## Time horizon

Former PEPs remain in scope for **at least 12 months** after leaving office. Longer retention applies when:
- The jurisdiction warrants elevated concern (high corruption risk, weak rule of law).
- The individual's risk profile otherwise warrants it (connected sanctioned entities, adverse media).
- The institution's risk appetite policy specifies a longer period (common: 24-36 months for Category 1).

## Workflow

### Step 1 — Compile the natural-person list

From `workflow-kyc-beneficial-ownership` and the customer questionnaire, list every natural person to be screened:
- All UBOs (≥25% ownership or control test).
- All directors and board members.
- All authorised signatories.
- All key management personnel disclosed in onboarding documents.

### Step 2 — PEP database check

Screen each person against PEP databases. As no single universal PEP database exists, use the institution's standard vendor (World-Check, Refinitiv, Dow Jones Risk & Compliance, or equivalent) supplemented by:
- Government gazette lookups for senior roles.
- Parliament/legislature registers.
- Central bank and senior public official registries.

### Step 3 — Classify each confirmed PEP

For each person where PEP status is confirmed:
- Assign category (1, 2, 3, or 4).
- Determine domestic vs foreign relative to the institution's jurisdiction.
- Record: role held, country of office, start/end date, date-since-office calculation.
- Determine if the person is a current PEP or former PEP; apply the 12-month minimum time horizon.

### Step 4 — EDD trigger

Any confirmed PEP (current or former within the time horizon):
- Triggers EDD tier regardless of KYC risk score.
- Requires source-of-wealth evidence at two-source corroboration standard.
- Requires MLRO approval before onboarding completion.

Category 4 (family members and close associates): screen for PEP connection and apply risk-based judgement. Where a Category 4 person has direct access to PEP assets or influence, treat as EDD trigger.

### Step 5 — Source-of-wealth standard for confirmed PEPs

Confirmed PEPs require source-of-wealth documentation at a minimum two-source-cross-check standard:
- Source 1: declared lifetime wealth narrative (employment history, business ownership, inheritance).
- Source 2: at least two independent corroborating documents (tax returns, audited financials, sale-of-business records, inheritance documents, prior banking references).

Declared-only SoW is non-compliant for PEPs.

### Step 6 — Produce the PEP screening table

Document every screened natural person, including those confirmed as non-PEP.

## Output format

```
PEP SCREENING TABLE
--------------------
Screening date: [DD-Mon-YYYY]

| Person | Role at customer | PEP status | Category | Domestic/Foreign | Role held | Country | In-office | Date-since-office | EDD trigger | SoW standard |
|--------|-----------------|------------|----------|-----------------|-----------|---------|-----------|------------------|-------------|--------------|
| Alice Smith | UBO (33%) | No PEP | — | — | — | — | — | — | No | CDD |
| Bob Jones | UBO (27%) | Yes — Current | 2 | Foreign (UAE → UK institution) | Member of Parliament | UAE | 2022-present | N/A | YES | EDD — 2-source SoW |
| Carol White | Control (GP) | Yes — Former | 1 | Foreign (US → UK institution) | Deputy Minister | US | 2018-2022 | 4 years | YES (>12m) | EDD — 2-source SoW |

OVERALL PEP DETERMINATION: PEPs identified — EDD mandatory for Bob Jones, Carol White
```

## Quality gates

- Every natural person listed in the screening scope is checked — no skipped individuals.
- Confirmed PEPs have category, domestic/foreign classification, in-office dates, and date-since-office documented.
- Former PEPs within the 12-month window are treated as active PEPs.
- Every PEP triggers EDD — no exceptions based on risk score or other factors.
- Category 4 persons are documented and given a risk-based determination.
- Non-PEP determinations are documented (not omitted) — "no PEP found" must appear in the record.

## Related skills

- `workflow-kyc-customer-intake` — calls this skill in Step 5 of the master onboarding pipeline
- `workflow-kyc-beneficial-ownership` — provides the natural-person list to be screened
- `workflow-kyc-source-of-funds` — PEP confirmed → EDD standard applies to SoF and SoW

## Routing

**Primary agent:** `cfa-esg-regulatory-analyst`
