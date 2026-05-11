# Phase 33 — Skill-Driven Harness

**Status:** Planning
**Date:** 2026-05-10
**Owner:** Robert Fall
**Predecessors:** Phase 32 Waves 1-2 (lockfile guard + ToolCatalogValidator + MCP helpers + FileJsonStore + shared specialist prompt). Phase 31 (CFA Harness Waves 1-4).

## Why Phase 33

Phase 31 produced a working harness with 9 code-defined specialists. Phase 32 Waves 1-2 consolidated the worst duplication. But a deeper problem remains:

**The harness re-implements machinery that already exists.** The CFA stack has four mature surfaces (skills, MCP plugins, plugin manifests, the `cfa` CLI). The Phase 31 harness uses MCP plugins correctly and bypasses the other three — system prompts are baked into TypeScript specialist files, the `cfa managed-agent` loader is ignored, and skills are documentation-only.

The result: ~2,300 lines of governance prose in 8 specialist `.ts` files, no shared loader between Claude Code (interactive) and the harness (programmatic), and any agent persona update requires a TypeScript code release.

Phase 33 replaces that with a **skill-driven architecture** where each surface owns its proper concern, and the harness consumes the CLI's existing manifest output instead of duplicating it.

## The four surfaces and their proper roles

| Surface | Owns | Already correct? |
|---|---|---|
| **Skills** (`.claude/skills/<id>/SKILL.md`) | Domain prose, governance rules, output standards, examples — the PERSONA's knowledge | Underused. Today: documentation-only for the harness. Phase 33: source of truth. |
| **MCP plugins** (cfa-core, cfa-data, cfa-pro) | The 623 callable tools (compute, data, vendor) | Correct. No change. |
| **Plugin manifests + marketplace** (`.claude-plugin/`) | Distribution: which MCPs ship together, which skills bundle with which plugin | Correct. Phase 32 Wave 5 (later) tidies symlinks. |
| **`cfa` CLI** (`cfa managed-agent deploy/validate/list`) | The loader: reads skill+agent+cookbook manifests, validates references, assembles deploy payloads | Already does this. Harness ignores it today. Phase 33: harness becomes a consumer. |
| **Harness runtime** (Phase 31) | Messages API dispatch, tool routing, audit, sessions, providers, transports | Correct. No change to `agent-loop.ts`, providers, MCP transports. |

## Target architecture

```
┌───────────────────────────────────────────────────────────────────┐
│  AUTHORING / DISTRIBUTION (the four mature surfaces)              │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  .claude/skills/cfa/                                              │
│    corp-finance-analyst-shared/SKILL.md      ← governance prose   │
│    corp-finance-analyst-derivatives/SKILL.md ← derivatives prose  │
│    corp-finance-analyst-equity/SKILL.md      ← equity prose       │
│    ... (1 shared + 9 per-agent skills)                            │
│                                                                   │
│  .claude/agents/cfa/                                              │
│    chief-analyst.md                          ← lightweight        │
│    derivatives-analyst.md                       agent manifest    │
│    equity-analyst.md                            (skill refs +     │
│    ...                                          tool allowlist)   │
│                                                                   │
│  managed-agent-cookbooks/                                         │
│    derivatives/agent.json                    ← cookbook (         │
│    equity/agent.json                            orchestration     │
│    ...                                          metadata)         │
│                                                                   │
│  plugins/cfa-core/.claude-plugin/plugin.json ← bundles MCP +      │
│  plugins/cfa-data/...                            skills + agents  │
│  plugins/cfa-pro/...                             into one         │
│                                                  shippable plugin │
└────────────────┬──────────────────────────────────────────────────┘
                 │
                 │  cfa managed-agent deploy <slug> --json
                 │     (or direct manifest read)
                 ▼
┌───────────────────────────────────────────────────────────────────┐
│  CFA HARNESS (Phase 31, plus Phase 33 skill loader)               │
├───────────────────────────────────────────────────────────────────┤
│                                                                   │
│  packages/harness/src/                                            │
│    skills/                              ← NEW (Phase 33)          │
│      loader.ts                            parse SKILL.md, resolve │
│      cli-loader.ts                        skill refs via cfa CLI  │
│      direct-loader.ts                     OR direct fs read       │
│                                                                   │
│    agents/registry.ts                   ← THIN (Phase 33)         │
│      loadAgent("derivatives") →                                   │
│        AgentDef built from manifest +                             │
│        skill bodies                                               │
│                                                                   │
│    agents/specialists/                  ← MOSTLY DELETED          │
│      (the 8 long .ts files become                                 │
│       1-line registry entries)                                    │
│                                                                   │
│    core/agent-loop.ts                   ← UNCHANGED (Phase 31)    │
│    core/providers/                      ← UNCHANGED               │
│    mcp-client/                          ← UNCHANGED               │
│    audit/, memory/, persistence/        ← UNCHANGED               │
└───────────────────────────────────────────────────────────────────┘
```

## What Phase 33 changes

1. **New `packages/harness/src/skills/`** — a small loader (~200 LOC) that:
   - Parses YAML frontmatter from `SKILL.md` files (`name`, `description`, `triggers`, `model`, `tools`, `references`).
   - Resolves skill references (`extends: corp-finance-analyst-shared`) by reading parent skill bodies.
   - Composes a final assembled system prompt: shared prose + specialist prose + closing footer.
   - Two loader strategies, both behind the same interface:
     - `cliLoader`: shells out to `cfa managed-agent deploy <slug> --json` (parses CLI output).
     - `directLoader`: reads `.claude/skills/`, `.claude/agents/`, `managed-agent-cookbooks/` directly (works without the CLI installed).
2. **`packages/harness/src/agents/registry.ts` becomes a thin lookup** — `getAgent(id)` calls the loader and caches results. No more 800-line specialist files.
3. **`packages/harness/src/agents/specialists/*.ts` are deleted** (all 8). Replaced by `.claude/skills/cfa/corp-finance-analyst-<domain>/SKILL.md`.
4. **`packages/harness/src/agents/specialists/shared-prompt.ts` (Wave 2C output) migrates verbatim** into `.claude/skills/cfa/corp-finance-analyst-shared/SKILL.md`.
5. **The 8 cfa specialist `.md` agent files** in `.claude/agents/cfa/` are updated to reference the new skills (most already do; minor cleanup).
6. **Existing tests still pass** — the public `getAgent("derivatives")` API is preserved; only the loading mechanism changes.

## What Phase 33 does NOT change

- `packages/harness/src/core/agent-loop.ts` — the dispatch loop is unchanged.
- `packages/harness/src/core/providers/{anthropic,openai,gemini}.ts` — provider adapters unchanged.
- `packages/harness/src/mcp-client/` — transports unchanged.
- `packages/harness/src/audit/`, `memory/`, `persistence/`, `security/` — unchanged.
- The 4 plugin MCP servers — unchanged. The 623 tool surface stays exactly as it is.
- The plugin marketplace and `.claude-plugin/` manifests — unchanged.
- The `cfa` CLI — unchanged. Phase 33 consumes its existing output; no CLI feature work needed.

## Wave plan

### Wave 1 — Skill loader + chief migration (3 days)

1. New `src/skills/{loader,frontmatter-parser,assembler}.ts`. Parses `SKILL.md`, resolves `extends:` refs, assembles final system prompt.
2. New `src/agents/registry.ts` lookup that returns assembled `AgentDef`s. Cache hits are O(1); cold loads do filesystem work.
3. Migrate **chief-analyst** first as the canary. Create `.claude/skills/cfa/corp-finance-analyst-chief/SKILL.md` with the existing chief prose. Delete `chief-analyst.ts`. Run the live Chaco acceptance test against the loader-built chief — assert `tool_uses ≥ 21`.
4. Wave 1 acceptance: chief dispatches identically to its TypeScript-defined version.

### Wave 2 — Specialist migration (2 days)

5. Migrate the 8 specialists in parallel via swarm: 8 agents, each authoring one `corp-finance-analyst-<domain>/SKILL.md` from the existing `.ts` system prompt + the shared-prompt fragments.
6. Delete `src/agents/specialists/{equity,credit,fixed-income,derivatives,quant-risk,macro,private-markets,esg-regulatory}.ts`.
7. `src/agents/specialists/shared-prompt.ts` content migrates into `corp-finance-analyst-shared/SKILL.md` and the file is deleted.
8. Wave 2 acceptance: live Wave 2 routing test (Chaco → derivatives + macro) still passes with `total tool_uses ≥ 21`.

### Wave 3 — CLI loader integration (1 day)

9. Implement `cliLoader` (shell out to `cfa managed-agent deploy <slug> --json`).
10. Add a `--loader` flag to the harness CLI (`direct` default, `cli` opt-in).
11. Validate parity: `directLoader.load("derivatives")` and `cliLoader.load("derivatives")` produce equivalent `AgentDef`s.

### Wave 4 — Cleanup + docs (1 day)

12. Delete `packages/harness/src/agents/specialists/` directory entirely.
13. Update Phase 31 DDD doc to reflect skill-driven contexts.
14. Write ADR-038 (skill-driven specialist model) and ADR-039 (CLI loader integration).
15. Update PRD F4-F7 to reflect skill-loaded agents.

**Total Phase 33: ~7 days, ~200 LOC added (skill loader), ~2,300 LOC removed (8 specialist `.ts` files), net ~−2,100 LOC.**

## Success metrics

| Metric | Pre-Phase-33 | Target |
|---|---|---|
| Specialist source LOC (8 files) | ~2,300 | 0 (deleted) |
| `shared-prompt.ts` (TS) | 78 | 0 (migrated to skill) |
| Skill files for specialists | 0 | 9 (8 specialists + 1 shared) |
| Lines per specialist (avg) | ~290 | ~50 (skill file) |
| Single source of truth (skill works in Claude Code AND harness) | no | yes |
| Persona update without code release | no | yes |
| Live acceptance tests passing | yes (Phase 31) | yes (post-migration) |

## Risk register

| Risk | Mitigation |
|---|---|
| Skill loader produces different system prompt than the TS-defined specialist | Wave 1 canary on chief: byte-compare the assembled prompt against the current `chief-analyst.ts` prompt before deleting the TS file. |
| `cfa` CLI version drift between consumers | Pin to the v1.1.0 tag (already on crates.io); cliLoader prints the resolved cfa version on startup. |
| `.claude/skills/cfa/*` reorganisation breaks Claude Code interactive use | Skills naming follows existing convention (`corp-finance-analyst-*`); existing skills under that namespace continue working; new ones add to it. |
| Specialist tool allowlists drift between agent manifest and skill | The skill file's `tools:` frontmatter and the agent manifest's `tools:` field are validated against each other at load time; mismatches throw at startup (ToolCatalogValidator pattern from Phase 32 Wave 1). |
| Loading skills synchronously slows dispatch startup | Loader caches per-process; cold load is a one-time fs walk (~10ms for 9 skills). The Anthropic API call dominates dispatch latency by 1000×. |

## Open questions

1. **Should the harness use the CLI loader by default or the direct loader?** Direct is simpler, no subprocess; CLI is more deterministic (one loader for all callers including future lexius / regfin). Wave 3 ships both behind a flag; pick a default after observing behaviour for a week.
2. **Do we update the existing `.claude/agents/cfa/*.md` files** to be Phase-33 canonical or leave them as legacy? Recommend canonical — the manifest format already supports skill references; minor cleanup makes them the source of truth.
3. **Do we publish the harness's skill loader to npm** so other consumers (lexius, regfin) can import it? Defer to Phase 34 if there's demand.

## Cross-references

- Phase 32 Wave 2 PR (currently open): `phase-32-wave-2-impl` → main; the `shared-prompt.ts` content from Wave 2C migrates into `corp-finance-analyst-shared/SKILL.md` in Phase 33 Wave 2.
- Phase 31 DDD: `docs/ddd/phase-31-harness.md` — the 6 bounded contexts. Phase 33 doesn't add a new context but reshapes the Agent Registry context to be skill-driven instead of code-defined.
- Phase 32 architecture plan: `docs/plans/phase-32-architecture.md` — the original consolidation plan. Phase 33 supersedes Wave 3+ of that plan (the hex layer extraction becomes much simpler once specialists are 50-line skills instead of 800-line TS files).
- ADR-037 (Agent Registry as Code): Phase 33 partly **revises** ADR-037 — the registry stays code, but agent **definitions** move to skills. ADR-038 will document the revision.
- The `cfa managed-agent` CLI: lives in the extracted corp-finance-core repo (https://github.com/rob-otix-ai/corp-finance-core), accessible via `cargo install --git ... corp-finance-cli --tag v1.1.0`.

## Recommendation

Land Phase 32 Wave 2 first (PR #41 currently open); merge to main; then start Phase 33 Wave 1 (skill loader + chief canary) immediately. Wave 1 alone proves the architecture; Waves 2-4 are mechanical migrations once Wave 1's loader works.
