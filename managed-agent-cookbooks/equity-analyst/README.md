# equity-analyst cookbook

> **Tier:** `Freemium` — cfa-core + FMP (free tier)

Headless deployment of the CFA Equity Analyst managed agent.

## What it does

Runs institutional-grade equity research on demand:
DCF valuation, trading comps, earnings quality screening (Beneish, Piotroski,
accrual quality), dividend analysis (H-Model DDM, payout sustainability), and
12-month target price derivation. All numbers sourced from cfa-core and FMP
MCP tools — never from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-equity-data-reader` | Haiku | FMP data pull, read-only |
| `cfa-equity-analyst-worker` | Sonnet | cfa-core compute (20 tools) |
| `cfa-equity-publisher` | Haiku | Write `./out/<ticker>-research-note.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/equity-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-core/` and
`.claude/skills/workflow-equity-research/`.

## Deploy

```bash
# Validate manifest first
cargo run -p corp-finance-cli -- managed-agent validate equity-analyst

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh equity-analyst --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh equity-analyst --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `FMP_MCP_URL` | URL of the FMP market data MCP server |

## Security tier: Tier 2 — Internal Research

- data-reader and analyst subagents are read-only (no file writes)
- publisher subagent has `write` toolset scoped to `./out/` only
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
