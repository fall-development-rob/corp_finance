# kyc-screener cookbook

> **Tier:** `CoreOnly` — cfa-core only

Headless deployment of the CFA KYC/AML Screener managed agent.

## What it does

Runs institutional-grade KYC/sanctions/PEP screening for prospective LPs
and counterparties: FATF 5-dimension risk scoring (customer / geographic
/ product / transaction / source-of-wealth), multi-list sanctions
screening (OFAC SDN, EU Consolidated, HMT UK, UN UNSC, FATF Grey/Black),
PEP classification, country risk grading, FATCA/CRS entity
classification, and SDD/CDD/EDD tier recommendation. All scores sourced
from cfa-core MCP tools — never from LLM text generation.

## Subagents

| Role | Model | Toolset |
|------|-------|---------|
| `cfa-kyc-identity-reader` | Haiku | Read-only JSON parsing |
| `cfa-kyc-screener-worker` | Sonnet | cfa-core compute (8 tools) |
| `cfa-kyc-reporter` | Haiku | Write `./out/<subject>-kyc-report.md` |

## Agent manifest

System prompt sourced from `.claude/agents/cfa/esg-regulatory-analyst.md`.
Skills loaded from `.claude/skills/corp-finance-analyst-regulatory/` and
`.claude/skills/workflow-operations-kyc/`.

## Deploy

```bash
# Validate manifest first
cfa managed-agent validate kyc-screener

# Dry-run (no API calls)
scripts/deploy-managed-agent.sh kyc-screener --dry-run

# Apply
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh kyc-screener --apply
```

## Required environment variables

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic API key (--apply only) |
| `CFA_CORE_MCP_URL` | URL of the cfa-core MCP server |

## Security tier: Tier 2 — Internal Research

- identity-reader and screener subagents are read-only (no file writes)
- reporter subagent has `write` toolset scoped to `./out/` only
- No internet access beyond the declared cfa-core MCP server
- No FMP/market-data MCP wired — KYC screening operates on intake JSON
  and cfa-core list lookups only, no live market data
- KYC reports may contain PII; output directory `./out/` should be
  configured with restricted access at the deployment layer
- No credentials in manifests — all injected via environment variables
