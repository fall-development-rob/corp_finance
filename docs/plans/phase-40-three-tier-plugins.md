# Phase 40 — Three-Tier Plugin Restructure

**Status**: Design (no code changes)
**Branch**: phase-33-skill-driven-planning
**Author**: Architect agent, 2026-05-10
**Upstream reference**: github.com/anthropics/financial-services/tree/main/plugins

---

## 1. Why This Restructure

The current layout has two monolithic plugins (`cfa-core`, `cfa-pro`) that violate the install-only-what-you-need principle. An equity researcher installs 66 skills even though they need 9. A fund-admin user gets PE tooling they will never invoke. A user with an LSEG subscription loads FactSet skill instructions they have no key for.

Anthropic's upstream `financial-services` repo has established a 3-tier model that solves this. Verified structure (fetched 2026-05-10):

- **agent-plugins/** — one plugin per cookbook-deployable agent (10 upstream: earnings-reviewer, gl-reconciler, kyc-screener, market-researcher, meeting-prep-agent, model-builder, month-end-closer, pitch-agent, statement-auditor, valuation-reviewer)
- **vertical-plugins/** — one plugin per business domain (7 upstream: equity-research, financial-analysis, fund-admin, investment-banking, operations, private-equity, wealth-management)
- **partner-built/** — one plugin per data vendor (2 upstream: lseg, spglobal)

**plugin.json shape** (verified from valuation-reviewer and lseg):
```json
{
  "name": "valuation-reviewer",
  "version": "0.1.0",
  "description": "...",
  "author": { "name": "Anthropic FSI" }
}
```

The upstream shape does not include a `dependencies` field. Dependencies are expressed implicitly via `from_plugin:` paths inside cookbook `agent.json` files. This keeps plugin.json minimal.

Partner-built plugins also ship a `.mcp.json` alongside `.claude-plugin/plugin.json`:
```json
{
  "mcpServers": {
    "lseg": { "type": "http", "url": "https://api.analytics.lseg.com/lfa/mcp/server-cl" }
  }
}
```

S&P Global's plugin.json additionally carries `homepage`, `repository`, `license`, and `keywords` fields because it is third-party authored. Our vendor wrappers will match that shape.

---

## 2. Tier Definitions

### Tier 1 — agent-plugins/

**Rule**: one directory per cookbook-deployable agent. Bundles the agent's `.yaml` manifest, the agent's `.md` system prompt, and any skills that are used exclusively by that one agent and no other.

**Why a separate tier**: agent plugins are the deployment unit. A user installs `agent-plugins/equity-analyst` to get that specific agent. Keeping the agent manifest and its private skills co-located makes the deployment boundary clear. An agent plugin has no hard dependency on vertical plugins at the file-system level; the cookbook `agent.json` carries the skill references. This enables an agent to be deployed without pulling in an entire vertical.

**Contents per plugin directory**:
```
plugins/agent-plugins/<slug>/
  .claude-plugin/plugin.json
  agents/<slug>.yaml
  agents/<slug>.md
  skills/                   # agent-exclusive skills only (the cfa/ specialist skills)
```

### Tier 2 — vertical-plugins/

**Rule**: one directory per business domain. Bundles cross-cutting workflow skills and slash commands that multiple agents in that domain use. No MCP server config.

**Why a separate tier**: workflow skills like `workflow-er-initiating-coverage` are consumed by both the equity-analyst agent and the sector-research cookbook. They belong to the domain, not to any single agent. Vertical plugins are the reuse layer. An agent-plugin's cookbook references vertical-plugin skills via their repo-relative path.

**Contents per plugin directory**:
```
plugins/vertical-plugins/<domain>/
  .claude-plugin/plugin.json
  skills/                   # workflow-<prefix>-* skills
  commands/                 # slash commands scoped to this domain
  hooks/                    # domain-specific Pre/PostToolUse hooks (if any)
```

### Tier 3 — partner-built/

**Rule**: one directory per data vendor. Bundles vendor-specific MCP server configuration, vendor-use skills (how-to-use instructions), and vendor-scoped commands.

**Why a separate tier**: vendor plugins are independently maintained (LSEG, S&P, FactSet, Moody's, Morningstar, PitchBook each have their own auth, rate limits, and schema drift). Isolating them means a user who does not have a FactSet subscription pays zero cognitive cost for FactSet tools. Vendor plugins may be authored by third parties (as S&P's plugin.json shows).

**Contents per plugin directory**:
```
plugins/partner-built/<vendor>/
  .claude-plugin/plugin.json
  .mcp.json                 # MCP server endpoint registration
  skills/                   # vendor-<slug> skill
  commands/                 # vendor-scoped slash commands (optional)
```

### Retained — cfa-core/ and cfa-data/

`cfa-core` continues to exist as the compute MCP backbone (WASM/NAPI bindings, 206 Rust tools). It becomes a pure infrastructure plugin: no skills, no agents, no commands. The `cfa-data` plugin is the free public data MCP (FRED, EDGAR, WB, YF, FIGI, geopolitical APIs). Both are depended upon by vertical and agent plugins but are not user-facing install targets themselves.

**Resolved decision**: `corp-finance-tools-*` (tool-reference skills) stay in `cfa-core` as the compute reference layer. They describe how to call the Rust tools and belong with the compute engine, not with any domain.

---

## 3. Complete Mapping Table — Current to Target

### 3a. Agents (9 .yaml + 9 .md = 18 files)

| Current path | Target tier | Target plugin | New path |
|---|---|---|---|
| plugins/cfa-core/agents/cfa/chief-analyst.yaml | agent-plugins | chief-analyst | plugins/agent-plugins/chief-analyst/agents/chief-analyst.yaml |
| plugins/cfa-core/agents/cfa/chief-analyst.md | agent-plugins | chief-analyst | plugins/agent-plugins/chief-analyst/agents/chief-analyst.md |
| plugins/cfa-core/agents/cfa/credit-analyst.yaml | agent-plugins | credit-analyst | plugins/agent-plugins/credit-analyst/agents/credit-analyst.yaml |
| plugins/cfa-core/agents/cfa/credit-analyst.md | agent-plugins | credit-analyst | plugins/agent-plugins/credit-analyst/agents/credit-analyst.md |
| plugins/cfa-core/agents/cfa/derivatives-analyst.yaml | agent-plugins | derivatives-analyst | plugins/agent-plugins/derivatives-analyst/agents/derivatives-analyst.yaml |
| plugins/cfa-core/agents/cfa/derivatives-analyst.md | agent-plugins | derivatives-analyst | plugins/agent-plugins/derivatives-analyst/agents/derivatives-analyst.md |
| plugins/cfa-core/agents/cfa/equity-analyst.yaml | agent-plugins | equity-analyst | plugins/agent-plugins/equity-analyst/agents/equity-analyst.yaml |
| plugins/cfa-core/agents/cfa/equity-analyst.md | agent-plugins | equity-analyst | plugins/agent-plugins/equity-analyst/agents/equity-analyst.md |
| plugins/cfa-core/agents/cfa/esg-regulatory-analyst.yaml | agent-plugins | esg-regulatory-analyst | plugins/agent-plugins/esg-regulatory-analyst/agents/esg-regulatory-analyst.yaml |
| plugins/cfa-core/agents/cfa/esg-regulatory-analyst.md | agent-plugins | esg-regulatory-analyst | plugins/agent-plugins/esg-regulatory-analyst/agents/esg-regulatory-analyst.md |
| plugins/cfa-core/agents/cfa/fixed-income-analyst.yaml | agent-plugins | fixed-income-analyst | plugins/agent-plugins/fixed-income-analyst/agents/fixed-income-analyst.yaml |
| plugins/cfa-core/agents/cfa/fixed-income-analyst.md | agent-plugins | fixed-income-analyst | plugins/agent-plugins/fixed-income-analyst/agents/fixed-income-analyst.md |
| plugins/cfa-core/agents/cfa/macro-analyst.yaml | agent-plugins | macro-analyst | plugins/agent-plugins/macro-analyst/agents/macro-analyst.yaml |
| plugins/cfa-core/agents/cfa/macro-analyst.md | agent-plugins | macro-analyst | plugins/agent-plugins/macro-analyst/agents/macro-analyst.md |
| plugins/cfa-core/agents/cfa/private-markets-analyst.yaml | agent-plugins | private-markets-analyst | plugins/agent-plugins/private-markets-analyst/agents/private-markets-analyst.yaml |
| plugins/cfa-core/agents/cfa/private-markets-analyst.md | agent-plugins | private-markets-analyst | plugins/agent-plugins/private-markets-analyst/agents/private-markets-analyst.md |
| plugins/cfa-core/agents/cfa/quant-risk-analyst.yaml | agent-plugins | quant-risk-analyst | plugins/agent-plugins/quant-risk-analyst/agents/quant-risk-analyst.yaml |
| plugins/cfa-core/agents/cfa/quant-risk-analyst.md | agent-plugins | quant-risk-analyst | plugins/agent-plugins/quant-risk-analyst/agents/quant-risk-analyst.md |

### 3b. Phase 33 Specialist Skills — cfa/ namespace (9 skills, agent-exclusive)

These skills are currently in `plugins/cfa-core/skills/cfa/`. Each maps directly to its owning agent. The specialist skill documents how that specific analyst persona operates and is not shared across agents.

| Current path | Target tier | Target plugin | New path |
|---|---|---|---|
| plugins/cfa-core/skills/cfa/corp-finance-analyst-chief/ | agent-plugins | chief-analyst | plugins/agent-plugins/chief-analyst/skills/corp-finance-analyst-chief/ |
| plugins/cfa-core/skills/cfa/corp-finance-analyst-credit/ | agent-plugins | credit-analyst | plugins/agent-plugins/credit-analyst/skills/corp-finance-analyst-credit/ |
| plugins/cfa-core/skills/cfa/corp-finance-analyst-derivatives/ | agent-plugins | derivatives-analyst | plugins/agent-plugins/derivatives-analyst/skills/corp-finance-analyst-derivatives/ |
| plugins/cfa-core/skills/cfa/corp-finance-analyst-equity/ | agent-plugins | equity-analyst | plugins/agent-plugins/equity-analyst/skills/corp-finance-analyst-equity/ |
| plugins/cfa-core/skills/cfa/corp-finance-analyst-esg-regulatory/ | agent-plugins | esg-regulatory-analyst | plugins/agent-plugins/esg-regulatory-analyst/skills/corp-finance-analyst-esg-regulatory/ |
| plugins/cfa-core/skills/cfa/corp-finance-analyst-fixed-income/ | agent-plugins | fixed-income-analyst | plugins/agent-plugins/fixed-income-analyst/skills/corp-finance-analyst-fixed-income/ |
| plugins/cfa-core/skills/cfa/corp-finance-analyst-macro/ | agent-plugins | macro-analyst | plugins/agent-plugins/macro-analyst/skills/corp-finance-analyst-macro/ |
| plugins/cfa-core/skills/cfa/corp-finance-analyst-private-markets/ | agent-plugins | private-markets-analyst | plugins/agent-plugins/private-markets-analyst/skills/corp-finance-analyst-private-markets/ |
| plugins/cfa-core/skills/cfa/corp-finance-analyst-quant-risk/ | agent-plugins | quant-risk-analyst | plugins/agent-plugins/quant-risk-analyst/skills/corp-finance-analyst-quant-risk/ |

### 3c. Broad Analyst Skills (4 skills) — Move to vertical-plugins/foundations/

**Resolved decision**: `corp-finance-analyst-{core,markets,regulatory,risk}` move to a new `vertical-plugins/foundations/` plugin. Rationale: these 4 skills are consumed by nearly every cookbook (all 15 cookbooks use at least one), making them foundational cross-cutting infrastructure, not domain-specific. They cannot live in `cfa-core` because `cfa-core` becomes a pure compute MCP after the restructure. They are not exclusive to any agent or vertical. A dedicated `foundations` plugin is the correct answer — it mirrors a base-layer pattern common in plugin architectures.

| Current path | Target tier | Target plugin | New path |
|---|---|---|---|
| plugins/cfa-core/skills/corp-finance-analyst-core/ | vertical-plugins | foundations | plugins/vertical-plugins/foundations/skills/corp-finance-analyst-core/ |
| plugins/cfa-core/skills/corp-finance-analyst-markets/ | vertical-plugins | foundations | plugins/vertical-plugins/foundations/skills/corp-finance-analyst-markets/ |
| plugins/cfa-core/skills/corp-finance-analyst-regulatory/ | vertical-plugins | foundations | plugins/vertical-plugins/foundations/skills/corp-finance-analyst-regulatory/ |
| plugins/cfa-core/skills/corp-finance-analyst-risk/ | vertical-plugins | foundations | plugins/vertical-plugins/foundations/skills/corp-finance-analyst-risk/ |

### 3d. Compute Tool Reference Skills (4 skills) — Stay in cfa-core

**Resolved decision**: `corp-finance-tools-{core,markets,regulatory,risk}` remain in `plugins/cfa-core/skills/`. These are documentation of the Rust compute tool API surface, not analyst workflow skills. They belong adjacent to the compute engine. The `cfa-core` plugin retains a `skills/` directory for these 4 reference skills only.

| Current path | Target tier | Target plugin | New path |
|---|---|---|---|
| plugins/cfa-core/skills/corp-finance-tools-core/ | cfa-core | cfa-core | plugins/cfa-core/skills/corp-finance-tools-core/ (unchanged) |
| plugins/cfa-core/skills/corp-finance-tools-markets/ | cfa-core | cfa-core | plugins/cfa-core/skills/corp-finance-tools-markets/ (unchanged) |
| plugins/cfa-core/skills/corp-finance-tools-regulatory/ | cfa-core | cfa-core | plugins/cfa-core/skills/corp-finance-tools-regulatory/ (unchanged) |
| plugins/cfa-core/skills/corp-finance-tools-risk/ | cfa-core | cfa-core | plugins/cfa-core/skills/corp-finance-tools-risk/ (unchanged) |

### 3e. Workflow Skills (66 skills) — Vertical plugins by prefix rule

Routing is deterministic by slug prefix. Rule: the prefix before the second hyphen determines the vertical. New paths follow `plugins/vertical-plugins/<domain>/skills/<original-slug>/`.

| Slug prefix | Count | Target vertical-plugin |
|---|---|---|
| `workflow-er-*` | 7 | equity-research |
| `workflow-ib-*` | 7 | investment-banking |
| `workflow-fund-*` | 6 | fund-admin |
| `workflow-pe-*` | 8 | private-equity |
| `workflow-wm-*` | 6 | wealth-management |
| `workflow-kyc-*` | 6 | operations |
| `workflow-fa-*` | 5 | financial-analysis |
| `workflow-ma-*` | 5 | financial-analysis (model-audit sub-domain) |
| `workflow-deal-*` | 3 | financial-analysis (deal standards) |
| `workflow-data-*` | 4 | financial-analysis (data hygiene) |
| `workflow-pptx-*` | 4 | financial-analysis (PPTX authoring) |
| `workflow-xlsx-*` | 4 | financial-analysis (XLSX authoring) |
| `workflow-confidentiality-disclaimers` | 1 | financial-analysis |

Full list: `workflow-er-{earnings-update, idea-screening, initiating-coverage, model-update, morning-note, sector-overview, thesis-tracker}` / `workflow-ib-{buyer-list, cim, datapack, merger-model, pitch-deck, process-letter, teaser}` / `workflow-fund-{break-trace, fpa-variance-commentary, gl-reconciliation, nav-tieout, period-end-accruals, period-rollforward}` / `workflow-pe-{ai-readiness, deal-sourcing, due-diligence, ic-memo, portfolio-monitoring, returns-analysis, unit-economics, value-creation-plan}` / `workflow-wm-{client-meeting-prep, client-report, financial-planning, investment-proposal, portfolio-rebalancing, tax-loss-harvesting}` / `workflow-kyc-{beneficial-ownership, customer-intake, monitoring-triggers, pep-screening, sanctions-screening, source-of-funds}` / remaining 26 → financial-analysis.

### 3f. FMP Skills (6 skills) — partner-built/fmp

**Resolved decision**: the 6 `fmp-*` skills are how-to-use instructions for the FMP commercial API. They are vendor knowledge, not domain knowledge. They move to `partner-built/fmp/` alongside the FMP MCP server. This is consistent with how `vendor-lseg` lives in `partner-built/lseg`.

| Current path | Target tier | Target plugin | New path |
|---|---|---|---|
| plugins/cfa-core/skills/fmp-etf-funds/ | partner-built | fmp | plugins/partner-built/fmp/skills/fmp-etf-funds/ |
| plugins/cfa-core/skills/fmp-market-data/ | partner-built | fmp | plugins/partner-built/fmp/skills/fmp-market-data/ |
| plugins/cfa-core/skills/fmp-news-intelligence/ | partner-built | fmp | plugins/partner-built/fmp/skills/fmp-news-intelligence/ |
| plugins/cfa-core/skills/fmp-research/ | partner-built | fmp | plugins/partner-built/fmp/skills/fmp-research/ |
| plugins/cfa-core/skills/fmp-sec-compliance/ | partner-built | fmp | plugins/partner-built/fmp/skills/fmp-sec-compliance/ |
| plugins/cfa-core/skills/fmp-technicals/ | partner-built | fmp | plugins/partner-built/fmp/skills/fmp-technicals/ |

The FMP MCP server dist currently in `plugins/cfa-pro/mcp/dist/` (the `@robotixai/fmp-mcp-server` package) moves to `plugins/partner-built/fmp/mcp/`. The `.mcp.json` for FMP registers the local STDIO server.

### 3g. Free Data Skills (5 skills) — partner-built/free-data

**Resolved decision**: `data-edgar`, `data-figi`, `data-fred`, `data-wb`, `data-yf` are vendor-specific API skills (even though free). They describe how to use external services (SEC EDGAR, OpenFIGI, FRED, World Bank, Yahoo Finance). Moving them to `partner-built/free-data/` makes this explicit — free-data is its own partner plugin, not a domain vertical.

| Current path | Target tier | Target plugin | New path |
|---|---|---|---|
| plugins/cfa-core/skills/data-edgar/ | partner-built | free-data | plugins/partner-built/free-data/skills/data-edgar/ |
| plugins/cfa-core/skills/data-figi/ | partner-built | free-data | plugins/partner-built/free-data/skills/data-figi/ |
| plugins/cfa-core/skills/data-fred/ | partner-built | free-data | plugins/partner-built/free-data/skills/data-fred/ |
| plugins/cfa-core/skills/data-wb/ | partner-built | free-data | plugins/partner-built/free-data/skills/data-wb/ |
| plugins/cfa-core/skills/data-yf/ | partner-built | free-data | plugins/partner-built/free-data/skills/data-yf/ |
| plugins/cfa-core/skills/data-mtnewswire/ | partner-built | free-data | plugins/partner-built/free-data/skills/data-mtnewswire/ |

The `cfa-data` plugin MCP server (FRED, EDGAR, WB, YF, FIGI endpoints) remains at `plugins/cfa-data/mcp/` and is registered in `plugins/partner-built/free-data/.mcp.json`.

### 3h. Geopolitical Skills (4 skills) — vertical-plugins/macro

These skills document how to use public geopolitical/alternative data APIs. They serve the macro-analyst agent and support cross-vertical risk analysis. A new `vertical-plugins/macro` plugin (we add this; Anthropic does not have it upstream) captures the geopolitical, monetary policy, and macro workflow domain.

| Current path | Target tier | Target plugin | New path |
|---|---|---|---|
| plugins/cfa-core/skills/geopolitical-alternative/ | vertical-plugins | macro | plugins/vertical-plugins/macro/skills/geopolitical-alternative/ |
| plugins/cfa-core/skills/geopolitical-conflict/ | vertical-plugins | macro | plugins/vertical-plugins/macro/skills/geopolitical-conflict/ |
| plugins/cfa-core/skills/geopolitical-environment/ | vertical-plugins | macro | plugins/vertical-plugins/macro/skills/geopolitical-environment/ |
| plugins/cfa-core/skills/geopolitical-trade/ | vertical-plugins | macro | plugins/vertical-plugins/macro/skills/geopolitical-trade/ |

### 3i. Cross-Cutting Infrastructure Skills (2 skills) — cfa-core

`specflow` (executable contract verification) and `cfa-managed-agent` (deployment orchestration) are repo infrastructure, not domain or vendor knowledge. They remain in `plugins/cfa-core/skills/`.

| Current path | Target | New path |
|---|---|---|
| plugins/cfa-core/skills/specflow/ | cfa-core | plugins/cfa-core/skills/specflow/ (unchanged) |
| plugins/cfa-core/skills/cfa-managed-agent/ | cfa-core | plugins/cfa-core/skills/cfa-managed-agent/ (unchanged) |

### 3j. Slash Commands (57 files) — Vertical plugins by subject matter

Commands co-locate with their vertical. Routing rule: derive vertical from the command's subject, not slug prefix (many commands lack a prefix). Target distribution:

| Target vertical | Commands (examples) | Count |
|---|---|---|
| equity-research | earnings, earnings-preview, initiate-coverage, model-update, morning-note, sector, screen, thesis, catalysts, one-pager, comps, dcf | ~12 |
| investment-banking | cim, teaser, pitch-deck, process-letter, buyer-list, merger-model | ~8 |
| private-equity | ic-memo, lbo, dd-checklist, dd-prep, screen-deal, returns, value-creation, unit-economics, ai-readiness, deal-tracker, source | ~10 |
| fund-admin | fund-ops, 3-statement-model | ~2 |
| wealth-management | client-report, client-review, financial-plan, rebalance, tlh, proposal, portfolio | ~7 |
| financial-analysis | acquisition-model, jurisdiction-comparison, fund-migration, property-valuation, competitive-analysis, debug-model | ~6 |
| operations | credit-analysis | ~1 |
| macro | conflict-risk, disaster-monitor, macro-rates, trade-policy, alt-data | ~5 |
| derivatives | derivatives-valuation, option-vol | ~2 |
| fixed-income | bond-analysis, bond-rv, fi-portfolio, fx-carry, swap-curve | ~4 |

### 3k. cfa-pro Vendor Skills (8 skills) — partner-built per vendor

| Current path | Target tier | Target plugin | New path |
|---|---|---|---|
| plugins/cfa-pro/skills/vendor-factset/ | partner-built | factset | plugins/partner-built/factset/skills/vendor-factset/ |
| plugins/cfa-pro/skills/vendor-lseg/ | partner-built | lseg | plugins/partner-built/lseg/skills/vendor-lseg/ |
| plugins/cfa-pro/skills/vendor-moodys/ | partner-built | moodys | plugins/partner-built/moodys/skills/vendor-moodys/ |
| plugins/cfa-pro/skills/vendor-morningstar/ | partner-built | morningstar | plugins/partner-built/morningstar/skills/vendor-morningstar/ |
| plugins/cfa-pro/skills/vendor-pitchbook/ | partner-built | pitchbook | plugins/partner-built/pitchbook/skills/vendor-pitchbook/ |
| plugins/cfa-pro/skills/vendor-sp-global/ | partner-built | sp-global | plugins/partner-built/sp-global/skills/vendor-sp-global/ |
| plugins/cfa-pro/skills/data-aiera/ | partner-built | aiera | plugins/partner-built/aiera/skills/data-aiera/ |
| plugins/cfa-pro/skills/data-daloopa/ | partner-built | daloopa | plugins/partner-built/daloopa/skills/data-daloopa/ |

The vendor MCP server (`plugins/cfa-pro/mcp/vendors/dist/`) splits: each vendor's MCP endpoint becomes its own `.mcp.json` in the corresponding `partner-built/<vendor>/` directory. The unified vendor MCP server package at `plugins/cfa-pro/mcp/` is deprecated and its endpoints redistributed.

### 3l. Hooks — vertical-plugins/financial-analysis

The current `plugins/cfa-core/hooks/hooks.json` contains 5 PreToolUse and 2 PostToolUse hooks covering financial-analysis, private-equity, investment-banking, equity-research, and security/audit concerns. After restructure, domain-specific hooks move to their vertical plugin. The security/audit hooks move to `plugins/cfa-core/hooks/hooks.json` (cross-cutting infra). A new `plugins/vertical-plugins/financial-analysis/hooks/hooks.json` carries the model-artefacts source-check hook. PE CONFIDENTIAL watermark hook moves to `plugins/vertical-plugins/private-equity/hooks/hooks.json`. IB citation hook moves to `plugins/vertical-plugins/investment-banking/hooks/`. ER provenance hook moves to `plugins/vertical-plugins/equity-research/hooks/`.

---

## 4. Plugin Manifests (plugin.json specs)

All manifests follow the verified upstream shape (no `dependencies` field; skill references are `from_plugin:` paths in cookbook JSON).

**agent-plugins** (24 total: 9 analyst + 15 cookbook): `{ "name": "<slug>", "version": "0.1.0", "description": "...", "author": { "name": "Anthropic FSI" } }`. The 15 cookbook agents co-locate their `agent.json` and `steering-examples.json` inside the plugin directory.

**vertical-plugins** (11):

| name | description |
|---|---|
| equity-research | Earnings analysis, initiating coverage, research workflows |
| investment-banking | M&A deal execution: CIM, teasers, merger models, pitch decks |
| private-equity | Deal sourcing, due diligence, IC memos, returns analysis |
| fund-admin | GL reconciliation, NAV tie-out, period-end accruals, FP&A |
| operations | KYC/AML: customer intake, sanctions, PEP, source-of-funds |
| wealth-management | Client meeting prep, financial planning, rebalancing, TLH |
| financial-analysis | Model audit, data hygiene, XLSX/PPTX authoring, deck review |
| foundations | Cross-cutting analyst foundations (4 broad corp-finance-analyst-* skills) |
| macro | Geopolitical risk, macro economics, monetary policy (new) |
| derivatives | Derivatives valuation, options, swaps (stub — new) |
| fixed-income | Bond analysis, yield curve, credit spreads (stub — new) |

`quant-risk` does not get its own vertical; quant-risk workflows absorb into `foundations`.

**partner-built** (10):

| name | version | author | MCP type |
|---|---|---|---|
| lseg | 1.0.0 | LSEG | http |
| sp-global | 1.0.0 | Kensho Technologies | http |
| factset | 0.1.0 | Anthropic FSI | http |
| moodys | 0.1.0 | Anthropic FSI | http |
| morningstar | 0.1.0 | Anthropic FSI | http |
| pitchbook | 0.1.0 | Anthropic FSI | http |
| fmp | 0.1.0 | Anthropic FSI | stdio (local Node.js) |
| free-data | 0.1.0 | Anthropic FSI | stdio (local Node.js) |
| aiera | 0.1.0 | Anthropic FSI | http (pending) |
| daloopa | 0.1.0 | Anthropic FSI | http (pending) |

---

## 5. Harness Loader Changes

Current defaults in `packages/mcp-server/src/tools/agent_infrastructure/cookbook.ts`: `agentsRoot = <workspace>/.claude/agents/cfa/`, `skillsRoot = <workspace>/.claude/skills/`. After restructure, agents live in 5+ trees. Required changes:

**5a. Multi-root agent discovery** — Add `pluginAgentRoots: string[]` that walks `plugins/agent-plugins/*/agents/`. Default: `[join(workspaceRoot, "plugins/agent-plugins")]`. The loader unions all `.yaml` files across roots.

**5b. Multi-root skill resolution** — `from_plugin:` paths remain relative from the cookbook file and self-resolve; no runtime loader change needed. Add `pluginSkillRoots: string[]` for the manifest linter to enumerate all skills: `["plugins/agent-plugins", "plugins/vertical-plugins", "plugins/partner-built", "plugins/cfa-core/skills"]`. The linter emits the attempted absolute path when a skill is not found.

**5c. Backwards compatibility** — Keep old `plugins/cfa-core/skills/<slug>` directories live through W1–W2; delete in W5 after W3 updates all cookbook references. Linter runs `--legacy-ok` during W1–W2, reverts to `--strict` after W3. No symlinks.

**5d. surface_parity.ts** — Hardcodes `plugins/cfa-core/mcp/src/server.ts`; `cfa-core/mcp/` is retained unchanged. No change needed.

---

## 6. Cookbook from_plugin Reference Migration

### Current state (15 cookbooks, typical pattern)
```json
{ "from_plugin": "../../plugins/cfa-core/skills/corp-finance-analyst-core" }
{ "from_plugin": "../../plugins/cfa-core/skills/workflow-er-initiating-coverage" }
{ "from_plugin": "../../plugins/cfa-pro/skills/vendor-lseg" }
```

### Target state (after W3)
```json
{ "from_plugin": "../../plugins/vertical-plugins/foundations/skills/corp-finance-analyst-core" }
{ "from_plugin": "../../plugins/vertical-plugins/equity-research/skills/workflow-er-initiating-coverage" }
{ "from_plugin": "../../plugins/partner-built/lseg/skills/vendor-lseg" }
```

### Migration mapping rules for automated update

| Old prefix | New prefix |
|---|---|
| `../../plugins/cfa-core/skills/corp-finance-analyst-core` | `../../plugins/vertical-plugins/foundations/skills/corp-finance-analyst-core` |
| `../../plugins/cfa-core/skills/corp-finance-analyst-markets` | `../../plugins/vertical-plugins/foundations/skills/corp-finance-analyst-markets` |
| `../../plugins/cfa-core/skills/corp-finance-analyst-regulatory` | `../../plugins/vertical-plugins/foundations/skills/corp-finance-analyst-regulatory` |
| `../../plugins/cfa-core/skills/corp-finance-analyst-risk` | `../../plugins/vertical-plugins/foundations/skills/corp-finance-analyst-risk` |
| `../../plugins/cfa-core/skills/workflow-er-*` | `../../plugins/vertical-plugins/equity-research/skills/workflow-er-*` |
| `../../plugins/cfa-core/skills/workflow-ib-*` | `../../plugins/vertical-plugins/investment-banking/skills/workflow-ib-*` |
| `../../plugins/cfa-core/skills/workflow-fund-*` | `../../plugins/vertical-plugins/fund-admin/skills/workflow-fund-*` |
| `../../plugins/cfa-core/skills/workflow-pe-*` | `../../plugins/vertical-plugins/private-equity/skills/workflow-pe-*` |
| `../../plugins/cfa-core/skills/workflow-wm-*` | `../../plugins/vertical-plugins/wealth-management/skills/workflow-wm-*` |
| `../../plugins/cfa-core/skills/workflow-kyc-*` | `../../plugins/vertical-plugins/operations/skills/workflow-kyc-*` |
| `../../plugins/cfa-core/skills/workflow-*` (remainder) | `../../plugins/vertical-plugins/financial-analysis/skills/workflow-*` |
| `../../plugins/cfa-core/skills/corp-finance-tools-*` | `../../plugins/cfa-core/skills/corp-finance-tools-*` (unchanged) |
| `../../plugins/cfa-pro/skills/vendor-lseg` | `../../plugins/partner-built/lseg/skills/vendor-lseg` |
| `../../plugins/cfa-pro/skills/vendor-sp-global` | `../../plugins/partner-built/sp-global/skills/vendor-sp-global` |
| `../../plugins/cfa-pro/skills/vendor-*` | `../../plugins/partner-built/<vendor>/skills/vendor-*` |

This migration is mechanical and can be executed by a single sed/jq pass over all `managed-agent-cookbooks/*/agent.json` files.

---

## 7. CI Workflow Updates

| Workflow | Change required |
|---|---|
| `manifest-check.yml` | Add `--plugin-roots plugins/agent-plugins,plugins/vertical-plugins,plugins/partner-built,plugins/cfa-core/skills` flag; add `--legacy-ok` during W1–W2 transition |
| `cookbooks.yml` | Update `skills_root` param if hardcoded; after W4 the linter self-discovers from cookbook `from_plugin:` paths |
| `typescript.yml` | No change — `plugins/cfa-core/mcp/` is unchanged |
| `surface-parity.yml` | No change — hardcoded path to `plugins/cfa-core/mcp/src/server.ts` remains valid |
| rust.yml, ci.yml, dependabot-auto.yml, lockfile-guard.yml, release.yml, publish.yml | No plugin path references; unchanged |

---

## 8. Migration Sequence

| Wave | Action | Duration | LOC delta | Gate |
|---|---|---|---|---|
| W1 | Create empty plugin dirs + stub plugin.json and .mcp.json | 1 day | +200 | CI green (old paths untouched) |
| W2 | Move skills + agents tier by tier; keep old paths alive | 2 days | 0 net | Linter `--legacy-ok`; cookbook-smoke passes |
| W3 | Update all 15 cookbook from_plugin references | 1 day | ~60 changed | `npm test`; flip linter back to `--strict` |
| W4 | Add multi-root config to harness loader + linter | 1 day | +140 | Full test suite green |
| W5 | Delete old monolithic cfa-core/cfa-pro/cfa-data dirs | 0.5 days | ~-250 files | CI must be green before delete |
| W6 | Update CI workflows (--plugin-roots flags) | 0.5 days | +30 | Workflow passes |
| W7 | Update CLAUDE.md, ADRs, DDD context maps | 0.5 days | +50 | Docs only |

**Total: ~6.5 working days. Net LOC: +430. File operations: ~375 moves/creates/deletes.**

---

## 9. Test Impact

| Test file | Change type | Estimated cases affected |
|---|---|---|
| `cookbook-smoke.test.ts` | Update `from_plugin:` fixture paths | ~15 |
| `yaml-manifest-migration.test.ts` | Scan `plugins/agent-plugins/*/agents/` instead of `plugins/cfa-core/agents/cfa/` | ~5 |
| `checker.test.ts` | Add `--plugin-roots` flag coverage + `--legacy-ok` mode | ~8 |
| `handoff-dispatch` tests | Update cookbook resolver fixture paths | ~5 |
| `cookbook.test.ts` | `agentsRoot`/`skillsRoot` default changes | ~4 |
| `surface-parity` tests | `plugins/cfa-core/mcp/` unchanged — no impact | 0 |

**Total: ~37 test case changes, all path-update-only. No test logic rewrites required.** The linter's `--strict` mode catches any missed references before merge.

---

## 10. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Missed skill move breaks a cookbook silently | Medium | High | `cookbook-smoke.test.ts` validates all 15 cookbooks end-to-end. The `--strict` linter catches dangling `from_plugin:` references. |
| Harness loader fails to find skill across multi-root | Medium | High | Explicit error message in loader naming all attempted absolute paths. Integration test for multi-root resolution in Wave 4. |
| Long-running PR causes merge conflicts mid-migration | High | Medium | Each wave is an atomic PR. No wave PR touches more than one concern. Maximum 7 PRs. |
| External consumers (npm package, deployment scripts) reference old paths | Low | Medium | `plugins/cfa-core/` is not published to npm; the npm package is `packages/cfa-core/` (NAPI bindings), which is unaffected. Cookbook deployment uses `from_plugin:` paths which are updated in Wave 3. |
| Vendor MCP server split breaks auth config | Low | High | Vendor MCP endpoints are HTTP URLs, not local processes. Splitting one multi-vendor server into per-vendor `.mcp.json` files is additive. Old unified config is deprecated only after each vendor `.mcp.json` is tested. |
| `surface-parity.yml` fails because it expects `plugins/cfa-core/mcp/` | Low | Low | `cfa-core/mcp/` is explicitly retained. No change needed. |
| CI runner on Windows fails on symlinks (if Option B selected) | N/A | N/A | Option A (keep old paths alive through W5) was selected; no symlinks. |

---

## 11. Open Questions — Resolved and Flagged

### Resolved in this design

**Q: Should `corp-finance-analyst-{core,markets,regulatory,risk}` move to `vertical-plugins/foundations/`?**
A: Yes. They are too cross-cutting for any single vertical and cannot stay in `cfa-core` (which becomes pure compute). `foundations` is the correct home.

**Q: Should `corp-finance-tools-*` be distributed across verticals?**
A: No. They are compute API references; stay in `cfa-core/skills/` adjacent to the compute engine.

**Q: Where do FMP/EDGAR/FRED/WB free-data skills go?**
A: FMP → `partner-built/fmp/`. EDGAR/FRED/WB/YF/FIGI → `partner-built/free-data/`. Each is vendor-specific API knowledge, not domain knowledge.

**Q: Should the 4 broad analyst skills become their own foundation tier plugin?**
A: Yes, as `vertical-plugins/foundations/` — cross-cutting, used by all 15 cookbooks, belongs in the skill tier not the compute tier.

**Q: The 9 Phase 33 specialist skills — do they move into each agent-plugin or stay together?**
A: Each moves into its corresponding agent-plugin. Agent-exclusive; co-location makes the bundle self-contained.

**Q: Does `chief-analyst` need its own plugin or live under `agent-plugins/chief/`?**
A: `agent-plugins/chief-analyst/` — full name, consistent with all other analyst agent plugins.

**Q: Cookbook subagents — own plugin tier?**
A: No. Upstream pattern: subagents remain inline in `agent.json`. Top-level cookbook agents each get an agent-plugin directory; subagents stay embedded.

### Flagged for user decision

**Q: Should `derivatives` and `fixed-income` be rich verticals or skill-only stubs in Phase 40?**
Recommendation: create stubs now; no domain-specific workflow skills exist yet for these verticals. Populate in a follow-on phase.

**Q: Should `aiera` and `daloopa` get partner-built plugins?**
Both are currently stub skills with minimal content. Consider removing rather than migrating empty shells. Flag before Wave 2.

**Q: Publication strategy for agent-plugins?**
Upstream Anthropic ships agent-plugins as installable via `claude install`. Does this repo follow suit? Determines whether each agent-plugin needs a `README.md` and install instructions. Out of scope for Phase 40; flag for GA planning.

---

## Appendix: Target Directory Tree (abbreviated)

```
plugins/
  agent-plugins/
    {chief,credit,derivatives,equity,esg-regulatory,fixed-income,macro,
     private-markets,quant-risk}-analyst/
      .claude-plugin/plugin.json
      agents/<slug>.{yaml,md}
      skills/corp-finance-analyst-<slug>/
    # cookbook agents (15, one dir each):
    {earnings-reviewer,gl-reconciler,kyc-screener,lseg-rates-monitor,
     model-builder,month-end-closer,pitch-deck-builder,sector-research,
     sp-credit-research,valuation-reviewer,wealth-meeting-prep,
     credit-analyst-cookbook,lp-statement-auditor,
     private-markets-analyst-cookbook}/
  vertical-plugins/
    equity-research/     # 7 workflow-er-* skills, 12 commands, hooks
    financial-analysis/  # 26 workflow-{fa,ma,deal,data,pptx,xlsx}-* skills, hooks
    foundations/         # 4 corp-finance-analyst-* skills
    fund-admin/          # 6 workflow-fund-* skills
    investment-banking/  # 7 workflow-ib-* skills, hooks
    macro/               # 4 geopolitical-* skills
    operations/          # 6 workflow-kyc-* skills
    private-equity/      # 8 workflow-pe-* skills, hooks
    wealth-management/   # 6 workflow-wm-* skills
    derivatives/         # stub — Phase 40
    fixed-income/        # stub — Phase 40
  partner-built/
    factset/ moodys/ morningstar/ pitchbook/ lseg/ sp-global/
      (.claude-plugin/plugin.json  .mcp.json  skills/vendor-<slug>/)
    fmp/
      (.claude-plugin/plugin.json  .mcp.json  mcp/  skills/fmp-*/)
    free-data/
      (.claude-plugin/plugin.json  .mcp.json  skills/data-*/)
    aiera/ daloopa/   # pending user decision
  cfa-core/            # compute backbone — unchanged structure
    mcp/               # WASM/NAPI; 206 Rust tools
    skills/corp-finance-tools-{core,markets,regulatory,risk}/
    skills/specflow/  skills/cfa-managed-agent/
    hooks/hooks.json   # security/audit hooks only
  cfa-data/            # deprecated after Wave 5
```
