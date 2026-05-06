# valuation-reviewer cookbook

> **Tier:** `Freemium` — cfa-core + FMP (free tier)

Headless deployment of the CFA PE Valuation Reviewer managed agent.

## What it does

Runs institutional-grade valuation review on a GP-supplied PE valuation
package: per-portfolio-company independent mark-to-market via DCF, trading
comps, SOTP, LBO floor, sensitivity analysis, and H-Model DDM. Compares
each independent mark against the GP's reported fair value and flags
variance >10%. All numbers sourced from cfa-core and FMP MCP tools —
never from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-val-package-reader` | Haiku | FMP + cfa-core comps/DCF/credit, read-only |
| `cfa-val-reviewer` | Sonnet | cfa-core compute (18 tools) |
| `cfa-val-publisher` | Haiku | Write `./out/<fund>-valuation-review.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/private-markets-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-core/` and
`.claude/skills/workflow-private-equity/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate valuation-reviewer

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh valuation-reviewer --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh valuation-reviewer --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `FMP_MCP_URL` | URL of the FMP market data MCP server |

## Security tier: Tier 2 — Internal Research

- package-reader and reviewer subagents are read-only (no file writes)
- publisher subagent has `write` toolset scoped to `./out/` only
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
- Independent marks must be derived solely from cfa-core MCP tools; GP
  text or commentary alone is never sufficient
