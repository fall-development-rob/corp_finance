---
name: cfa-managed-agent
description: "Deploy and manage CFA managed-agent cookbooks to the Anthropic Managed Agents API. Validate cookbooks, list by cost tier (free vs paid-vendor), audit skill coverage, assemble deployment payloads, and route handoff events. All logic in deterministic Rust (corp-finance-core::managed_agent); this skill is documentation only."
---

# CFA Managed Agent Tooling

You have access to the managed-agent cookbook tooling for CFA agents. **All work happens in deterministic Rust** in `crates/corp-finance-core/src/managed_agent/`. This skill describes the available surfaces — CLI, MCP tools, NAPI bindings — and when to use which. The skill itself contains no business rules; if you need to know what makes a cookbook valid, read the Rust source or call the validate tool.

## Cost-tier discovery — before anything else

CFA cookbooks are tagged by cost tier so users can run the system **without paying for any vendor data subscription**:

| Tier | Meaning |
|---|---|
| `core_only` | Runs against `cfa-core` MCP only. User supplies inputs as JSON. **Always free.** |
| `freemium` | `cfa-core` + free public data (FRED, EDGAR, FIGI, YF, WB, geopolitical) and/or FMP free tier. **No paid subscription required.** |
| `paid_vendor` | Requires LSEG, S&P Global, FactSet, Morningstar, Moody's, PitchBook, Aiera, or Daloopa subscription. User must supply credentials at deploy time. |

To enumerate cookbooks by tier, prefer the MCP tool over scraping the registry:

```
managed_agent_list({ tier: "core_only" })   # 5 cookbooks
managed_agent_list({ tier: "freemium" })    # 8 cookbooks
managed_agent_list({ tier: "paid_vendor" }) # 2 cookbooks
managed_agent_list({})                      # all 15, grouped
```

For users on the free tier, **never** suggest a `paid_vendor` cookbook unless they've explicitly indicated they have the vendor credentials. The 13 non-paid cookbooks cover equity research, IB pitch / merger / LBO, PE deal screening / IC memo / LP audit, wealth planning, KYC, GL recon, model audit, and earnings analysis end-to-end.

## MCP tools (preferred for in-conversation use)

All exposed by the `cfa-core` MCP server:

| Tool | What it does |
|---|---|
| `managed_agent_list` | Enumerate cookbooks by cost tier. Registry-only, no I/O. |
| `managed_agent_validate` | Schema-validate one cookbook. Checks slug allowlist, agent.json shape, system prompt + skill + subagent path resolution, steering examples. Returns per-check pass/fail. |
| `managed_agent_check_all` | Walk every cookbook under `cookbooks_root` and validate each. Returns aggregate counts + per-cookbook outcome. |
| `managed_agent_sync` | Coverage audit between cookbooks and skills. Flags missing skills (referenced but not on disk) and orphan skills (on disk but unreferenced). Does NOT copy files — skills are resolved at deploy time in our pattern. |
| `managed_agent_deploy` | Assemble a deploy-ready JSON payload for one cookbook: ${VAR} substitution from `env_vars`, system-prompt inlining, skill resolution. Returns orchestrator + subagent + skill payloads. The consumer POSTs to `/v1/agents`. |
| `managed_agent_orchestrate` | Route a single `handoff_request` event to a target cookbook. Validates event type and target slug against the allowlist. |

## CLI (for shell / CI use)

```bash
cfa managed-agent list                          # all cookbooks
cfa managed-agent list --tier=core-only         # zero-dependency cookbooks
cfa managed-agent list --tier=freemium
cfa managed-agent list --tier=paid-vendor

cfa managed-agent validate <slug>               # one cookbook
cfa managed-agent check-all                     # every cookbook
cfa managed-agent sync                          # cookbook ↔ skill coverage
cfa managed-agent deploy <slug>                 # assembled JSON to stdout
cfa managed-agent orchestrate <slug>            # stdin event-loop router
```

`scripts/test-cookbooks.sh` runs validate + dry-run deploy on every cookbook (CI smoke harness).

## When to use which surface

- **In a conversation, picking a cookbook for a user**: `managed_agent_list` MCP tool with the appropriate tier filter.
- **Validating a cookbook the user just edited**: `managed_agent_validate` MCP tool, or CLI `cfa managed-agent validate <slug>`.
- **CI gate**: `scripts/test-cookbooks.sh`.
- **Programmatic deploy from a Node host**: `managedAgentDeploy(JSON.stringify(input))` via the NAPI binding directly (no MCP overhead).

## Architecture invariant

There is one and only one source of truth for what a valid cookbook is: `crates/corp-finance-core/src/managed_agent/validate.rs`. The CLI, MCP tool, and NAPI binding are pure adapters. Do not duplicate validation rules in this skill, in cookbook READMEs, or anywhere else — call the validator.

## Where the cookbooks live

- Cookbook source: `managed-agent-cookbooks/<slug>/agent.json` + `subagents/*.json` + `steering-examples.json` + `README.md`
- Skill referenced via `from_skill: <slug>` resolves to `.claude/skills/<slug>/SKILL.md`
- System prompt referenced via `system.file` resolves first to cookbook-relative, then to `.claude/agents/cfa/<name>.md`
- Allowlist + tier registry: `crates/corp-finance-core/src/managed_agent/types.rs` (`COOKBOOK_REGISTRY` + `ALLOWED_SLUGS`)

## Skipped upstream items (and why)

- **chronograph, egnyte** vendor MCPs — closed-source vendor APIs we don't have keys for.
- **MS Teams add-in installer** — we are headless / CLI-only.

## Skipped because they belonged in the rules engine, not here

The upstream Python tooling shipped `validate-cookbook.py` and `check.py` as separate scripts. Both fold into one Rust function (`validate::validate_manifest`) and one batched walker (`validate::validate_all`). If you find yourself wanting to add a new validation rule, add it to `validate.rs` and write a Rust unit test, not a markdown checklist.
