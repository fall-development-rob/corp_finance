# model-builder cookbook

> **Tier:** `CoreOnly` — cfa-core (user supplies fundamentals JSON)

Headless deployment of the CFA Model Builder managed agent.

## What it does

Constructs institutional-grade financial models on demand: integrated
three-statement (IS / BS / CF with circular reference resolution), DCF
(FCFF) with WACC and terminal value, Monte Carlo DCF for stochastic
valuation, LBO with multi-tranche debt schedule, sensitivity grids on
key drivers, and base / bull / bear scenarios. All numbers sourced
from cfa-core and FMP MCP tools — never from LLM text generation.

Excel export is not available — output is delivered as a
markdown-tabular model summary at `./out/<ticker>-model.md`.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-model-data-reader` | Haiku | FMP fundamentals, read-only |
| `cfa-model-builder-worker` | Sonnet | cfa-core compute (17 tools) |
| `cfa-model-exporter` | Haiku | Write `./out/<ticker>-model.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/equity-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-core/` and
`.claude/skills/corp-finance-tools-core/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate model-builder

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh model-builder --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh model-builder --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `FMP_MCP_URL` | URL of the FMP market data MCP server |

## Security tier: Tier 2 — Internal Research

- data-reader and modeler subagents are read-only (no file writes)
- exporter subagent has `write` toolset scoped to `./out/` only
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
