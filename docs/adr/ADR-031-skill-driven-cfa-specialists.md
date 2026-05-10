# ADR-031: Skill-Driven CFA Specialist Definitions

## Status: Accepted

## Date: 2026-05-10

## Deciders

- Robert Fall
- CFA Agent platform engineering

## Tags

`agents`, `skills`, `harness`, `prose-as-data`, `single-source-of-truth`, `top-level-await`

## Context

Through Phase 32, every CFA analyst persona (chief plus eight specialists)
existed as a TypeScript `AgentDef` constant — system prompt embedded as a
template literal, tool allow-list as a string array, model and token limits
as inline fields. The same prose was also being authored, separately, in
`.claude/skills/cfa/corp-finance-analyst-<id>/SKILL.md` for direct use by
Claude Code's skill ecosystem and for cross-tool consumption.

The duplication was actively harmful:

- Edits to a persona's prose required a TypeScript change, a recompile, and
  a parallel markdown edit — easy to forget one side.
- The `AgentDef` template literals required `\`` and `${...}` escapes that
  do not exist in the markdown source, so byte-equivalence was not
  obvious from inspection.
- The prose lived inside the harness package, opaque to non-TS tooling
  (managed-agent cookbooks, doc generators, audit pipelines) that wants
  to read the canonical persona text.
- The eight specialist files plus chief totalled ~2,500 LOC of prose
  embedded in TypeScript modules with no test value beyond the
  hand-authored content itself.

Phase 33 ran a four-wave migration:

- **Wave 1** built a provider-neutral `SkillLoader` with a minimal-correct
  YAML frontmatter parser, a markdown-body assembler, and a chief-analyst
  byte-equivalence canary against the existing TS source.
- **Wave 2** migrated the eight specialists to skill files and added eight
  more byte-equivalence canaries — one per specialist — that imported the
  TS source as ground-truth oracle.
- **Wave 3** added `createSkillRegistry()`, an async factory that produces
  the full nine-agent registry purely from skill files, and proved parity
  through that surface as well.
- **Wave 4** (this ADR) deletes the TS sources, switches `registry.ts` to
  load via `createSkillRegistry()` at module init, removes the now-circular
  canary tests, and locks the skill files in as canonical.

## Decision

### 1. Skill files are canonical

`.claude/skills/cfa/corp-finance-analyst-<id>/SKILL.md` is the single
source of truth for every CFA agent's system prompt and frontmatter
metadata (tools, model, max-tokens, max-recursion-depth). The thin
manifests at `.claude/agents/cfa/<id>.md` use `extends:` to reference the
skill and carry only the harness-specific frontmatter that is not part of
the skill itself.

### 2. Harness loads via top-level await

`packages/harness/src/agents/registry.ts` calls `createSkillRegistry()` at
module init using top-level await. The harness is pure ESM (`"type":
"module"`), vitest supports top-level await out of the box, and the
loader resolves all nine agents in parallel via `Promise.all`. The
`registry`, `chiefAnalyst`, `defaultDelegates`, `getAgent`, and
`defaultMCPServers` exports are preserved bit-for-bit so existing
consumers (CLI, agent tests, specialist routing tests) keep working with
no changes.

### 3. Byte-equivalence proven before deletion

The nine `*-skill-canary.test.ts` files held the deletion gate. Each one
loaded its skill, normalised whitespace per a fixed rule, and compared
against the TypeScript constant. All nine passed before we deleted the TS
sources; with the sources gone the canaries become circular (skill loaded
twice, comparing itself) and were removed in this wave. The
`skill-registry.test.ts` Test 2 was rewritten as a structural shape
assertion for the same reason.

### 4. We did NOT use the `cfa managed-agent deploy` CLI

The CLI pathway in the external `corp-finance-core` repo is shaped for
managed-agent cookbook payloads — composite orchestrators and
multi-subagent bundles — not single `AgentDef` consumption inside a
harness. It also lives outside this monorepo, which would force a
cross-repo build dependency for a per-load operation. The direct skill
loader in `packages/harness/src/skills/` is the correct primitive at this
layer. Cross-repo cookbook integration is deferred to a future phase
that warrants a managed-agent surface in the harness.

## Alternatives Considered

**Dual-track (keep both TS and skills)** — Rejected. Two sources of truth
for the same prose is exactly the harm the migration was undoing.

**Env-flag fallback to TS at runtime** — Rejected. Indefinitely deferring
a decision is not a strategy. The canary tests gave us a hard parity gate;
either parity holds and we delete the legacy, or it doesn't and we don't
ship the migration.

**Use the `cfa managed-agent deploy` CLI as the loader** — Deferred. See
Decision §4.

**Lazy load (synchronous-first, defer skill loading)** — Rejected. The
nine-agent registry is small, the skill files are local, and load time is
single-digit milliseconds. Top-level await keeps the surface synchronous
to consumers and avoids the complication of a deferred-resolution pattern.

## Consequences

### Positive

- One place to author analyst prose: the markdown skill file.
- ~2,500 LOC of duplicated content removed (TS sources + canary tests).
- The skill loader is provider-neutral; non-harness tooling (cookbooks,
  doc generators, audit middleware) can read the same canonical text.
- Editing prose no longer triggers a TypeScript rebuild.
- The harness no longer carries persona-specific TypeScript modules; new
  specialists are added by dropping a skill file and a manifest.

### Negative

- Module load is async (top-level await). Consumers that import the
  registry at the top of their module-graph wait for the loader to finish
  before any code runs. In practice this is sub-100ms and invisible.
- Editing prose now requires a markdown edit instead of a TS edit; the
  loader-side YAML parser must stay minimal-correct.
- A malformed skill file fails the `createSkillRegistry()` strict-startup
  contract and crashes the process at import time. This is intended
  (loud over silent) but makes for a worse failure mode than a typo in a
  TypeScript file (which surfaces at compile time).

### Neutral

- The `defaultMCPServers` export is unrelated to skill loading and
  remains in `registry.ts` unchanged.
- The `extends:` indirection from `.claude/agents/cfa/<id>.md` to the
  skill is preserved as the official agent-manifest convention; nothing
  in this ADR changes how Claude Code itself discovers the agents.

## Future Work

- A managed-agent cookbook publisher that consumes the same skill files
  and emits payloads for the Anthropic Managed Agents API.
- A doc-gen pipeline that lifts the SKILL.md frontmatter into a
  human-readable persona catalogue.
- Move the loader-side YAML parser to a shared package if a second
  consumer beyond the harness appears.

## Links

- Depends on: ADR-026 (Plugin/Packages Dual-Mode and Surface Parity) —
  surface-parity discipline that motivated keeping the registry surface
  bit-for-bit identical across the migration.
- Skill files: `.claude/skills/cfa/corp-finance-analyst-<id>/SKILL.md`
- Agent manifests: `.claude/agents/cfa/<id>-analyst.md`
- Loader: `packages/harness/src/skills/`
- Registry: `packages/harness/src/agents/skill-registry.ts`,
  `packages/harness/src/agents/registry.ts`
