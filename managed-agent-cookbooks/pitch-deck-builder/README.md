# pitch-deck-builder cookbook

> **Tier:** `Freemium` — cfa-core + FMP (free tier)

Headless deployment of the CFA Pitch Deck Builder managed agent.

## What it does

Assembles institutional-grade investment banking pitch decks on demand:
trading comps benchmarking, merger accretion/dilution, sum-of-the-parts
valuation, sensitivity analysis, and a slide-ready markdown outline.
All numbers sourced from cfa-core and FMP MCP tools — never from LLM
text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-pitch-data-reader` | Haiku | FMP fundamentals + cfa-core comps/credit, read-only |
| `cfa-pitch-modeler` | Sonnet | cfa-core compute (15 tools) |
| `cfa-pitch-deck-author` | Haiku | Write `./out/<target>-pitchdeck.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/private-markets-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-core/` and
`.claude/skills/workflow-investment-banking/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate pitch-deck-builder

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh pitch-deck-builder --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh pitch-deck-builder --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `FMP_MCP_URL` | URL of the FMP market data MCP server |

## Security tier: Tier 2 — Internal Research

- data-reader and modeler subagents are read-only (no file writes)
- deck-author subagent has `write` toolset scoped to `./out/` only
- No internet access beyond declared MCP servers
- No credentials in manifests — all injected via environment variables
