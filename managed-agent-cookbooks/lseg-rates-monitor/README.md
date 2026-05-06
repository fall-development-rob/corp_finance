# lseg-rates-monitor cookbook

Headless deployment of the CFA Fixed Income Rates Monitor managed agent.

## What it does

Runs daily institutional-grade rates monitoring using the LSEG vendor MCP for
yield curves, FX, and macro data, and the cfa-core MCP for bond pricing,
duration/convexity, Nelson-Siegel and Svensson curve fitting, swap valuation,
FX forward pricing, repo analytics, and scenario analysis. Produces a curve
snapshot, key spread table, and rates commentary. All numbers sourced from tool
calls — never from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-rates-reader` | Haiku | LSEG yield curves + FX + macro pulls, read-only |
| `cfa-rates-analyst-worker` | Sonnet | cfa-core compute (bond, duration, curves, swap, FX, repo, scenarios) |
| `cfa-rates-publisher` | Haiku | Write `./out/rates-monitor-<date>.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/fixed-income-analyst.md`.
Skills loaded from `.claude/skills/vendor-lseg/` and
`.claude/skills/corp-finance-analyst-markets/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate lseg-rates-monitor

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh lseg-rates-monitor --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh lseg-rates-monitor --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `VENDOR_MCP_URL` | URL of the vendor MCP server (LSEG access) |

## Security tier: Tier 2 — Internal Research

- rates-reader and analyst subagents are read-only (no file writes)
- publisher subagent has `write` toolset scoped to `./out/` only
- LSEG credentials (LSEG_CLIENT_ID, LSEG_CLIENT_SECRET) are owned by the vendor MCP server, not by this cookbook
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
