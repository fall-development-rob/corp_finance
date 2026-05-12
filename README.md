# cfa_agent

Institutional-grade CFA agent stack: a deterministic, byte-stable dispatch loop on top of 623 financial tools, with 15 deployable managed-agent cookbooks, a 3-tier plugin architecture, and a closed learning loop from validation failures back to skill prose.

All financial math runs in 128-bit decimal precision via Rust (compiled to WASM and NAPI). The dispatch runtime is TypeScript on `@anthropic-ai/sdk`. **Two runs of the same cookbook against the same inputs produce byte-identical output.**

> **[Wiki](https://github.com/fall-development-rob/corp_finance/wiki)** — module reference, data source catalogue, architecture details.

## Repo layout

```
plugins/                       3-tier plugin architecture (Phase 40)
  cfa-core/                    compute backbone — WASM MCP server with 227 cfa-core tools,
                               4 corp-finance-tools-* reference skills, specflow,
                               cfa-managed-agent, security/audit hooks, agents/cfa/ canonical
                               YAML manifests for the 9 main analysts
  agent-plugins/               24 plugins — one per deployable agent (9 specialists + 15 cookbooks)
  vertical-plugins/            11 plugins — one per business domain (er, ib, pe, fa,
                               fund-admin, ops, wm, macro, foundations, derivatives stub,
                               fixed-income stub); hosts workflow-* skills and slash commands
  partner-built/               10 plugins — one per data vendor (lseg, sp-global, factset,
                               moodys, morningstar, pitchbook, fmp, free-data, aiera stub,
                               daloopa stub); each has its own .mcp.json

packages/                      TypeScript workspace
  harness/                     dispatch runtime — agent loop, MCP client, hybrid router,
                               reasoning bank (local RuVector + S3), skill-editor CLI
  mcp-server/                  agent infrastructure tools (cookbook validator, surface-parity,
                               wasm-build, zod-transform)
  fmp-mcp-server/              180 FMP tools (free tier with API key)
  data-mcp-server/             129 free public data tools (FRED, EDGAR, FIGI, YF, WB,
                               ACLED, UCDP, GDELT, GDACS, USGS, WTO, Polymarket, ...)
  vendor-mcp-server/           87 paid vendor tools (LSEG, S&P, FactSet, Morningstar,
                               Moody's, PitchBook)
  bindings/                    NAPI bindings to corp-finance-core (consumed by cfa-core MCP)
  mcp-utils/                   shared MCP server utilities

managed-agent-cookbooks/       15 deployable cookbooks (YAML manifests + subagents/*.yaml)
                               consumed by harness.dispatchCookbook() at runtime

data/tools-catalog.json        canonical 623-tool catalog (Phase 25 Tier A1) — used by
                               cookbook tool-name lint, regenerated from MCP server sources

docs/                          adr/, ddd/, contracts/, plans/, skill-editor-templates/
```

`corp-finance-core` (Rust library + CLI) was extracted to its own crates.io-published repo in Phase 29 (Wave 18). This repo consumes it via `packages/bindings` (NAPI) and `plugins/cfa-core/mcp/wasm` (WASM).

## How it all fits together

```
   user prompt (CLI / Claude Code slash command)
        │
        ▼
┌────────────────────┐
│  cfa-harness       │  packages/harness — dispatch loop on @anthropic-ai/sdk
│  (TypeScript)      │  • routes via WorkflowRouter (deterministic static workflows)
│                    │    or falls through to LLM dispatch
│                    │  • validates every subagent output against its output_schema
│                    │  • writes structured audit entries to the reasoning bank
└────────┬───────────┘
         │
         ▼
┌────────────────────┐    ┌──────────────────────────────────────────┐
│  cookbook manifest │ ─▶ │  4 MCP servers (623 tools total)         │
│  agent.yaml +      │    │  • cfa-core (227)  — compute backbone    │
│  subagents/*.yaml  │    │  • fmp     (180)   — FMP free tier       │
│                    │    │  • data    (129)   — free public sources │
│                    │    │  • vendor   (87)   — paid (LSEG/SP/...)  │
└────────────────────┘    └──────────────────────────────────────────┘
         │
         ▼
┌────────────────────┐
│  reasoning bank    │  packages/harness/src/reasoning — RuVector locally,
│                    │  S3+MinIO in CI (Phase 41 W0). Indexes every dispatch
│                    │  with validation_failed metadata for outlier detection.
└────────┬───────────┘
         │
         ▼
   skill-editor pipeline (Phase 41, deterministic):
     outliers.ts → remediation-emitter.ts → docs/proposed-skill-updates/<file>.yaml
                                                       │
                                                       ▼
                                       human PR review → skill-editor apply
                                                       │
                                                       ▼
                                       byte-deterministic SKILL.md edit
```

**Determinism invariant.** Every component that writes to a versioned file (skill, manifest, agent definition) is a pure function over its inputs. No LLM synthesises free prose into the production path. See `docs/adr/ADR-044-phase-41-deterministic-learning-loop.md`.

## Quick Start

```bash
npm install                                          # Turborepo — installs 7 packages
npm run build                                        # Builds harness + 4 MCP servers
npm test                                             # Vitest — 744 tests
npx tsx scripts/generate-tool-catalog.ts             # Regenerate data/tools-catalog.json
npx tsx scripts/lint-cookbook-tool-names.ts          # Lint cookbooks against catalog
npx tsx scripts/check-manifests.ts --strict          # Static manifest linter
```

### As an MCP server

```json
{
  "mcpServers": {
    "cfa-core": {
      "command": "node",
      "args": ["/path/to/plugins/cfa-core/mcp/dist/server.js"]
    }
  }
}
```

### As a dispatch CLI

```bash
export ANTHROPIC_API_KEY=sk-ant-...

# Dispatch a deployable cookbook end-to-end
cfa-harness cookbook equity-analyst --input ./input.json --out ./out

# Apply a remediation proposal (Phase 41 closed loop)
cfa-harness skill-editor apply docs/proposed-skill-updates/<file>.yaml
```

## Tool catalog and cookbook lint (Phase 25 Tier A1)

`data/tools-catalog.json` is the canonical list of every MCP tool name across the 4 in-repo servers (227 + 180 + 129 + 87 = 623). It is generated deterministically from source (`scripts/generate-tool-catalog.ts`) and committed as the audit reference.

The cookbook lint (`scripts/lint-cookbook-tool-names.ts`) walks every cookbook agent.yaml and subagents/*.yaml, finds every `mcp_toolset` block with explicit `configs[].name`, and verifies each name resolves to a real catalog entry. It classifies any failure as `unknown_tool`, `unknown_server`, or `prefix_mismatch`.

Two CI gates (`.github/workflows/tool-name-lint.yml`):
1. **Catalog freshness (strict)** — `data/tools-catalog.json` must match a fresh regeneration. Catches developers who add a tool but forget to regenerate.
2. **Cookbook drift (strict)** — every `configs[].name` must resolve. Catches the verb-prefixed-name bug class (`calculate_target_price`, `build_lbo`, `calculate_dupont`, ...).

## Closed learning loop (Phase 41)

Validation failures during dispatch become structured skill remediations, fully deterministically:

| Step | Component | Output |
|------|-----------|--------|
| 1 | `validator.ts` rejects a subagent response | `ReasoningEntry.metadata.validation_failed = true` |
| 2 | `indexer.ts` writes to bank (local RuVector or shared S3) | persisted audit entry |
| 3 | `outliers.ts` (cron or manual) runs 4 detectors | typed `OutlierReport` |
| 4 | `remediation-emitter.ts` (pure fn) → structured YAML | `docs/proposed-skill-updates/<file>.yaml` |
| 5 | CI cron opens a PR with the YAMLs | reviewable artifact |
| 6 | Human reviews + approves | merged |
| 7 | `cfa-harness skill-editor apply <file>` | byte-deterministic SKILL.md edit |
| 8 | `scripts/archive-skill-proposal.sh` | moved to `docs/proposed-skill-updates/archive/<YYYY-MM>/` |

Templates for the apply step live under `docs/skill-editor-templates/` as version-controlled static files. No LLM is invoked in the proposal or apply path. See `docs/adr/ADR-044-phase-41-deterministic-learning-loop.md`.

## What's inside (by tool count)

| Area | cfa-core tools | Notable |
|------|---------------:|---------|
| Valuation & modelling | ~25 | DCF, WACC, comps, three-statement, LBO, merger, SOTP, target_price |
| Fixed income | ~20 | Bond pricing, curves, duration/convexity, MBS, TIPS, repo, rate models |
| Derivatives | ~15 | Options (BS/CRR), Greeks, IV surface, SABR, forwards, swaps, exotics |
| Credit | ~25 | Altman Z, CDS, CVA, CLO waterfall/coverage/scenario, CECL, migration, scorecard |
| Risk & quant | ~25 | Factor models, Black-Litterman, VaR/CVaR, risk parity, pairs, momentum, Brinson |
| Real estate | ~10 | Rent roll, comparable sales, HBU, replacement cost, benchmarking, acquisition |
| PE & VC | ~20 | LBO, waterfall, fund returns, J-curve, commitment pacing, SAFE, dilution |
| Regulatory | ~20 | Basel III, AIFMD, MiFID II best execution, GIPS, KYC/AML, FATCA/CRS, BEPS |
| ESG & climate | ~10 | ESG scoring, carbon markets, CBAM, green bonds, SLL covenants |
| Fund structures | ~10 | US/UK/EU onshore, Cayman/BVI/Lux/Ireland offshore, transfer pricing, treaty |

Plus **180** FMP tools, **129** free public data tools, and **87** paid vendor tools. Full breakdown in `data/tools-catalog.json` and the [Modules wiki page](https://github.com/fall-development-rob/corp_finance/wiki/Modules).

## Managed-agent cookbooks

15 cookbooks under `managed-agent-cookbooks/`, each a YAML manifest tree consumed by `dispatchCookbook()`:

| Cookbook | Tier | Domain |
|----------|------|--------|
| equity-analyst | CoreOnly | DCF, comps, earnings quality, target price |
| credit-analyst | CoreOnly | PD, LGD, CDS, CVA, credit scoring |
| private-markets-analyst | CoreOnly | LBO, waterfall, J-curve, secondaries |
| earnings-reviewer | CoreOnly | Beneish, Piotroski, accrual & revenue quality |
| sector-research | CoreOnly | Comps, peer benchmarking, sector valuation |
| model-builder | CoreOnly | Three-statement, sensitivity, scenarios |
| pitch-deck-builder | CoreOnly | Comps, target price, executive summary export |
| valuation-reviewer | CoreOnly | DCF review, WACC, sensitivity audit |
| kyc-screener | Freemium | Sanctions, KYC risk, entity classification |
| gl-reconciler | Freemium | Variance analysis, three-way recon |
| month-end-closer | Freemium | Accruals, close checklist, variance |
| lp-statement-auditor | Freemium | Investor net returns, GP economics, NAV |
| wealth-meeting-prep | Freemium | Retirement planning, tax-loss harvesting, estate |
| sp-credit-research | Paid (S&P) | S&P-driven credit research |
| lseg-rates-monitor | Paid (LSEG) | LSEG-driven fixed income monitoring |

13 of 15 cookbooks run with no paid subscription. List by tier: `cfa managed-agent list --tier=core-only`. See [`managed-agent-cookbooks/README.md`](managed-agent-cookbooks/README.md).

## CI gates

| Workflow | What it checks | Mode |
|----------|----------------|------|
| `ci.yml` | Lint, typecheck, build | Strict |
| `typescript.yml` | TS compile + harness tests | Strict |
| `rust.yml` | Rust build (bindings + WASM) | Strict |
| `cookbooks.yml` | Cookbook discovery + smoke + builds 4 MCP servers | Strict (smoke); informational (legacy Rust validate, post-YAML) |
| `tool-name-lint.yml` | Catalog freshness + cookbook drift | **Strict** (both gates) |
| `manifest-check.yml` | Static manifest linter (cross-refs, schema shape) | Strict |
| `surface-parity.yml` | Drift between packages/mcp-server NAPI and plugins/cfa-core/mcp WASM | Strict |
| `lockfile-guard.yml` | No nested package-lock.json in workspaces | Strict |
| `skill-editor-cron.yml` | Weekly outlier scan → PR with remediation YAMLs | Mon 06:00 UTC |

## Data sources

| Server | Tools | Cost | Sources |
|--------|------:|------|---------|
| cfa-core | 227 | Free, offline | Pure Rust compute via WASM, no network calls |
| data | 129 | Free (3 free signups) | FRED, EDGAR, FIGI, Yahoo Finance, World Bank, UCDP, GDELT, GDACS, USGS, WTO, Polymarket, CoinGecko, UNHCR, Open-Meteo (no key); ACLED, NASA FIRMS, EIA (free signup) |
| fmp | 180 | Freemium | Financial Modeling Prep (quotes, financials, technicals, news) |
| vendor | 87 | Paid | LSEG, S&P Global, FactSet, Morningstar, Moody's, PitchBook |

See [`docs/VENDOR_FREE_PATH.md`](docs/VENDOR_FREE_PATH.md) for the free-only path.

## Documentation

| Resource | Description |
|----------|-------------|
| **[Wiki](https://github.com/fall-development-rob/corp_finance/wiki)** | Full technical reference |
| [`docs/VENDOR_FREE_PATH.md`](docs/VENDOR_FREE_PATH.md) | Free-tier path; how to add paid layers |
| `docs/adr/` | Architecture Decision Records (ADR-015 to ADR-044) |
| `docs/plans/` | Active design specs; completed phases under `archive/` |
| `docs/skill-editor-templates/` | Canonical static templates used by the apply CLI |
| `docs/contracts/` | Specflow executable contracts |
| `docs/ddd/` | Domain models per bounded context |

## License

MIT
