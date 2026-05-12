# Phase 26 — `capital-raise-prep` Cookbook (Scoped)

**Status**: Queued (design only — not implementation-blocked)
**Author**: Captured 2026-05-12 from user prompt
**Owner**: TBD
**Estimated effort**: ~2 days, no new MCP servers

---

## 1. Why This Design Is Scoped

The original idea is a full 5-role institutional capital-raise team (Principal, BD
Director, BD Associate, IR Manager, Operations Lead). The full version requires CRM
(Affinity/DealCloud/Salesforce), email threading, calendar, and data-room state — none
of which exist as MCP servers in this repo. Building those connectors is a separate
phase.

This scoped version preserves the multi-agent shape but produces *prep packets*, not
*execute the raise*. Outputs are JSON/markdown deliverables — IC-style brief, KYC pack,
LP target list with reasoning, fee/structure comparison — that a human IR team can use
to drive an actual raise via their existing CRM.

The full CRM-integrated version stays on the roadmap for a future phase.

---

## 2. Cookbook Shape

```
managed-agent-cookbooks/capital-raise-prep/
  agent.yaml                          (chief: capital-raise-orchestrator)
  steering-examples.json
  subagents/
    principal-adjudicator.yaml        (opus, low-frequency final review)
    bd-architect.yaml                 (sonnet, ecosystem map)
    bd-scout.yaml                     (sonnet, target list + screening)
    ir-nurturer.yaml                  (sonnet, KYC/AML, data-room brief)
    ops-analyst.yaml                  (sonnet, fund economics synthesis)
```

The orchestrator handoff pattern matches existing cookbooks (e.g. `equity-analyst`):
chief dispatches to each subagent in turn, collects structured JSON, and the
Principal-Adjudicator subagent produces the final go/no-go recommendation.

## 3. Per-Subagent Tool Surface

| Subagent | Existing MCP tools (no new servers needed) |
|----------|---------------------------------------------|
| **Ops Analyst** | `gp_economics`, `fund_fee_calculator`, `investor_net_returns`, `nav_calculator`, `returns_calculator`, `j_curve_model`, `commitment_pacing` |
| **IR Nurturer** | `kyc_risk_assessment`, `sanctions_screening`, `entity_classification`, `jurisdiction_comparison`, `lux_ireland_fund_structure`, `cayman_fund_structure`, `singapore_vcc_structure`, `treaty_network`, `withholding_tax` |
| **BD Architect** | (data-mcp-server) `edgar_company_facts`, `edgar_filings`, `wb_country_indicators`, `acled_country_summary` — ecosystem context |
| **BD Scout** | (fmp-mcp-server) `fmp_company_profile`, `fmp_executive_compensation`, `fmp_ma_search`, `fmp_funding_round` — target screening |
| **Principal Adjudicator** | tools: read-only. No compute. Receives JSON from the other 4 and produces a go/no-go with reasoning. |

## 4. Inputs and Outputs

**Input** (steering example):
```json
{
  "fund": {
    "name": "Robotix PE Fund III",
    "vintage": "2026",
    "strategy": "lower-middle-market industrials",
    "target_size_usd": 250000000,
    "domicile": "Cayman",
    "gp_commit_pct": 2.0,
    "management_fee_pct": 2.0,
    "carry_pct": 20.0,
    "hurdle_pct": 8.0
  },
  "target_lp_profile": "north-american-family-offices",
  "outreach_window_weeks": 12
}
```

**Output schema** (parent agent):
```yaml
output_schema:
  type: object
  required:
    - executive_summary
    - lp_target_list
    - structure_comparison
    - kyc_pack
    - go_no_go
    - tool_calls
  properties:
    executive_summary: { type: string, maxLength: 4000 }
    lp_target_list:
      type: array
      maxItems: 50
      items:
        type: object
        properties:
          name: { type: string }
          tier: { type: string, enum: ["A", "B", "C"] }
          reasoning: { type: string, maxLength: 600 }
          warm_path: { type: string, maxLength: 200 }
    structure_comparison:
      type: object
      properties:
        cayman: { type: object }
        lux_scsp: { type: object }
        delaware_lp: { type: object }
    kyc_pack:
      type: object
      properties:
        sanctions_screening_completed: { type: boolean }
        flagged_entities: { type: array }
    go_no_go:
      type: string
      enum: ["proceed", "proceed-with-changes", "hold"]
    tool_calls:
      type: array
```

## 5. What This Cookbook Does Not Do

- Does **not** send emails, schedule meetings, or update a CRM.
- Does **not** route based on live LP engagement signals.
- Does **not** maintain post-close state for the next raise.
- Does **not** produce attorney-reviewed final docs.

All of the above belong to the full version (Phase 27+ if pursued).

## 6. Dependencies (None Block This Scope)

- Existing cookbook infrastructure: ✓ shipped (Phase 36/40/41)
- Tool-name lint and audit hash gates: ✓ shipped (Phase 25 Tier A1/A2)
- All needed MCP tools: ✓ present in cfa-core, data, fmp servers
- Output schema validator: ✓ shipped (Phase 39)

## 7. Wave Plan

| Wave | Description |
|------|-------------|
| W1 | Author `agent.yaml` + 5 subagent YAMLs with output schemas |
| W2 | `steering-examples.json` covering 3 scenarios (lower-MM, distressed credit, infrastructure) |
| W3 | Regenerate `data/cookbook-audits.json` and `data/tools-catalog.json` (both gates will catch this) |
| W4 | Add cookbook to `managed-agent-cookbooks/README.md` deployment table |
| W5 | Smoke test via existing `cookbook-runtime-integration.test.ts` pattern |

## 8. Open Questions

**8a. Naming.** `capital-raise-prep` vs `lp-outreach-prep` vs `fundraise-orchestrator`.
The "prep" suffix is honest about what the scoped version is; the unsuffixed name is
reserved for the full CRM-integrated version.

**8b. Adjudicator pattern.** This is the first cookbook where one subagent's role is to
review the others' outputs rather than compute. The harness already supports this via
`callable_agents` and the validator chain; no new infra needed.

**8c. Vertical-plugin home.** Likely a new `plugins/vertical-plugins/capital-formation/`
with associated skills (`workflow-cf-lp-targeting`, `workflow-cf-structure-comparison`,
`workflow-cf-kyc-pack`). Skill content is the agent-side prose; cookbook is the
deployment.

---

## Appendix: Where the Full Version Goes

A future Phase 27 (or similar) would add:

- New MCP servers: `crm-mcp-server` (Affinity/DealCloud/Salesforce), `comms-mcp-server`
  (email threading, calendar), `dataroom-mcp-server` (state, FAQ logging)
- Live engagement signals: who's clicked the data room, who's ghosted, who's asked the
  same question twice
- Stateful raise tracking across multiple weeks (not just a one-shot prep)
- The "Trust Tax" instrumentation: time between yes and wire, per LP

That phase is the actual product; this scoped version is the analytical spine that the
full version will build on.
