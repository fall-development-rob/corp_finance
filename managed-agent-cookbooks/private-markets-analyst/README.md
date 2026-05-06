# private-markets-analyst cookbook

Headless deployment of the CFA Private Markets Analyst managed agent.

## What it does

Runs institutional-grade private markets analysis on demand: LBO modelling,
IRR/MOIC attribution, sources & uses, debt schedules, GP/LP waterfall
distributions, merger accretion/dilution, venture capital dilution analysis,
infrastructure project finance, CLO analytics (waterfall, OC/IC, tranche),
and fund of funds J-curve/pacing. All numbers sourced from cfa-core MCP tools.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-pm-deal-reader` | Haiku | FMP + cfa-core comps/credit, read-only |
| `cfa-pm-modeler` | Sonnet | cfa-core compute (31 tools) |
| `cfa-pm-memo-writer` | Haiku | Write `./out/<deal-name>-*.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/private-markets-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-core/`,
`.claude/skills/workflow-private-equity/`, and
`.claude/skills/workflow-investment-banking/`.

## Deploy

```bash
cargo run -p corp-finance-cli -- managed-agent validate private-markets-analyst
scripts/deploy-managed-agent.sh private-markets-analyst --dry-run
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh private-markets-analyst --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |
| `FMP_MCP_URL` | URL of the FMP market data MCP server |

## Security tier: Tier 2 — Internal Research

- deal-reader and modeler subagents are read-only (no file writes)
- memo-writer subagent has `write` toolset scoped to `./out/` only
- No credentials in manifests — injected via environment variables
- LBO/returns models enforce base/bull/bear scenarios as per agent spec
