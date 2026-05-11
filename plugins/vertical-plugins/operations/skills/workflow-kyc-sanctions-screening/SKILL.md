---
name: "workflow-kyc-sanctions-screening"
description: |
  WHAT: Sanctions screening of a customer entity, its UBOs, directors, and authorised signatories against OFAC SDN, EU consolidated, UK HMT, UN consolidated, and FATF blacklist using `sanctions_screening` with Levenshtein fuzzy matching (0-100 score); true/false positive determination with MLRO clearance; SAR filing obligation triage.
  WHEN: Invoke when onboarding any customer or counterparty (individual or entity), performing periodic KYC refresh, remediating a sanctions alert, or responding to a sanctions list update that may affect existing customers.
---

# KYC — Sanctions Screening

## What this skill covers

Sanctions screening of all relevant parties (entity, UBOs, directors, authorised signatories) against the five mandatory lists, fuzzy-match scoring, true/false positive determination, disposition decision, and SAR filing triage. A confirmed sanctions match is an absolute red line — it blocks onboarding regardless of all other factors.

## Tool references

| Tool | Use |
|------|-----|
| `sanctions_screening` | Levenshtein fuzzy matching against OFAC SDN, EU consolidated, UK HMT, UN consolidated, FATF blacklist; returns 0-100 match score per candidate |

## Lists screened

| List | Jurisdiction | Scope |
|------|-------------|-------|
| OFAC SDN | US | Individuals and entities; US-nexus mandatory |
| EU Consolidated | EU | All EU-member institutions; broad geographic scope |
| UK HMT | UK | Post-Brexit UK regime; often aligns with EU but diverges on Russia/Iran |
| UN Consolidated | Global | Al-Qaeda, ISIL/Da'esh, Taliban; universal jurisdiction |
| FATF Blacklist | Global | Jurisdictions subject to call for action (currently: Iran, North Korea, Myanmar, Russian Federation for EDD) |

Supplementary (optional): PEP-specific databases, adverse-media screening platforms, targeted financial sanctions from individual jurisdictions (OFAC sectoral, OFAC secondary sanctions).

## Screening inputs per party

| Party type | Required inputs |
|------------|----------------|
| Individual | Full name, all known aliases/maiden names, date of birth, country of citizenship/residence, nationality |
| Entity | Legal name, all trading names/DBAs, country of incorporation, registration number, LEI if available |

## Workflow

### Step 1 — Compile the screening list

Identify every party to be screened:
- The customer entity itself.
- Every UBO (natural persons from `workflow-kyc-beneficial-ownership`).
- Every director and authorised signatory.
- For funds: every general partner entity and controlling individual.

### Step 2 — Call `sanctions_screening`

For each party, call `sanctions_screening` with the party's full inputs. The tool applies Levenshtein fuzzy matching against all five mandatory lists.

Output per party: list of candidates with match scores (0-100).

### Step 3 — Score interpretation and triage

| Score range | Action |
|-------------|--------|
| 0-69 | Clear — document the clear result |
| 70-89 | Manual review required — compare attributes |
| 90-100 | High-likelihood true positive — escalate immediately |

### Step 4 — True / false positive determination

For every candidate with a score ≥70:
1. Compare: full name, date of birth (for individuals), country of incorporation/residence, known aliases, and any unique identifiers (passport number, registration number, LEI).
2. Identify a specific, documented differentiating attribute that distinguishes the screened party from the list entry (e.g., different date of birth, different country, different middle name).
3. Classify:
   - **True positive**: no reliable differentiating attribute; party matches the list entry.
   - **False positive**: specific documented differentiating attribute exists; attributes confirm two different persons/entities.
   - **Borderline (ambiguous)**: score 70-89 with insufficient attribute data to distinguish → escalate to MLRO; do not proceed.

False positives must document the specific differentiating attribute — they cannot be discarded silently.

### Step 5 — Disposition

| Determination | Action |
|--------------|--------|
| True positive | Block onboarding / freeze relationship immediately. File SAR within 24 hours (terrorism financing) or 30 days (other) per jurisdiction. Do not tip off the customer (tipping-off prohibition applies). |
| False positive | Document the differentiating attribute. Obtain MLRO clearance. Clear in the KYC file. |
| Borderline | Escalate to MLRO. Do not proceed until MLRO provides clearance or escalates further. |
| Clear | Record clean result in the KYC file. |

### Step 6 — Produce the screening table

For every screened party, produce an entry in the output table regardless of result (clean results must be documented, not omitted).

## Output format

```
SANCTIONS SCREENING TABLE
--------------------------
Screening date: [DD-Mon-YYYY]
Lists screened: OFAC SDN, EU Consolidated, UK HMT, UN Consolidated, FATF Blacklist

| Party | Party type | List | Top match | Match score | Candidate DOB/Reg | List entry DOB/Reg | Differentiating attribute | Disposition | MLRO ref |
|-------|-----------|------|-----------|-------------|------------------|-------------------|--------------------------|-------------|----------|
| Alice Smith | UBO | OFAC SDN | Alice Smith (Iran) | 82 | DOB: 1978-03-14, UK | DOB: 1975-11-02, IR | Different DOB and nationality | False positive | MLRO-2026-0312 |
| Bob Jones | UBO | All lists | — | <70 | — | — | — | Clear | — |
| Customer Corp | Entity | All lists | — | <70 | — | — | — | Clear | — |

OVERALL DISPOSITION: Clear / Blocked / Pending MLRO
```

## Quality gates

- Every party (entity + all UBOs + all directors + all signatories) screened — no skipped parties.
- Every screened party has a result in the output table, including clean results.
- Score ≥70 always triggers manual attribute comparison — no score-only verdicts.
- False positives document the specific differentiating attribute — "different person" is not sufficient.
- Borderline cases go to MLRO — no unilateral clearance by the analyst.
- SAR filing triage documented for every true positive.

## Related skills

- `workflow-kyc-customer-intake` — calls this skill in Step 4 of the master onboarding pipeline
- `workflow-kyc-beneficial-ownership` — provides the list of UBOs to be screened
- `workflow-kyc-monitoring-triggers` — sanctions list updates are an event-driven monitoring trigger requiring re-screening

## Routing

**Primary agent:** `cfa-esg-regulatory-analyst`
