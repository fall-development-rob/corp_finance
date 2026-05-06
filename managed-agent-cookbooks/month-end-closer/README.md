# month-end-closer cookbook

> **Tier:** `CoreOnly` — cfa-core only

Headless deployment of the CFA Month-End Closer managed agent for corporate close
orchestration.

## What it does

Runs institutional-grade month-end close workflow: trial balance variance analysis
vs prior period and budget, accrual schedules with draft journal entries,
working-capital roll-forward, three-statement articulation, scenario analysis on
flagged variances, break-even sensitivity, and DuPont decomposition for
management commentary. All numbers sourced from cfa-core MCP tools.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-close-tb-reader` | Haiku | Trial balance + prior period intake, read-only |
| `cfa-close-worker` | Sonnet | cfa-core compute (variance, working capital, three-statement, scenarios, break-even, DuPont) |
| `cfa-close-reporter` | Haiku | Write `./out/<entity>-<period>-close-pack.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/esg-regulatory-analyst.md`.
Skills loaded from `.claude/skills/workflow-fund-admin/` and
`.claude/skills/corp-finance-analyst-core/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate month-end-closer

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh month-end-closer --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh month-end-closer --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |

## Security tier: Tier 2 — Internal Research

- tb-reader and closer subagents are read-only (no file writes)
- reporter subagent has `write` toolset scoped to `./out/` only
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
- Draft-only output: this cookbook produces close packs for controller review; nothing is posted to the ledger
