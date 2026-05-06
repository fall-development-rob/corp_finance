# credit-analyst cookbook

> **Tier:** `Freemium` — cfa-core + FMP (free tier)

Headless deployment of the CFA Credit Analyst managed agent.

## What it does

Runs institutional-grade credit risk analysis on demand: full credit ratio
suite with synthetic rating, debt capacity sizing, covenant compliance testing,
Altman Z-Score distress screening, credit scorecard (WoE/IV), Merton structural
PD, intensity model hazard rates, PD calibration, CDS pricing, CVA/DVA,
credit portfolio VaR (Gaussian copula), and rating migration analysis.
All numbers sourced from cfa-core MCP tools.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-credit-data-reader` | Haiku | FMP financial data + Altman pre-screen, read-only |
| `cfa-credit-scorer` | Sonnet | cfa-core compute (15 tools) |
| `cfa-credit-reporter` | Haiku | Write `./out/<issuer>-credit-opinion.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/credit-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-core/`.

## Deploy

```bash
cargo run -p corp-finance-cli -- managed-agent validate credit-analyst
scripts/deploy-managed-agent.sh credit-analyst --dry-run
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh credit-analyst --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `FMP_MCP_URL` | URL of the FMP market data MCP server |

## Security tier: Tier 2 — Internal Research

- data-reader and credit-scorer subagents are read-only (no file writes)
- reporter subagent has `write` toolset scoped to `./out/` only
- Z-Score < 1.81 and covenant headroom < 15% trigger mandatory red flags per agent spec
- No credentials in manifests — injected via environment variables
