# lp-statement-auditor cookbook

Headless deployment of the CFA LP Statement Auditor managed agent.

## What it does

Runs institutional-grade quarterly LP capital-account statement audit:
independent re-derivation of management fees, GP carry waterfall, NAV
with equalisation, investor TWR/IRR/DPI/RVPI/TVPI, J-curve cohort
position, and UBTI/ECI screening for tax-exempt LPs. Reconciles every
line of the GP-supplied LP statement and flags variance >50bps on fees
and >1% on NAV. All numbers sourced from cfa-core MCP tools — never
from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-lp-statement-reader` | Haiku | Read-only JSON parsing |
| `cfa-lp-auditor` | Sonnet | cfa-core compute (16 tools) |
| `cfa-lp-reporter` | Haiku | Write `./out/<fund>-Q<n>-lp-audit.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/private-markets-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-regulatory/` and
`.claude/skills/workflow-fund-admin/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate lp-statement-auditor

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh lp-statement-auditor --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh lp-statement-auditor --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |

## Security tier: Tier 2 — Internal Research

- statement-reader and auditor subagents are read-only (no file writes)
- reporter subagent has `write` toolset scoped to `./out/` only
- No internet access beyond the declared cfa-core MCP server
- No FMP/market-data MCP wired — fund admin operates on supplied
  statement JSON only, no live market lookups
- No credentials in manifests — all injected via environment variables
