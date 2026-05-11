# ADR-043: Three-Tier Plugin Architecture

## Status: Accepted

## Date: 2026-05-11

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`plugins`, `architecture`, `agent-plugins`, `vertical-plugins`, `partner-built`, `phase-40`

## Context

### Pre-Phase-40 layout

Through Phase 39, the plugin tree was monolithic. Two directories held everything:

- `plugins/cfa-core/` — 9 specialist agents, all 66 thin workflow skills, 6 FMP skills,
  6 free-data skills, 2 infrastructure skills (specflow, cfa-managed-agent), 57 slash
  commands, 5 PreToolUse + 2 PostToolUse hooks, and the WASM/NAPI compute MCP server
- `plugins/cfa-pro/` — 6 paid-vendor skills (LSEG, S&P, FactSet, Moody's, Morningstar,
  PitchBook) + 2 stub vendor skills (Aiera, Daloopa) + a unified multi-vendor MCP server

This layout has three concrete problems:

1. **No install granularity.** An equity researcher installing the plugin stack receives all
   66 workflow skills and all 6 vendor skill files even if they have no vendor subscriptions
   and need only equity-research workflows. There is no `claude plugin install
   equity-research`; it is all-or-nothing.

2. **Cognitive cost for unused vendors.** Every cookbook and agent has visibility into vendor
   skills for LSEG, FactSet, S&P, and others simultaneously. Users without subscriptions
   encounter tool descriptions they cannot use.

3. **Drift from Anthropic upstream.** Anthropic's canonical `financial-services` repository
   uses a three-tier model (`agent-plugins/`, `vertical-plugins/`, `partner-built/`) with
   one plugin per deployable agent, one plugin per business domain, and one plugin per data
   vendor. Our monolithic layout prevented cookbook-level compatibility: a cookbook authored
   against the upstream tier structure references paths that do not exist in our repo.

### Trigger

The user pointed at the Anthropic upstream layout (verified at
`github.com/anthropics/financial-services` on 2026-05-10) and asked to adopt it. The upstream
ships 10 agent-plugins, 7 vertical-plugins, and 2 partner-built plugins. Each plugin has a
minimal `.claude-plugin/plugin.json` (name, version, description, author). Partner-built
plugins also carry a `.mcp.json` for MCP server endpoint registration.

Phase 40 Wave 1-W5 executed the full migration. W6 updated CI workflows. W7 (this ADR)
documents the decision.

---

## Decision

Split `plugins/` into three independently installable tiers plus a retained thin compute
backbone:

### Tier 1 — `plugins/agent-plugins/` (24 plugins)

One plugin per deployable agent. Contains the agent's `.yaml` manifest, `.md` system prompt,
and skills used exclusively by that one agent.

- 9 specialist analyst agents: chief-analyst, credit-analyst, derivatives-analyst,
  equity-analyst, esg-regulatory-analyst, fixed-income-analyst, macro-analyst,
  private-markets-analyst, quant-risk-analyst
- 15 cookbook agents: earnings-reviewer, gl-reconciler, kyc-screener, lseg-rates-monitor,
  model-builder, month-end-closer, pitch-deck-builder, sector-research, sp-credit-research,
  valuation-reviewer, wealth-meeting-prep, credit-analyst-cookbook, lp-statement-auditor,
  private-markets-analyst-cookbook, plus one additional cookbook agent

Each analyst plugin co-locates its Phase 33 specialist skill
(`corp-finance-analyst-<slug>/`) inside the plugin directory. Skills exclusive to one agent
are not shared.

### Tier 2 — `plugins/vertical-plugins/` (11 plugins)

One plugin per business domain. Holds cross-cutting workflow skills and slash commands
consumed by multiple agents in that domain. No MCP server configuration.

| Plugin | Skills | Commands |
|---|---|---|
| equity-research | 7 `workflow-er-*` skills | ~12 commands |
| investment-banking | 7 `workflow-ib-*` skills | ~8 commands |
| private-equity | 8 `workflow-pe-*` skills | ~10 commands |
| fund-admin | 6 `workflow-fund-*` skills | ~2 commands |
| operations | 6 `workflow-kyc-*` skills | ~1 command |
| wealth-management | 6 `workflow-wm-*` skills | ~7 commands |
| financial-analysis | 26 `workflow-{fa,ma,deal,data,pptx,xlsx,confidentiality}-*` skills | ~6 commands |
| foundations | 4 `corp-finance-analyst-{core,markets,regulatory,risk}` skills | — |
| macro | 4 `geopolitical-*` skills | ~5 commands |
| derivatives | stub | ~2 commands |
| fixed-income | stub | ~4 commands |

The `foundations` plugin hosts the four broad analyst skills consumed by all 15 cookbooks.
These are cross-cutting infrastructure, not domain-specific, and cannot live in `cfa-core`
(which becomes a pure compute plugin after the migration).

### Tier 3 — `plugins/partner-built/` (10 plugins)

One plugin per data vendor. Bundles vendor-specific MCP server configuration (`.mcp.json`),
vendor-use skill files, and any vendor-scoped slash commands.

| Plugin | MCP type | Author |
|---|---|---|
| lseg | HTTP | LSEG |
| sp-global | HTTP | Kensho Technologies |
| factset | HTTP | Anthropic FSI |
| moodys | HTTP | Anthropic FSI |
| morningstar | HTTP | Anthropic FSI |
| pitchbook | HTTP | Anthropic FSI |
| fmp | STDIO (local Node.js) | Anthropic FSI |
| free-data | STDIO (local Node.js) | Anthropic FSI |
| aiera | HTTP (stub) | Anthropic FSI |
| daloopa | HTTP (stub) | Anthropic FSI |

The FMP MCP server dist (`@robotixai/fmp-mcp-server`) moved from `plugins/cfa-pro/mcp/` to
`plugins/partner-built/fmp/mcp/`. The `cfa-data` MCP server (FRED, EDGAR, WB, YF, FIGI)
moved to `plugins/partner-built/free-data/.mcp.json`.

### Retained — `plugins/cfa-core/`

The compute backbone is retained as a thin infrastructure plugin:

- `mcp/` — WASM/NAPI compute MCP server (~206 Rust tools, 128-bit Decimal precision)
- `skills/corp-finance-tools-{core,markets,regulatory,risk}/` — 4 compute API reference skills
- `skills/specflow/` and `skills/cfa-managed-agent/` — infrastructure skills
- `hooks/hooks.json` — security and audit hooks (cross-cutting; not domain-specific)

`cfa-core` is not a user-facing install target. It is depended upon by vertical and agent
plugins but not directly installable on its own. The old `cfa-pro` directory is deleted.

### Plugin manifests

Every plugin directory contains `.claude-plugin/plugin.json` matching the upstream shape:

```json
{
  "name": "<slug>",
  "version": "0.1.0",
  "description": "...",
  "author": { "name": "Anthropic FSI" }
}
```

Third-party-authored plugins (lseg, sp-global) carry additional fields: `homepage`,
`repository`, `license`, `keywords`. The upstream shape has no `dependencies` field;
cross-plugin skill references are expressed via `from_plugin:` paths inside cookbook
`agent.json` files.

### Three operational modes

**Installation.** `claude plugin install plugins/<tier>/<slug>` installs a single plugin.
Per-plugin npm packaging for broader distribution is deferred to a GA pass (see Future Work).
In the monorepo, all plugins are available locally without installation steps.

**Composition.** Cookbooks reference skills across tiers via repo-relative `from_plugin:`
paths. Example: an equity-research cookbook's `agent.json` carries:

```json
{ "from_plugin": "../../plugins/vertical-plugins/equity-research/skills/workflow-er-initiating-coverage" }
{ "from_plugin": "../../plugins/vertical-plugins/foundations/skills/corp-finance-analyst-core" }
{ "from_plugin": "../../plugins/partner-built/lseg/skills/vendor-lseg" }
```

The dependency graph flows: agent-plugin → vertical-plugin → partner-built, with `cfa-core`
as a shared compute substrate. No runtime enforcement exists yet (see Future Work).

**Discovery.** The harness loader was extended in W4 to walk all three tiers:
`pluginAgentRoots: ["plugins/agent-plugins"]` for agent YAML discovery, and
`pluginSkillRoots: ["plugins/agent-plugins", "plugins/vertical-plugins",
"plugins/partner-built", "plugins/cfa-core/skills"]` for the manifest linter's skill
enumeration. A legacy fallback to the old `plugins/cfa-core/agents/cfa/` path was kept
through W4 and removed in W5.

---

## Consequences

### Positive

- **Independent installability.** An LSEG-only user installs `partner-built/lseg` and any
  relevant vertical plugins; FactSet, Moody's, and PitchBook directories remain absent from
  their workspace. This was not possible in the monolithic layout.
- **Vendor isolation.** Agents receive system prompts and tool lists scoped to installed
  plugins only. A cookbook configured without `partner-built/factset` never sees FactSet
  tool descriptions.
- **Vertical bundling.** A fund-admin team installs `vertical-plugins/fund-admin` and gets
  exactly the six GL/NAV/FP&A workflow skills and two commands they need, without equity
  research or PE tooling.
- **Upstream compatibility.** Plugin manifests, cookbook `from_plugin:` paths, and tier
  directory names are now aligned with Anthropic's `financial-services` upstream. Cookbooks
  can be authored or forked against either repo without path translation.
- **Manifest linter coverage.** The `manifest-check.yml --strict` CI workflow validates all
  93 plugin manifests across all tiers. A missing or malformed `plugin.json` is a CI
  failure, not a runtime surprise.
- **Zero test regressions.** 631 of 639 harness tests passed through the full migration
  (W1-W5). The 8 skipped tests are pre-existing skips unrelated to the restructure.

### Negative

- **Directory count increase.** The repo now contains 45 plugin directories (24 + 11 + 10)
  where it previously had 2. Navigating the repo tree requires understanding tier semantics.
  Mitigation: each tier has a clear naming convention and the phase-40 plan document
  (`docs/plans/archive/phase-40-three-tier-plugins.md`) provides the full mapping table.
- **Skill resolution uses multi-root walk.** The harness loader previously resolved skills
  from a single root; it now walks four roots. For large workspaces with many plugins this
  adds a small enumeration cost at startup. Mitigation: the loader caches the manifest index
  for the process lifetime and the walk is O(n) over directory entries, not recursive.
- **Longer `from_plugin:` paths in cookbooks.** Old references were
  `../../plugins/cfa-core/skills/<slug>`; new references span
  `../../plugins/vertical-plugins/<tier>/skills/<slug>`. Path depth increased by one
  segment. Mitigation: the W3 path migration was mechanical (automated sed/jq pass over all
  15 cookbook `agent.json` files); no cookbook was updated manually.
- **Cross-plugin dependencies not enforced.** The `plugin.json` manifest does not carry a
  `dependencies` field (upstream omits it; dependencies are expressed via `from_plugin:`
  references at cookbook level). An agent-plugin can reference a vertical-plugin skill that
  is not installed, and no validation will catch this until the cookbook is run.
  See Future Work.

### Neutral

- The 4 `corp-finance-tools-*` compute reference skills remain in `cfa-core/skills/`
  adjacent to the compute engine. This resolves the design question of whether tool-reference
  skills belong in foundations or in cfa-core: they document the Rust API surface, not
  analyst workflow patterns.
- The `foundations` vertical plugin has no domain-specific workflow skills; it exists solely
  as a home for the 4 broad `corp-finance-analyst-*` skills consumed by all 15 cookbooks.
  It can be merged into `cfa-core` in a future phase if the distinction between "analyst
  foundation" and "compute API reference" is found to be too fine-grained.
- The migration preserved the pre-existing decision (ADR-031) that skill bodies are prose
  documentation, not logic. No skill content was changed; only file paths were updated.

---

## Alternatives Considered

**Stay monolithic (cfa-core + cfa-pro status quo)** — Rejected. Blocks per-plugin install
granularity. Prevents cookbook-level compatibility with Anthropic upstream. The cognitive
cost of 90+ skills always-visible to every agent and every cookbook is a recurring friction
point reported in operator feedback.

**Two-tier layout (agent-plugins + partner-built, skip verticals)** — Rejected. Vertical
workflow skills like `workflow-er-initiating-coverage` span the equity-analyst agent and
multiple cookbooks (sector-research, earnings-reviewer). Without a vertical tier these skills
would either be duplicated into each consuming agent-plugin (violating the DRY principle from
Phase 33's skill decomposition) or consolidated into an oversized `cfa-core` that defeats
the purpose of the restructure.

**Per-cookbook plugin only** — Rejected. Each cookbook would need to embed its own copies of
cross-cutting skills. With 15 cookbooks each copying 4-8 foundational skills, the repo would
carry 60-120 duplicate skill files. Phase 33 (ADR-031) explicitly introduced the thin-skill
decomposition to eliminate this duplication.

**Single plugin with subdirectories** — Rejected. Equivalent to the monolithic layout with
renamed directories. Does not enable install granularity because `claude plugin install`
operates at plugin-directory granularity.

---

## Future Work

The following items are out of scope for Phase 40 and should be addressed in follow-on phases:

- **Per-plugin `dependencies` validation.** The `plugin.json` manifest `dependencies` field
  is parsed by the linter but not checked against installed plugins at runtime. A follow-on
  wave should add install-time validation: if `agent-plugins/equity-analyst` declares
  `"dependencies": ["vertical-plugins/equity-research", "vertical-plugins/foundations"]`,
  the harness should warn or error when those plugins are absent.

- **Per-plugin npm distribution.** `claude plugin install` currently works only within the
  monorepo. Packaging each plugin as a standalone npm package for external distribution
  (matching the Anthropic upstream pattern for LSEG and S&P Global) is deferred to a GA
  release pass.

- **Cross-tier plugin composition tests.** The W4 integration tests cover multi-root skill
  resolution for known paths but do not exhaustively verify all 45 plugin directories under
  all composition combinations. A dedicated `plugin-composition.test.ts` should assert that
  each cookbook's `from_plugin:` references resolve against the installed tier tree.

- **Derivatives and fixed-income vertical plugins.** Both are stubs with no domain workflow
  skills. Derivatives valuation, options, and swaps workflows should populate
  `vertical-plugins/derivatives/` in a follow-on phase. Bond analysis, yield curve, and
  credit spread workflows should populate `vertical-plugins/fixed-income/`.

- **Aiera and Daloopa partner-built plugins.** Both are stubs with minimal content. If no
  commitment to flesh them out exists within two phases, they should be removed to keep the
  partner-built tier accurate.

---

## Links

- Full migration design: `docs/plans/archive/phase-40-three-tier-plugins.md`
- Harness multi-root loader: `packages/harness/src/core/cookbook.ts` (W4 changes)
- Manifest linter: `.github/workflows/manifest-check.yml` (`--plugin-roots` flag, W6)
- Plugin manifests: `plugins/*/\.claude-plugin/plugin.json` (93 files)
- Depends on: ADR-031 (skill-driven CFA specialists) — skill bodies unchanged; paths migrated
- Extends: ADR-031 — multi-tier plugin discovery supersedes the single-root skill resolution
- Related: ADR-040 (RuVector reasoning bank) — distinct concern
- Related: ADR-042 (hybrid deterministic/LLM dispatch) — distinct concern; cookbook runtime
- Upstream reference: `github.com/anthropics/financial-services/tree/main/plugins`
