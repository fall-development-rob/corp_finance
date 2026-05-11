<!-- Adapted from: plugins/vertical-plugins/private-equity/commands/ai-readiness.md (anthropics/financial-services) -->
---
description: Scan the portfolio for the highest-leverage AI opportunities
argument-hint: "[path to quarterly materials folder, or list of company names]"
---

# AI Readiness Assessment

Run an AI readiness scan across portfolio companies using the AI Readiness Assessment workflow from `workflow-private-equity`.

## What It Does

Evaluates each portfolio company through a three-gate screen (data availability, internal ownership, 30-day pilot feasibility), scores and ranks all opportunities by annualised EBITDA impact, identifies cross-portfolio replays where one solution addresses multiple companies, and produces an operating partner deliverable: a one-page executive summary plus per-company supporting detail.

## Agent

Routes to `cfa-private-markets-analyst`.

## Key Tools

`mcp__cfa-core__analyze_working_capital`, `mcp__cfa-core__build_sensitivity_grid`, `mcp__cfa-core__analyze_strategy`

## When to Use

- Quarterly portfolio reviews where technology value creation is on the agenda
- Annual planning cycles when building or refreshing the operating partner's value creation programme
- Pre-100-day planning immediately post-close, when identifying quick wins
- When the investment team wants to compare AI deployment progress across the portfolio

## Usage

Provide either a path to the quarterly materials folder or a list of company names. If neither is supplied, the agent will ask which companies to include and request their most recent quarterly financials.

```
/ai-readiness [path to quarterly materials or company names]
```

**Example inputs:**
- `/ai-readiness portfolio/Q1-2026/` — scan all companies with materials in the folder
- `/ai-readiness AcmeCo, BetaCorp, GammaTech` — named companies; agent will request financials
- `/ai-readiness` — interactive mode; agent asks for company list and materials

## Step-by-Step Pipeline

1. **Gather inputs**: collect the latest quarterly financials and operational KPIs for each company in scope
   - If materials are provided, parse revenue, EBITDA, headcount, and operational workflow descriptions
   - Call `mcp__cfa-core__analyze_working_capital` per company to establish operational efficiency baseline

2. **Apply three-gate evaluation**: for each company and each candidate AI use case, assess:
   - **G1 — Data Availability**: clean inputs accessible within 30 days at negligible cost?
   - **G2 — Internal Ownership**: named manager with authority, budget, and accountability confirmed?
   - **G3 — Pilot Feasibility**: 30-day pilot scoped with off-the-shelf tooling, no procurement blocker?
   - Record gate status: GO / WAIT (with named blocker) / PASS (with reason)

3. **Score and rank GO opportunities**: for each GO-status use case, score 1–5 on EBITDA impact, implementation speed, data readiness, and ownership strength; multiply to raw score; sort descending

4. **Quantify EBITDA impact**: call `mcp__cfa-core__build_sensitivity_grid` to stress-test each impact estimate across optimistic / base / conservative assumptions

5. **Identify cross-portfolio replays**: scan for use cases deployable across two or more companies with configuration-only changes; calculate combined impact and shared implementation cost saving

6. **Frame portfolio-level impact**: call `mcp__cfa-core__analyze_strategy` to position AI initiatives alongside other VCP levers; roll up total addressable EBITDA impact with a 60–70% capture probability applied to GO opportunities

7. **Produce deliverable**: one-page executive summary (portfolio snapshot, top 3 ranked opportunities, replay candidates, next steps) plus per-company sections (gate rationale, scored table, implementation sketch, risk flags) and portfolio roll-up

## Output Format

```
PORTFOLIO AI READINESS — [Date]
Companies assessed: N  |  GO: N  |  WAIT: N  |  PASS: N

TOP RANKED OPPORTUNITIES
Rank | Company   | Use Case           | EBITDA Impact | Speed    | Owner
1    | [Co]      | [Use case]         | $Xk ann.      | XX days  | [Name/TBC]
2    | [Co]      | [Use case]         | $Xk ann.      | XX days  | [Name/TBC]
3    | [Co]      | [Use case]         | $Xk ann.      | XX days  | [Name/TBC]

REPLAY CANDIDATES
[Use case] — applies to [Co A, Co B]: combined $Xk impact; shared implementation saves ~$Xk

PORTFOLIO ROLL-UP
Total addressable (GO): $Xm annualised EBITDA impact
Expected capture (60–70%): $Xm
Impact on portfolio EBITDA bridge: +X% on weighted portfolio EBITDA

NEXT STEPS
[Co A]: Kick off AP automation pilot — owner [Name], start [date], success metric [KPI]
[Co B]: Resolve G2 blocker (identify owner) by [date]; re-gate next quarter
```
