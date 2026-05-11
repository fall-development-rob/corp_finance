---
name: "workflow-kyc-customer-intake"
description: |
  WHAT: End-to-end KYC customer onboarding pipeline that collects identification documents and questionnaire data, coordinates UBO verification, runs `kyc_risk_assessment` (FATF 5-dimension scoring), determines the DD tier (SDD/CDD/EDD), and produces an auditable KYC file ready for MLRO sign-off.
  WHEN: Invoke when onboarding a new investor (individual, corporate, fund-of-funds, trust, foundation) or a new counterparty (prime broker, lender, vendor, distribution partner) and a complete risk-rated KYC file must be produced.
---

# KYC — Customer Intake and Onboarding

## What this skill covers

The master onboarding pipeline that orchestrates all KYC sub-workflows into a single risk-rated, auditable KYC file. Coordinates identification document collection, UBO verification, sanctions/PEP screening, country risk classification, FATF-aligned risk scoring, DD tier determination, source of funds verification, and ongoing monitoring setup.

## Core principle

Documentation is the deliverable. A KYC file that cannot be reconstructed by an auditor is a failed file, even if the underlying conclusion is correct.

## Tool references

| Tool | Use |
|------|-----|
| `kyc_risk_assessment` | FATF 5-dimension risk scoring: customer type (25), geographic (25), product (20), transaction (15), source of wealth (15) |
| `sanctions_screening` | Levenshtein fuzzy matching against OFAC/EU/HMT/UN/FATF lists (see `workflow-kyc-sanctions-screening`) |
| `country_risk_assessment` | 12-factor sovereign risk + regulatory overlay (see `workflow-kyc-beneficial-ownership` and `workflow-kyc-monitoring-triggers`) |

## Document collection requirements

| Customer type | Required documents |
|---------------|--------------------|
| Individual | Government-issued photo ID, proof of address (<3 months old: utility bill/bank statement), tax ID |
| Corporate | Certificate of incorporation, register of directors, register of shareholders, articles, proof of registered address |
| Trust | Trust deed, list of trustees/settlors/beneficiaries, registered address |
| Fund | PPM, investment management agreement, regulator registration evidence |

## Workflow

### Step 1 — Collect identification documents

Gather all required documents per the customer type table above. Record:
- Document type, issuing authority, issue/expiry date, document reference.
- Any document expired or missing → intake is blocked until resolved.

### Step 2 — Collect KYC questionnaire

Capture from the customer:
- Customer type (individual / corporate / fund / trust).
- Primary jurisdiction of residence/incorporation.
- Business activity or occupation.
- Expected investment size and frequency.
- Source of funds (origin of the specific transaction).
- Source of wealth (origin of total accumulated wealth — required for CDD and EDD).
- Transaction profile: frequency, channel (wire/custody), geographic counterparties.

### Step 3 — Identify UBOs and run UBO Verification

For every entity customer: invoke `workflow-kyc-beneficial-ownership` for each natural person owning ≥25% directly or via aggregated indirect ownership, and for every controller (board majority, contractual veto, special voting share).

The highest individual UBO risk rating drives entity-level uplift.

### Step 4 — Sanctions screening

Invoke `workflow-kyc-sanctions-screening` for: the customer entity, every UBO, every director, and every authorised signatory.
A true positive blocks onboarding regardless of all other factors.

### Step 5 — PEP screening

Invoke `workflow-kyc-pep-screening` for every UBO, director, and authorised signatory.
A confirmed PEP triggers EDD regardless of risk score.

### Step 6 — Country risk classification

For the customer's primary jurisdiction, registered office jurisdiction, and the jurisdiction of every UBO: assess country risk as per the country risk workflow embedded in `workflow-kyc-beneficial-ownership`.

### Step 7 — KYC risk scoring

Call `kyc_risk_assessment` with the assembled data. The tool applies FATF 5-dimension scoring:

| Dimension | Weight | Inputs |
|-----------|--------|--------|
| Customer type | 25 | Individual vs corporate vs fund; PEP flag |
| Geographic | 25 | Primary jurisdiction FATF status, sanctions regime |
| Product | 20 | Investment product, complexity, liquidity |
| Transaction | 15 | Volume, frequency, channel, geographic reach |
| Source of wealth | 15 | Documented vs opaque; cross-corroborated vs single-source |

Score interpretation: <30 = low (SDD eligible); 30-70 = medium (CDD); >70 = high (EDD mandatory).

### Step 8 — DD tier determination

Apply the DD tier matrix:

| Tier | Trigger |
|------|---------|
| SDD | Score <30, low-risk jurisdiction, regulated counterparty (e.g., listed bank) |
| CDD | Default (30-70), neutral jurisdiction, no PEP, no sanctions concerns |
| EDD | Score >70, high-risk jurisdiction, confirmed PEP, sanctions-adjacent, complex ownership structure |

The DD tier is the highest tier triggered by any single factor. PEP → EDD regardless of score. High-risk jurisdiction in UBO chain → EDD regardless of score.

### Step 9 — Source of funds / source of wealth verification

Invoke `workflow-kyc-source-of-funds` with the determined DD tier.

### Step 10 — Set up ongoing monitoring

Invoke `workflow-kyc-monitoring-triggers` to define monitoring rules and the next periodic review date.

### Step 11 — MLRO review and sign-off

Draft the KYC file. The MLRO posts the final decision (approve / reject / further review).

## Output format — KYC file

```
KYC FILE
---------
Customer name: [name]
Customer type: [individual / corporate / fund / trust]
Primary jurisdiction: [country]
Intake date: [DD-Mon-YYYY]
DD tier: [SDD / CDD / EDD]
MLRO clearance status: [pending / approved / rejected]

1. Identification documents [list with type, date, reference]
2. KYC questionnaire summary
3. UBO chain [natural persons ≥25% / control; per `workflow-kyc-beneficial-ownership`]
4. Sanctions screening table [per `workflow-kyc-sanctions-screening`]
5. PEP screening table [per `workflow-kyc-pep-screening`]
6. Country risk table [per jurisdiction]
7. KYC risk score [overall + per-dimension]
8. DD tier determination [tier + triggering factors]
9. SoF / SoW evidence list [per `workflow-kyc-source-of-funds`]
10. Ongoing monitoring rules and next review date [per `workflow-kyc-monitoring-triggers`]
11. MLRO sign-off block: "Prepared by: [date] | Reviewed by MLRO: | Decision: approve / reject / further review"
```

## Quality gates

- Every UBO ≥25% identified to natural-person level — no corporate UBO accepted as terminus.
- All four screening types run (sanctions + PEP + country risk + KYC risk score) before file is complete.
- DD tier cites the highest-triggering factor — not an averaged judgement.
- EDD files carry two-source SoF/SoW corroboration.
- MLRO sign-off precedes onboarding completion.
- File is reconstructable by a regulator from documented evidence alone.

## Related skills

- `workflow-kyc-beneficial-ownership` — UBO chain verification (Step 3)
- `workflow-kyc-sanctions-screening` — OFAC/EU/HMT/UN/FATF screening (Step 4)
- `workflow-kyc-pep-screening` — PEP categories 1-4 (Step 5)
- `workflow-kyc-source-of-funds` — SDD/CDD/EDD documentation tier (Step 9)
- `workflow-kyc-monitoring-triggers` — ongoing monitoring setup (Step 10)

## Routing

**Primary agent:** `cfa-esg-regulatory-analyst`
