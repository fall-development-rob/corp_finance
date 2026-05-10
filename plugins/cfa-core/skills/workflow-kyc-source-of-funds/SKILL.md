---
name: "workflow-kyc-source-of-funds"
description: |
  WHAT: Source-of-funds (SoF) and source-of-wealth (SoW) documentation tiering for KYC due diligence — defines evidence requirements at SDD, CDD, and EDD levels, distinguishes SoF (origin of the specific transaction) from SoW (origin of total accumulated wealth), and specifies two-source corroboration for EDD including corporate UBO and trust-beneficiary special cases.
  WHEN: Invoke after the DD tier has been determined (SDD/CDD/EDD) and the SoF/SoW evidence package must be assembled and documented before MLRO sign-off.
---

# KYC — Source of Funds and Source of Wealth

## What this skill covers

Structuring and documenting the source-of-funds (SoF) and source-of-wealth (SoW) evidence package for each DD tier. Defines what evidence is required, at what standard, and for which parties. Ensures EDD files achieve the two-source corroboration standard required for FATF compliance and MLRO sign-off.

## Core distinction

| Concept | Definition |
|---------|------------|
| Source of Funds (SoF) | The origin of the **specific transaction or investment** being made. Where did this money come from immediately before it arrived? (e.g., sale of a property, redemption of a prior fund, salary/bonus payment) |
| Source of Wealth (SoW) | The origin of the individual's or entity's **total accumulated wealth**. How did they accumulate everything they own over their lifetime? (e.g., entrepreneurship, inheritance, executive career, investment returns) |

Both are required for EDD. SoF alone is insufficient for high-risk customers.

## DD tier evidence matrix

| Tier | SoF standard | SoW standard |
|------|-------------|-------------|
| SDD | Self-declared only (no supporting document required) | Not required |
| CDD | Declared origin + at least one supporting document (bank statement showing inbound wire, sale contract, salary slip/payroll record, dividend confirmation) | Declared origin sufficient (employment / inheritance / business sale) — no supporting document required |
| EDD | Two-source corroboration: (1) declared origin in writing + (2) at least two independent supporting documents | Two-source corroboration: lifetime wealth narrative + at least two independent supporting documents |

## EDD SoW two-source standard — acceptable evidence

Each must be from an independent source (not self-prepared by the customer):

| SoW claim | Primary document | Corroborating document |
|-----------|-----------------|----------------------|
| Employment / executive career | Employment contract or letter confirming seniority and remuneration | Tax returns (last 2-3 years) or payroll records |
| Business ownership and sale | Sale-of-business agreement (SPA) | Audited accounts of the business at time of sale |
| Inheritance | Will, grant of probate, or notarised inheritance document | Estate valuation report or solicitor's confirmation |
| Investment returns | Fund or brokerage statement showing exit proceeds | Corresponding entry confirmation or original investment record |
| Real estate sale | Conveyancing completion statement | Land registry or title transfer document |
| Prior banking wealth | Bank reference letter (from correspondent bank) | Investment portfolio statement |

The customer's own declarations, internal documents, or documents prepared specifically for this KYC process do not count as independent corroboration.

## Special cases

### Corporate UBO source of wealth

For corporate UBOs (natural persons who are UBOs of a corporate customer):
- SoW traces to the **UBO's personal wealth**, not the entity's revenue or assets.
- Required: how did this individual accumulate the capital that allowed them to own/control this entity?
- Acceptable: salary/bonus history, prior business sale, inherited wealth.
- Not acceptable: "wealth = company value" without tracing the individual's personal contribution.

### Trust beneficiaries

For trust structures, SoW traces to the **settlor's wealth** — the person who originally contributed assets to the trust:
- Required: settlor's SoW at EDD standard where the trust is an EDD trigger.
- For discretionary trusts with multiple beneficiaries: screen each beneficiary individually; EDD applies if any beneficiary is a PEP or high-risk.

## Workflow

### Step 1 — Confirm the DD tier

Receive the DD tier determination from `workflow-kyc-customer-intake` Step 8. This sets the evidence standard.

### Step 2 — Collect SoF declaration

Request the customer's written declaration of the origin of the specific investment or transaction:
- What is the nature of the funds? (business income, property sale, redemption of prior investment, inheritance, gift)
- When were the funds received and from what source/counterparty?
- What account did the funds come from? (provide bank name and account holder confirmation)

### Step 3 — Collect SoF supporting documents

Per the tier matrix:
- **SDD**: declaration suffices. Proceed.
- **CDD**: collect at least one supporting document (bank statement, sale contract, salary slip, redemption confirmation). The document must show the specific funds entering the customer's account.
- **EDD**: collect at least two independent supporting documents for the SoF. Both must be from sources independent of the customer.

### Step 4 — Collect SoW declaration and evidence (CDD and EDD)

Request the SoW narrative:
- A chronological account of wealth accumulation (career history, businesses founded or acquired, major asset sales, inheritance events).
- Approximate wealth breakdown (what % from employment, business, investment, inheritance, etc.).

For **EDD**: collect at least two independent corroborating documents per the evidence matrix above.

### Step 5 — Assess adequacy and flag gaps

Review the evidence package:
- Is the declared origin plausible given the customer's stated background?
- Do the supporting documents directly corroborate the declared origin?
- Are there any unexplained gaps (e.g., large wealth event with no documentary trace)?

Gap = a material wealth element that cannot be corroborated by any available document. Flag to MLRO for judgement on whether the gap is explainable or blocks onboarding.

### Step 6 — Produce the SoF/SoW evidence list

Compile the evidence register for the KYC file.

## Output format

```
SOURCE OF FUNDS / SOURCE OF WEALTH EVIDENCE REGISTER
------------------------------------------------------
Customer: [name] | DD tier: [SDD / CDD / EDD]

SOURCE OF FUNDS
Declared origin: [description]
Supporting documents:
  1. [Document type, issuing party, date, reference]
  2. [Document type, issuing party, date, reference]  ← EDD only
Adequacy: Sufficient / Gap identified

SOURCE OF WEALTH (CDD and EDD)
Declared narrative: [summary]
Corroborating documents:
  1. [Document type, issuing party, date, reference]
  2. [Document type, issuing party, date, reference]  ← EDD — second corroborating doc
Two-source standard met: Yes / No — if No, describe gap
Adequacy: Sufficient / Gap identified — [describe gap if applicable]

MLRO NOTES: [any flags for MLRO judgement]
```

## Quality gates

- SDD: declaration-only is compliant — do not require supporting documents.
- CDD: at least one supporting document required — declaration-only is non-compliant.
- EDD: two independent supporting documents required for both SoF and SoW — one document is non-compliant.
- Customer-prepared documents do not count as independent corroboration.
- Corporate UBO SoW traces to the individual, not the entity — entity financials alone are non-compliant.
- Trust SoW traces to the settlor — beneficiary documentation alone is non-compliant.
- Gaps flagged explicitly to MLRO — not silently accepted or ignored.

## Related skills

- `workflow-kyc-customer-intake` — calls this skill in Step 9 after DD tier is determined
- `workflow-kyc-pep-screening` — PEP confirmation triggers EDD SoF/SoW standard automatically
- `workflow-kyc-monitoring-triggers` — material SoW change (e.g., new wealth event, inheritance) is an event-driven monitoring trigger

## Routing

**Primary agent:** `cfa-esg-regulatory-analyst`
