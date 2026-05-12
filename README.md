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
data/cookbook-audits.json      byte-stable audit (Phase 25 Tier A2) — sha256 hash per
                               cookbook + per-file inventory, used to detect drift and
                               gate deploy-time integrity
data/cookbook-replays.json     byte-stable replay contract (Phase 25 Tier A3) — projection
                               fingerprint per loaded cookbook (parent + subagent tool
                               sets, system-prompt + schema sha256), catches loader
                               regressions + structured drift
data/cookbook-costs.json       worst-case cost estimate (Phase 25 Tier C1) — per-agent
                               token + USD breakdown, deterministic; current grand total
                               $9.19 per full-cycle invocation across all 15 cookbooks
data/cookbook-traces/          synthetic-trace evaluation (Phase 25 Tier C4) — one JSON
                               file per cookbook with the full assembled deploy payload
                               (system prompt text, tools, schemas) for byte-diffable
                               release review

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
npm test                                             # Vitest — 773 tests
npx tsx scripts/generate-tool-catalog.ts             # Regenerate data/tools-catalog.json
npx tsx scripts/lint-cookbook-tool-names.ts          # Lint cookbooks against catalog
npx tsx scripts/generate-cookbook-audits.ts          # Regenerate data/cookbook-audits.json
npx tsx scripts/generate-cookbook-replays.ts         # Regenerate data/cookbook-replays.json
npx tsx scripts/generate-cookbook-costs.ts           # Regenerate data/cookbook-costs.json
npx tsx scripts/generate-cookbook-traces.ts          # Regenerate data/cookbook-traces/*.trace.json
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

## Cookbook audit hashing (Phase 25 Tier A2)

`data/cookbook-audits.json` is a byte-stable audit catalog: one sha256 master hash per cookbook plus a per-file inventory of every file that influences its behavior — the cookbook directory (`agent.yaml`, `subagents/*.yaml`, `steering-examples.json`), the resolved `system.file` for each agent/subagent, and every text file under each `skills[].from_plugin` directory.

Two runs against the same disk state produce byte-identical output. Use cases:

- **Deploy-time integrity**: deployment tooling records the master hash. Before issuing the API call, recompute against the live tree; mismatch blocks the deploy.
- **PR review**: hash diff in `data/cookbook-audits.json` shows which cookbooks changed and which file(s) inside them. CI surfaces a one-line summary of added / removed / changed cookbooks vs the merge base.
- **Audit trail**: prove which exact version of a cookbook was deployed at a specific time.

CI gate (`.github/workflows/cookbook-audit.yml`):

1. **Audit freshness (strict)** — `data/cookbook-audits.json` must match a fresh regeneration. PRs that change cookbook content must include the regenerated audit; CI fails otherwise.

Spot-check a single cookbook: `npx tsx scripts/generate-cookbook-audits.ts --slug equity-analyst`.

## Cookbook replay contracts (Phase 25 Tier A3)

`data/cookbook-replays.json` is a byte-stable snapshot of what the harness `CookbookLoader` actually produces from each cookbook on disk: parent + subagent IDs, models, sorted tool sets, `block_tools`, sha256 of the assembled system prompt (after skill bodies + `system.file` + `system.text` + `system.append` are concatenated), and sha256 of the output/input schemas.

Complements Tier A2:

- **Audit hash** → catches **byte changes** in cookbook content (file diff).
- **Replay contract** → catches **projection changes** — what the loader produces.

Three bug classes the replay catches that the audit cannot:

1. Loader regressions (`projectTools` / system-prompt assembly bugs) that change loader output without any file diff.
2. Structured drift visibility: PR reviewers see "analyst subagent lost tool X" instead of "this YAML file changed".
3. Subagent surface drift: tools + model + block_tools per subagent diff in one JSON block per cookbook.

CI gate (`.github/workflows/cookbook-replay.yml`):

1. **Replay freshness (strict)** — `data/cookbook-replays.json` must match a fresh regeneration via the live `CookbookLoader`. PRs that change cookbook content or skill bodies must include the regenerated replay.

Spot-check a single cookbook: `npx tsx scripts/generate-cookbook-replays.ts --slug equity-analyst`.

## Cookbook contracts (Phase 25 Tier A4)

`docs/contracts/feature_managed_agents.yml` declares the architectural invariants every managed-agent cookbook must hold. Each rule (MA-001 through MA-007) and invariant (MA-INV-001 through MA-INV-004) is enforced by `packages/harness/tests/contracts/managed-agent-contracts.test.ts`, which runs as part of the standard harness vitest suite. Violations fail CI on the regular TypeScript workflow.

| Rule | Invariant |
|------|-----------|
| MA-001 | Every cookbook has `agent.yaml` + `steering-examples.json` |
| MA-002 | Every parent declares `callable_agents[]` (delegation pattern) |
| MA-003 | Every cookbook has exactly 3 subagents (read/work/publish) |
| MA-004 | Subagents using `cfa-core` (compute) gate tools explicit-allow |
| MA-005 | Every subagent declares an `output_schema` |
| MA-006 | Every parent `system.append` carries the anti-injection reminder |
| MA-007 | Cookbook slugs match `^[a-z][a-z0-9-]{1,62}[a-z0-9]$` (deploy ID format) |
| MA-008 | Every parent declares a valid semver `version` |
| MA-INV-001 | Exactly 15 cookbooks |
| MA-INV-002 | Exactly 45 subagents (3 × 15) |
| MA-INV-003 | `data/cookbook-audits.json` has one entry per cookbook |
| MA-INV-004 | `data/cookbook-replays.json` has one entry per cookbook |

MA-004 is narrowed to `cfa-core` because data-fetcher subagents (e.g. `*-reader`) intentionally use broad-allow on `fmp` / `data` / `vendor` servers for retrieval flexibility. Compute determinism is what materially matters; the rule reflects the architectural intent rather than blanket strictness.

## Cookbook versioning (Phase 25 Tier C2)

Every parent `agent.yaml` carries a `version: "<semver>"` field validated against semver-2.0 (MAJOR.MINOR.PATCH, optional `-PRERELEASE` and `+BUILDMETA` suffixes). The current baseline is `1.0.0` across all 15 cookbooks.

The version is:

- **Surfaced in `data/cookbook-audits.json`** at the per-cookbook level — reviewers see a version diff in PRs alongside the master hash and file inventory.
- **Folded into the audit hash implicitly** — it lives inside `agent.yaml` bytes, so bumping the version is a content change that updates the hash naturally.
- **Validated by contract MA-008** — every parent must declare a semver-valid version; CI fails on missing or malformed values.

Deploy-time tooling (in the sibling `corp-finance-core` repo) consumes the version to generate distinct deployment IDs per release and to support rollback. Bumping a version with no other content change is still a release event: the audit hash will move because `agent.yaml` bytes changed.

## Cookbook cost telemetry (Phase 25 Tier C1)

`data/cookbook-costs.json` is a deterministic worst-case USD cost estimate per cookbook. Each agent in a cookbook (parent + 3 subagents) is priced as `(system_prompt_tokens × input_rate) + (max_tokens × output_rate)` using the canonical Anthropic API rates committed in `MODEL_PRICING`:

| Model | Input $/Mtok | Output $/Mtok |
|-------|-------------:|--------------:|
| `claude-opus-4-7` | 15 | 75 |
| `claude-sonnet-4-6` | 3 | 15 |
| `claude-haiku-4-5` | 1 | 5 |

Token counts come from a 4-char-per-token heuristic over the assembled system prompt (skill bodies + `system.file` + `system.text` + `system.append`). Output budgets use the agent's `max_tokens` field (default 4096). The estimate is a worst-case ceiling — every agent assumed to use its full budget — so real invocations cost less. Tool-call round-trips are not yet modeled.

Current baseline ($9.19 total per cycle across all 15 cookbooks):

| Tier | Range | Examples |
|------|-------|----------|
| ~$0.50 | DCF / valuation / credit scoring | credit-analyst, valuation-reviewer, sp-credit-research, equity-analyst |
| ~$0.55-0.60 | Multi-tool synthesis | earnings-reviewer, model-builder, pitch-deck-builder, sector-research, wealth-meeting-prep |
| ~$0.80-0.90 | Heavy data fetch + ops | gl-reconciler, lp-statement-auditor, kyc-screener |

CI gate (`.github/workflows/cookbook-cost.yml`):

1. **Cost freshness (strict)** — `data/cookbook-costs.json` must match a fresh regeneration. Catches cookbooks that change model, max_tokens, or system prompt without updating the cost estimate. Cost drift becomes a reviewable PR-level signal.

Spot-check: `npx tsx scripts/generate-cookbook-costs.ts --slug equity-analyst`.

## Synthetic-trace evaluation (Phase 25 Tier C4)

`data/cookbook-traces/<slug>.trace.json` — one file per cookbook containing the FULL assembled deploy payload: parent + subagent definitions with the actual system-prompt text (not just a hash), tool allowlists, output/input schemas (canonicalised), audit hash, version, and the steering events that drive the cookbook.

The trace is the artifact PR reviewers read when they need to know "did the actual prompt that gets sent to Claude change, and how?". File sizes range from ~55KB (compact cookbooks) to ~200KB (heavy ones like `kyc-screener` and `gl-reconciler` whose skills are large), totaling ~1.4MB across all 15 cookbooks.

Complements the earlier tiers — each catches a different bug class:

| Catalog | Granularity | Catches |
|---------|-------------|---------|
| `data/tools-catalog.json` | tool names | misspelled tool references |
| `data/cookbook-audits.json` | file bytes | any cookbook content change |
| `data/cookbook-replays.json` | projection hashes | loader output drift |
| `data/cookbook-traces/*.trace.json` | full assembled payload | prompt drift across versions (reviewable as text diff) |

CI gate (`.github/workflows/cookbook-trace.yml`):

1. **Trace freshness (strict)** — every per-cookbook trace must match a fresh regeneration. PRs that change cookbook content, skills, system prompts, or version must include regenerated traces. Stale trace files (slug removed) are also flagged.

Spot-check: `npx tsx scripts/generate-cookbook-traces.ts --slug equity-analyst`.

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
| `cookbook-audit.yml` | Cookbook audit hash freshness | **Strict** |
| `cookbook-replay.yml` | Cookbook replay contract freshness (loader projection) | **Strict** |
| `cookbook-cost.yml` | Cookbook cost-estimate freshness | **Strict** |
| `cookbook-trace.yml` | Synthetic-trace evaluation (full-prompt diff) | **Strict** |
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
