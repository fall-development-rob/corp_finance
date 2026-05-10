---
name: "workflow-kyc-beneficial-ownership"
description: |
  WHAT: Ultimate Beneficial Owner (UBO) verification — tracing ownership chains to natural persons at the ≥25% threshold or control test, collecting per-UBO identification, running country risk assessment via `country_risk_assessment`, and producing a composite per-UBO risk rating that drives entity-level DD tier uplift.
  WHEN: Invoke when onboarding any corporate entity, trust, fund-of-funds, or other non-natural-person customer and beneficial ownership must be established before sanctions, PEP, and risk scoring can proceed.
---

# KYC — Beneficial Ownership Verification

## What this skill covers

Tracing the ownership and control structure of a customer entity through parent layers to identify every natural person who qualifies as an Ultimate Beneficial Owner (UBO), collecting per-UBO identification and country risk data, and producing a composite UBO risk chain that feeds the entity-level KYC risk score.

## Core principle

Beneficial ownership is non-negotiable. Any natural person owning ≥25% (directly or via aggregated indirect ownership) or exercising effective control must be identified, verified, and individually risk-rated.

## Definitions

| Concept | Definition |
|---------|------------|
| UBO — ownership threshold | Any natural person with ≥25% direct or aggregated indirect equity interest |
| UBO — control test | Any natural person who controls the entity via board majority, contractual veto right, or special voting share, regardless of ownership percentage |
| Terminus | A natural person — the chain stops here. Corporate entities at any layer are not a terminus |
| Aggregated indirect ownership | Sum of ownership across multiple indirect paths (e.g., 15% via Entity A + 12% via Entity B = 27% → UBO) |

## Tool references

| Tool | Use |
|------|-----|
| `country_risk_assessment` | 12-factor sovereign risk model + FATF/sanctions regulatory overlay for each UBO's jurisdiction(s) |

## Workflow

### Step 1 — Trace the ownership chain

Starting from the customer entity, map every parent layer:
- Collect the register of shareholders (or equivalent) at each layer.
- Record: entity name, jurisdiction of incorporation, registration number, % ownership.
- Continue up each branch until a natural person is reached at every terminal node.

Document the full tree in an ownership diagram (text adjacency list is acceptable):

```
Customer Corp (primary entity)
├── 60% → HoldCo Ltd [Jersey]
│   ├── 55% → Alice Smith [individual — UBO: 0.60 × 0.55 = 33.0%]
│   └── 45% → Bob Jones [individual — UBO: 0.60 × 0.45 = 27.0%]
└── 40% → PE Fund LP [Cayman]
    └── GP: GreenPeak Capital LLC [Delaware — control test: yes]
        └── Controlling partner: Carol White [individual — control UBO]
```

### Step 2 — Apply the ≥25% ownership threshold

Calculate each natural person's aggregated indirect ownership across all paths. Any person at ≥25% is a UBO. Record:
- Name, date of birth, nationality, jurisdictions of citizenship and residence.
- Ownership percentage (direct or calculated indirect).
- Ownership path (which parent entities).

### Step 3 — Apply the control test

Identify any natural person who controls the entity regardless of their ownership percentage:
- Board majority control: a person who can appoint or remove a majority of directors.
- Contractual veto: a person whose consent is required for material decisions (identified from shareholder agreements, side letters, or partnership agreements).
- Special voting rights: a person holding a golden share, supervoting share, or analogous instrument.

All such persons are UBOs even if their equity interest is <25%.

### Step 4 — Collect per-UBO identification

For each UBO:
- Government-issued photo ID (passport preferred).
- Proof of address (<3 months: utility bill or bank statement).
- Tax identification number.
- Occupation and employer (or source of wealth narrative if private individual).
- Primary jurisdiction(s) of residence and citizenship.

### Step 5 — Per-UBO country risk assessment

For each UBO, call `country_risk_assessment` for every relevant jurisdiction (country of citizenship, country of residence, country of business activity if different).

The tool applies a 12-factor sovereign risk model (GDP growth, inflation, fiscal balance, debt/GDP, current account, FX reserves, political stability, rule of law, external debt, short-term debt/reserves, default history, dollarisation) and overlays:
- FATF blacklist (call for action) → prohibited / embargoed posture.
- FATF greylist (increased monitoring) → EDD trigger.
- Sanctions regimes: OFAC comprehensive, sectoral (Russia/Iran/etc.), regional (Hong Kong/Crimea/Donbas).
- Tax cooperation: OECD BEPS minimum standards, EU tax-haven list.

Composite jurisdiction rating: Low / Medium / High / Prohibited.

### Step 6 — Compute composite UBO risk rating

Per UBO, the composite risk rating is the highest of:
- Country risk rating (from Step 5).
- Sanctions screening result (from `workflow-kyc-sanctions-screening`).
- PEP determination (from `workflow-kyc-pep-screening`).

The highest individual UBO risk rating drives the entity-level uplift in the overall KYC risk score.

## Output format

```
UBO VERIFICATION REPORT
------------------------
Entity: [customer name]

OWNERSHIP CHAIN
[Adjacency list or indented tree]

UBO TABLE
| UBO name | Path | Ownership % / Control | Citizenship | Residence | Country risk | Sanctions | PEP | Composite rating |
|----------|------|-----------------------|-------------|-----------|--------------|-----------|-----|-----------------|
| Alice Smith | HoldCo Ltd | 33.0% | UK | UK | Low | Clear | No | Low |
| Bob Jones | HoldCo Ltd | 27.0% | UAE | UAE | Medium | Clear | No | Medium |
| Carol White | GreenPeak Capital LLC | Control (GP) | US | US | Low | Clear | No | Low |

ENTITY-LEVEL RISK UPLIFT: Medium (driven by Bob Jones — UAE residence, country risk Medium)
```

## Quality gates

- No corporate entity is accepted as a terminus — chain must reach natural persons.
- Aggregated indirect ownership computed across all paths — not just the largest path.
- Control test applied to all contractual documents reviewed (SHA, partnership agreement, side letters).
- Per-UBO country risk run for every jurisdiction of citizenship and residence — not just primary.
- Composite UBO risk rating is the maximum of country risk, sanctions, and PEP — not an average.

## Related skills

- `workflow-kyc-customer-intake` — master onboarding pipeline; calls this skill in Step 3
- `workflow-kyc-sanctions-screening` — sanctions check inputs into composite UBO risk rating
- `workflow-kyc-pep-screening` — PEP determination inputs into composite UBO risk rating

## Routing

**Primary agent:** `cfa-esg-regulatory-analyst`
