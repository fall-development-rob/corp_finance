# gl-reconciler cookbook

Headless deployment of the CFA GL Reconciler managed agent for fund-administration
general-ledger reconciliation.

## What it does

Runs institutional-grade GL reconciliation for a fund period: parses the GL extract,
classifies entries (cash, accruals, expenses, FX), independently recomputes fund
fees, NAV, GP economics, and withholding tax, runs variance and working-capital
analysis, then produces a break list with adjusted balances. All numbers sourced
from cfa-core MCP tools — never from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-gl-ledger-reader` | Haiku | GL extract parsing + entry classification, read-only |
| `cfa-gl-reconciler-worker` | Sonnet | cfa-core compute (fund fees, NAV, GP economics, variance, WHT) |
| `cfa-gl-publisher` | Haiku | Write `./out/<fund>-Q<n>-gl-recon.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/esg-regulatory-analyst.md`.
Skills loaded from `.claude/skills/workflow-fund-admin/` and
`.claude/skills/corp-finance-analyst-regulatory/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate gl-reconciler

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh gl-reconciler --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh gl-reconciler --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |

## Security tier: Tier 2 — Internal Research

- ledger-reader and reconciler subagents are read-only (no file writes)
- publisher subagent has `write` toolset scoped to `./out/` only
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
- Diagnose-only: this cookbook flags and root-causes breaks; controllers post adjustments
