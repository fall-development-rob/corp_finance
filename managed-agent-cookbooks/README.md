# CFA Managed-Agent Cookbooks

Each subdirectory defines one deployable CFA managed agent. Pattern adapted from
`anthropics/financial-services managed-agent-cookbooks/` and rewritten to use
our deterministic Rust tooling — no Python.

## Directory layout

```
managed-agent-cookbooks/
  <agent-slug>/
    agent.json              # Orchestrator manifest (required)
    README.md               # Cookbook docs (required)
    steering-examples.json  # Sample trigger events (required)
    subagents/
      <role>.json           # Depth-1 subagent manifests
```

## agent.json format

```json
{
  "name": "cfa-<slug>",
  "model": "claude-opus-4-7",
  "system": {
    "file": "../../../.claude/agents/cfa/<slug>.md",
    "append": "You are running headless. Produce output files in ./out/; do not assume an interactive terminal. Every number must originate from a tool call — never from LLM generation.\n"
  },
  "tools": [
    {
      "type": "agent_toolset_20260401",
      "default_config": { "enabled": false },
      "configs": [
        { "name": "read", "enabled": true },
        { "name": "glob", "enabled": true }
      ]
    },
    {
      "type": "mcp_toolset",
      "mcp_server_name": "cfa-core",
      "default_config": { "enabled": true }
    }
  ],
  "mcp_servers": [
    { "type": "url", "name": "cfa-core", "url": "${CFA_CORE_MCP_URL}" },
    { "type": "url", "name": "fmp",      "url": "${FMP_MCP_URL}" }
  ],
  "skills": [
    { "from_skill": "corp-finance-analyst-core" }
  ],
  "callable_agents": [
    { "manifest": "./subagents/data-reader.json" },
    { "manifest": "./subagents/analyst.json" },
    { "manifest": "./subagents/publisher.json" }
  ]
}
```

Adaptation notes vs upstream `pitch-agent` pattern:
- `from_skill` (not `from_plugin`) references our `.claude/skills/` tree
- MCP server names are `cfa-core`, `fmp`, `data`, `vendor` — not `capiq` / `daloopa`
- System prompt files resolve from `.claude/agents/cfa/<name>.md`
- All validation/linting is Rust (`cfa managed-agent ...`); the upstream Python tooling has no equivalent here

## steering-examples.json format

A JSON array of trigger event objects:

```json
[
  { "event": "...", "description": "..." }
]
```

Each `event` is the plain-text trigger that would be posted to `/v1/agents/<slug>/invoke`.

## Validate

```bash
# Single cookbook
cargo run -p corp-finance-cli -- managed-agent validate <slug>

# Every cookbook in this directory
cargo run -p corp-finance-cli -- managed-agent check-all

# Smoke-test all cookbooks (validate + dry-run deploy)
scripts/test-cookbooks.sh
```

## Deploy

```bash
# Dry-run (no API calls, prints final payload)
scripts/deploy-managed-agent.sh <slug> --dry-run

# Apply (requires ANTHROPIC_API_KEY)
ANTHROPIC_API_KEY=sk-ant-... scripts/deploy-managed-agent.sh <slug> --apply
```

## Available cookbooks

| Slug | Domain | Skills | Subagents |
|------|--------|--------|-----------|
| `equity-analyst` | Equity research | corp-finance-analyst-core | data-reader, analyst, publisher |
| `private-markets-analyst` | PE / IB | corp-finance-analyst-core | deal-reader, modeler, memo-writer |
| `credit-analyst` | Credit | corp-finance-analyst-core | data-reader, credit-scorer, reporter |
| `pitch-deck-builder` | IB pitch | workflow-investment-banking | data-reader, modeler, deck-author |
| `sector-research` | Sector ER | workflow-equity-research | data-reader, analyst, publisher |
| `earnings-reviewer` | Earnings | workflow-equity-research | transcript-reader, analyst, publisher |
| `model-builder` | DCF/LBO | corp-finance-tools-core | data-reader, modeler, exporter |
| `wealth-meeting-prep` | Wealth | workflow-wealth-management | portfolio-reader, analyst, brief-writer |
| `valuation-reviewer` | PE marks | workflow-private-equity | package-reader, reviewer, publisher |
| `lp-statement-auditor` | Fund admin | workflow-fund-admin | statement-reader, auditor, reporter |
| `gl-reconciler` | Fund admin | workflow-fund-admin | ledger-reader, reconciler, publisher |
| `month-end-closer` | Close ops | workflow-fund-admin | tb-reader, closer, reporter |
| `kyc-screener` | AML/KYC | workflow-operations-kyc | identity-reader, screener, reporter |
| `lseg-rates-monitor` | Rates / FX | vendor-lseg | rates-reader, analyst, publisher |
| `sp-credit-research` | Credit research | vendor-sp-global | data-reader, credit-scorer, publisher |

## Security tiers

All CFA cookbooks are classified **Tier 2 — Internal Research**:
- Reader/analyst subagents are read-only (no write or execute toolset)
- Only the publisher / reporter / writer subagent has `write` access, scoped to `./out/`
- MCP tool scope is bounded per-subagent via `default_config: { enabled: false }` plus an explicit `configs` allowlist
- `ANTHROPIC_API_KEY` and all `${*_MCP_URL}` variables must be injected at runtime; never stored in manifests

## Adding a new cookbook

1. Create `managed-agent-cookbooks/<new-slug>/`
2. Add `agent.json`, `README.md`, `steering-examples.json`
3. Add subagent manifests under `subagents/`
4. Add the slug to `ALLOWED_SLUGS` in `crates/corp-finance-core/src/managed_agent/types.rs`
5. Run `cargo run -p corp-finance-cli -- managed-agent validate <new-slug>`
6. Run `scripts/deploy-managed-agent.sh <new-slug> --dry-run`

## Skipped upstream items (and why)

- **MS Teams add-in installer** — we are headless / CLI-only; no MS 365 surface to install into
- **chronograph / egnyte MCP connectors** — closed-source vendor APIs we don't have keys for; revisit if access is granted
