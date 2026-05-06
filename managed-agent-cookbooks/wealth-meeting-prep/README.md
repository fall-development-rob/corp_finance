# wealth-meeting-prep cookbook

> **Tier:** `Freemium` — cfa-core + FMP (free tier)

Headless deployment of the CFA Wealth Meeting Prep managed agent.

## What it does

Runs institutional-grade client meeting prep for wealth management:
portfolio drift assessment, mean-variance / Black-Litterman / risk-parity
rebalance proposals, tax-loss harvesting candidates, retirement readiness
projection, concentrated stock and wealth transfer planning. All numbers
sourced from cfa-core and FMP MCP tools — never from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-wealth-portfolio-reader` | Haiku | FMP holdings + cfa-core portfolio stats, read-only |
| `cfa-wealth-analyst-worker` | Sonnet | cfa-core compute (14 tools) |
| `cfa-wealth-brief-writer` | Haiku | Write `./out/<client>-meeting-brief.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/quant-risk-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-core/` and
`.claude/skills/workflow-wealth-management/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate wealth-meeting-prep

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh wealth-meeting-prep --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh wealth-meeting-prep --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `FMP_MCP_URL` | URL of the FMP market data MCP server |

## Security tier: Tier 2 — Internal Research

- portfolio-reader and analyst subagents are read-only (no file writes)
- brief-writer subagent has `write` toolset scoped to `./out/` only
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
