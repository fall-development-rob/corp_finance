# earnings-reviewer cookbook

> **Tier:** `Freemium` — cfa-core + FMP + data (FRED/EDGAR free)

Headless deployment of the CFA Earnings Reviewer managed agent.

## What it does

Produces institutional-grade quarterly earnings reaction notes on demand:
parses FMP transcripts and EDGAR 8-K / 10-Q filings, refreshes DCF
on updated guidance, runs the earnings-quality screening suite (Beneish
M-Score, Piotroski F-Score, accrual / revenue quality, composite,
red-flag scoring), runs sensitivity / scenario grids, and derives a
refreshed 12-month target price. All numbers sourced from cfa-core,
FMP, and the data MCP server — never from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-earnings-transcript-reader` | Haiku | FMP transcripts + EDGAR 8-K/10-Q, read-only |
| `cfa-earnings-analyst-worker` | Sonnet | cfa-core compute (15 tools) |
| `cfa-earnings-publisher` | Haiku | Write `./out/<ticker>-Q<n>-earnings-note.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/equity-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-core/` and
`.claude/skills/workflow-equity-research/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate earnings-reviewer

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh earnings-reviewer --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh earnings-reviewer --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `FMP_MCP_URL` | URL of the FMP market data MCP server |
| `DATA_MCP_URL` | URL of the data MCP server (EDGAR 8-K, 10-Q) |

## Security tier: Tier 2 — Internal Research

- transcript-reader and analyst subagents are read-only (no file writes)
- publisher subagent has `write` toolset scoped to `./out/` only
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
