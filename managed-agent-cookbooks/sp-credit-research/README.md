# sp-credit-research cookbook

Headless deployment of the CFA Credit Research managed agent backed by the
S&P Global vendor MCP server.

## What it does

Runs institutional-grade credit research on demand: pulls S&P Global tearsheet,
ratings history, earnings call transcripts, and Capital IQ financials (with FMP
fallback for fundamentals), then computes the full credit ratio suite with
synthetic rating, debt capacity sizing, covenant compliance, Altman Z-Score,
Merton structural PD, logistic-regression scorecard, PD model validation, CDS
pricing, and CVA/DVA. Produces a rating opinion, covenant check, and spread view.
All numbers sourced from cfa-core MCP tools — never from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-sp-credit-data-reader` | Haiku | S&P vendor MCP + FMP fallback, read-only |
| `cfa-sp-credit-scorer` | Sonnet | cfa-core compute (credit metrics, debt capacity, PD models, CDS, CVA) |
| `cfa-sp-credit-publisher` | Haiku | Write `./out/<issuer>-credit-note.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/credit-analyst.md`.
Skills loaded from `.claude/skills/vendor-sp-global/` and
`.claude/skills/corp-finance-analyst-core/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate sp-credit-research

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh sp-credit-research --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh sp-credit-research --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `VENDOR_MCP_URL` | URL of the vendor MCP server (S&P Global access) |
| `FMP_MCP_URL` | URL of the FMP market data MCP server (fundamentals fallback) |

## Security tier: Tier 2 — Internal Research

- data-reader and credit-scorer subagents are read-only (no file writes)
- publisher subagent has `write` toolset scoped to `./out/` only
- S&P Global API key (SP_GLOBAL_API_KEY) is owned by the vendor MCP server, not by this cookbook
- Z-Score < 1.81 and covenant headroom < 15% trigger mandatory red flags per agent spec
- Synthetic rating divergence vs S&P actual rating is always disclosed
- No credentials in manifests — all injected via environment variables
