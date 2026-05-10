# packages/mcp-server Audit — Phase 33 Cleanup Plan

**Branch**: phase-33-skill-driven-planning  
**Date**: 2026-05-10  
**Auditor**: pathfinder research agent  
**Scope**: 15,037 LOC across 247 files in `packages/mcp-server/src/`

---

## Executive Summary

The `packages/mcp-server` package (`@robotixai/corp-finance-mcp`) was the original NAPI-backed MCP server exposing ~277 corp-finance tools. After Phase 30 (commit 9850410), the plugin at `plugins/cfa-core/mcp/src/server.ts` became the production MCP server for cfa-core tools (WASM-backed, registered in `~/.claude.json`). **The `packages/mcp-server` package is NOT registered in `~/.claude.json` for the cfa_agent project** — confirmed: the project entry lists zero servers from this package at runtime.

Despite that, the package is still fully load-bearing in three distinct ways:

1. **CI gate**: `surface-parity.yml` runs `npm run check:surface-parity` which executes `packages/mcp-server/src/tools/agent_infrastructure/surface_parity.ts` against the plugin. This is the primary drift-detection mechanism for the entire codebase.
2. **CI gate**: `cookbooks.yml` runs `npm run cookbooks:validate` which executes `packages/mcp-server/src/tools/agent_infrastructure/cookbook.ts` to validate all 15 managed-agent cookbooks.
3. **CI gate**: `cookbooks.yml` builds `packages/mcp-server` itself as a TypeScript compilation check.

The remaining 86 finance domain tool files (~4,282 LOC) and 73 hand-written schema files (~6,877 LOC) are fully superseded by the plugin but are still compiled to confirm no TypeScript regressions. The NAPI bindings layer (`bindings.ts`, ~318 LOC) drives these tool files but is itself not called at runtime by any external package.

**LOC verdict: ~11,159 LOC deletable (74%), ~3,878 LOC must be kept.**

---

## 1. What `packages/mcp-server` Exports

From `packages/mcp-server/package.json`:

- **name**: `@robotixai/corp-finance-mcp`
- **main**: `dist/index.js`
- **bin**: `corp-finance-mcp` → `dist/index.js`
- **dependencies**: `@modelcontextprotocol/sdk`, `@robotixai/corp-finance-bindings` (file link), `zod`

### Script Status

| Script | Status | Reason |
|---|---|---|
| `build` | KEEP | CI compiles the package; `cookbooks.yml` runs `npm run build` |
| `start` | DEAD | Not invoked in any CI workflow or external consumer |
| `dev` | DEAD | Local dev helper; package is not the active MCP server |
| `typecheck` | KEEP | Implicitly exercised by the build step |
| `test` | KEEP | `cookbooks.yml` runs vitest (middleware/audit.test.ts, agent_infrastructure/*.test.ts) |
| `schemas:export:*` (6x) | DEAD | These invoke `cargo test` in `$COREREPO` to regenerate JSON Schema sources. No CI workflow runs them. The generated output already exists in `src/schemas/generated/`. They are developer-only regeneration scripts and have no CI caller. |
| `schemas:gen:*` (6x + all) | DEAD | Call `schemas:export:*` then run `tsx zod_transform.ts`. No CI caller. The generated files already exist. |
| `check:surface-parity` | KEEP — CRITICAL | `surface-parity.yml` runs this on every push/PR. If deleted the CI gate breaks. |
| `cookbooks:validate` | KEEP — CRITICAL | `cookbooks.yml` runs this on every push/PR. If deleted the CI gate breaks. |
| `wasm:build` | DEAD in CI | Not referenced in any CI workflow. `plugins/cfa-core/README.md` documents `npm --prefix packages/mcp-server run wasm:build` as a local developer command. Not a CI dependency. |

---

## 2. External Consumers of `@robotixai/corp-finance-mcp`

From the grep across `packages/`, `plugins/`, `scripts/`, `docs/`:

```
docs/adr/ADR-001.md              — DOCS ONLY (historical architecture reference)
docs/adr/ADR-027-wave16-wasm-port-strategy.md — DOCS ONLY (strategy reference)
docs/prd/PRD-gap-remediation.md  — DOCS ONLY
docs/prd/PRD-institutional-real-estate.md — DOCS ONLY
docs/prd/PRD-offshore-fund-structures.md  — DOCS ONLY
docs/prd/PRD-phase26-memory-audit.md      — DOCS ONLY
docs/prd/PRD.md                           — DOCS ONLY
```

**No TypeScript, JSON, or MJS file outside `packages/mcp-server/` imports this package at runtime.** The `~/.claude.json` project entry for `/home/robert/cfa_agent` lists zero MCP servers from this package (confirmed via python3 query). All seven consumer files are documentation only.

---

## 3. What `plugins/cfa-core/mcp/src/server.ts` Does

The plugin server (766 LOC) is a WASM-backed passthrough. It loads `../wasm/corp_finance_wasm.js` at startup and exposes **282 financial calculation tools** across all domain modules via a single `passthroughShape` Zod schema (`z.record(z.any())`). It does not import from `packages/mcp-server` at all.

The WASM module covers every tool domain that the NAPI server covers for pure-math tools:
- Valuation (DCF, WACC, comps), PE/LBO, fixed income, derivatives, FX/commodities, offshore structures, earnings quality, financial forensics, bank analytics, emerging markets, carbon markets, CLO analytics, fund of funds, private wealth, venture, credit scoring, capital allocation, index construction, regulatory, ESG, insurance, macro economics, behavioral finance, and the Wave 17b residuals (merger model, three-statement, Monte Carlo).

**The plugin duplicates the entire finance domain functionality of `packages/mcp-server/src/tools/*.ts`.** The only tools the plugin does not cover are those in the `.surface-allowlist.json` (56 entries) — tools requiring native facilities: SQLite (rusqlite), ed25519-dalek, hnsw_rs SIMD, petgraph, filesystem, subprocess orchestration.

---

## 4. Source-File Overlap Audit

### `packages/mcp-server/src/tools/*.ts` — 86 files

Every one of these 86 files registers zero callers outside of `index.ts` and themselves. The inner-module call count (from the `for` loop) is 0 for all 86. They are imported only by `packages/mcp-server/src/index.ts`, which is the NAPI server entry point.

**None of these tool files are imported from outside `packages/mcp-server/`.**

Classification table (grouped by domain):

| Group | Files | LOC | Classification | Reason |
|---|---|---|---|---|
| Finance domain tools | `valuation.ts`, `credit.ts`, `pe.ts`, `portfolio.ts`, `scenarios.ts`, `ma.ts`, `jurisdiction.ts`, `fixed_income.ts`, `derivatives.ts`, `three_statement.ts`, `monte_carlo.ts`, `quant_risk.ts`, `restructuring.ts`, `real_assets.ts`, `fx_commodities.ts`, `securitization.ts`, `venture.ts`, `esg.ts`, `regulatory.ts`, `private_credit.ts`, `insurance.ts`, `fpa.ts`, `wealth.ts`, `crypto.ts`, `municipal.ts`, `structured_products.ts`, `trade_finance.ts`, `credit_derivatives.ts`, `convertibles.ts`, `lease_accounting.ts`, `pension.ts`, `sovereign.ts`, `real_options.ts`, `equity_research.ts`, `commodity_trading.ts`, `quant_strategies.ts`, `treasury.ts`, `infrastructure.ts`, `behavioral.ts`, `performance_attribution.ts`, `credit_portfolio.ts`, `macro_economics.ts`, `compliance.ts`, `onshore_structures.ts`, `offshore_structures.ts`, `transfer_pricing.ts`, `tax_treaty.ts`, `fatca_crs.ts`, `substance_requirements.ts`, `regulatory_reporting.ts`, `aml_compliance.ts`, `volatility_surface.ts`, `portfolio_optimization.ts`, `risk_budgeting.ts`, `market_microstructure.ts`, `interest_rate_models.ts`, `mortgage_analytics.ts`, `inflation_linked.ts`, `repo_financing.ts`, `credit_scoring.ts`, `capital_allocation.ts`, `clo_analytics.ts`, `fund_of_funds.ts`, `earnings_quality.ts`, `bank_analytics.ts`, `dividend_policy.ts`, `carbon_markets.ts`, `private_wealth.ts`, `emerging_markets.ts`, `index_construction.ts`, `institutional_real_estate.ts`, `financial_forensics.ts` | ~3,800 LOC | **DELETE** | Superseded by WASM plugin; zero external consumers |
| Native-only tools (allowlisted) | `workflows.ts`, `managed_agent.ts`, `mcp_servers.ts`, `memory.ts`, `audit.ts`, `cost.ts`, `security.ts`, `multi_agent.ts`, `federation.ts`, `self_learning.ts`, `agent_invoke.ts`, `attest.ts`, `office.ts` | ~480 LOC | **DEAD / DELETE** | These expose the native (NAPI-only) allowlisted tools. The NAPI server is not registered in Claude Code for the cfa_agent project. The tools themselves run through `packages/bindings` → Rust, but no agent calls this server. If the NAPI server were ever reactivated these would be needed; otherwise deletable. |
| agent_infrastructure tools | `agent_infrastructure/cookbook.ts`, `agent_infrastructure/surface_parity.ts`, `agent_infrastructure/wasm_build.ts`, `agent_infrastructure/zod_transform.ts`, `agent_infrastructure/index.ts`, `agent_infrastructure/*.test.ts` | 2,490 LOC | **KEEP** | Back the four CI scripts: `cookbooks:validate`, `check:surface-parity`, `wasm:build`, and the schema regeneration pipeline. `cookbooks:validate` and `check:surface-parity` are hard CI gates. |

### Classification summary for `tools/`:

- **KEEP**: 9 files (agent_infrastructure/), 2,490 LOC
- **DELETE**: 77 files (86 finance + 13 native-only minus agent_infrastructure), 4,282 LOC

---

## 5. `agent_infrastructure` Subdir

The `cfa-managed-agent` SKILL.md (`plugins/cfa-core/skills/cfa-managed-agent/SKILL.md`) references `agent_infrastructure` in documentation only. The passage that might look like a runtime pointer says:

> "All logic in deterministic Rust (corp-finance-core::managed_agent); this skill is documentation only."

The skill routes the model to use `managed_agent_*` MCP tools (served by the NAPI server via `tools/managed_agent.ts`). The `agent_infrastructure` subdir backs the *dev-time* tools (`cookbook_validate_all`, `surface_parity_check`, `wasm_build`, `schemas_refresh`) registered on the NAPI server but invoked via `npm run` scripts in CI — not by the Claude Code agent at inference time.

**Decision: KEEP in `packages/mcp-server/src/tools/agent_infrastructure/`. Do NOT move to plugin.** Rationale: the plugin is WASM-only; `cookbook.ts` spawns the `cfa` binary subprocess, `wasm_build.ts` spawns bash, and `surface_parity.ts` reads the filesystem — none of these are WASM-compatible. They belong with the NAPI server. The surface-allowlist already documents all four tools as `NAPI-only` with the correct reasons.

---

## 6. Subdirectory Bucket

| Dir / File | LOC | Decision | Rationale |
|---|---|---|---|
| `tools/agent_infrastructure/` | 2,490 | **KEEP** | Backs `check:surface-parity`, `cookbooks:validate`, `wasm:build`, and `schemas_refresh` — all active CI or developer scripts. Cannot move to WASM plugin (subprocess + FS I/O). |
| `tools/*.ts` (86 finance + native files) | 4,282 | **DELETE** | Superseded by plugin for finance tools. Native-only tools have no active server registration. Zero external consumers. |
| `middleware/audit.ts` | 138 | **KEEP** | Used by `index.ts` (`withAudit` wrapper) and has an active test in `middleware/audit.test.ts`. Removing it breaks the build. |
| `middleware/audit.test.ts` | 195 | **KEEP** | Active vitest test; CI runs `npm test`. |
| `formatters/response.ts` | 22 | **KEEP** | Imported by `tools/agent_infrastructure/index.ts` (`wrapResponse`). Deleting it breaks the build. |
| `schemas/generated/` (6 domains × N files) | 530 | **KEEP** | Imported by `tools/office.ts`, `tools/memory.ts`, `tools/attest.ts`, `tools/cost.ts`, and `tools/audit.ts`. If those tool files are deleted, the generated schemas become orphan — but only after the tool files are deleted first. **Deletable only in the same commit that removes the tool files that import them.** |
| `schemas/*.ts` (73 hand-written files) | 6,877 | **DELETE** | These are Zod schemas used only by the 86 NAPI tool files (each tool file imports from its sibling schema). No external consumer, no plugin use. Deleting the tool files first makes these automatically orphan; then delete. |
| `schemas/index.ts` | 289 | **DELETE** | Re-export barrel for all hand-written schemas. Same fate as the schema files it exports. |
| `bindings.ts` | 318 | **KEEP (short term) / DELETE (long term)** | ESM shim for `@robotixai/corp-finance-bindings`. Required by the tool files for as long as they exist. After the 86 tool files are deleted, this file's only value is as documentation of the NAPI binding surface. Once the tool deletion is complete, `bindings.ts` becomes orphan and can be deleted. |
| `index.ts` | 185 | **KEEP (short term) / SHRINK (long term)** | NAPI server entry point; imports all 86 tool modules + agent_infrastructure. After the 86 tool files are deleted, `index.ts` should be reduced to import only `registerAgentInfrastructureTools` + the native-only allowlisted modules. If the NAPI server is not intended to run at all post-cleanup, `index.ts` and `bindings.ts` can both be replaced by a stub that simply starts the server with agent_infrastructure tools only. |

---

## 7. Scripts Audit

| Script | Keep? | Reason |
|---|---|---|
| `build` | YES | CI dependency (cookbooks.yml Step "Build packages/mcp-server") |
| `start` | NO | No CI use; NAPI server not registered |
| `dev` | NO | No CI use; development convenience only |
| `typecheck` | YES | Compile-time safety; run by TypeScript build |
| `test` | YES | CI: vitest runs `audit.test.ts` + `agent_infrastructure/*.test.ts` |
| `schemas:export:office` | NO | Not in any CI workflow; one-shot dev regeneration |
| `schemas:export:attest` | NO | Same |
| `schemas:export:memory` | NO | Same |
| `schemas:export:audit` | NO | Same |
| `schemas:export:cost` | NO | Same |
| `schemas:export:observability` | NO | Same |
| `schemas:gen:office` | NO | Calls export + zod_transform.ts; no CI dependency |
| `schemas:gen:attest` | NO | Same |
| `schemas:gen:memory` | NO | Same |
| `schemas:gen:audit` | NO | Same |
| `schemas:gen:cost` | NO | Same |
| `schemas:gen:observability` | NO | Same |
| `schemas:gen:all` | NO | Same |
| `check:surface-parity` | YES — CRITICAL | surface-parity.yml CI gate |
| `cookbooks:validate` | YES — CRITICAL | cookbooks.yml CI gate |
| `wasm:build` | NO in CI | Not referenced by any CI workflow. Referenced in `plugins/cfa-core/README.md` as a local dev command only. Safe to keep as a convenience script but not a CI dependency. |

**Scripts to remove from package.json** (safe to drop; no CI workflow references them):
`start`, `dev`, `schemas:export:*` (×6), `schemas:gen:*` (×6 + all), `wasm:build`.

**Warning on `wasm:build`**: `plugins/cfa-core/README.md` shows `npm --prefix packages/mcp-server run wasm:build` as the local WASM rebuild command. If this script is dropped, that README instruction breaks. Recommend replacing with a direct `tsx` invocation or moving the script to a plugin-local script.

---

## 8. Final Cleanup Recommendation

### LOC Accounting

| Category | Files | LOC | Decision |
|---|---|---|---|
| `tools/agent_infrastructure/` (4 impl + 4 tests + 1 index) | 9 | 2,490 | KEEP |
| `middleware/audit.ts` + `audit.test.ts` | 2 | 333 | KEEP |
| `formatters/response.ts` | 1 | 22 | KEEP |
| `schemas/generated/` (auto-generated, 26 files) | 26 | 530 | KEEP (until tool files deleted) |
| `bindings.ts` | 1 | 318 | KEEP SHORT-TERM |
| `index.ts` | 1 | 185 | KEEP + SHRINK |
| `.surface-allowlist.json` | 1 | n/a | KEEP (active CI input) |
| **KEEP subtotal** | **41** | **~3,878** | |
| `tools/*.ts` (86 finance + native-only files) | 86 | 4,282 | DELETE |
| `schemas/*.ts` (73 hand-written, incl. index.ts) | 73 | 6,877 | DELETE |
| **DELETE subtotal** | **159** | **11,159** | |
| **TOTAL** | **200** | **15,037** | **74% deletable** |

---

### DELETE (159 files, ~11,159 LOC)

**`packages/mcp-server/src/tools/` — 86 files:**
```
packages/mcp-server/src/tools/agent_invoke.ts
packages/mcp-server/src/tools/aml_compliance.ts
packages/mcp-server/src/tools/attest.ts
packages/mcp-server/src/tools/audit.ts
packages/mcp-server/src/tools/bank_analytics.ts
packages/mcp-server/src/tools/behavioral.ts
packages/mcp-server/src/tools/capital_allocation.ts
packages/mcp-server/src/tools/carbon_markets.ts
packages/mcp-server/src/tools/clo_analytics.ts
packages/mcp-server/src/tools/commodity_trading.ts
packages/mcp-server/src/tools/compliance.ts
packages/mcp-server/src/tools/convertibles.ts
packages/mcp-server/src/tools/cost.ts
packages/mcp-server/src/tools/credit.ts
packages/mcp-server/src/tools/credit_derivatives.ts
packages/mcp-server/src/tools/credit_portfolio.ts
packages/mcp-server/src/tools/credit_scoring.ts
packages/mcp-server/src/tools/crypto.ts
packages/mcp-server/src/tools/derivatives.ts
packages/mcp-server/src/tools/dividend_policy.ts
packages/mcp-server/src/tools/earnings_quality.ts
packages/mcp-server/src/tools/emerging_markets.ts
packages/mcp-server/src/tools/equity_research.ts
packages/mcp-server/src/tools/esg.ts
packages/mcp-server/src/tools/fatca_crs.ts
packages/mcp-server/src/tools/federation.ts
packages/mcp-server/src/tools/financial_forensics.ts
packages/mcp-server/src/tools/fixed_income.ts
packages/mcp-server/src/tools/fpa.ts
packages/mcp-server/src/tools/fund_of_funds.ts
packages/mcp-server/src/tools/fx_commodities.ts
packages/mcp-server/src/tools/index_construction.ts
packages/mcp-server/src/tools/inflation_linked.ts
packages/mcp-server/src/tools/infrastructure.ts
packages/mcp-server/src/tools/institutional_real_estate.ts
packages/mcp-server/src/tools/insurance.ts
packages/mcp-server/src/tools/interest_rate_models.ts
packages/mcp-server/src/tools/jurisdiction.ts
packages/mcp-server/src/tools/lease_accounting.ts
packages/mcp-server/src/tools/ma.ts
packages/mcp-server/src/tools/macro_economics.ts
packages/mcp-server/src/tools/managed_agent.ts
packages/mcp-server/src/tools/market_microstructure.ts
packages/mcp-server/src/tools/mcp_servers.ts
packages/mcp-server/src/tools/memory.ts
packages/mcp-server/src/tools/monte_carlo.ts
packages/mcp-server/src/tools/mortgage_analytics.ts
packages/mcp-server/src/tools/multi_agent.ts
packages/mcp-server/src/tools/municipal.ts
packages/mcp-server/src/tools/office.ts
packages/mcp-server/src/tools/offshore_structures.ts
packages/mcp-server/src/tools/onshore_structures.ts
packages/mcp-server/src/tools/pe.ts
packages/mcp-server/src/tools/pension.ts
packages/mcp-server/src/tools/performance_attribution.ts
packages/mcp-server/src/tools/portfolio.ts
packages/mcp-server/src/tools/portfolio_optimization.ts
packages/mcp-server/src/tools/private_credit.ts
packages/mcp-server/src/tools/private_wealth.ts
packages/mcp-server/src/tools/quant_risk.ts
packages/mcp-server/src/tools/quant_strategies.ts
packages/mcp-server/src/tools/real_assets.ts
packages/mcp-server/src/tools/real_options.ts
packages/mcp-server/src/tools/regulatory.ts
packages/mcp-server/src/tools/regulatory_reporting.ts
packages/mcp-server/src/tools/repo_financing.ts
packages/mcp-server/src/tools/restructuring.ts
packages/mcp-server/src/tools/risk_budgeting.ts
packages/mcp-server/src/tools/scenarios.ts
packages/mcp-server/src/tools/securitization.ts
packages/mcp-server/src/tools/security.ts
packages/mcp-server/src/tools/self_learning.ts
packages/mcp-server/src/tools/sovereign.ts
packages/mcp-server/src/tools/structured_products.ts
packages/mcp-server/src/tools/substance_requirements.ts
packages/mcp-server/src/tools/tax_treaty.ts
packages/mcp-server/src/tools/three_statement.ts
packages/mcp-server/src/tools/trade_finance.ts
packages/mcp-server/src/tools/transfer_pricing.ts
packages/mcp-server/src/tools/treasury.ts
packages/mcp-server/src/tools/valuation.ts
packages/mcp-server/src/tools/venture.ts
packages/mcp-server/src/tools/volatility_surface.ts
packages/mcp-server/src/tools/wealth.ts
packages/mcp-server/src/tools/workflows.ts
```

**`packages/mcp-server/src/schemas/` — 73 hand-written schema files + generated (post tool-deletion):**
- All 73 `*.ts` files at `packages/mcp-server/src/schemas/*.ts` (hand-written Zod schemas)
- All 26 `*.ts` files at `packages/mcp-server/src/schemas/generated/**/*.ts` (auto-generated; can be deleted after the tool files that import them are deleted)

After deleting the tool files, `bindings.ts` becomes orphan and can be deleted in a follow-up commit.

---

### MOVE TO `plugins/cfa-core/mcp/` (0 files)

No files need to move. The agent_infrastructure tools are correctly NAPI-only (they spawn subprocesses and read the filesystem). The surface-allowlist already documents all four tools with appropriate reasons. The plugin cannot host these tools.

---

### KEEP (rationale)

```
packages/mcp-server/src/tools/agent_infrastructure/ — backs check:surface-parity and cookbooks:validate CI gates
packages/mcp-server/src/middleware/audit.ts         — used by index.ts; has active vitest test
packages/mcp-server/src/middleware/audit.test.ts    — active vitest coverage
packages/mcp-server/src/formatters/response.ts      — imported by agent_infrastructure/index.ts (wrapResponse)
packages/mcp-server/src/bindings.ts                 — needed while tool files exist; orphan after deletion
packages/mcp-server/src/index.ts                    — entry point; must be shrunk to import only agent_infrastructure after tool deletion
packages/mcp-server/.surface-allowlist.json         — active input to surface_parity.ts (CI reads this file)
packages/mcp-server/src/schemas/generated/          — imported by 5 tool files; delete in same commit as those files
```

---

### OPEN QUESTIONS

1. **`bindings.ts` long-term fate**: After the 86 tool files are deleted, `bindings.ts` (~318 LOC) has no importer. It documents the full NAPI binding surface (230+ exported functions). Recommend retaining as `packages/mcp-server/src/bindings.ts` for reference, or moving to `packages/bindings/src/bindings-surface.ts` as a type-only reference. Decision needed from owner.

2. **`index.ts` post-cleanup form**: Once the 86 tool files are gone, `index.ts` must either (a) register only `agent_infrastructure` tools (making the NAPI server a 4-tool dev-only server), or (b) be deleted entirely and replaced by a README note that `npm run check:surface-parity` / `npm run cookbooks:validate` are the only entry points. **If option (b), the `surface-parity.yml` and `cookbooks.yml` workflows need updating** to call `tsx` directly rather than `npm run build` → `node dist/index.js`. Option (a) is simpler: keep `index.ts`, remove the 86 imports.

3. **`wasm:build` script removal**: If removed, `plugins/cfa-core/README.md` line `npm --prefix packages/mcp-server run wasm:build` breaks. Either update the README to point to a direct `tsx` invocation, or move `wasm_build.ts` invocation to a plugin-local script.

4. **`schemas:gen:*` and `schemas:export:*` scripts**: If removed from `package.json`, the workflow for regenerating the generated schema files (which are still KEPT post-cleanup) must be documented elsewhere. Recommend a comment in `packages/mcp-server/src/schemas/generated/README.md` or inline comment in `package.json`. The `zod_transform.ts` tool itself stays (it is in `agent_infrastructure/`); only the npm convenience scripts pointing at `$COREREPO` are removed.

5. **`start` script and binary entrypoint**: The `bin.corp-finance-mcp` in `package.json` points to `dist/index.js`. If the NAPI server is never run directly by the Claude Code agent (confirmed: not registered in `~/.claude.json`), the `bin` entry can be removed. But removal triggers an `npm publish` diff that could break any downstream consumers who installed this package globally via `npx @robotixai/corp-finance-mcp`.

---

## Recommended Order of Operations for the Cleanup PR

Each step leaves the build green.

### Commit 1: Delete 86 finance/native tool files + their schema imports

Delete all 86 `packages/mcp-server/src/tools/*.ts` files (excluding `agent_infrastructure/`). Simultaneously delete the 73 `packages/mcp-server/src/schemas/*.ts` hand-written files and the 26 `packages/mcp-server/src/schemas/generated/**/*.ts` generated files. Update `packages/mcp-server/src/index.ts` to remove all 86 `register*Tools` imports — leave only `registerAgentInfrastructureTools` and the `withAudit` middleware call.

**Build must still pass** because `index.ts` still imports `agent_infrastructure`, `middleware/audit`, and `formatters/response`.

**Test must still pass** because `audit.test.ts` and `agent_infrastructure/*.test.ts` are untouched.

**`check:surface-parity` must still pass** because `surface_parity.ts` reads `packages/mcp-server/src/tools/` (now only the agent_infrastructure subdir) and compares against the plugin. The tool count reported as "packages_total" will drop from ~277 to 4 (the four agent_infrastructure tools). All four are in the allowlist. Drift = 0. CI green.

### Commit 2: Drop dead npm scripts from package.json

Remove from `scripts`: `start`, `dev`, `schemas:export:*` (6), `schemas:gen:*` (7), and optionally `wasm:build` (with README update).

**No build impact.** CI workflows only call `build`, `test`, `check:surface-parity`, and `cookbooks:validate`.

### Commit 3: Clean up bindings.ts (optional)

After the tool files are gone, `bindings.ts` has no runtime importers. Either delete it or rename to `packages/mcp-server/src/bindings-reference.ts` as a surface documentation artifact. This is cosmetic only.

### Commit 4: Shrink index.ts to a minimal stub

Replace the current `index.ts` (185 LOC, 87 imports) with a 15-line stub that:
1. Creates the MCP server with `withAudit`
2. Calls `registerAgentInfrastructureTools(server)`
3. Connects the stdio transport

This makes the package purpose crystal-clear: it is a developer-tooling MCP server, not a production finance computation server.

---

## Top 5 Surprises

1. **`packages/mcp-server` has zero runtime registrations in Claude Code.** The `~/.claude.json` cfa_agent project entry lists no servers from this package. The NAPI server is compiled and CI-tested but never invoked by any agent. This was expected per ADR-027 but worth confirming explicitly — the package exists purely for CI tooling and TypeScript build integrity.

2. **`agent_infrastructure` is more critical than its 2,490 LOC suggests.** It backs two hard CI gates (`surface-parity.yml` and `cookbooks.yml`) that run on every PR. If `agent_infrastructure/` were deleted, both CI workflows would fail on the `npm run check:surface-parity` and `npm run cookbooks:validate` steps respectively. This is the only non-deletable subdir in `tools/`.

3. **The NAPI tool files all import `formatters/response.ts` via `wrapResponse`, but `response.ts` also contains the now-identity-stub `coerceNumbers` function.** The original `coerceNumbers` was a recursive type coercer that broke string fields. The comment in `response.ts` explicitly says it was retained for API compatibility but does nothing. This is dead code within a file that is otherwise load-bearing for `agent_infrastructure/index.ts`.

4. **The 73 hand-written schema files (6,877 LOC) are the largest single deletable block.** They are used only by the 86 tool files and are never imported externally. Because they are co-located in `src/schemas/` and not in a shared package, there is no downstream breakage risk. The `schemas/index.ts` barrel re-exports all of them; both the barrel and the individual files can be deleted together.

5. **`wasm:build` is NOT a CI dependency despite being referenced in `plugins/cfa-core/README.md`.** A developer reading the README would assume it is a standard build step. In reality, the WASM artifacts are committed to the repo under `plugins/cfa-core/mcp/wasm/` and are not rebuilt in CI. The `wasm:build` script is a local developer utility for rebuilding artifacts from a separate `corp-finance-core` checkout. Deleting it from `package.json` without updating the README would silently break the documented workflow.

---

## Scripts Requiring Consumer Changes Before Deletion

If the following scripts are removed from `package.json`, their downstream references must be updated first:

| Script | Reference to update | Action |
|---|---|---|
| `wasm:build` | `plugins/cfa-core/README.md` line: `npm --prefix packages/mcp-server run wasm:build` | Replace with `tsx packages/mcp-server/src/tools/agent_infrastructure/wasm_build.ts` or add a plugin-local script |
| `schemas:gen:*` | No CI workflow reference, but `agent_infrastructure/index.ts` MCP tool description says "Replaces gen-*-schemas.mjs" — this is documentation only, not a script call | No action required |
| `start` | No external reference | Safe to drop |
| `dev` | No external reference | Safe to drop |
