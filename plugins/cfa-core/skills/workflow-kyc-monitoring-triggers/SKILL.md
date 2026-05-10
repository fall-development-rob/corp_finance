---
name: "workflow-kyc-monitoring-triggers"
description: |
  WHAT: Ongoing KYC monitoring configuration and periodic review scheduling — defines event-driven trigger conditions (ownership change, jurisdiction change, sanctions list update, adverse media), transaction monitoring thresholds, FATF-aligned periodic review cadence (low 36m / medium 24m / high 12m), and alert remediation workflow including SAR/STR filing triage.
  WHEN: Invoke at onboarding completion to set the monitoring rules and review date, and whenever an event-driven trigger fires (ownership change, sanctions update, adverse media, large transaction, PEP detection post-onboarding).
---

# KYC — Ongoing Monitoring Triggers and Periodic Review

## What this skill covers

Configuration of the post-onboarding monitoring framework for each KYC file: event-driven trigger definitions, transaction monitoring thresholds, periodic review cadence by DD tier, and the alert remediation workflow (triage, investigation, disposition, SAR/STR filing triage).

## Core principle

KYC is ongoing, not one-shot. A clean screening at onboarding does not mean a clean screening at year two. Sanctions lists, PEP registries, ownership structures, and jurisdictions all change — monitoring must be systematic and documented.

## Periodic review cadence (FATF-aligned)

| DD tier at onboarding | Default review interval |
|----------------------|------------------------|
| SDD | 36 months |
| CDD | 24 months |
| EDD | 12 months |

The review interval is the **maximum** — an event-driven trigger may require an earlier review at any time. After each periodic review, the interval restarts from the date of MLRO sign-off on the refreshed file.

## Event-driven trigger conditions

The following events require re-screening and re-scoring before the next scheduled review:

| Trigger | Action required |
|---------|----------------|
| Ownership change | Re-run `workflow-kyc-beneficial-ownership`; re-screen new UBOs; re-run `kyc_risk_assessment` |
| Jurisdiction change | Re-run country risk for the new jurisdiction; re-score; uplift DD tier if warranted |
| Sanctions list update | Re-run `workflow-kyc-sanctions-screening` for all parties; resolve any new potential matches |
| Adverse media hit | Triage severity; investigate; document; escalate to MLRO if material |
| PEP detection post-onboarding | Confirm PEP status; trigger EDD uplift; run `workflow-kyc-pep-screening`; MLRO sign-off |
| Large or unusual transaction | Compare to expected transaction profile in KYC file; investigate if >2× stated volume or unexpected geography |
| Regulator inquiry | Treat as immediate priority; produce refreshed KYC file within timeline specified by regulator |

## Transaction monitoring thresholds

At onboarding, set transaction monitoring parameters based on the customer's stated profile:

| Parameter | SDD default | CDD default | EDD default |
|-----------|------------|-------------|-------------|
| Single-transaction threshold | 3× stated average | 2× stated average | 1.5× stated average |
| Periodic volume threshold (monthly) | >150% of stated monthly volume | >125% | >110% |
| New geography alert | Any country not in stated geographic profile | Same | Same |
| Cash or cash-equivalent transactions | $10,000 (or local equivalent) | $10,000 | $5,000 |

These are starting defaults — MLRO may adjust based on customer specifics.

## Periodic review workflow

When a scheduled review date is reached or an event-driven trigger fires:

### Step 1 — Re-collect updated documents
- Refresh proof of address (must be <3 months old for re-verification).
- Refresh ID if the document has expired.
- Confirm the UBO chain is unchanged or capture any updates.

### Step 2 — Re-screen
- Re-run `workflow-kyc-sanctions-screening` against current lists (lists are dynamic — a prior clear is not a current clear).
- Re-run `workflow-kyc-pep-screening` for all natural persons.
- Re-run `country_risk_assessment` for all relevant jurisdictions if jurisdiction risk classification may have changed (FATF greylist updates, new sanctions regimes).

### Step 3 — Re-score
- Call `kyc_risk_assessment` with the current data.
- If the score has changed materially (>15 points), assess whether a tier change is warranted.

### Step 4 — Re-tier if warranted
- Uplift or downgrade the DD tier based on changed circumstances.
- Tier upgrades (e.g., CDD → EDD): apply the new tier's evidence requirements retroactively. Collect missing SoW corroboration before the refreshed file is complete.
- Tier downgrades (e.g., EDD → CDD): require MLRO approval and must be documented with rationale.

### Step 5 — Update monitoring rules and next review date
- Confirm or adjust transaction thresholds based on any change in transaction profile.
- Update the next review date to the appropriate interval from today.

### Step 6 — MLRO sign-off
- Submit the refreshed KYC file for MLRO sign-off.
- MLRO signs off on any tier change, any disposition of adverse-media findings, and any true-positive determination.

## Alert remediation workflow

When an alert fires (sanctions hit, adverse media, unusual transaction):

### Step 1 — Alert triage
Classify: true positive (confirmed match or confirmed AML activity), false positive, or material new information requiring investigation.

### Step 2 — Investigation
Document:
- Alert source (transaction monitoring system, sanctions list update, media source).
- The specific alert details.
- The customer's response or provided evidence.
- Any third-party verification obtained.

### Step 3 — Disposition

| Determination | Action |
|--------------|--------|
| Cleared — false positive | Document the differentiating attribute; obtain MLRO clearance; close the alert |
| Cleared — resolved without filing | Document mitigation evidence; MLRO clearance; close the alert |
| SAR/STR filing required | File within jurisdictional deadline: 24 hours (terrorism financing) or 30 days (other); tipping-off prohibition applies |
| Off-boarding required | Initiate exit workflow; freeze account if sanctions match confirmed |

### Step 4 — Documentation
Record: alert ID, date, alert source, investigation steps, third-party evidence, disposition, MLRO sign-off reference, and filing reference if applicable.

## Output format

```
MONITORING CONFIGURATION
------------------------
Customer: [name] | DD tier: [SDD/CDD/EDD] | Onboarding date: [DD-Mon-YYYY]

PERIODIC REVIEW SCHEDULE
Next review date: [DD-Mon-YYYY]
Review interval: [12 / 24 / 36 months]

TRANSACTION MONITORING THRESHOLDS
| Parameter | Threshold |
|-----------|-----------|
| Single transaction | $[x] or [n]× stated average |
| Monthly volume | [n]% of stated monthly profile |
| New geography | Alert on any country not in: [list of stated geographies] |
| Cash / cash-equivalent | $[x] |

EVENT-DRIVEN TRIGGERS (active)
| Trigger | Status | Last checked | Action if fired |
|---------|--------|--------------|----------------|
| Sanctions list update | Active | [date] | Re-screen all parties |
| Adverse media | Active | [date] | Triage + investigate |
| Ownership change | Active | [date] | Re-run UBO verification |
| Jurisdiction change | Active | [date] | Re-run country risk |
| PEP detection | Active | [date] | Trigger EDD |
```

## Quality gates

- Periodic review interval set to FATF standard for the DD tier — no longer interval without MLRO approval.
- Event-driven triggers documented with explicit action definitions — not generic "review if something changes."
- Alert remediation produces a documented disposition for every alert — no unresolved or undocumented alerts.
- SAR/STR filing deadlines tracked per jurisdiction — 24h (terrorism financing), 30 days (other).
- Tier downgrades require explicit MLRO approval with documented rationale.
- Re-screening at periodic review re-runs actual tools — not a paper-shuffle.

## Related skills

- `workflow-kyc-customer-intake` — calls this skill in Step 10 to configure monitoring at onboarding
- `workflow-kyc-sanctions-screening` — re-run at every periodic review and on sanctions list update trigger
- `workflow-kyc-pep-screening` — re-run at every periodic review and on PEP detection trigger
- `workflow-kyc-beneficial-ownership` — re-run when ownership change trigger fires

## Routing

**Primary agent:** `cfa-esg-regulatory-analyst`
